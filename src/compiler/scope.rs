use std::collections::HashMap;

use super::file_tree::ResourceLocation;

pub struct Scope {
  pub parent: usize,
  pub children: HashMap<String, usize>,
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
    self.children.insert(name, child);
  }
}