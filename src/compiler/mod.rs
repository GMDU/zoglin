use std::collections::HashSet;
use std::mem::take;
use std::{collections::HashMap, path::Path};

use expression::{verify_types, ConditionKind, Expression, ExpressionKind, NbtValue};
use file_tree::{ResourceLocation, ScoreboardLocation, StorageLocation};
use scope::FunctionDefinition;
use serde::Serialize;

use crate::parser::ast::{
  self, ArrayType, Command, ElseStatement, File, FunctionCall, IfStatement, Index, KeyValue,
  Member, ParameterKind, RangeIndex, ReturnType, Statement, StaticExpr, WhileLoop, ZoglinResource,
};

use crate::error::{raise_error, raise_floating_error, Location, Result};

use self::{
  file_tree::{FileResource, FileTree, Function, Item, Namespace, TextResource},
  scope::Scope,
};
mod binary_operation;
mod expression;
mod file_tree;
mod internals;
mod register;
mod scope;

#[derive(Default)]
pub struct Compiler {
  tick_functions: Vec<String>,
  load_functions: Vec<String>,
  scopes: Vec<Scope>,
  comptime_scopes: Vec<HashMap<String, Expression>>,
  current_scope: usize,
  counters: HashMap<String, usize>,
  namespaces: HashMap<String, Namespace>,
  used_scoreboards: HashSet<String>,
  function_registry: HashMap<ResourceLocation, FunctionDefinition>,
}

#[derive(Clone)]
struct FunctionContext {
  location: ResourceLocation,
  return_type: ReturnType,
  is_nested: bool,
  has_nested_returns: bool,
  code: Vec<String>,
}

#[derive(Serialize)]
struct FunctionTag<'a> {
  values: &'a [String],
}

impl Compiler {
  fn push_scope(&mut self, name: String, parent: usize) -> usize {
    self.scopes.push(Scope::new(parent));
    let index = self.scopes.len() - 1;
    self.scopes[parent].add_child(name, index);
    index
  }

  fn enter_scope(&mut self, name: &String) {
    self.current_scope = self.scopes[self.current_scope]
      .get_child(name)
      .expect("Child has already been added");
  }

  fn exit_scope(&mut self) {
    self.current_scope = self.scopes[self.current_scope].parent;
  }

  fn add_function(&mut self, scope: usize, name: String, location: ResourceLocation) {
    self.scopes[scope].function_registry.insert(name, location);
  }

  fn lookup_resource(&self, resource: &ZoglinResource) -> Option<ResourceLocation> {
    if resource.namespace.is_some() {
      return None;
    }

    let first = resource.modules.first().unwrap_or(&resource.name);
    let valid_function = resource.modules.is_empty();
    let mut index = self.current_scope;

    while index != 0 {
      let scope = &self.scopes[index];
      if valid_function {
        if let Some(function_definition) = scope.function_registry.get(first) {
          return Some(function_definition.clone());
        }
      }
      if let Some(resource_location) = scope.imported_items.get(first) {
        return Some(resource_location.clone());
      }

      index = scope.parent;
    }
    None
  }

  fn lookup_comptime(&self, name: &str) -> Option<Expression> {
    for scope in self.comptime_scopes.iter().rev() {
      if let Some(value) = scope.get(name) {
        return Some(value.clone());
      }
    }

    let mut index = self.current_scope;

    while index != 0 {
      let scope = &self.scopes[index];
      if let Some(value) = scope.comptime_values.get(name) {
        return Some(value.clone());
      }

      index = scope.parent;
    }

    None
  }

  fn get_location(&mut self, location: ResourceLocation) -> &mut Vec<Item> {
    if !self.namespaces.contains_key(&location.namespace) {
      self.namespaces.insert(
        location.namespace.clone(),
        Namespace {
          name: location.namespace.clone(),
          items: Vec::new(),
        },
      );
    }

    let namespace = self
      .namespaces
      .get_mut(&location.namespace)
      .expect("Namespace has been inserted");
    namespace.get_module(location.modules)
  }

  fn add_import(&mut self, scope: usize, name: String, location: ResourceLocation) {
    self.scopes[scope].imported_items.insert(name, location);
  }

  fn add_item(&mut self, location: ResourceLocation, item: Item) -> Result<()> {
    let items = self.get_location(location);
    for i in items.iter() {
      match (i, &item) {
        (
          Item::Function(Function { name: name1, .. }),
          Item::Function(Function {
            name: name2,
            location,
            ..
          }),
        ) if name1 == name2 => {
          return Err(raise_error(
            location.clone(),
            format!("Function \"{name2}\" is already defined."),
          ));
        }
        (Item::TextResource(res1), Item::TextResource(res2)) if res1 == res2 => {
          return Err(raise_error(
            res2.location.clone(),
            format!(
              "{}{} \"{}\" is already defined.",
              res2
                .kind
                .chars()
                .nth(0)
                .expect("Identifiers can't be empty")
                .to_uppercase(),
              &res2.kind[1..],
              res2.name
            ),
          ));
        }
        _ => {}
      }
    }

    items.push(item);
    Ok(())
  }

  fn next_counter(&mut self, counter_name: &str) -> usize {
    if let Some(counter) = self.counters.get_mut(counter_name) {
      *counter += 1;
      return *counter;
    };

    self.counters.insert(counter_name.to_string(), 0);
    0
  }

