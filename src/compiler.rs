use std::collections::HashSet;
use std::hash::Hash;
use std::mem::take;
use std::ops::{Deref, DerefMut};
use std::{collections::HashMap, path::Path};

use ecow::{eco_format, EcoString};
use expression::{verify_types, ConditionKind, Expression, ExpressionKind, NbtValue};
use file_tree::{ResourceLocation, ScoreboardLocation, StorageLocation};
use scope::{CalledFunction, ComptimeFunction, FunctionDefinition, Imported};
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
mod builtins;
mod expression;
mod file_tree;
mod internals;
mod register;
mod scope;
mod utils;

use utils::ToEcoString;

#[derive(Default)]
pub struct Compiler {
  tick_functions: Vec<EcoString>,
  load_functions: Vec<EcoString>,
  scopes: Vec<Scope>,
  comptime_scopes: Vec<HashMap<EcoString, Expression>>,
  current_scope: usize,
  counters: HashMap<EcoString, usize>,
  namespaces: HashMap<EcoString, Namespace>,
  // TODO: Refactor used scoreboards to be a HashMap
  used_scoreboards: HashSet<UsedScoreboard>,
  constant_scoreboard_values: HashSet<i32>,
  function_registry: HashMap<ResourceLocation, FunctionDefinition>,
  comptime_function_registry: HashMap<ResourceLocation, ComptimeFunction>,
}

enum RefOrOwned<'a, T> {
  Ref(&'a mut T),
  Owned(T),
}

impl<'a, T> RefOrOwned<'a, T> {
  fn moved(self) -> T {
    match self {
      RefOrOwned::Ref(_) => panic!("Cannot move a reference"),
      RefOrOwned::Owned(t) => t,
    }
  }
}

impl<'a, T> From<T> for RefOrOwned<'a, T> {
  fn from(value: T) -> Self {
    RefOrOwned::Owned(value)
  }
}

impl<'a, T> From<&'a mut T> for RefOrOwned<'a, T> {
  fn from(value: &'a mut T) -> Self {
    RefOrOwned::Ref(value)
  }
}

impl<'a, T> AsRef<T> for RefOrOwned<'a, T> {
  fn as_ref(&self) -> &T {
    match self {
      RefOrOwned::Ref(r) => r,
      RefOrOwned::Owned(v) => v,
    }
  }
}

impl<'a, T> AsMut<T> for RefOrOwned<'a, T> {
  fn as_mut(&mut self) -> &mut T {
    match self {
      RefOrOwned::Ref(r) => r,
      RefOrOwned::Owned(v) => v,
    }
  }
}

impl<'a, T> Deref for RefOrOwned<'a, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    self.as_ref()
  }
}

impl<'a, T> DerefMut for RefOrOwned<'a, T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.as_mut()
  }
}

struct FunctionContext<'a> {
  location: RefOrOwned<'a, ResourceLocation>,
  return_type: ReturnType,
  is_nested: bool,
  has_nested_returns: RefOrOwned<'a, bool>,
  code: RefOrOwned<'a, Vec<EcoString>>,
}

impl<'a> FunctionContext<'a> {
  fn new(location: ResourceLocation, return_type: ReturnType) -> FunctionContext<'a> {
    FunctionContext {
      location: RefOrOwned::Owned(location),
      return_type,
      is_nested: false,
      has_nested_returns: RefOrOwned::Owned(false),
      code: RefOrOwned::Owned(Vec::new()),
    }
  }

  fn child<'b>(&'b mut self, inherits_code: bool) -> FunctionContext<'b>
  where
    'a: 'b,
  {
    FunctionContext {
      location: self.location.as_mut().into(),
      return_type: self.return_type,
      is_nested: true,
      has_nested_returns: self.has_nested_returns.as_mut().into(),
      code: if inherits_code {
        self.code.as_mut().into()
      } else {
        RefOrOwned::Owned(Vec::new())
      },
    }
  }
}

#[derive(Serialize)]
struct FunctionTag<'a> {
  values: &'a [EcoString],
}

#[derive(Eq)]
pub struct UsedScoreboard {
  name: EcoString,
  criteria: EcoString,
}

