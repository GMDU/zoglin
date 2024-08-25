use crate::parser::ast::{
  self, File, Function, Import, Item, Module, Namespace, ParameterKind, ReturnType,
};

use super::{
  file_tree::{ResourceLocation, ScoreboardLocation},
  scope::{FunctionDefinition, Scope},
  Compiler, FunctionContext,
};

impl Compiler {
  pub fn register(&mut self, ast: &mut File) {
    self.scopes.push(Scope::new(0));
    for namespace in ast.items.iter_mut() {
      self.register_namespace(namespace, 0);
    }
  }

  fn register_namespace(&mut self, namespace: &mut Namespace, parent_scope: usize) {
    let index = self.push_scope(namespace.name.clone(), parent_scope);

    for item in namespace.items.iter_mut() {
      let mut resource = ResourceLocation::new_module(&namespace.name, &[]);
      self.register_item(item, &mut resource, index);
    }
  }

  fn register_item(
    &mut self,
    item: &mut Item,
    location: &mut ResourceLocation,
    parent_scope: usize,
  ) {
    match item {
      Item::Module(module) => self.register_module(module, location, parent_scope),

      Item::Import(import) => self.register_import(import, location, parent_scope),

      Item::Function(function) => self.register_function(function, location, parent_scope),

      Item::Resource(_) => {}

      Item::ComptimeAssignment(_, _) => {
        let Item::ComptimeAssignment(name, value) = item.take() else {
          unreachable!()
        };
        self.register_comptime_assignment(name, value, location, parent_scope)
      }
      Item::None => {}
    }
  }

  fn register_module(
    &mut self,
    module: &mut Module,
    location: &mut ResourceLocation,
    parent_scope: usize,
  ) {
    let index = self.push_scope(module.name.clone(), parent_scope);

    location.modules.push(module.name.clone());

    for item in module.items.iter_mut() {
      self.register_item(item, location, index);
    }

    location.modules.pop();
  }

  fn register_import(&mut self, import: &Import, location: &ResourceLocation, scope: usize) {
    let name = import
      .alias
      .clone()
      .unwrap_or_else(|| import.path.name.clone());
    let path = ResourceLocation::from_zoglin_resource(location, &import.path, false);
    self.add_import(scope, name, path);
  }

  fn register_function(&mut self, function: &Function, location: &ResourceLocation, scope: usize) {
    let function_path = location.join(&function.name);

    let function_location = location.clone().with_name(&function.name);

    if function
      .parameters
      .iter()
      .any(|param| matches!(param.kind, ParameterKind::Scoreboard))
    {
      self
        .used_scoreboards
        .insert(ScoreboardLocation::new(function_location.clone(), "").scoreboard_string());
    }

    let definition = FunctionDefinition {
      location: function_location.clone(),
      arguments: function.parameters.clone(),
      return_type: function.return_type,
    };

    self.add_function(scope, function.name.clone(), function_location.clone());

    self.function_registry.insert(function_location, definition);

    if &function.name == "tick" && location.modules.is_empty() {
      self.tick_functions.push(function_path);
    } else if &function.name == "load" && location.modules.is_empty() {
      self.load_functions.push(function_path);
    }
  }

  fn register_comptime_assignment(
    &mut self,
    name: String,
    value: ast::Expression,
    location: &ResourceLocation,
    scope: usize,
  ) {
    let mut context = FunctionContext {
      location: location.clone(),
      return_type: ReturnType::Direct,
      is_nested: false,
      has_nested_returns: false,
      code: Vec::new(),
    };
    // TODO: Add some sort of validation
    let compiled_value = self
      .compile_expression(value, &mut context, false)
      .expect("TODO: return error");
    self.scopes[scope]
      .comptime_values
      .insert(name, compiled_value);
  }
}
