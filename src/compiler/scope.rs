use std::collections::HashMap;

use ecow::EcoString;

use crate::parser::ast::{Parameter, ReturnType, Statement};

use super::{expression::Expression, file_tree::ResourceLocation};

#[derive(Clone)]
pub struct FunctionDefinition {
  pub location: ResourceLocation,
  pub arguments: Vec<Parameter>,
  pub return_type: ReturnType,
}

pub struct CalledFunction {
  pub location: ResourceLocation,
  pub return_type: ReturnType,
}

#[derive(Clone)]
pub struct ComptimeFunction {
  pub location: ResourceLocation,
  pub parameters: Vec<Parameter>,
  pub body: Vec<Statement>,
}

pub struct Scope {
  pub parent: usize,
  pub children: HashMap<EcoString, Vec<usize>>,
  pub function_registry: HashMap<EcoString, ResourceLocation>,
  pub comptime_functions: HashMap<EcoString, ResourceLocation>,
  pub imported_items: HashMap<EcoString, Imported>,
  pub comptime_values: HashMap<EcoString, Expression>,
}

#[derive(Debug)]
pub enum Imported {
  Comptime(ResourceLocation),
  ModuleOrFunction(ResourceLocation),
}

impl Scope {
  pub fn new(parent_index: usize) -> Scope {
    Scope {
      parent: parent_index,
      children: HashMap::new(),
      function_registry: HashMap::new(),
      comptime_functions: HashMap::new(),
      imported_items: HashMap::new(),
      comptime_values: HashMap::new(),
    }
  }

  pub fn add_child(&mut self, name: EcoString, child: usize) {
    if let Some(children) = self.children.get_mut(&name) {
      children.push(child);
    } else {
      self.children.insert(name, vec![child]);
    }
  }

  pub fn get_child(&mut self, name: &EcoString) -> Option<usize> {
    self
      .children
      .get_mut(name)
      .map(|children| children.remove(0))
  }
}