  fn next_scoreboard(&mut self, namespace: &str) -> ScoreboardLocation {
    self
      .used_scoreboards
      .insert(format!("zoglin.internal.{namespace}.vars"));
    ScoreboardLocation {
      scoreboard: ResourceLocation::new_function("zoglin", &["internal", namespace, "vars"]),
      name: format!("$var_{}", self.next_counter("scoreboard")),
    }
  }

  fn next_storage(&mut self, namespace: &str) -> StorageLocation {
    StorageLocation::new(
      ResourceLocation::new_function("zoglin", &["internal", namespace, "vars"]),
      format!("var_{}", self.next_counter("storage")),
    )
  }

  fn next_function(&mut self, function_type: &str, namespace: &str) -> ResourceLocation {
    ResourceLocation::new_function(
      "zoglin",
      &[
        "generated",
        namespace,
        function_type,
        &format!(
          "fn_{}",
          self.next_counter(&format!("function:{}", function_type))
        ),
      ],
    )
  }
}

impl Compiler {
  pub fn compile(mut ast: File, output: &String) -> Result<()> {
    let mut compiler = Compiler::default();

    compiler.register(&mut ast);
    let tree = compiler.compile_tree(ast)?;
    tree.generate(output)?;
    Ok(())
  }

  fn compile_tree(&mut self, ast: File) -> Result<FileTree> {
    for namespace in ast.items {
      self.compile_namespace(namespace)?;
    }

    let load_json = FunctionTag {
      values: &self.load_functions,
    };

    let load_text = serde_json::to_string_pretty(&load_json).expect("Json is valid");

    let load = Item::TextResource(TextResource {
      name: "load".to_string(),
      kind: "tags/function".to_string(),
      is_asset: false,
      text: load_text,
      location: Location::blank(),
    });

    let location = ResourceLocation::new_module("minecraft", &[]);
    self.add_item(location.clone(), load)?;

    if !self.tick_functions.is_empty() {
      let tick_json = FunctionTag {
        values: &self.tick_functions,
      };
      let tick_text = serde_json::to_string_pretty(&tick_json).expect("Json is valid");

      let tick: Item = Item::TextResource(TextResource {
        name: "tick".to_string(),
        kind: "tags/function".to_string(),
        is_asset: false,
        text: tick_text,
        location: Location::blank(),
      });
      self.add_item(location, tick)?;
    }

    let namespaces = take(&mut self.namespaces);
    Ok(FileTree {
      namespaces: namespaces.into_values().collect(),
    })
  }

  fn compile_namespace(&mut self, namespace: ast::Namespace) -> Result<()> {
    self
      .load_functions
      .insert(0, format!("zoglin:generated/{}/load", namespace.name));

    self.enter_scope(&namespace.name);
    self.comptime_scopes.push(HashMap::new());

    let resource = ResourceLocation::new_module(&namespace.name, &[]);

    for item in namespace.items {
      self.compile_item(item, &resource)?;
    }

    self.exit_scope();
    self.comptime_scopes.pop();

    let load_function = Item::Function(Function {
      name: "load".to_string(),
      commands: take(&mut self.used_scoreboards)
        .into_iter()
        .map(|scoreboard| format!("scoreboard objectives add {scoreboard} dummy"))
        .collect(),
      location: Location::blank(),
    });
    self.add_item(
      ResourceLocation::new_module("zoglin", &["generated", &namespace.name]),
      load_function,
    )?;

    Ok(())
  }

  fn compile_item(&mut self, item: ast::Item, location: &ResourceLocation) -> Result<()> {
    match item {
      ast::Item::Module(module) => self.compile_module(module, location.clone()),
      ast::Item::Import(_) => Ok(()),
      ast::Item::Function(function) => self.compile_ast_function(function, location),
      ast::Item::Resource(resource) => self.compile_resource(resource, location),
      ast::Item::ComptimeAssignment(_, _) => Ok(()),
      ast::Item::None => Ok(()),
    }
  }

  fn compile_module(&mut self, module: ast::Module, mut location: ResourceLocation) -> Result<()> {
    self.enter_scope(&module.name);
    self.comptime_scopes.push(HashMap::new());

    location.modules.push(module.name);

    for item in module.items {
      self.compile_item(item, &location)?;
    }

    self.exit_scope();
    self.comptime_scopes.pop();
    Ok(())
  }

  fn compile_resource(
    &mut self,
    resource: ast::Resource,
    location: &ResourceLocation,
  ) -> Result<()> {
    match resource.content {
      ast::ResourceContent::Text(name, text) => {
        let resource = TextResource {
          kind: resource.kind,
          name,
          is_asset: resource.is_asset,
          location: resource.location,
          text,
        };
        self.add_item(location.clone(), Item::TextResource(resource))
      }
      ast::ResourceContent::File(path, file) => {
        let file_path = Path::new(&file)
          .parent()
          .expect("Directory must have a parent");
        let resource = FileResource {
          kind: resource.kind,
          is_asset: resource.is_asset,
          path: file_path
            .join(path)
            .to_str()
            .expect("Path must be valid")
            .to_string(),
          location: resource.location,
        };
        self.add_item(location.clone(), Item::FileResource(resource))
      }
    }
  }

