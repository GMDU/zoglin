use std::{cell::RefCell, path::Path};

use serde::Serialize;

use crate::parser::ast::{
  self, Command, Expression, File, FunctionCall, Statement, StaticExpr, ZoglinResource,
};

use self::{
  file_tree::{
    FileResource, FileTree, Function, Item, Module, Namespace, ResourceLocation, TextResource,
  },
  scope::Scope,
};
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
}

#[derive(Serialize)]
struct FunctionTag<'a> {
  values: &'a [String],
}

enum ExpressionType {
  Void,
  Integer(i32)
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

  fn register_import(&mut self, scope: usize, name: String, location: ResourceLocation) {
    self.scopes[scope].imported_items.insert(name, location);
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
      }),
    }
  }

  pub fn compile(&self, output: &String) {
    self.register();
    let tree = self.compile_tree();
    tree.generate(output);
  }

  fn compile_tree(&self) -> FileTree {
    let mut namespaces = Vec::new();

    for namespace in self.ast.items.iter() {
      namespaces.push(self.compile_namespace(namespace));
    }

    let state = self.state.borrow();
    if state.load_functions.len() > 0 || state.tick_functions.len() > 0 {
      let mut mc_namespace = Namespace {
        name: "minecraft".to_string(),
        items: Vec::new(),
      };

      let tick_json = FunctionTag {
        values: &state.tick_functions,
      };
      let load_json = FunctionTag {
        values: &state.load_functions,
      };
      let tick_text = serde_json::to_string_pretty(&tick_json).unwrap();
      let load_text = serde_json::to_string_pretty(&load_json).unwrap();

      mc_namespace.items.push(Item::TextResource(TextResource {
        name: "tick".to_string(),
        kind: "tags/function".to_string(),
        is_asset: false,
        text: tick_text,
      }));
      mc_namespace.items.push(Item::TextResource(TextResource {
        name: "load".to_string(),
        kind: "tags/function".to_string(),
        is_asset: false,
        text: load_text,
      }));

      namespaces.push(mc_namespace);
    }

    FileTree { namespaces }
  }

  fn compile_namespace(&self, namespace: &ast::Namespace) -> Namespace {
    self.state.borrow_mut().enter_scope(&namespace.name);
    let mut items = Vec::new();

    for item in namespace.items.iter() {
      let mut resource = ResourceLocation {
        namespace: namespace.name.clone(),
        modules: Vec::new(),
      };
      items.push(self.compile_item(item, &mut resource));
    }

    self.state.borrow_mut().exit_scope();

    Namespace {
      name: namespace.name.clone(),
      items,
    }
  }

  fn compile_item(&self, item: &ast::Item, location: &mut ResourceLocation) -> Item {
    match item {
      ast::Item::Module(module) => Item::Module(self.compile_module(module, location)),
      ast::Item::Import(_) => Item::Ignored,
      ast::Item::Function(function) => Item::Function(self.compile_function(function, location)),
      ast::Item::Resource(resource) => self.compile_resource(resource, location),
    }
  }

  fn compile_module(&self, module: &ast::Module, location: &mut ResourceLocation) -> Module {
    self.state.borrow_mut().enter_scope(&module.name);

    location.modules.push(module.name.clone());
    let mut items = Vec::new();

    for item in module.items.iter() {
      items.push(self.compile_item(item, location));
    }

    self.state.borrow_mut().exit_scope();
    Module {
      name: module.name.clone(),
      items,
    }
  }

  fn compile_resource(&self, resource: &ast::Resource, _location: &ResourceLocation) -> Item {
    match &resource.content {
      ast::ResourceContent::Text(name, text) => {
        return Item::TextResource(TextResource {
          kind: resource.kind.clone(),
          name: name.clone(),
          is_asset: resource.is_asset,
          text: text.clone(),
        })
      }
      ast::ResourceContent::File(path, file) => {
        let file_path = Path::new(file).parent().unwrap();
        return Item::FileResource(FileResource {
          kind: resource.kind.clone(),
          is_asset: resource.is_asset,
          path: file_path.join(path).to_str().unwrap().to_string(),
        });
      }
    }
  }

  fn compile_statement(&self, statement: &Statement, location: &ResourceLocation) -> Vec<String> {
    match statement {
      Statement::Command(command) => vec![self.compile_command(command, location)],
      Statement::Comment(comment) => vec![comment.clone()],
      Statement::Expression(expression) => self.compile_expression(expression, location).0,
    }
  }

  fn compile_function(&self, function: &ast::Function, location: &ResourceLocation) -> Function {
    let commands = function
      .items
      .iter()
      .flat_map(|statement| self.compile_statement(statement, &location))
      .collect();

    Function {
      name: function.name.clone(),
      commands,
    }
  }

  fn compile_command(&self, command: &Command, location: &ResourceLocation) -> String {
    let mut result = String::new();

    for part in command.parts.iter() {
      match part {
        ast::CommandPart::Literal(lit) => result.push_str(&lit),
        ast::CommandPart::Expression(expr) => {
          result.push_str(&self.compile_static_expr(expr, location))
        }
      }
    }

    result
  }

  fn compile_expression(&self, expression: &Expression, location: &ResourceLocation) -> (Vec<String>, ExpressionType) {
    match expression {
      Expression::FunctionCall(function_call) => (vec![self.compile_function_call(function_call, location)], ExpressionType::Void),
      Expression::Integer(integer) => (Vec::new(), ExpressionType::Integer(integer.clone())),
      Expression::Variable(_) => todo!(),
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
}
