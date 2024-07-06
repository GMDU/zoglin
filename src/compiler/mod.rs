use std::{cell::RefCell, collections::HashMap, mem::replace, path::Path};

use expression::{verify_types, ConditionKind, ExpressionType};
use file_tree::{FunctionLocation, ScoreboardLocation, StorageLocation};
use serde::Serialize;

use crate::parser::ast::{
  self, ArrayType, Command, ElseStatement, Expression, File, FunctionCall, IfStatement, Statement,
  StaticExpr, ZoglinResource,
};

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
  ast: File,
  state: RefCell<CompilerState>,
}

struct CompilerState {
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

impl CompilerState {
  fn push_scope(&mut self, name: String, parent: usize) -> usize {
    self.scopes.push(Scope::new(parent));
    let index = self.scopes.len() - 1;
    self.scopes[parent].add_child(name, index);
    index
  }

  fn enter_scope(&mut self, name: &String) {
    self.current_scope = *self.scopes[self.current_scope].children.get(name).unwrap();
  }

  fn exit_scope(&mut self) {
    self.current_scope = self.scopes[self.current_scope].parent;
  }

  fn register_function(&mut self, scope: usize, name: String, location: ResourceLocation) {
    self.scopes[scope].function_registry.insert(name, location);
  }

  fn lookup_resource(&self, resource: &ZoglinResource) -> Option<&ResourceLocation> {
    if resource.namespace.is_some() {
      return None;
    }

    let first = resource.modules.first().unwrap_or(&resource.name);
    let valid_function = resource.modules.is_empty();
    let mut index = self.current_scope;

    while index != 0 {
      let scope = &self.scopes[index];
      if valid_function {
        if let Some(resource_location) = scope.function_registry.get(first) {
          return Some(resource_location);
        }
      }
      if let Some(resource_location) = scope.imported_items.get(first) {
        return Some(resource_location);
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

    let namespace = self.namespaces.get_mut(&location.namespace).unwrap();
    namespace.get_module(location.modules)
  }

  fn register_import(&mut self, scope: usize, name: String, location: ResourceLocation) {
    self.scopes[scope].imported_items.insert(name, location);
  }

  fn register_item(&mut self, location: ResourceLocation, item: Item) {
    self.get_location(location).push(item);
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
  pub fn new(ast: File) -> Compiler {
    Compiler {
      ast,
      state: RefCell::new(CompilerState {
        tick_functions: Vec::new(),
        load_functions: Vec::new(),
        scopes: Vec::new(),
        current_scope: 0,
        counters: HashMap::new(),
        namespaces: HashMap::new(),
      }),
    }
  }

  pub fn compile(&self, output: &String) {
    self.register();
    let tree = self.compile_tree();
    tree.generate(output);
  }

  fn compile_tree(&self) -> FileTree {
    for namespace in self.ast.items.iter() {
      self.compile_namespace(namespace);
    }

    let mut state = self.state.borrow_mut();

    if state.load_functions.len() > 0 || state.tick_functions.len() > 0 {
      let tick_json = FunctionTag {
        values: &state.tick_functions,
      };

      let load_json = FunctionTag {
        values: &state.load_functions,
      };

      let tick_text = serde_json::to_string_pretty(&tick_json).unwrap();
      let load_text = serde_json::to_string_pretty(&load_json).unwrap();

      let tick = Item::TextResource(TextResource {
        name: "tick".to_string(),
        kind: "tags/function".to_string(),
        is_asset: false,
        text: tick_text,
      });

      let load = Item::TextResource(TextResource {
        name: "load".to_string(),
        kind: "tags/function".to_string(),
        is_asset: false,
        text: load_text,
      });

      let location = ResourceLocation {
        namespace: "minecraft".to_string(),
        modules: Vec::new(),
      };

      state.register_item(location.clone(), tick);
      state.register_item(location, load);
    }

    let namespaces = replace(&mut state.namespaces, HashMap::new());
    FileTree {
      namespaces: namespaces.into_values().collect(),
    }
  }

  fn compile_namespace(&self, namespace: &ast::Namespace) {
    self.state.borrow_mut().enter_scope(&namespace.name);

    for item in namespace.items.iter() {
      let mut resource = ResourceLocation {
        namespace: namespace.name.clone(),
        modules: Vec::new(),
      };
      self.compile_item(item, &mut resource);
    }

    self.state.borrow_mut().exit_scope();
  }

  fn compile_item(&self, item: &ast::Item, location: &mut ResourceLocation) {
    match item {
      ast::Item::Module(module) => self.compile_module(module, location),
      ast::Item::Import(_) => {}
      ast::Item::Function(function) => self.compile_ast_function(function, location),
      ast::Item::Resource(resource) => self.compile_resource(resource, location),
    }
  }

  fn compile_module(&self, module: &ast::Module, location: &mut ResourceLocation) {
    self.state.borrow_mut().enter_scope(&module.name);

    location.modules.push(module.name.clone());

    for item in module.items.iter() {
      self.compile_item(item, location);
    }

    self.state.borrow_mut().exit_scope();
  }

  fn compile_resource(&self, resource: &ast::Resource, location: &ResourceLocation) {
    match &resource.content {
      ast::ResourceContent::Text(name, text) => {
        let resource = TextResource {
          kind: resource.kind.clone(),
          name: name.clone(),
          is_asset: resource.is_asset,
          text: text.clone(),
        };
        self
          .state
          .borrow_mut()
          .register_item(location.clone(), Item::TextResource(resource))
      }
      ast::ResourceContent::File(path, file) => {
        let file_path = Path::new(file).parent().unwrap();
        let resource = FileResource {
          kind: resource.kind.clone(),
          is_asset: resource.is_asset,
          path: file_path.join(path).to_str().unwrap().to_string(),
        };
        self
          .state
          .borrow_mut()
          .register_item(location.clone(), Item::FileResource(resource))
      }
    }
  }

  fn compile_statement(
    &self,
    statement: &Statement,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) {
    match statement {
      Statement::Command(command) => code.push(self.compile_command(command, location)),
      Statement::Comment(comment) => code.push(comment.clone()),
      Statement::Expression(expression) => {
        self.compile_expression(expression, location, code);
      }
      Statement::IfStatement(if_statement) => {
        self.compile_if_statement(code, if_statement, location, false)
      }
    };
  }

  fn compile_ast_function(&self, function: &ast::Function, location: &ResourceLocation) {
    let fn_location = FunctionLocation {
      module: location.clone(),
      name: function.name.clone(),
    };

    self.compile_function(fn_location, &function.items);
  }

  fn compile_function(&self, location: FunctionLocation, block: &Vec<Statement>) {
    let mut commands = Vec::new();
    for item in block {
      self.compile_statement(item, &location, &mut commands);
    }

    let function = Function {
      name: location.name,
      commands,
    };

    self
      .state
      .borrow_mut()
      .register_item(location.module, Item::Function(function));
  }

  fn compile_command(&self, command: &Command, location: &FunctionLocation) -> String {
    let mut result = String::new();

    for part in command.parts.iter() {
      match part {
        ast::CommandPart::Literal(lit) => result.push_str(&lit),
        ast::CommandPart::Expression(expr) => {
          result.push_str(&self.compile_static_expr(expr, &location.module))
        }
      }
    }

    result
  }

  fn compile_expression(
    &self,
    expression: &Expression,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> ExpressionType {
    match expression {
      Expression::FunctionCall(function_call) => {
        code.push(self.compile_function_call(function_call, &location.module));
        ExpressionType::Void
      }
      Expression::Byte(b) => ExpressionType::Byte(*b),
      Expression::Short(s) => ExpressionType::Short(*s),
      Expression::Integer(i) => ExpressionType::Integer(*i),
      Expression::Long(l) => ExpressionType::Long(*l),
      Expression::Float(f) => ExpressionType::Float(*f),
      Expression::Double(d) => ExpressionType::Double(*d),
      Expression::Boolean(b) => ExpressionType::Boolean(*b),
      Expression::String(s) => ExpressionType::String(s.clone()),
      Expression::Array(typ, a) => self.compile_array(code, *typ, a, location),
      Expression::Variable(variable) => ExpressionType::Storage(
        StorageLocation::from_zoglin_resource(location.clone(), variable),
      ),
      Expression::BinaryOperation(binary_operation) => {
        self.compile_binary_operation(binary_operation, location, code)
      }
    }
  }

  fn compile_array(
    &self,
    code: &mut Vec<String>,
    typ: ArrayType,
    expressions: &[Expression],
    location: &FunctionLocation,
  ) -> ExpressionType {
    let mut types = Vec::new();

    for expr in expressions {
      types.push(self.compile_expression(expr, location, code));
    }

    if !verify_types(&types, typ) {
      match typ {
        ArrayType::Any => panic!("Arrays can only contain values of the same type"),
        ArrayType::Byte => panic!("Byte arrays can only byte values"),
        ArrayType::Int => panic!("Int arrays can only integer values"),
        ArrayType::Long => panic!("Long arrays can only long values"),
      }
    }

    match typ {
        ArrayType::Any => ExpressionType::Array(types),
        ArrayType::Byte => ExpressionType::ByteArray(types),
        ArrayType::Int => ExpressionType::IntArray(types),
        ArrayType::Long => ExpressionType::LongArray(types),
    }
  }

  fn compile_static_expr(&self, expr: &StaticExpr, location: &ResourceLocation) -> String {
    match expr {
      StaticExpr::FunctionCall(call) => self.compile_function_call(call, location),
      StaticExpr::ResourceRef {
        resource,
        is_fn: true,
      } => self.resolve_zoglin_resource(resource, location).to_string(),
      StaticExpr::ResourceRef {
        resource,
        is_fn: false,
      } => ResourceLocation::from_zoglin_resource(location, resource).to_string(),
    }
  }

  fn compile_function_call(
    &self,
    function_call: &FunctionCall,
    location: &ResourceLocation,
  ) -> String {
    let mut command = "function ".to_string();

    let path = self.resolve_zoglin_resource(&function_call.path, location);
    command.push_str(&path.to_string());

    command
  }

  fn resolve_zoglin_resource(
    &self,
    resource: &ast::ZoglinResource,
    location: &ResourceLocation,
  ) -> ResourceLocation {
    let mut result = ResourceLocation {
      namespace: String::new(),
      modules: Vec::new(),
    };

    if let Some(namespace) = &resource.namespace {
      if namespace.len() == 0 {
        result.namespace = location.namespace.clone();
      } else {
        result.namespace = namespace.clone();
      }
    } else {
      if let Some(resolved) = self.state.borrow().lookup_resource(resource) {
        result = resolved.clone();
        if resource.modules.len() > 1 {
          result.modules.extend_from_slice(&resource.modules[1..]);
        }
        if !resource.modules.is_empty() {
          result.modules.push(resource.name.clone());
        }
        return result;
      } else {
        result = location.clone();
      }
    }
    result.modules.extend(resource.modules.clone());
    result.modules.push(resource.name.clone());

    result
  }

  fn compile_if_statement(
    &self,
    code: &mut Vec<String>,
    if_statement: &IfStatement,
    location: &FunctionLocation,
    is_child: bool,
  ) {
    if if_statement.child.is_some() && !is_child {
      let if_function = self
        .state
        .borrow_mut()
        .next_function("if", location.module.namespace.clone());
      code.push(format!("function {}", if_function.to_string()));
      let mut function_code = Vec::new();

      self.compile_if_else_chain(&mut function_code, if_statement, location);
      self.state.borrow_mut().register_item(
        if_function.module,
        Item::Function(Function {
          name: if_function.name,
          commands: function_code,
        }),
      );

      return;
    }

    let condition = self.compile_expression(&if_statement.condition, location, code);

    let check_code = match condition.to_condition() {
      ConditionKind::Known(false) => {
        return;
      }
      ConditionKind::Known(true) => {
        for item in if_statement.block.iter() {
          self.compile_statement(item, &location, code);
        }
        return;
      }
      ConditionKind::Check(check_code) => check_code,
    };

    let function = self
      .state
      .borrow_mut()
      .next_function("if", location.module.namespace.clone());

    code.push(format!(
      "execute {condition} {run_str} function {function}",
      condition = check_code,
      run_str = if is_child { "run return run" } else { "run" },
      function = function.to_string()
    ));

    self.compile_function(function, &if_statement.block)
  }

  fn compile_if_else_chain(
    &self,
    code: &mut Vec<String>,
    if_statement: &IfStatement,
    location: &FunctionLocation,
  ) {
    self.compile_if_statement(code, if_statement, location, true);
    match if_statement.child.as_ref() {
      Some(ElseStatement::IfStatement(if_stmt)) => {
        self.compile_if_else_chain(code, &if_stmt, location)
      }

      Some(ElseStatement::Block(block)) => {
        for item in block.iter() {
          self.compile_statement(item, &location, code);
        }
      }

      None => {}
    }
  }
}