  fn compile_statement(
    &mut self,
    statement: Statement,
    context: &mut FunctionContext,
  ) -> Result<()> {
    match statement {
      Statement::Command(command) => {
        let result = self.compile_command(command, context)?;
        context.code.push(result);
      }
      Statement::Comment(comment) => {
        context.code.push(comment);
      }
      Statement::Expression(expression) => {
        self.compile_expression(expression, context, true)?;
      }
      Statement::IfStatement(if_statement) => {
        let mut sub_context = context.clone();
        sub_context.has_nested_returns = false;
        self.comptime_scopes.push(HashMap::new());
        self.compile_if_statement(if_statement, &mut sub_context)?;
        context.code = sub_context.code;
        if sub_context.has_nested_returns {
          context.has_nested_returns = true;
          self.generate_nested_return(context);
        }
        self.comptime_scopes.pop();
      }
      Statement::WhileLoop(while_loop) => {
        let mut sub_context: FunctionContext = context.clone();
        sub_context.has_nested_returns = false;
        self.comptime_scopes.push(HashMap::new());
        self.compile_while_loop(while_loop, &mut sub_context)?;
        context.code = sub_context.code;
        if sub_context.has_nested_returns {
          context.has_nested_returns = true;
          self.generate_nested_return(context);
        }
        self.comptime_scopes.pop();
      }
      Statement::Return(value) => self.compile_return(value, context)?,
    }
    Ok(())
  }

  fn generate_nested_return(&mut self, context: &mut FunctionContext) {
    let return_command = match context.return_type {
      ReturnType::Storage | ReturnType::Scoreboard => {
        "return run scoreboard players reset $should_return"
      }
      ReturnType::Direct => &format!("return run function {}", self.reset_direct_return()),
    };
    context. code.push(format!("execute if score $should_return zoglin.internal.vars matches -2147483648..2147483647 run {return_command}"));
  }

  fn compile_ast_function(
    &mut self,
    function: ast::Function,
    location: &ResourceLocation,
  ) -> Result<()> {
    let fn_location = location.clone().with_name(&function.name);
    let mut context = FunctionContext {
      location: fn_location,
      return_type: function.return_type,
      is_nested: false,
      has_nested_returns: false,
      code: Vec::new(),
    };
    self.comptime_scopes.push(HashMap::new());

    self.compile_block(&mut context, function.items)?;
    self.comptime_scopes.pop();
    self.add_function_item(function.location, context.location, context.code)
  }

  fn add_function_item(
    &mut self,
    location: Location,
    fn_location: ResourceLocation,
    commands: Vec<String>,
  ) -> Result<()> {
    let (module, name) = fn_location.try_split().expect("Is a function location");
    let function = Function {
      name,
      location,
      commands,
    };

    self.add_item(module, Item::Function(function))
  }

  fn compile_block(&mut self, context: &mut FunctionContext, block: Vec<Statement>) -> Result<()> {
    for item in block {
      self.compile_statement(item, context)?;
    }
    Ok(())
  }

  fn compile_command(&mut self, command: Command, context: &mut FunctionContext) -> Result<String> {
    let mut result = String::new();
    let mut is_macro = false;
    let mut has_macro_prefix = false;

    for (i, part) in command.parts.into_iter().enumerate() {
      match part {
        ast::CommandPart::Literal(lit) => {
          if i == 0 && lit.starts_with('$') {
            has_macro_prefix = true;
          }

          result.push_str(&lit)
        }
        ast::CommandPart::Expression(expr) => {
          let (code, needs_macro) = self.compile_static_expr(expr, context)?;
          is_macro = is_macro || needs_macro;
          result.push_str(&code)
        }
      }
    }

    if is_macro && !has_macro_prefix {
      result.insert(0, '$');
    }

    Ok(result)
  }