impl UsedScoreboard {
  pub fn new_dummy(name: EcoString) -> UsedScoreboard {
    UsedScoreboard {
      name,
      criteria: "dummy".into(),
    }
  }
}

impl PartialEq for UsedScoreboard {
  fn eq(&self, other: &Self) -> bool {
    self.name == other.name
  }
}

impl Hash for UsedScoreboard {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.name.hash(state)
  }
}

impl Compiler {
  fn push_scope(&mut self, name: EcoString, parent: usize) -> usize {
    self.scopes.push(Scope::new(parent));
    let index = self.scopes.len() - 1;
    self.scopes[parent].add_child(name, index);
    index
  }

  fn enter_scope(&mut self, name: &EcoString) {
    self.current_scope = self.scopes[self.current_scope]
      .get_child(name)
      .expect("Child has already been added");
  }

  fn exit_scope(&mut self) {
    self.current_scope = self.scopes[self.current_scope].parent;
  }

  fn add_function(&mut self, scope: usize, name: EcoString, location: ResourceLocation) {
    self.scopes[scope].function_registry.insert(name, location);
  }

  fn add_comptime_function(&mut self, scope: usize, name: EcoString, location: ResourceLocation) {
    self.scopes[scope].comptime_functions.insert(name, location);
  }

  fn use_scoreboard(&mut self, name: EcoString, criteria: EcoString) {
    let exists = self.used_scoreboards.insert(UsedScoreboard {
      name: name.clone(),
      criteria: criteria.clone(),
    });
    if criteria != "dummy" && !exists {
      let scoreboard = UsedScoreboard { name, criteria };
      self.used_scoreboards.remove(&scoreboard);
      self.used_scoreboards.insert(scoreboard);
    }
  }

  fn use_scoreboard_dummy(&mut self, name: EcoString) {
    self
      .used_scoreboards
      .insert(UsedScoreboard::new_dummy(name));
  }

  fn lookup_resource(&self, resource: &ZoglinResource, comptime: bool) -> Option<ResourceLocation> {
    if resource.namespace.is_some() {
      return None;
    }

    let first = resource.modules.first().unwrap_or(&resource.name);
    let valid_function = resource.modules.is_empty();
    let mut index = self.current_scope;

    while index != 0 {
      let scope = &self.scopes[index];
      if valid_function {
        if !comptime {
          if let Some(function_definition) = scope.function_registry.get(first) {
            return Some(function_definition.clone());
          }
        } else if let Some(location) = scope.comptime_functions.get(first) {
          return Some(location.clone());
        }
      }
      if let Some(imported) = scope.imported_items.get(first) {
        match imported {
          Imported::ModuleOrFunction(path) if (!comptime || !resource.modules.is_empty()) => {
            return Some(path.clone())
          }
          Imported::Comptime(path) if comptime => return Some(path.clone()),

          _ => {}
        }
      }

      index = scope.parent;
    }
    None
  }

