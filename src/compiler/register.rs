use crate::parser::ast::{Function, Import, Item, Module, Namespace};

use super::{file_tree::ResourceLocation, scope::Scope, Compiler};

impl Compiler {
  pub fn register(&self) {
    self.state.borrow_mut().scopes.push(Scope::new(0));
    for namespace in self.ast.items.iter() {
      self.register_namespace(namespace, 0);
    }
  }

  fn register_namespace(&self, namespace: &Namespace, parent_scope: usize) {
    let index = self
      .state
      .borrow_mut()
      .push_scope(namespace.name.clone(), parent_scope);

    for item in namespace.items.iter() {
      let mut resource = ResourceLocation {
        namespace: namespace.name.clone(),
        modules: Vec::new(),
      };
      self.register_item(item, &mut resource, index);
    }
  }

  fn register_item(&self, item: &Item, location: &mut ResourceLocation, parent_scope: usize) {
    match item {
      Item::Module(module) => self.register_module(module, location, parent_scope),

      Item::Import(import) => self.register_import(import, location, parent_scope),

      Item::Function(function) => self.register_function(function, location, parent_scope),

      Item::Resource(_) => {}
    }
  }

  fn register_module(&self, module: &Module, location: &mut ResourceLocation, parent_scope: usize) {
    let index = self
      .state
      .borrow_mut()
      .push_scope(module.name.clone(), parent_scope);

    location.modules.push(module.name.clone());

    for item in module.items.iter() {
      self.register_item(item, location, index);
    }

    location.modules.pop();
  }

  fn register_import(&self, import: &Import, location: &ResourceLocation, scope: usize) {
    let name = import
      .alias
      .clone()
      .unwrap_or_else(|| import.path.name.clone());
    let path = ResourceLocation::from_zoglin_resource(location, &import.path);
    self.state.borrow_mut().register_import(scope, name, path);
  }

  fn register_function(&self, function: &Function, location: &ResourceLocation, scope: usize) {
    let function_path = location.join(&function.name);
    let mut state = self.state.borrow_mut();

    let mut function_location = location.clone();
    function_location.modules.push(function.name.clone());
    state.register_function(scope, function.name.clone(), function_location);

    if &function.name == "tick" && location.modules.len() < 1 {
      state.tick_functions.push(function_path);
    } else if &function.name == "load" && location.modules.len() < 1 {
      state.load_functions.push(function_path);
    }
  }
}
