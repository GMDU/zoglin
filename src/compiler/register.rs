use crate::parser::ast::{File, Function, Import, Item, Module, Namespace, ParameterKind};

use super::{
  file_tree::{FunctionLocation, ResourceLocation, ScoreboardLocation},
  scope::{FunctionDefinition, Scope},
  Compiler,
};

impl Compiler {
  pub fn register(&mut self, ast: &File) {
    self.scopes.push(Scope::new(0));
    for namespace in ast.items.iter() {
      self.register_namespace(namespace, 0);
    }
  }

  fn register_namespace(&mut self, namespace: &Namespace, parent_scope: usize) {
    let index = self.push_scope(namespace.name.clone(), parent_scope);

    for item in namespace.items.iter() {
      let mut resource = ResourceLocation {
        namespace: namespace.name.clone(),
        modules: Vec::new(),
      };
      self.register_item(item, &mut resource, index);
    }
  }

  fn register_item(&mut self, item: &Item, location: &mut ResourceLocation, parent_scope: usize) {
    match item {
      Item::Module(module) => self.register_module(module, location, parent_scope),

      Item::Import(import) => self.register_import(import, location, parent_scope),

      Item::Function(function) => self.register_function(function, location, parent_scope),

      Item::Resource(_) => {}
    }
  }

  fn register_module(
    &mut self,
    module: &Module,
    location: &mut ResourceLocation,
    parent_scope: usize,
  ) {
    let index = self.push_scope(module.name.clone(), parent_scope);

    location.modules.push(module.name.clone());

    for item in module.items.iter() {
      self.register_item(item, location, index);
    }

    location.modules.pop();
  }

  fn register_import(&mut self, import: &Import, location: &ResourceLocation, scope: usize) {
    let name = import
      .alias
      .clone()
      .unwrap_or_else(|| import.path.name.clone());
    let path = FunctionLocation::from_zoglin_resource(location, &import.path, false);
    self.add_import(scope, name, path.flatten());
  }

  fn register_function(&mut self, function: &Function, location: &ResourceLocation, scope: usize) {
    let function_path = location.join(&function.name);

    let function_location = FunctionLocation {
      module: location.clone(),
      name: function.name.clone(),
    };

    if function
      .parameters
      .iter()
      .any(|param| matches!(param.kind, ParameterKind::Scoreboard))
    {
      self.used_scoreboards.insert(
        ScoreboardLocation::new(function_location.clone().flatten(), "").scoreboard_string(),
      );
    }

    let definition = FunctionDefinition {
      location: function_location.clone(),
      arguments: function.parameters.clone(),
      return_type: function.return_type,
    };

    self.add_function(
      scope,
      function.name.clone(),
      function_location.clone().flatten(),
    );

    self
      .function_registry
      .insert(function_location.flatten(), definition);

    if &function.name == "tick" && location.modules.is_empty() {
      self.tick_functions.push(function_path);
    } else if &function.name == "load" && location.modules.is_empty() {
      self.load_functions.push(function_path);
    }
  }
}
