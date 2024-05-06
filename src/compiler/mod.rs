use std::{cell::RefCell, collections::HashMap, path::Path};

use serde::Serialize;

use crate::parser::ast::{self, Expression, File, FunctionCall, Statement};

use self::file_tree::{
  FileResource, FileTree, Function, Item, Module, Namespace, ResourceLocation, TextResource,
};
mod file_tree;

pub struct Compiler {
  ast: File,
  state: RefCell<CompilerState>,
}

struct CompilerState {
  tick_functions: Vec<String>,
  load_functions: Vec<String>,
  scopes: Vec<Scope>,
}

struct Scope {
  function_registry: HashMap<String, ResourceLocation>,
  imported_items: HashMap<String, ResourceLocation>,
}

impl Scope {
  fn new() -> Scope {
    Scope {
      function_registry: HashMap::new(),
      imported_items: HashMap::new(),
    }
  }
}

#[derive(Serialize)]
struct FunctionTag<'a> {
  values: &'a [String],
}

impl CompilerState {
  fn enter_scope(&mut self) {
    self.scopes.push(Scope::new());
  }

  fn exit_scope(&mut self) {
    self.scopes.pop();
  }

  fn register_function(&mut self, name: String, location: ResourceLocation) {
    self
      .scopes
      .last_mut()
      .unwrap()
      .function_registry
      .insert(name, location);
  }

  fn lookup_function(&self, name: &String) -> Option<&ResourceLocation> {
    for scope in self.scopes.iter().rev() {
      if let Some(resource_location) = scope.function_registry.get(name) {
        return Some(resource_location);
      }
    }
    None
  }

  fn register_import(&mut self, name: String, location: ResourceLocation) {
    self
      .scopes
      .last_mut()
      .unwrap()
      .imported_items
      .insert(name, location);
  }

  fn lookup_import(&self, name: &String) -> Option<&ResourceLocation> {
    for scope in self.scopes.iter().rev() {
      if let Some(resource_location) = scope.imported_items.get(name) {
        return Some(resource_location);
      }
    }
    None
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
      }),
    }
  }

  pub fn compile(&self, output: &String) {
    let tree = self.compile_tree();
    tree.generate(output);
  }

  fn compile_tree(&self) -> FileTree {
    let mut namespaces = Vec::new();

    for namepace in self.ast.items.iter() {
      namespaces.push(self.compile_namespace(namepace));
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
        kind: "tags/functions".to_string(),
        text: tick_text,
      }));
      mc_namespace.items.push(Item::TextResource(TextResource {
        name: "load".to_string(),
        kind: "tags/functions".to_string(),
        text: load_text,
      }));

      namespaces.push(mc_namespace);
    }

    FileTree { namespaces }
  }

  fn compile_namespace(&self, namespace: &ast::Namespace) -> Namespace {
    self.state.borrow_mut().enter_scope();
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
      ast::Item::Import(import) => {
        self.compile_import(import, location);
        Item::Ignored
      }
      ast::Item::Function(function) => Item::Function(self.compile_function(function, location)),
      ast::Item::Resource(resource) => self.compile_resource(resource, location),
    }
  }

  fn compile_module(&self, module: &ast::Module, location: &mut ResourceLocation) -> Module {
    self.state.borrow_mut().enter_scope();

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

  fn compile_import(&self, import: &ast::Import, location: &ResourceLocation) {
    let name = import.path.name.clone();
    let path = ResourceLocation::from_zoglin_resource(location, &import.path);
    self.state.borrow_mut().register_import(name, path);
  }

  fn compile_resource(&self, resource: &ast::Resource, _location: &ResourceLocation) -> Item {
    match &resource.content {
      ast::ResourceContent::Text(name, text) => {
        return Item::TextResource(TextResource {
          kind: resource.kind.clone(),
          name: name.clone(),
          text: text.clone(),
        })
      }
      ast::ResourceContent::File(path, file) => {
        let file_path = Path::new(file).parent().unwrap();
        return Item::FileResource(FileResource {
          kind: resource.kind.clone(),
          path: file_path.join(path).to_str().unwrap().to_string(),
        });
      }
    }
  }

  fn compile_statement(&self, statement: &Statement, location: &ResourceLocation) -> String {
    match statement {
      Statement::Command(command) => command.clone(),
      Statement::Comment(comment) => comment.clone(),
      Statement::Expression(expression) => self.compile_expression(expression, location),
    }
  }

  fn compile_function(&self, function: &ast::Function, location: &ResourceLocation) -> Function {
    let commands = function
      .items
      .iter()
      .map(|statement| self.compile_statement(statement, &location))
      .collect();
    let function_path = location.join(&function.name);
    let mut state = self.state.borrow_mut();

    state.register_function(function.name.clone(), location.clone());

    if &function.name == "tick" && location.modules.len() < 1 {
      state.tick_functions.push(function_path);
    } else if &function.name == "load" && location.modules.len() < 1 {
      state.load_functions.push(function_path);
    }

    Function {
      name: function.name.clone(),
      commands,
    }
  }

  fn compile_expression(&self, expression: &Expression, location: &ResourceLocation) -> String {
    let Expression::FunctionCall(function_call) = expression;
    self.compile_function_call(function_call, location)
  }

  fn compile_function_call(
    &self,
    function_call: &FunctionCall,
    location: &ResourceLocation,
  ) -> String {
    let mut path = "function ".to_string();

    if function_call.path.namespace.is_none() && function_call.path.modules.is_empty() {
      if let Some(location) = self
        .state
        .borrow()
        .lookup_function(&function_call.path.name)
      {
        path.push_str(&location.join(&function_call.path.name));
        return path;
      }
    }
    if let Some(namespace) = &function_call.path.namespace {
      if namespace.len() == 0 {
        path.push_str(&location.namespace);
      } else {
        path.push_str(&namespace);
      }
      path.push(':');
    } else {
      let first = if function_call.path.modules.len() > 0 {
        &function_call.path.modules[0]
      } else {
        &function_call.path.name
      };

      if let Some(import) = self.state.borrow().lookup_import(first) {
        path.push_str(&import.to_string());
        if import.modules.len() > 0 {
          path.push('/');
        }
      } else {
        path.push_str(&location.to_string());
        if location.modules.len() > 0 {
          path.push('/');
        }
      }
    }
    path.push_str(&function_call.path.modules.join("/"));
    if function_call.path.modules.len() > 0 {
      path.push('/');
    }
    path.push_str(&function_call.path.name);
    path
  }
}