  fn compile_expression(
    &mut self,
    expression: ast::Expression,
    context: &mut FunctionContext,
    ignored: bool,
  ) -> Result<Expression> {
    Ok(match expression {
      ast::Expression::FunctionCall(function_call) => {
        let location = function_call.path.location.clone();
        let (command, definition) = self.compile_function_call(function_call, context)?;
        match definition.return_type {
          ReturnType::Storage => {
            let storage = StorageLocation::new(definition.location, "return".to_string());
            if !ignored {
              context
                .code
                .push(format!("data modify storage {storage} set value false",))
            }
            context.code.push(command);
            Expression {
              location,
              kind: ExpressionKind::Storage(storage),
              needs_macro: false,
            }
          }
          ReturnType::Scoreboard => {
            let scoreboard = ScoreboardLocation::new(definition.location, "return");
            if !ignored {
              context
                .code
                .push(format!("scoreboard players set {scoreboard} 0",))
            }
            context.code.push(command);
            Expression {
              location,
              kind: ExpressionKind::Scoreboard(scoreboard),
              needs_macro: false,
            }
          }
          ReturnType::Direct => {
            let scoreboard = self.next_scoreboard(&context.location.namespace);
            context.code.push(format!(
              "execute store result score {scoreboard} run {command}",
            ));
            Expression {
              location,
              kind: ExpressionKind::Scoreboard(scoreboard),
              needs_macro: false,
            }
          }
        }
      }
      ast::Expression::Byte(b, location) => Expression::new(ExpressionKind::Byte(b), location),
      ast::Expression::Short(s, location) => Expression::new(ExpressionKind::Short(s), location),
      ast::Expression::Integer(i, location) => {
        Expression::new(ExpressionKind::Integer(i), location)
      }
      ast::Expression::Long(l, location) => Expression::new(ExpressionKind::Long(l), location),
      ast::Expression::Float(f, location) => Expression::new(ExpressionKind::Float(f), location),
      ast::Expression::Double(d, location) => Expression::new(ExpressionKind::Double(d), location),
      ast::Expression::Boolean(b, location) => {
        Expression::new(ExpressionKind::Boolean(b), location)
      }
      ast::Expression::String(s, location) => Expression::new(ExpressionKind::String(s), location),
      ast::Expression::Array(typ, a, location) => self.compile_array(typ, a, location, context)?,
      ast::Expression::Compound(key_values, location) => {
        self.compile_compound(key_values, location, context)?
      }
      ast::Expression::Variable(variable) => Expression::new(
        ExpressionKind::Storage(StorageLocation::from_zoglin_resource(
          &context.location,
          &variable,
        )),
        variable.location,
      ),
      ast::Expression::ScoreboardVariable(variable) => Expression::new(
        ExpressionKind::Scoreboard(ScoreboardLocation::from_zoglin_resource(
          &context.location,
          &variable,
        )),
        variable.location,
      ),
      ast::Expression::MacroVariable(name, location) => Expression::with_macro(
        ExpressionKind::Macro(StorageLocation::new(
          context.location.clone(),
          format!("__{name}"),
        )),
        location,
        true,
      ),
      ast::Expression::ComptimeVariable(name, location) => {
        if let Some(value) = self.lookup_comptime(&name) {
          return Ok(value.clone());
        } else {
          return Err(raise_error(
            location,
            format!("The compile-time variable {name} is not in scope."),
          ));
        }
      }
      ast::Expression::BinaryOperation(binary_operation) => {
        self.compile_binary_operation(binary_operation, context)?
      }
      ast::Expression::UnaryExpression(unary_expression) => {
        self.compile_unary_expression(unary_expression, context)?
      }
      ast::Expression::Index(index) => self.compile_index(index, context)?,
      ast::Expression::RangeIndex(index) => self.compile_range_index(index, context)?,
      ast::Expression::Member(member) => self.compile_member(member, context)?,
    })
  }

  fn compile_array(
    &mut self,
    typ: ArrayType,
    expressions: Vec<ast::Expression>,
    location: Location,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    let mut types = Vec::new();

    for expr in expressions {
      types.push(self.compile_expression(expr, context, false)?);
    }

    let err_msg = match typ {
      ArrayType::Any => "Arrays can only contain values of the same type",
      ArrayType::Byte => "Byte arrays can only byte values",
      ArrayType::Int => "Int arrays can only integer values",
      ArrayType::Long => "Long arrays can only long values",
    };
    let data_type = verify_types(&types, typ, err_msg)?;

    let kind = match typ {
      ArrayType::Any => ExpressionKind::Array {
        values: types,
        data_type,
      },
      ArrayType::Byte => ExpressionKind::ByteArray(types),
      ArrayType::Int => ExpressionKind::IntArray(types),
      ArrayType::Long => ExpressionKind::LongArray(types),
    };

    Ok(Expression::new(kind, location))
  }

  fn compile_compound(
    &mut self,
    key_values: Vec<KeyValue>,
    location: Location,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    let mut types = HashMap::new();

    for KeyValue {
      key,
      value,
      location,
    } in key_values
    {
      if types
        .insert(key, self.compile_expression(value, context, false)?)
        .is_some()
      {
        return Err(raise_error(location, "Duplicate keys not allowed"));
      }
    }

    Ok(Expression::new(ExpressionKind::Compound(types), location))
  }

  // Returns whether the expression requires a macro command
  fn compile_static_expr(
    &mut self,
    expr: StaticExpr,
    context: &mut FunctionContext,
  ) -> Result<(String, bool)> {
    match expr {
      StaticExpr::FunctionCall(call) => Ok((self.compile_function_call(call, context)?.0, false)),
      StaticExpr::FunctionRef { path } => Ok((
        if let Some(path) = path {
          self
            .resolve_zoglin_resource(path, &context.location.clone().module())?
            .to_string()
        } else {
          context.location.to_string()
        },
        false,
      )),
      StaticExpr::MacroVariable(name) => Ok((format!("$(__{name})"), true)),
      StaticExpr::ComptimeVariable(name) => {
        if let Some(value) = self.lookup_comptime(&name) {
          value
            .kind
            .to_comptime_string(true)
            .ok_or(raise_floating_error(
              // TODO: Add location
              "This value cannot be statically resolved.",
            ))
            .map(|value| (value, false))
        } else {
          Err(raise_floating_error(
            // TODO: Add a location here
            format!("The compile-time variable {name} is not in scope."),
          ))
        }
      }

      StaticExpr::ResourceRef { resource } => Ok((
        ResourceLocation::from_zoglin_resource(
          &context.location.clone().module(),
          &resource,
          false,
        )
        .to_string(),
        false,
      )),
    }
  }

