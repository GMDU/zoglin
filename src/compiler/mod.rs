use std::mem::take;
use std::{collections::HashMap, path::Path};

use expression::{verify_types, ConditionKind, Expression};
use file_tree::{FunctionLocation, ScoreboardLocation, StorageLocation};
use scope::{FunctionDefinition, ItemDefinition};
use serde::Serialize;

use crate::parser::ast::{
  self, ArrayType, Command, ElseStatement, File, FunctionCall, IfStatement, KeyValue, Statement,
  StaticExpr, ZoglinResource,
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

pub struct Compiler {
  tick_functions: Vec<String>,
  load_functions: Vec<String>,
  scopes: Vec<Scope>,
  current_scope: usize,
  counters: HashMap<String, usize>,
  namespaces: HashMap<String, Namespace>,
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

  fn next_scoreboard(&mut self) -> ScoreboardLocation {
    ScoreboardLocation {
      scoreboard: vec!["zoglin", "internal", "vars"]
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
    let mut compiler = Compiler {
      tick_functions: Vec::new(),
      load_functions: Vec::new(),
      scopes: Vec::new(),
      current_scope: 0,
      counters: HashMap::new(),
      namespaces: HashMap::new(),
    };

    compiler.register(&ast);
    let tree = compiler.compile_tree(ast)?;
    tree.generate(output)?;
    Ok(())
  }

  fn compile_tree(&mut self, ast: File) -> Result<FileTree> {
    for namespace in ast.items {
      self.compile_namespace(namespace)?;
    }

    if !self.load_functions.is_empty() || !self.tick_functions.is_empty() {
      let tick_json = FunctionTag {
        values: &self.tick_functions,
      };

      let load_json = FunctionTag {
        values: &self.load_functions,
      };

      let tick_text = serde_json::to_string_pretty(&tick_json).expect("Json is valid");
      let load_text = serde_json::to_string_pretty(&load_json).expect("Json is valid");

      let tick: Item = Item::TextResource(TextResource {
        name: "tick".to_string(),
        kind: "tags/function".to_string(),
        is_asset: false,
        text: tick_text,
        location: Location::blank(),
      });

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

      self.add_item(location.clone(), tick)?;
      self.add_item(location, load)?;
    }

    let namespaces = take(&mut self.namespaces);
    Ok(FileTree {
      namespaces: namespaces.into_values().collect(),
    })
  }

  fn compile_namespace(&mut self, namespace: ast::Namespace) -> Result<()> {
    self.enter_scope(&namespace.name);

    let resource: ResourceLocation = ResourceLocation {
      namespace: namespace.name,
      modules: Vec::new(),
    };

    for item in namespace.items {
      self.compile_item(item, &resource)?;
    }

    self.exit_scope();
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
    match statement {
      Statement::Command(command) => {
        let result = self.compile_command(code, command, location)?;
        code.push(result);
      }
      Statement::Comment(comment) => code.push(comment),
      Statement::Expression(expression) => {
        self.compile_expression(expression, location, code)?;
      }
      Statement::IfStatement(if_statement) => {
        self.compile_if_statement(code, if_statement, location)?;
      }
    };
    Ok(())
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

    self.compile_function(function.location, fn_location, function.items)
  }

  fn compile_function(
    &mut self,
    location: Location,
    fn_location: FunctionLocation,
    block: Vec<Statement>,
  ) -> Result<()> {
    let mut commands = Vec::new();
    for item in block {
      self.compile_statement(item, &fn_location, &mut commands)?;
    }

    let function = Function {
      name: fn_location.name,
      location,
      commands,
    };

    self.add_item(fn_location.module, Item::Function(function))?;
    Ok(())
  }

  fn compile_command(
    &mut self,
    code: &mut Vec<String>,
    command: Command,
    location: &FunctionLocation,
  ) -> Result<String> {
    let mut result = String::new();

    for part in command.parts {
      match part {
        ast::CommandPart::Literal(lit) => result.push_str(&lit),
        ast::CommandPart::Expression(expr) => {
          result.push_str(&self.compile_static_expr(code, expr, location)?)
        }
      }
    }

    Ok(result)
  }

  fn compile_expression(
    &mut self,
    expression: ast::Expression,
    fn_location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> Result<Expression> {
    Ok(match expression {
      ast::Expression::FunctionCall(function_call) => {
        let location = function_call.path.location.clone();
        let result = self.compile_function_call(code, function_call, fn_location)?;
        code.push(result);
        Expression::Void(location)
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
      types.push(self.compile_expression(expr, fn_location, code)?);
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
        .insert(key, self.compile_expression(value, fn_location, code)?)
        .is_some()
      {
        return Err(raise_error(location, "Duplicate keys not allowed"));
      }
    }

    Ok(Expression::Compound(types, location))
  }

  fn compile_static_expr(
    &mut self,
    code: &mut Vec<String>,
    expr: StaticExpr,
    location: &FunctionLocation,
  ) -> Result<String> {
    match expr {
      StaticExpr::FunctionCall(call) => self.compile_function_call(code, call, location),
      StaticExpr::ResourceRef {
        resource,
        is_fn: true,
      } => Ok(
        self
          .resolve_zoglin_resource(resource, &location.module)?
          .location()
          .to_string(),
      ),

      StaticExpr::ResourceRef {
        resource,
        is_fn: false,
      } => Ok(ResourceLocation::from_zoglin_resource(&location.module, &resource).to_string()),
    }
  }

  fn compile_function_call(
    &mut self,
    code: &mut Vec<String>,
    function_call: FunctionCall,
    location: &FunctionLocation,
  ) -> Result<String> {
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

    for (parameter, argument) in function_definition
      .arguments
      .into_iter()
      .zip(function_call.arguments)
    {
      let expr = self.compile_expression(argument, location, code)?;
      let storage = StorageLocation {
        storage: function_definition.location.clone().flatten(),
        name: parameter,
      };
      self.set_storage(code, &storage, &expr)?;
    }

    Ok(format!("function {}", function_definition.location.to_string()))
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
      loop {
        self.compile_if_statement_without_child(
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
            for item in block {
              self.compile_statement(item, location, &mut function_code)?;
            }
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
    let condition = self.compile_expression(condition, location, code)?;

    let check_code = match condition.to_condition(self, code)? {
      ConditionKind::Known(false) => {
        return Ok(());
      }
      ConditionKind::Known(true) => {
        for item in body {
          self.compile_statement(item, location, code)?;
        }
        return Ok(());
      }
      ConditionKind::Check(check_code) => check_code,
    };

    let function = self.next_function("if", location.module.namespace.clone());

    code.push(format!(
      "execute {condition} {run_str} function {function}",
      condition = check_code,
      run_str = if is_child { "run return run" } else { "run" },
      function = function.to_string()
    ));

    self.compile_function(Location::blank(), function, body)
  }
}