  fn lookup_comptime_variable(&self, name: &str) -> Option<Expression> {
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

  fn add_import(&mut self, scope: usize, name: EcoString, imported: Imported) {
    self.scopes[scope].imported_items.insert(name, imported);
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
            eco_format!("Function \"{name2}\" is already defined."),
          ));
        }
        (Item::TextResource(res1), Item::TextResource(res2)) if res1 == res2 => {
          return Err(raise_error(
            res2.location.clone(),
            eco_format!(
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

    self.counters.insert(counter_name.to_eco_string(), 0);
    0
  }

  fn next_scoreboard(&mut self, namespace: &str) -> ScoreboardLocation {
    self.use_scoreboard_dummy(eco_format!("zoglin.internal.{namespace}.vars"));
    ScoreboardLocation {
      scoreboard: ResourceLocation::new_function("zoglin", &["internal", namespace, "vars"]),
      name: eco_format!("$var_{}", self.next_counter("scoreboard")),
    }
  }

  fn constant_scoreboard(&mut self, value: i32) -> ScoreboardLocation {
    self.use_scoreboard_dummy("zoglin.internal.constants".into());
    self.constant_scoreboard_values.insert(value);
    ScoreboardLocation {
      scoreboard: ResourceLocation::new_function("zoglin", &["internal", "constants"]),
      name: eco_format!("${value}"),
    }
  }

  fn next_storage(&mut self, namespace: &str) -> StorageLocation {
    StorageLocation::new(
      ResourceLocation::new_function("zoglin", &["internal", namespace, "vars"]),
      eco_format!("var_{}", self.next_counter("storage")),
    )
  }

  fn next_function(&mut self, function_type: &str, namespace: &str) -> ResourceLocation {
    ResourceLocation::new_function(
      "zoglin",
      &[
        "generated",
        namespace,
        function_type,
        &eco_format!(
          "fn_{}",
          self.next_counter(&eco_format!("function:{}", function_type))
        ),
      ],
    )
  }
}

impl Compiler {
  pub fn compile(mut ast: File, output: &str) -> Result<()> {
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
      name: "load".to_eco_string(),
      kind: "tags/function".to_eco_string(),
      is_asset: false,
      text: load_text.into(),
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
        name: "tick".to_eco_string(),
        kind: "tags/function".to_eco_string(),
        is_asset: false,
        text: tick_text.into(),
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
      .insert(0, eco_format!("zoglin:generated/{}/load", namespace.name));

    self.enter_scope(&namespace.name);
    self.comptime_scopes.push(HashMap::new());

    let resource = ResourceLocation::new_module(&namespace.name, &[]);

    for item in namespace.items {
      self.compile_item(item, &resource)?;
    }

    self.exit_scope();
    self.comptime_scopes.pop();

    let load_commands = self
      .used_scoreboards
      .iter()
      .map(|scoreboard| {
        eco_format!(
          "scoreboard objectives add {} {}",
          scoreboard.name,
          scoreboard.criteria
        )
      })
      .chain(self.constant_scoreboard_values.iter().map(|value| {
        eco_format!("scoreboard players set ${value} zoglin.internal.constants {value}")
      }))
      .collect();

    let load_function = Item::Function(Function {
      name: "load".to_eco_string(),
      commands: load_commands,
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
      ast::Item::ComptimeFunction(_) => todo!(),
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
        let file_path = Path::new(file.as_str())
          .parent()
          .expect("Directory must have a parent");
        let resource = FileResource {
          kind: resource.kind,
          is_asset: resource.is_asset,
          path: file_path
            .join(path.as_str())
            .to_str()
            .expect("Path must be valid")
            .to_eco_string(),
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
      Statement::If(if_statement) => {
        let mut sub_context = context.child(true);
        sub_context.has_nested_returns = RefOrOwned::Owned(false);

        self.comptime_scopes.push(HashMap::new());
        self.compile_if_statement(if_statement, &mut sub_context)?;
        if *sub_context.has_nested_returns {
          *context.has_nested_returns = true;
          self.generate_nested_return(context);
        }
        self.comptime_scopes.pop();
      }
      Statement::WhileLoop(while_loop) => {
        let mut sub_context = context.child(true);
        sub_context.has_nested_returns = RefOrOwned::Owned(false);

        self.comptime_scopes.push(HashMap::new());
        self.compile_while_loop(while_loop, &mut sub_context)?;
        if *sub_context.has_nested_returns {
          *context.has_nested_returns = true;
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
      ReturnType::Storage | ReturnType::Scoreboard if context.is_nested => "return 0",
      ReturnType::Storage | ReturnType::Scoreboard => &eco_format!(
        "return run scoreboard players reset $should_return zoglin.internal.{namespace}.vars",
        namespace = context.location.namespace
      ),
      ReturnType::Direct => &eco_format!(
        "return run function {}",
        self.reset_direct_return(&context.location.namespace)
      ),
    };
    context.code.push(eco_format!("execute if score $should_return zoglin.internal.{namespace}.vars matches -2147483648..2147483647 run {return_command}", namespace = context.location.namespace));
  }

  fn compile_ast_function(
    &mut self,
    function: ast::Function,
    location: &ResourceLocation,
  ) -> Result<()> {
    let fn_location = location.clone().with_name(&function.name);
    let mut context = FunctionContext::new(fn_location, function.return_type);
    self.comptime_scopes.push(HashMap::new());

    self.compile_block(&mut context, function.items)?;
    self.comptime_scopes.pop();
    self.add_function_item(
      function.location,
      context.location.moved(),
      context.code.moved(),
    )
  }

  fn add_function_item(
    &mut self,
    location: Location,
    fn_location: ResourceLocation,
    commands: Vec<EcoString>,
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

  fn compile_command(
    &mut self,
    command: Command,
    context: &mut FunctionContext,
  ) -> Result<EcoString> {
    let mut result = EcoString::new();
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

    result = result.trim().into();

    if is_macro && !has_macro_prefix {
      result = eco_format!("${result}")
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
      ast::Expression::FunctionCall(function_call) if function_call.comptime => {
        self.compile_comptime_call(function_call, context)?
      }
      ast::Expression::FunctionCall(function_call) => {
        let location = function_call.path.location.clone();
        let (command, called) = self.compile_function_call(function_call, context)?;
        match called.return_type {
          ReturnType::Storage => {
            let storage = StorageLocation::new(called.location, "return".to_eco_string());
            if !ignored {
              context
                .code
                .push(eco_format!("data modify storage {storage} set value false",))
            }
            context.code.push(command);
            Expression {
              location,
              kind: ExpressionKind::Storage(storage),
              needs_macro: false,
            }
          }
          ReturnType::Scoreboard => {
            let scoreboard = ScoreboardLocation::new(called.location, "$return");
            if !ignored {
              context
                .code
                .push(eco_format!("scoreboard players set {scoreboard} 0",))
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
            context.code.push(eco_format!(
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
          eco_format!("__{name}"),
        )),
        location,
        true,
      ),
      ast::Expression::ComptimeVariable(name, location) => {
        if let Some(value) = self.lookup_comptime_variable(&name) {
          return Ok(value.clone());
        } else {
          return Err(raise_error(
            location,
            eco_format!("The compile-time variable {name} is not in scope."),
          ));
        }
      }
      ast::Expression::BinaryOperation(binary_operation) => {
        self.compile_binary_operation(binary_operation, context)?
      }
      ast::Expression::UnaryOperation(unary_expression) => {
        self.compile_unary_expression(unary_expression, context)?
      }
      ast::Expression::Index(index) => self.compile_index(index, context)?,
      ast::Expression::RangeIndex(index) => self.compile_range_index(index, context)?,
      ast::Expression::Member(member) => self.compile_member(member, context)?,
      ast::Expression::BuiltinVariable(_name, _location) => todo!("Builtin variables"),
      ast::Expression::BuiltinFunction(name, arguments, location) => {
        self.compile_builtin_function(&name, arguments, location, context)?
      }
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
  ) -> Result<(EcoString, bool)> {
    match expr {
      StaticExpr::FunctionCall(call) => {
        if call.comptime {
          let value = self
            .compile_comptime_call(call, context)?
            .kind
            .to_comptime_string(true)
            .ok_or(raise_floating_error(
              // TODO: Add location
              "This value cannot be statically resolved.",
            ))?;
          Ok((value, false))
        } else {
          Ok((self.compile_function_call(call, context)?.0, false))
        }
      }
      StaticExpr::FunctionRef { path } => Ok((
        if let Some(path) = path {
          self
            .resolve_zoglin_resource(path, &context.location.clone().module(), false)?
            .to_eco_string()
        } else {
          context.location.to_eco_string()
        },
        false,
      )),
      StaticExpr::MacroVariable(name) => Ok((eco_format!("$(__{name})"), true)),
      StaticExpr::ComptimeVariable(name) => {
        if let Some(value) = self.lookup_comptime_variable(&name) {
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
            eco_format!("The compile-time variable {name} is not in scope."),
          ))
        }
      }

      StaticExpr::ResourceRef { resource } => Ok((
        ResourceLocation::from_zoglin_resource(&context.location.clone().module(), &resource)
          .to_eco_string(),
        false,
      )),
    }
  }

  fn compile_function_call(
    &mut self,
    function_call: FunctionCall,
    context: &mut FunctionContext,
  ) -> Result<(EcoString, CalledFunction)> {
    let src_location = function_call.path.location.clone();

    let path = self.resolve_zoglin_resource(
      function_call.path,
      &context.location.clone().module(),
      false,
    )?;
    let function_definition = if let Some(function_definition) = self.function_registry.get(&path) {
      function_definition.clone()
    } else {
      FunctionDefinition {
        location: path.clone(),
        arguments: Vec::new(),
        return_type: ReturnType::Direct,
      }
    };

    let has_macro_args = function_definition
      .arguments
      .iter()
      .any(|param| param.kind == ParameterKind::Macro);
    let parameter_storage = function_definition.location.clone();

    let mut arguments = function_call.arguments.into_iter();

    let mut default_context =
      FunctionContext::new(function_definition.location.clone(), ReturnType::Direct);

    for parameter in function_definition.arguments {
      let argument = match (arguments.next(), parameter.default) {
        (Some(arg), _) => self.compile_expression(arg, context, false)?,
        (None, Some(parameter)) => {
          let expr = self.compile_expression(parameter, &mut default_context, false)?;
          context.code.extend(take(default_context.code.as_mut()));
          expr
        }
        (None, None) => return Err(raise_error(src_location, "Expected more arguments")),
      };

      match parameter.kind {
        ParameterKind::Storage => {
          let storage = StorageLocation::new(parameter_storage.clone(), parameter.name);
          self.set_storage(&mut context.code, &storage, &argument)?;
        }
        ParameterKind::Scoreboard => {
          let scoreboard = ScoreboardLocation::new(
            parameter_storage.clone(),
            &eco_format!("${}", &parameter.name),
          );
          self.set_scoreboard(&mut context.code, &scoreboard, &argument)?;
        }
        ParameterKind::Macro => {
          let storage = StorageLocation::new(
            parameter_storage.clone(),
            eco_format!("__{}", parameter.name),
          );
          self.set_storage(&mut context.code, &storage, &argument)?;
        }
        ParameterKind::CompileTime => todo!(),
      }
    }

    let command = if has_macro_args {
      eco_format!(
        "function {} with storage {parameter_storage}",
        function_definition.location
      )
    } else {
      eco_format!("function {}", function_definition.location)
    };
    Ok((
      command,
      CalledFunction {
        location: function_definition.location,
        return_type: function_definition.return_type,
      },
    ))
  }

  fn compile_comptime_call(
    &mut self,
    function_call: FunctionCall,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    let source_location = function_call.path.location.clone();

    let resource = self.resolve_zoglin_resource(function_call.path, &context.location, true)?;
    let comptime_function = self
      .comptime_function_registry
      .get(&resource)
      .ok_or(raise_error(
        source_location.clone(),
        eco_format!("Compile-time function &{resource} does not exist"),
      ))?
      .clone();

    let mut arguments = function_call.arguments.into_iter();

    self.comptime_scopes.push(HashMap::new());

    let mut default_context =
      FunctionContext::new(comptime_function.location.clone(), ReturnType::Direct);

    for parameter in comptime_function.parameters {
      let argument = match (arguments.next(), parameter.default) {
        (Some(argument), _) => self.compile_expression(argument, context, false)?,
        (None, Some(default)) => {
          let expr = self.compile_expression(default, &mut default_context, false)?;
          context.code.extend(take(default_context.code.as_mut()));
          expr
        }
        (None, None) => return Err(raise_error(source_location, "Expected more arguments")),
      };
      self
        .comptime_scopes
        .last_mut()
        .expect("The must be at least one scope")
        .insert(parameter.name.clone(), argument);
    }

    let mut return_value = None;

    for statement in comptime_function.body {
      match statement {
        // TODO: Handle returns in comptime-if
        Statement::Return(value) => {
          return_value = match value {
            Some(value) => Some(self.compile_expression(value, context, false)?),
            None => None,
          };
          break;
        }
        // TODO: Prevent returns in nested blocks
        _ => self.compile_statement(statement, context)?,
      }
    }

    self.comptime_scopes.pop();
    Ok(return_value.unwrap_or(Expression::new(ExpressionKind::Void, source_location)))
  }

  fn resolve_zoglin_resource(
    &mut self,
    resource: ast::ZoglinResource,
    location: &ResourceLocation,
    comptime: bool,
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
    } else if let Some(resolved) = self.lookup_resource(&resource, comptime) {
      let mut result = resolved;

      if resource.modules.len() > 1 {
        result.modules.extend_from_slice(&resource.modules[1..]);
      }
      if !resource.modules.is_empty() {
        result.modules.push(resource.name);
      }
      return Ok(result);
    } else {
      resource_location = location.clone().module();
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

      context.code.push(eco_format!("function {if_function}"));
      let mut sub_context = context.child(false);

      let mut if_statement = if_statement;
      loop {
        self.compile_if_statement_without_child(
          if_statement.condition,
          if_statement.block,
          &mut sub_context,
          true,
        )?;
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
          commands: sub_context.code.moved(),
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

    let mut sub_context = context.child(false);
    self.compile_block(&mut sub_context, body)?;

    let command = match sub_context.code.len() {
      0 => return Ok(()),
      1 => &sub_context.code[0],
      _ => {
        let function = self.next_function("if", &sub_context.location.namespace);
        let fn_str = function.to_eco_string();
        self.add_function_item(Location::blank(), function, sub_context.code.moved())?;
        &eco_format!("function {fn_str}")
      }
    };

    let execute_command = eco_format!(
      "execute {condition} {run_str} {command}",
      condition = check_code,
      run_str = if is_child { "run return run" } else { "run" },
    );
    context.code.push(execute_command);
    Ok(())
  }

  fn compile_return(
    &mut self,
    value: Option<ast::Expression>,
    context: &mut FunctionContext,
  ) -> Result<()> {
    if context.is_nested {
      *context.has_nested_returns = true;
    }

    let has_value = value.is_some();
    if let Some(value) = value {
      let expression = self.compile_expression(value, context, false)?;

      match context.return_type {
        ReturnType::Storage => {
          let return_storage =
            StorageLocation::new(context.location.clone(), "return".to_eco_string());
          self.set_storage(&mut context.code, &return_storage, &expression)?;
        }
        ReturnType::Scoreboard => {
          let scoreboard = ScoreboardLocation::new(context.location.clone(), "$return");
          self.use_scoreboard_dummy(scoreboard.scoreboard_string());
          self.set_scoreboard(&mut context.code, &scoreboard, &expression)?;
        }
        ReturnType::Direct => {
          if context.is_nested {
            self.set_scoreboard(
              &mut context.code,
              &ScoreboardLocation::of_internal(&context.location.namespace, "$should_return"),
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
        &ScoreboardLocation::of_internal(&context.location.namespace, "$should_return"),
        &Expression::new(ExpressionKind::Integer(1), Location::blank()),
      )?;
    }

    if has_value {
      if context.return_type != ReturnType::Direct || context.is_nested {
        context.code.push("return 0".to_eco_string())
      }
    } else {
      context.code.push("return fail".to_eco_string());
    }

    Ok(())
  }

  fn compile_while_loop(
    &mut self,
    while_loop: WhileLoop,
    context: &mut FunctionContext,
  ) -> Result<()> {
    let mut sub_context = context.child(false);
    let condition = self.compile_expression(while_loop.condition, &mut sub_context, false)?;

    match condition.to_condition(
      self,
      &mut sub_context.code,
      &sub_context.location.namespace,
      true,
    )? {
      ConditionKind::Known(false) => {}
      ConditionKind::Known(true) => {
        let fn_location = self.next_function("while", &sub_context.location.namespace);

        self.compile_block(&mut sub_context, while_loop.block)?;

        sub_context.code.push(eco_format!("function {fn_location}"));
        let function_call = eco_format!("function {fn_location}");
        self.add_function_item(Location::blank(), fn_location, sub_context.code.moved())?;

        context.code.push(function_call);
      }

      ConditionKind::Check(check_code) => {
        let fn_location = self.next_function("while", &sub_context.location.namespace);
        sub_context
          .code
          .push(eco_format!("execute {check_code} run return 0"));

        self.compile_block(&mut sub_context, while_loop.block)?;
        sub_context.code.push(eco_format!("function {fn_location}"));

        let function_call = eco_format!("function {fn_location}");
        self.add_function_item(Location::blank(), fn_location, sub_context.code.moved())?;

        context.code.push(function_call);
      }
    }

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
        storage.name = eco_format!("{}[{index}]", storage.name);
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
      storage.name = eco_format!("{}[$({})]", storage.name, index.name);
      return Ok(Expression::with_macro(
        ExpressionKind::Storage(storage),
        location,
        true,
      ));
    }

    let dynamic_index = self.dynamic_index();
    let storage = dynamic_index.clone();
    let fn_command = eco_format!("function {dynamic_index} with storage {storage}");

    self.set_storage(
      &mut context.code,
      &StorageLocation::new(storage.clone(), "target".to_eco_string()),
      &left,
    )?;
    self.set_storage(
      &mut context.code,
      &StorageLocation::new(storage.clone(), "__index".to_eco_string()),
      &index,
    )?;
    context.code.push(fn_command);
    Ok(Expression::new(
      ExpressionKind::Storage(StorageLocation::new(storage, "return".to_eco_string())),
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
          ExpressionKind::String(s[start..end].to_eco_string()),
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
    location: Location,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    let dynamic_index = if end.is_some() {
      self.dynamic_range_index()
    } else {
      self.dynamic_range_index_no_end()
    };

    let storage = dynamic_index.clone();
    let fn_command = eco_format!("function {dynamic_index} with storage {storage}");

    self.set_storage(
      &mut context.code,
      &StorageLocation::new(storage.clone(), "target".to_eco_string()),
      &left,
    )?;
    self.set_storage(
      &mut context.code,
      &StorageLocation::new(storage.clone(), "__start".to_eco_string()),
      &start,
    )?;
    if let Some(end) = end {
      self.set_storage(
        &mut context.code,
        &StorageLocation::new(storage.clone(), "__end".to_eco_string()),
        &end,
      )?;
    }
    context.code.push(fn_command);
    Ok(Expression::new(
      ExpressionKind::Storage(StorageLocation::new(storage, "return".to_eco_string())),
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
      ast::MemberKind::Dynamic(expr) => self.compile_expression(expr, context, false)?,
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
            eco_format!("Key '{member}' does not exist"),
          ))
          .cloned()
      }

      ExpressionKind::Storage(mut storage) | ExpressionKind::Macro(mut storage)
        if member_value.is_some() =>
      {
        storage.name = eco_format!("{}.{}", storage.name, member_value.expect("Value is some"));
        Ok(Expression::new(ExpressionKind::Storage(storage), location))
      }

      ExpressionKind::Compound(_) | ExpressionKind::Storage(_) | ExpressionKind::Macro(_) => {
        self.compile_dynamic_member(left, member, location, context)
      }
    }
  }

  fn compile_dynamic_member(
    &mut self,
    left: Expression,
    member: Expression,
    location: Location,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    if let ExpressionKind::Macro(member) = member.kind {
      let mut storage =
        self.move_to_storage(&mut context.code, left, &context.location.namespace)?;
      storage.name = eco_format!("{}.\"$({})\"", storage.name, member.name);
      return Ok(Expression::with_macro(
        ExpressionKind::Storage(storage),
        location,
        true,
      ));
    }

    let dynamic_member = self.dynamic_member();
    let storage = dynamic_member.clone();
    let fn_command = eco_format!("function {dynamic_member} with storage {storage}");

    self.set_storage(
      &mut context.code,
      &StorageLocation::new(storage.clone(), "target".to_eco_string()),
      &left,
    )?;
    self.set_storage(
      &mut context.code,
      &StorageLocation::new(storage.clone(), "__member".to_eco_string()),
      &member,
    )?;
    context.code.push(fn_command);
    Ok(Expression::new(
      ExpressionKind::Storage(StorageLocation::new(storage, "return".to_eco_string())),
      location,
    ))
  }
}