  fn compile_function_call(
    &mut self,
    function_call: FunctionCall,
    context: &mut FunctionContext,
  ) -> Result<(String, FunctionDefinition)> {
    let src_location = function_call.path.location.clone();

    let path =
      self.resolve_zoglin_resource(function_call.path, &context.location.clone().module())?;
    let mut function_definition =
      if let Some(function_definition) = self.function_registry.get(&path) {
        function_definition.clone()
      } else {
        FunctionDefinition {
          location: path.clone(),
          arguments: Vec::new(),
          return_type: ReturnType::Direct,
        }
      };

    if function_call.arguments.len() != function_definition.arguments.len() {
      return Err(raise_error(
        src_location,
        format!(
          "Incorrect number of arguments. Expected {}, got {}",
          function_definition.arguments.len(),
          function_call.arguments.len()
        ),
      ));
    }

    let has_macro_args = function_definition
      .arguments
      .iter()
      .any(|param| param.kind == ParameterKind::Macro);
    let parameter_storage = function_definition.location.clone();

    for (parameter, argument) in take(&mut function_definition.arguments)
      .into_iter()
      .zip(function_call.arguments)
    {
      let expr = self.compile_expression(argument, context, false)?;
      match parameter.kind {
        ParameterKind::Storage => {
          let storage = StorageLocation::new(parameter_storage.clone(), parameter.name);
          self.set_storage(
            &mut context.code,
            &storage,
            &expr,
            &context.location.namespace,
          )?;
        }
        ParameterKind::Scoreboard => {
          let scoreboard = ScoreboardLocation::new(parameter_storage.clone(), &parameter.name);
          self.set_scoreboard(&mut context.code, &scoreboard, &expr)?;
        }
        ParameterKind::Macro => {
          let storage =
            StorageLocation::new(parameter_storage.clone(), format!("__{}", parameter.name));
          self.set_storage(
            &mut context.code,
            &storage,
            &expr,
            &context.location.namespace,
          )?;
        }
        ParameterKind::CompileTime => todo!(),
      }
    }

    let command = if has_macro_args {
      format!(
        "function {} with storage {parameter_storage}",
        function_definition.location
      )
    } else {
      format!("function {}", function_definition.location)
    };
    Ok((command, function_definition))
  }

  fn resolve_zoglin_resource(
    &mut self,
    resource: ast::ZoglinResource,
    location: &ResourceLocation,
  ) -> Result<ResourceLocation> {
    let mut resource_location = ResourceLocation::new_module("", &[]);

    if let Some(namespace) = resource.namespace {
      if namespace.is_empty() {
        resource_location.namespace.clone_from(&location.namespace);
      } else if namespace == "~" {
        resource_location.namespace.clone_from(&location.namespace);
        resource_location
          .modules
          .extend(location.modules.iter().cloned());
      } else {
        resource_location.namespace = namespace;
      }
    } else if let Some(resolved) = self.lookup_resource(&resource) {
      let mut result = resolved;

      if resource.modules.len() > 1 {
        result.modules.extend_from_slice(&resource.modules[1..]);
      }
      if !resource.modules.is_empty() {
        result.modules.push(resource.name);
      }
      return Ok(result);
    } else {
      resource_location = location.clone();
    }

    resource_location.modules.extend(resource.modules);

    Ok(resource_location.with_name(&resource.name))
  }

  fn compile_if_statement(
    &mut self,
    if_statement: IfStatement,
    context: &mut FunctionContext,
  ) -> Result<()> {
    if if_statement.child.is_some() {
      let if_function = self.next_function("if", &context.location.namespace);

      context.code.push(format!("function {if_function}"));
      let mut sub_context = FunctionContext {
        location: context.location.clone(),
        return_type: context.return_type,
        is_nested: true,
        has_nested_returns: context.has_nested_returns,
        code: Vec::new(),
      };

      let mut if_statement = if_statement;
      loop {
        self.compile_if_statement_without_child(
          if_statement.condition,
          if_statement.block,
          &mut sub_context,
          true,
        )?;
        context.has_nested_returns = sub_context.has_nested_returns;
        match if_statement.child {
          Some(ElseStatement::IfStatement(if_stmt)) => {
            if_statement = *if_stmt;
          }

          Some(ElseStatement::Block(block)) => {
            self.compile_block(&mut sub_context, block)?;

            break;
          }

          None => break,
        }
      }

      let (module, name) = if_function.try_split().expect("Is a function");

      self.add_item(
        module,
        Item::Function(Function {
          name,
          commands: sub_context.code,
          location: Location::blank(),
        }),
      )?;

      return Ok(());
    }
    self.compile_if_statement_without_child(
      if_statement.condition,
      if_statement.block,
      context,
      false,
    )
  }

