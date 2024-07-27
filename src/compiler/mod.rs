use std::collections::HashSet;
use std::mem::take;
use std::{collections::HashMap, path::Path};

use expression::{verify_types, ConditionKind, Expression};
use file_tree::{FunctionLocation, ScoreboardLocation, StorageLocation};
use scope::{FunctionDefinition, ItemDefinition};
use serde::Serialize;

use crate::parser::ast::{
  self, ArrayType, Command, ElseStatement, File, FunctionCall, IfStatement, KeyValue,
  ParameterKind, Statement, StaticExpr, ZoglinResource,
};

use crate::error::{raise_error, Location, Result};

use self::{
  file_tree::{FileResource, FileTree, Function, Item, Namespace, ResourceLocation, TextResource},
  scope::Scope,
};
mod binary_operation;
mod expression;
mod file_tree;
mod register;
mod scope;

#[derive(Default)]
pub struct Compiler {
  tick_functions: Vec<String>,
  load_functions: Vec<String>,
  scopes: Vec<Scope>,
  current_scope: usize,
  counters: HashMap<String, usize>,
  namespaces: HashMap<String, Namespace>,
  used_scoreboards: HashSet<String>,
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

  fn add_function(&mut self, scope: usize, name: String, function: FunctionDefinition) {
    self.scopes[scope].function_registry.insert(name, function);
  }

  fn lookup_resource(&self, resource: &ZoglinResource) -> Option<ItemDefinition> {
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
          return Some(ItemDefinition::Function(function_definition.clone()));
        }
      }
      if let Some(resource_location) = scope.imported_items.get(first) {
        return Some(resource_location.clone());
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

  fn add_import(&mut self, scope: usize, name: String, definition: ItemDefinition) {
    self.scopes[scope].imported_items.insert(name, definition);
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
      scoreboard: vec!["zoglin", "internal", namespace, "vars"]
        .into_iter()
        .map(|s| s.to_string())
        .collect(),
      name: format!("$var_{}", self.next_counter("scoreboard")),
    }
  }

  fn next_storage(&mut self) -> StorageLocation {
    StorageLocation {
      storage: ResourceLocation {
        namespace: "zoglin".to_string(),
        modules: vec!["internal".to_string(), "vars".to_string()],
      },
      name: format!("var_{}", self.next_counter("storage")),
    }
  }

  fn next_function(&mut self, function_type: &str, namespace: String) -> FunctionLocation {
    FunctionLocation {
      module: ResourceLocation {
        namespace: "zoglin".to_string(),
        modules: vec![
          "generated".to_string(),
          namespace,
          function_type.to_string(),
        ],
      },
      name: format!(
        "fn_{}",
        self.next_counter(&format!("function:{}", function_type))
      ),
    }
  }
}

impl Compiler {
  pub fn compile(ast: File, output: &String) -> Result<()> {
    let mut compiler = Compiler::default();

    compiler.register(&ast);
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

    let location = ResourceLocation {
      namespace: "minecraft".to_string(),
      modules: Vec::new(),
    };
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

    let resource: ResourceLocation = ResourceLocation {
      namespace: namespace.name.clone(),
      modules: Vec::new(),
    };

    for item in namespace.items {
      self.compile_item(item, &resource)?;
    }

    self.exit_scope();

    let load_function = Item::Function(Function {
      name: "load".to_string(),
      commands: take(&mut self.used_scoreboards)
        .into_iter()
        .map(|scoreboard| format!("scoreboard objectives add {scoreboard} dummy"))
        .collect(),
      location: Location::blank(),
    });
    self.add_item(
      ResourceLocation {
        namespace: "zoglin".to_string(),
        modules: vec!["generated".to_string(), namespace.name],
      },
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
    }
  }

