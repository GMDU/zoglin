use std::collections::HashMap;

use crate::parser::ast::{Parameter, ReturnType};

use super::file_tree::{FunctionLocation, ResourceLocation};

// #[derive(Clone)]
// pub enum ItemDefinition {
//   Function(FunctionDefinition),
//   // Resource(ResourceLocation),
//   Unknown(FunctionLocation),
// }

// impl ItemDefinition {
//   pub fn modules(&mut self) -> &mut Vec<String> {
//     match self {
//       ItemDefinition::Function(f) => &mut f.location.module.modules,
//       // ItemDefinition::Resource(r) => &mut r.modules,
//       ItemDefinition::Unknown(r) => &mut r.module.modules,
//     }
//   }

//   pub fn location(&self) -> &ResourceLocation {
//     match self {
//       ItemDefinition::Function(f) => &f.location.module,
//       // ItemDefinition::Resource(r) => r,
//       ItemDefinition::Unknown(r) => &r.module,
//     }
//   }

//   pub fn fn_location(&self) -> &FunctionLocation {
//     match self {
//       ItemDefinition::Function(f) => &f.location,
//       // ItemDefinition::Resource(r) => r,
//       ItemDefinition::Unknown(r) => r,
//     }
//   }
// }

#[derive(Clone)]
pub struct FunctionDefinition {
  pub location: FunctionLocation,
  pub arguments: Vec<Parameter>,
  pub return_type: ReturnType,
}

pub struct Scope {
  pub parent: usize,
  pub children: HashMap<String, Vec<usize>>,
  pub function_registry: HashMap<String, ResourceLocation>,
  pub imported_items: HashMap<String, ResourceLocation>,
}

impl Scope {
  pub fn new(parent_index: usize) -> Scope {
    Scope {
      parent: parent_index,
      children: HashMap::new(),
      function_registry: HashMap::new(),
      imported_items: HashMap::new(),
    }
  }

  pub fn add_child(&mut self, name: String, child: usize) {
    if let Some(children) = self.children.get_mut(&name) {
      children.push(child);
    } else {
      self.children.insert(name, vec![child]);
    }
  }

  pub fn get_child(&mut self, name: &String) -> Option<usize> {
    self
      .children
      .get_mut(name)
      .map(|children| children.remove(0))
  }
}