  fn compile_if_statement_without_child(
    &mut self,
    condition: ast::Expression,
    body: Vec<Statement>,
    context: &mut FunctionContext,
    is_child: bool,
  ) -> Result<()> {
    let condition = self.compile_expression(condition, context, false)?;

    let check_code =
      match condition.to_condition(self, &mut context.code, &context.location.namespace, false)? {
        ConditionKind::Known(false) => return Ok(()),
        ConditionKind::Known(true) => {
          self.compile_block(context, body)?;
          return Ok(());
        }
        ConditionKind::Check(check_code) => check_code,
      };

    let mut sub_context = FunctionContext {
      location: context.location.clone(),
      return_type: context.return_type,
      is_nested: true,
      has_nested_returns: context.has_nested_returns,
      code: Vec::new(),
    };
    self.compile_block(&mut sub_context, body)?;

    let command = match sub_context.code.len() {
      0 => return Ok(()),
      1 => &sub_context.code[0],
      _ => {
        let function = self.next_function("if", &context.location.namespace);
        let fn_str = function.to_string();
        self.add_function_item(Location::blank(), function, sub_context.code)?;
        &format!("function {fn_str}")
      }
    };

    context.code.push(format!(
      "execute {condition} {run_str} {command}",
      condition = check_code,
      run_str = if is_child { "run return run" } else { "run" },
    ));
    context.has_nested_returns = sub_context.has_nested_returns;
    Ok(())
  }

  fn compile_return(
    &mut self,
    value: Option<ast::Expression>,
    context: &mut FunctionContext,
  ) -> Result<()> {
    if context.is_nested {
      context.has_nested_returns = true;
    }

    let has_value = value.is_some();
    if let Some(value) = value {
      let expression = self.compile_expression(value, context, false)?;

      match context.return_type {
        ReturnType::Storage => {
          let return_storage = StorageLocation::new(context.location.clone(), "return".to_string());
          self.set_storage(
            &mut context.code,
            &return_storage,
            &expression,
            &context.location.namespace,
          )?;
        }
        ReturnType::Scoreboard => {
          let scoreboard = ScoreboardLocation::new(context.location.clone(), "return");
          self.used_scoreboards.insert(scoreboard.scoreboard_string());
          self.set_scoreboard(&mut context.code, &scoreboard, &expression)?;
        }
        ReturnType::Direct => {
          if context.is_nested {
            self.set_scoreboard(
              &mut context.code,
              &ScoreboardLocation::of_internal("should_return"),
              &expression,
            )?;
          } else {
            context.code.push(expression.to_return_command()?)
          }
        }
      }
    }

    if context.return_type != ReturnType::Direct && context.is_nested {
      self.set_scoreboard(
        &mut context.code,
        &ScoreboardLocation::of_internal("should_return"),
        &Expression::new(ExpressionKind::Integer(1), Location::blank()),
      )?;
    }

    if has_value {
      if context.return_type != ReturnType::Direct || context.is_nested {
        context.code.push("return 0".to_string())
      }
    } else {
      context.code.push("return fail".to_string());
    }

    Ok(())
  }

  fn compile_while_loop(
    &mut self,
    while_loop: WhileLoop,
    context: &mut FunctionContext,
  ) -> Result<()> {
    let mut sub_context = FunctionContext {
      location: context.location.clone(),
      return_type: context.return_type,
      is_nested: true,
      has_nested_returns: context.has_nested_returns,
      code: Vec::new(),
    };

    let condition = self.compile_expression(while_loop.condition, context, false)?;

    match condition.to_condition(
      self,
      &mut sub_context.code,
      &context.location.namespace,
      true,
    )? {
      ConditionKind::Known(false) => {}
      ConditionKind::Known(true) => {
        let fn_location = self.next_function("while", &context.location.namespace);

        self.compile_block(&mut sub_context, while_loop.block)?;

        sub_context.code.push(format!("function {fn_location}"));
        context.code.push(format!("function {fn_location}"));
        self.add_function_item(Location::blank(), fn_location, sub_context.code)?;
      }

      ConditionKind::Check(check_code) => {
        let fn_location = self.next_function("while", &context.location.namespace);
        context.code.push(format!("function {fn_location}"));
        sub_context
          .code
          .push(format!("execute {check_code} run return 0"));

        self.compile_block(&mut sub_context, while_loop.block)?;
        sub_context.code.push(format!("function {fn_location}"));
        self.add_function_item(Location::blank(), fn_location, sub_context.code)?;
      }
    }

    context.has_nested_returns = sub_context.has_nested_returns;

    Ok(())
  }

  fn compile_index(&mut self, index: Index, context: &mut FunctionContext) -> Result<Expression> {
    let location = index.left.location();
    let left = self.compile_expression(*index.left, context, false)?;
    let index = self.compile_expression(*index.index, context, false)?;

    match left.kind {
      ExpressionKind::Void
      | ExpressionKind::Byte(_)
      | ExpressionKind::Short(_)
      | ExpressionKind::Integer(_)
      | ExpressionKind::Long(_)
      | ExpressionKind::Float(_)
      | ExpressionKind::Double(_)
      | ExpressionKind::Boolean(_)
      | ExpressionKind::String(_)
      | ExpressionKind::SubString(_, _, _)
      | ExpressionKind::Compound(_)
      | ExpressionKind::Scoreboard(_)
      | ExpressionKind::Condition(_) => {
        Err(raise_error(left.location, "Can only index into arrays."))
      }

      ExpressionKind::ByteArray(values)
      | ExpressionKind::IntArray(values)
      | ExpressionKind::LongArray(values)
      | ExpressionKind::Array { values, .. }
        if index.kind.numeric_value().is_some() =>
      {
        let numeric = index.kind.numeric_value().expect("Numeric value exists");
        let numeric = if numeric > 0 {
          numeric as usize
        } else if -numeric as usize > values.len() {
          return Err(raise_error(location, "Index out of bounds."));
        } else {
          (values.len() as i32 + numeric) as usize
        };

        values
          .into_iter()
          .nth(numeric)
          .ok_or(raise_error(location, "Index out of bound."))
      }

      ExpressionKind::Storage(mut storage) | ExpressionKind::Macro(mut storage)
        if index.kind.numeric_value().is_some() =>
      {
        let index = index.kind.numeric_value().expect("Numeric value exists");
        storage.name = format!("{}[{index}]", storage.name);
        Ok(Expression::new(ExpressionKind::Storage(storage), location))
      }

      ExpressionKind::ByteArray(_)
      | ExpressionKind::IntArray(_)
      | ExpressionKind::LongArray(_)
      | ExpressionKind::Array { .. }
      | ExpressionKind::Storage(_)
      | ExpressionKind::Macro(_) => self.compile_dynamic_index(left, index, location, context),
    }
  }