  fn compile_module(&mut self, module: ast::Module, mut location: ResourceLocation) -> Result<()> {
    self.enter_scope(&module.name);

    location.modules.push(module.name);

    for item in module.items {
      self.compile_item(item, &location)?;
    }

    self.exit_scope();
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
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> Result<()> {
    Ok(match statement {
      Statement::Command(command) => {
        let result = self.compile_command(code, command, location)?;
        code.push(result);
      }
      Statement::Comment(comment) => {
        code.push(comment);
      }
      Statement::Expression(expression) => {
        self.compile_expression(expression, location, code, true)?;
      }
      Statement::IfStatement(if_statement) => {
        self.compile_if_statement(code, if_statement, location)?;
      }
      Statement::Return(value) => self.compile_return(code, value, location)?,
    })
  }

  fn compile_ast_function(
    &mut self,
    function: ast::Function,
    location: &ResourceLocation,
  ) -> Result<()> {
    let fn_location = FunctionLocation {
      module: location.clone(),
      name: function.name,
    };

    let commands = self.compile_block(&fn_location, function.items)?;
    self.add_function_item(function.location, fn_location, commands)
  }

  fn add_function_item(
    &mut self,
    location: Location,
    fn_location: FunctionLocation,
    commands: Vec<String>,
  ) -> Result<()> {
    let function = Function {
      name: fn_location.name,
      location,
      commands,
    };

    self.add_item(fn_location.module, Item::Function(function))
  }

  fn compile_block(
    &mut self,
    location: &FunctionLocation,
    block: Vec<Statement>,
  ) -> Result<Vec<String>> {
    let mut commands = Vec::new();
    for item in block {
      self.compile_statement(item, location, &mut commands)?;
    }
    Ok(commands)
  }

  fn compile_command(
    &mut self,
    code: &mut Vec<String>,
    command: Command,
    location: &FunctionLocation,
  ) -> Result<String> {
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
          let (code, needs_macro) = self.compile_static_expr(code, expr, location)?;
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
    fn_location: &FunctionLocation,
    code: &mut Vec<String>,
    ignored: bool,
  ) -> Result<Expression> {
    Ok(match expression {
      ast::Expression::FunctionCall(function_call) => {
        let location = function_call.path.location.clone();
        let (command, fn_location) =
          self.compile_function_call(code, function_call, fn_location)?;
        if !ignored {
          code.push(format!(
            "data modify storage {} return set value false",
            fn_location.to_string()
          ));
        }
        code.push(command);
        Expression::Storage(
          StorageLocation::new(fn_location, "return".to_string()),
          location,
        )
      }
      ast::Expression::Byte(b, location) => Expression::Byte(b, location),
      ast::Expression::Short(s, location) => Expression::Short(s, location),
      ast::Expression::Integer(i, location) => Expression::Integer(i, location),
      ast::Expression::Long(l, location) => Expression::Long(l, location),
      ast::Expression::Float(f, location) => Expression::Float(f, location),
      ast::Expression::Double(d, location) => Expression::Double(d, location),
      ast::Expression::Boolean(b, location) => Expression::Boolean(b, location),
      ast::Expression::String(s, location) => Expression::String(s, location),
      ast::Expression::Array(typ, a, location) => {
        self.compile_array(code, typ, a, location, fn_location)?
      }
      ast::Expression::Compound(key_values, location) => {
        self.compile_compound(code, key_values, location, fn_location)?
      }
      ast::Expression::Variable(variable) => Expression::Storage(
        StorageLocation::from_zoglin_resource(fn_location.clone(), &variable),
        variable.location,
      ),
      ast::Expression::ScoreboardVariable(variable) => Expression::Scoreboard(
        ScoreboardLocation::from_zoglin_resource(fn_location.clone(), &variable),
        variable.location,
      ),
      ast::Expression::MacroVariable(name, location) => Expression::Macro(name, location),
      ast::Expression::BinaryOperation(binary_operation) => {
        self.compile_binary_operation(binary_operation, fn_location, code)?
      }
    })
  }

  fn compile_array(
    &mut self,
    code: &mut Vec<String>,
    typ: ArrayType,
    expressions: Vec<ast::Expression>,
    location: Location,
    fn_location: &FunctionLocation,
  ) -> Result<Expression> {
    let mut types = Vec::new();

    for expr in expressions {
      types.push(self.compile_expression(expr, fn_location, code, false)?);
    }

    let err_msg = match typ {
      ArrayType::Any => "Arrays can only contain values of the same type",
      ArrayType::Byte => "Byte arrays can only byte values",
      ArrayType::Int => "Int arrays can only integer values",
      ArrayType::Long => "Long arrays can only long values",
    };
    let data_type = verify_types(&types, typ, err_msg)?;

    Ok(match typ {
      ArrayType::Any => Expression::Array {
        values: types,
        location,
        data_type,
      },
      ArrayType::Byte => Expression::ByteArray(types, location),
      ArrayType::Int => Expression::IntArray(types, location),
      ArrayType::Long => Expression::LongArray(types, location),
    })
  }

  fn compile_compound(
    &mut self,
    code: &mut Vec<String>,
    key_values: Vec<KeyValue>,
    location: Location,
    fn_location: &FunctionLocation,
  ) -> Result<Expression> {
    let mut types = HashMap::new();

    for KeyValue {
      key,
      value,
      location,
    } in key_values
    {
      if types
        .insert(
          key,
          self.compile_expression(value, fn_location, code, false)?,
        )
        .is_some()
      {
        return Err(raise_error(location, "Duplicate keys not allowed"));
      }
    }

    Ok(Expression::Compound(types, location))
  }

  // Returns whether the expression requires a macro command
  fn compile_static_expr(
    &mut self,
    code: &mut Vec<String>,
    expr: StaticExpr,
    location: &FunctionLocation,
  ) -> Result<(String, bool)> {
    match expr {
      StaticExpr::FunctionCall(call) => {
        Ok((self.compile_function_call(code, call, location)?.0, false))
      }
      StaticExpr::ResourceRef {
        resource,
        is_fn: true,
      } => Ok((
        self
          .resolve_zoglin_resource(resource, &location.module)?
          .location()
          .to_string(),
        false,
      )),
      StaticExpr::MacroVariable(name) => Ok((format!("$({name})"), true)),

      StaticExpr::ResourceRef {
        resource,
        is_fn: false,
      } => Ok((
        ResourceLocation::from_zoglin_resource(&location.module, &resource).to_string(),
        false,
      )),
    }
  }

  fn compile_function_call(
    &mut self,
    code: &mut Vec<String>,
    function_call: FunctionCall,
    location: &FunctionLocation,
  ) -> Result<(String, ResourceLocation)> {
    let src_location = function_call.path.location.clone();
    let path = self.resolve_zoglin_resource(function_call.path, &location.module)?;
    let function_definition = if let ItemDefinition::Function(function_definition) = path {
      function_definition
    } else {
      FunctionDefinition {
        location: path.fn_location().clone(),
        arguments: Vec::new(),
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

    if has_macro_args {
      code.push("data remove storage zoglin:internal/vars macro_args".to_string());
    }

    for (parameter, argument) in function_definition
      .arguments
      .into_iter()
      .zip(function_call.arguments)
    {
      let expr = self.compile_expression(argument, location, code, false)?;
      match parameter.kind {
        ParameterKind::Storage => {
          let storage = StorageLocation {
            storage: function_definition.location.clone().flatten(),
            name: parameter.name,
          };
          self.set_storage(code, &storage, &expr)?;
        }
        ParameterKind::Scoreboard => {
          let scoreboard = ScoreboardLocation::from_named_resource_location(
            function_definition.location.clone().flatten(),
            &parameter.name,
          );
          self.set_scoreboard(code, &scoreboard, &expr)?;
        }
        ParameterKind::Macro => {
          let storage = StorageLocation {
            storage: ResourceLocation {
              namespace: "zoglin".to_string(),
              modules: vec!["internal".to_string(), "vars".to_string()],
            },
            name: format!("macro_args.{}", parameter.name),
          };
          self.set_storage(code, &storage, &expr)?;
        }
        ParameterKind::CompileTime => todo!(),
      }
    }

    let command = if has_macro_args {
      format!(
        "function {} with storage zoglin:internal/vars macro_args",
        function_definition.location.to_string()
      )
    } else {
      format!("function {}", function_definition.location.to_string())
    };
    Ok((command, function_definition.location.flatten()))
  }

  fn resolve_zoglin_resource(
    &mut self,
    resource: ast::ZoglinResource,
    location: &ResourceLocation,
  ) -> Result<ItemDefinition> {
    let mut resource_location = ResourceLocation {
      namespace: String::new(),
      modules: Vec::new(),
    };

    if let Some(namespace) = resource.namespace {
      if namespace.is_empty() {
        resource_location.namespace.clone_from(&location.namespace);
      } else {
        resource_location.namespace = namespace;
      }
    } else if let Some(resolved) = self.lookup_resource(&resource) {
      let mut result = resolved;

      if resource.modules.len() > 1 {
        result.modules().extend_from_slice(&resource.modules[1..]);
      }
      if !resource.modules.is_empty() {
        result.modules().push(resource.name);
      }
      return Ok(result);
    } else {
      resource_location = location.clone();
    }

    resource_location.modules.extend(resource.modules);

    Ok(ItemDefinition::Unknown(FunctionLocation {
      module: resource_location,
      name: resource.name,
    }))
  }

  fn compile_if_statement(
    &mut self,
    code: &mut Vec<String>,
    if_statement: IfStatement,
    location: &FunctionLocation,
  ) -> Result<()> {
    if if_statement.child.is_some() {
      let if_function = self.next_function("if", location.module.namespace.clone());
      code.push(format!("function {}", if_function.to_string()));
      let mut function_code = Vec::new();

      let mut if_statement = if_statement;
      loop {self.compile_if_statement_without_child(
            &mut function_code,
            if_statement.condition,
            if_statement.block,
            &if_function,
            true,
          )?;
        match if_statement.child {
          Some(ElseStatement::IfStatement(if_stmt)) => {
            if_statement = *if_stmt;
          }

          Some(ElseStatement::Block(block)) => {
            let commands = self.compile_block(location, block)?;
            code.extend(commands);
            break;
          }

          None => break,
        }
      }

      self.add_item(
        if_function.module,
        Item::Function(Function {
          name: if_function.name,
          commands: function_code,
          location: Location::blank(),
        }),
      )?;

      return Ok(());
    }
    self.compile_if_statement_without_child(
      code,
      if_statement.condition,
      if_statement.block,
      location,
      false,
    )
  }

  fn compile_if_statement_without_child(
    &mut self,
    code: &mut Vec<String>,
    condition: ast::Expression,
    body: Vec<Statement>,
    location: &FunctionLocation,
    is_child: bool,
  ) -> Result<()> {
    let condition = self.compile_expression(condition, location, code, false)?;

    let check_code = match condition.to_condition(self, code, &location.module.namespace)? {
      ConditionKind::Known(false) => {
        return Ok(())
      }
      ConditionKind::Known(true) => {
        let commands  = self.compile_block(location, body)?;
        code.extend(commands);
        return Ok(());
      }
      ConditionKind::Check(check_code) => check_code,
    };

    let commands = self.compile_block(location, body)?;

    let command = match commands.len() {
      0 => return Ok(()),
      1 => &commands[0],
      _ => {
        let function = self.next_function("if", location.module.namespace.clone());
        let fn_str = function.to_string();
        self.add_function_item(Location::blank(), function, commands)?;
        &format!("function {fn_str}")
      }
    };

    code.push(format!(
      "execute {condition} {run_str} {command}",
      condition = check_code,
      run_str = if is_child { "run return run" } else { "run" },
    ));
    Ok(())
  }

  fn compile_return(
    &mut self,
    code: &mut Vec<String>,
    value: Option<ast::Expression>,
    location: &FunctionLocation,
  ) -> Result<()> {
    if let Some(value) = value {
      let expression = self.compile_expression(value, location, code, false)?;
      let return_storage = StorageLocation::new(location.clone().flatten(), "return".to_string());
      self.set_storage(code, &return_storage, &expression)?;
    }

    code.push("return 418".to_string());

    Ok(())
  }
}
