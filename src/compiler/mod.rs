use std::cell::RefCell;

use serde::Serialize;

use crate::parser::ast::{self, File};

use self::file_tree::{FileTree, Function, Item, Module, Namespace, Resource};
mod file_tree;

pub struct Compiler {
  ast: File,
  state: RefCell<CompilerState>,
}

struct CompilerState {
  tick_functions: Vec<String>,
  load_functions: Vec<String>,
}

#[derive(Serialize)]
struct FunctionTag {
  values: Vec<String>,
}

impl Compiler {
  pub fn new(ast: File) -> Compiler {
    Compiler {
      ast,
      state: RefCell::new(CompilerState {
        tick_functions: Vec::new(),
        load_functions: Vec::new(),
      }),
    }
  }

  pub fn compile(&self) {
    let tree = self.compile_tree();
    tree.generate(".".to_string());
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
        values: state.tick_functions.clone(),
      };
      let load_json = FunctionTag {
        values: state.load_functions.clone(),
      };
      let tick_text = serde_json::to_string_pretty(&tick_json).unwrap();
      let load_text = serde_json::to_string_pretty(&load_json).unwrap();

      mc_namespace.items.push(Item::Resource(Resource {
        name: "tick".to_string(),
        kind: "tags/functions".to_string(),
        text: tick_text,
      }));
      mc_namespace.items.push(Item::Resource(Resource {
        name: "load".to_string(),
        kind: "tags/functions".to_string(),
        text: load_text,
      }));

      namespaces.push(mc_namespace);
    }

    FileTree { namespaces }
  }

  fn compile_namespace(&self, namespace: &ast::Namespace) -> Namespace {
    let mut items = Vec::new();

    for item in namespace.items.iter() {
      items.push(self.compile_item(item, namespace.name.clone() + ":"));
    }

    Namespace {
      name: namespace.name.clone(),
      items,
    }
  }

  fn compile_item(&self, item: &ast::Item, path: String) -> Item {
    match item {
      ast::Item::Module(module) => Item::Module(self.compile_module(module, path)),
      ast::Item::Function(function) => Item::Function(self.compile_function(function, path)),
    }
  }

  fn compile_module(&self, module: &ast::Module, path: String) -> Module {
    let mut items = Vec::new();

    for item in module.items.iter() {
      items.push(self.compile_item(item, path.clone() + &module.name + "/"));
    }

    Module {
      name: module.name.clone(),
      items,
    }
  }

  fn compile_function(&self, function: &ast::Function, path: String) -> Function {
    let commands = function
      .items
      .iter()
      .map(|ast::Statement::Command(cmd)| cmd.clone())
      .collect();
    let function_path = path + &function.name;
    if &function.name == "tick" {
      self.state.borrow_mut().tick_functions.push(function_path);
    } else if &function.name == "load" {
      self.state.borrow_mut().load_functions.push(function_path);
    }

    Function {
      name: function.name.clone(),
      commands,
    }
  }
}