  fn compile_dynamic_index(
    &mut self,
    left: Expression,
    index: Expression,
    location: Location,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    if let ExpressionKind::Macro(index) = index.kind {
      let mut storage =
        self.move_to_storage(&mut context.code, left, &context.location.namespace)?;
      storage.name = format!("{}[$({})]", storage.name, index.name);
      return Ok(Expression::with_macro(
        ExpressionKind::Storage(storage),
        location,
        true,
      ));
    }

    let dynamic_index = self.dynamic_index();
    let storage = dynamic_index.clone();
    let fn_command = format!("function {dynamic_index} with storage {storage}");

    self.set_storage(
      &mut context.code,
      &StorageLocation::new(storage.clone(), "target".to_string()),
      &left,
      &context.location.namespace,
    )?;
    self.set_storage(
      &mut context.code,
      &StorageLocation::new(storage.clone(), "__index".to_string()),
      &index,
      &context.location.namespace,
    )?;
    context.code.push(fn_command);
    Ok(Expression::new(
      ExpressionKind::Storage(StorageLocation::new(storage, "return".to_string())),
      location,
    ))
  }

  fn compile_range_index(
    &mut self,
    index: RangeIndex,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    let location = index.left.location();
    let left = self.compile_expression(*index.left, context, false)?;
    let start = if let Some(start) = index.start {
      self.compile_expression(*start, context, false)?
    } else {
      Expression::new(ExpressionKind::Integer(0), location.clone())
    };
    let end = if let Some(end) = index.end {
      Some(self.compile_expression(*end, context, false)?)
    } else {
      None
    };

    let range_is_const = start.kind.numeric_value().is_some()
      && !end
        .as_ref()
        .is_some_and(|e| e.kind.numeric_value().is_none());

    match left.kind {
      ExpressionKind::Void
      | ExpressionKind::Byte(_)
      | ExpressionKind::Short(_)
      | ExpressionKind::Integer(_)
      | ExpressionKind::Long(_)
      | ExpressionKind::Float(_)
      | ExpressionKind::Double(_)
      | ExpressionKind::Boolean(_)
      | ExpressionKind::Compound(_)
      | ExpressionKind::Scoreboard(_)
      | ExpressionKind::Array { .. }
      | ExpressionKind::ByteArray(_)
      | ExpressionKind::IntArray(_)
      | ExpressionKind::LongArray(_)
      | ExpressionKind::Condition(_) => {
        Err(raise_error(left.location, "Can only range index strings."))
      }

      ExpressionKind::String(s) if range_is_const => {
        let start = start.kind.numeric_value().expect("Value is some");
        if start < 0 {
          return Err(raise_error(location, "Range index out of bounds."));
        }
        let start = start as usize;

        let end = end
          .and_then(|e| e.kind.numeric_value())
          .unwrap_or(s.len() as i32);

        let end = if end > 0 {
          end as usize
        } else if -end as usize > s.len() {
          return Err(raise_error(location, "Range index out of bounds."));
        } else {
          (s.len() as i32 + end) as usize
        };

        if start >= s.len() || end > s.len() {
          return Err(raise_error(location, "Range index out of bounds."));
        }

        if end <= start {
          return Err(raise_error(
            location,
            "Start must come before end in range index.",
          ));
        }

        Ok(Expression::new(
          ExpressionKind::String(s[start..end].to_string()),
          location,
        ))
      }

      ExpressionKind::SubString(storage, sub_start, sub_end) if range_is_const => {
        let start = start.kind.numeric_value().expect("Value is some");
        if start < 0 {
          return Err(raise_error(location, "Range index out of bounds."));
        }

        let end = end.and_then(|e| e.kind.numeric_value());

        if let Some(end) = end {
          if end >= 0 && end <= start {
            return Err(raise_error(
              location,
              "Start must come before end in range index.",
            ));
          }
        }

        let end = match (end, sub_end) {
          (None, None) => None,
          (None, Some(end)) | (Some(end), None) => Some(end),
          (Some(a), Some(b)) => Some(a + b),
        };

        Ok(Expression::new(
          ExpressionKind::SubString(storage, start + sub_start, end),
          location,
        ))
      }

      ExpressionKind::Storage(storage) | ExpressionKind::Macro(storage) if range_is_const => {
        let start = start.kind.numeric_value().expect("Value is some");
        if start < 0 {
          return Err(raise_error(location, "Range index out of bounds."));
        }

        let end = end.and_then(|e| e.kind.numeric_value());

        if let Some(end) = end {
          if end >= 0 && end <= start {
            return Err(raise_error(
              location,
              "Start must come before end in range index.",
            ));
          }
        }

        Ok(Expression::new(
          ExpressionKind::SubString(storage, start, end),
          location,
        ))
      }

      ExpressionKind::String(_)
      | ExpressionKind::Storage(_)
      | ExpressionKind::Macro(_)
      | ExpressionKind::SubString(_, _, _) => {
        self.compile_dynamic_range_index(left, start, end, location, context)
      }
    }
  }

  // TODO: Handle case where one of the indices is static
  // TODO: Handle case where both start and end are macros
  fn compile_dynamic_range_index(
    &mut self,
    left: Expression,
    start: Expression,
    end: Option<Expression>,
    location: Location,context: &mut FunctionContext,
  ) -> Result<Expression> {
    let dynamic_index = if end.is_some() {
      self.dynamic_range_index()
    } else {
      self.dynamic_range_index_no_end()
    };

    let storage = dynamic_index.clone();
    let fn_command = format!("function {dynamic_index} with storage {storage}");

    self.set_storage(
      &mut context.code,
      &StorageLocation::new(storage.clone(), "target".to_string()),
      &left,
      &context.location.namespace,
    )?;
    self.set_storage(
      &mut context.code,
      &StorageLocation::new(storage.clone(), "__start".to_string()),
      &start,
      &context.location.namespace,
    )?;
    if let Some(end) = end {
      self.set_storage(
        &mut context.code,
        &StorageLocation::new(storage.clone(), "__end".to_string()),
        &end,
        &context.location.namespace,
      )?;
    }
   context. code.push(fn_command);
    Ok(Expression::new(
      ExpressionKind::Storage(StorageLocation::new(storage, "return".to_string())),
      location,
    ))
  }

  fn compile_member(
    &mut self,
    member: Member,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    let location = member.left.location();
    let left = self.compile_expression(*member.left, context, false)?;
    let member = match *member.member {
      ast::MemberKind::Literal(lit) => {
        Expression::new(ExpressionKind::String(lit), location.clone())
      }
      ast::MemberKind::Dynamic(expr) => self.compile_expression(expr,context, false)?,
    };
    let member_value = match member.kind.compile_time_value() {
      Some(value) => match value {
        NbtValue::String(s) => Some(s),
        _ => return Err(raise_error(location, "Can only use strings as members")),
      },
      None => None,
    };

    match left.kind {
      ExpressionKind::Void
      | ExpressionKind::Byte(_)
      | ExpressionKind::Short(_)
      | ExpressionKind::Integer(_)
      | ExpressionKind::Long(_)
      | ExpressionKind::Float(_)
      | ExpressionKind::Double(_)
      | ExpressionKind::Boolean(_)
      | ExpressionKind::String(_)
      | ExpressionKind::SubString(_, _, _)
      | ExpressionKind::Scoreboard(_)
      | ExpressionKind::Array { .. }
      | ExpressionKind::ByteArray(_)
      | ExpressionKind::IntArray(_)
      | ExpressionKind::LongArray(_)
      | ExpressionKind::Condition(_) => Err(raise_error(
        left.location,
        "Can only access members on compounds.",
      )),

      ExpressionKind::Compound(map) if member_value.is_some() => {
        let member = member_value.expect("Value is some");
        map
          .get(&member)
          .ok_or(raise_error(
            location,
            format!("Key '{member}' does not exist"),
          ))
          .cloned()
      }

      ExpressionKind::Storage(mut storage) | ExpressionKind::Macro(mut storage)
        if member_value.is_some() =>
      {
        storage.name = format!("{}.{}", storage.name, member_value.expect("Value is some"));
        Ok(Expression::new(ExpressionKind::Storage(storage), location))
      }

      ExpressionKind::Compound(_) | ExpressionKind::Storage(_) | ExpressionKind::Macro(_) => {
        self.compile_dynamic_member( left, member, location, context)
      }
    }
  }

  fn compile_dynamic_member(
    &mut self,
    left: Expression,
    member: Expression,
    location: Location,context: &mut FunctionContext
  ) -> Result<Expression> {
    if let ExpressionKind::Macro(member) = member.kind {
      let mut storage = self.move_to_storage(&mut context.code, left, &context.location.namespace)?;
      storage.name = format!("{}.\"$({})\"", storage.name, member.name);
      return Ok(Expression::with_macro(
        ExpressionKind::Storage(storage),
        location,
        true,
      ));
    }

    let dynamic_member = self.dynamic_member();
    let storage = dynamic_member.clone();
    let fn_command = format!("function {dynamic_member} with storage {storage}");

    self.set_storage(
      &mut context.code,
      &StorageLocation::new(storage.clone(), "target".to_string()),
      &left,
      &context.location.namespace,
    )?;
    self.set_storage(
      &mut context.code,
      &StorageLocation::new(storage.clone(), "__member".to_string()),
      &member,
      &context.location.namespace,
    )?;
    context.code.push(fn_command);
    Ok(Expression::new(
      ExpressionKind::Storage(StorageLocation::new(storage, "return".to_string())),
      location,
    ))
  }
}
