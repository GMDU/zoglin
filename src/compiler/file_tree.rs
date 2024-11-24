use ecow::{eco_format, EcoString};
use glob::glob;
use serde::Serialize;
use std::{fmt::Display, fs, path::Path};

use crate::{
  error::{raise_error, raise_floating_error, Location, Result},
  parser::ast::{self, ZoglinResource},
};

#[derive(Debug)]
pub struct FileTree {
  pub namespaces: Vec<Namespace>,
}

#[derive(Serialize)]
struct PackMcmeta {
  pack: Pack,
}

#[derive(Serialize)]
struct Pack {
  pack_format: usize,
  description: &'static str,
}

const DEFAULT_MCMETA: PackMcmeta = PackMcmeta {
  pack: Pack {
    pack_format: 48,
    description: "",
  },
};

impl FileTree {
  pub fn generate(&self, root_path: &str) -> Result<()> {
    let _ = fs::remove_dir_all(root_path);
    let working_path = Path::new(root_path).join("data");
    fs::create_dir_all(working_path).map_err(raise_floating_error)?;

    let text = serde_json::to_string_pretty(&DEFAULT_MCMETA).expect("Json is valid");
    fs::write(Path::new(root_path).join("pack.mcmeta"), text).map_err(raise_floating_error)?;

    for namespace in self.namespaces.iter() {
      namespace.generate(root_path)?;
    }
    Ok(())
  }
}

#[derive(Debug)]
pub struct Namespace {
  pub name: EcoString,
  pub items: Vec<Item>,
}

impl Namespace {
  fn generate(&self, path: &str) -> Result<()> {
    for item in self.items.iter() {
      item.generate(path, &ResourceLocation::new_module(&self.name, &[]))?;
    }
    Ok(())
  }

  pub fn get_module(&mut self, mut path: Vec<EcoString>) -> &mut Vec<Item> {
    if path.is_empty() {
      return &mut self.items;
    }

    let first = path.remove(0);

    if let Some(index) = self
      .items
      .iter()
      .position(|item| matches!(item, Item::Module(module) if module.name == first))
    {
      let Item::Module(module) = &mut self.items[index] else {
        unreachable!();
      };
      return module.get_module(path);
    };

    self.items.push(Item::Module(Module {
      name: first,
      items: Vec::new(),
    }));
    let Some(Item::Module(module)) = self.items.last_mut() else {
      unreachable!("Module was just added");
    };

    module.get_module(path)
  }
}

#[derive(Debug)]
pub enum Item {
  Module(Module),
  Function(Function),
  TextResource(TextResource),
  FileResource(FileResource),
}

impl Item {
  fn generate(&self, root_path: &str, local_path: &ResourceLocation) -> Result<()> {
    match self {
      Item::Module(module) => module.generate(root_path, local_path),
      Item::Function(function) => function.generate(root_path, local_path),
      Item::TextResource(resource) => resource.generate(root_path, local_path),
      Item::FileResource(resource) => resource.generate(root_path, local_path),
    }
  }
}

#[derive(Debug)]
pub struct Module {
  pub name: EcoString,
  pub items: Vec<Item>,
}

impl Module {
  fn generate(&self, root_path: &str, local_path: &ResourceLocation) -> Result<()> {
    let mut local_path = local_path.clone();
    local_path.modules.push(self.name.clone());
    for item in self.items.iter() {
      item.generate(root_path, &local_path)?;
    }
    Ok(())
  }

  fn get_module(&mut self, mut path: Vec<EcoString>) -> &mut Vec<Item> {
    if path.is_empty() {
      return &mut self.items;
    }

    let first = path.remove(0);

    if let Some(index) = self
      .items
      .iter()
      .position(|item| matches!(item, Item::Module(module) if module.name == first ))
    {
      let Item::Module(module) = &mut self.items[index] else {
        unreachable!();
      };
      return module.get_module(path);
    };

    self.items.push(Item::Module(Module {
      name: first,
      items: Vec::new(),
    }));
    let Some(Item::Module(module)) = self.items.last_mut() else {
      unreachable!("Module was just added");
    };

    module.get_module(path)
  }
}

#[derive(Debug)]
pub struct Function {
  pub name: EcoString,
  pub commands: Vec<EcoString>,
  pub location: Location,
}

impl Function {
  fn generate(&self, root_path: &str, local_path: &ResourceLocation) -> Result<()> {
    let dir_path = Path::new(root_path)
      .join("data")
      .join(local_path.namespace.as_str())
      .join("function")
      .join(local_path.modules.join("/"));

    fs::create_dir_all(&dir_path).map_err(raise_floating_error)?;
    let file_path = dir_path.join((self.name.clone() + ".mcfunction").as_str());
    fs::write(file_path, self.commands.join("\n")).map_err(raise_floating_error)
  }
}

#[derive(Debug)]
pub struct TextResource {
  pub name: EcoString,
  pub kind: EcoString,
  pub is_asset: bool,
  pub text: EcoString,
  pub location: Location,
}

impl PartialEq for TextResource {
  fn eq(&self, other: &Self) -> bool {
    self.name == other.name && self.kind == other.kind && self.is_asset == other.is_asset
  }
}

impl TextResource {
  fn generate(&self, root_path: &str, local_path: &ResourceLocation) -> Result<()> {
    let resource_dir = if self.is_asset { "assets" } else { "data" };
    let mut dir_path = Path::new(root_path)
      .join(resource_dir)
      .join(local_path.namespace.as_str());

    if self.kind.as_str() != "." {
      dir_path.push(self.kind.as_str());
    }

    dir_path.push(local_path.modules.join("/"));

    fs::create_dir_all(&dir_path).map_err(raise_floating_error)?;
    let file_path = dir_path.join((self.name.clone() + ".json").as_str());
    fs::write(file_path, self.text.as_str()).map_err(raise_floating_error)
  }
}

#[derive(Debug)]
pub struct FileResource {
  pub kind: EcoString,
  pub is_asset: bool,
  pub path: EcoString,
  pub location: Location,
}

impl FileResource {
  fn generate(&self, root_path: &str, local_path: &ResourceLocation) -> Result<()> {
    let resource_dir = if self.is_asset { "assets" } else { "data" };
    let mut dir_path = Path::new(&root_path)
      .join(resource_dir)
      .join(local_path.namespace.as_str());

    if self.kind.as_str() != "." {
      dir_path.push(self.kind.as_str());
    }

    dir_path.push(local_path.modules.join("/"));

    fs::create_dir_all(&dir_path).map_err(raise_floating_error)?;
    for entry in glob(&self.path).map_err(|e| raise_error(self.location.clone(), e.msg))? {
      match entry {
        Ok(path) => {
          let filename = path.file_name().expect("Path should be valid");
          if Path::new(&path).is_file() {
            fs::copy(&path, dir_path.join(filename)).map_err(raise_floating_error)?;
          }
        }
        Err(e) => return Err(raise_floating_error(e)),
      };
    }

    Ok(())
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceLocation {
  pub namespace: EcoString,
  pub modules: Vec<EcoString>,
  kind: ResourceKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ResourceKind {
  Function,
  Module,
}

impl Display for ResourceLocation {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}:{}", self.namespace, self.modules.join("/"))
  }
}

impl ResourceLocation {
  pub fn from_zoglin_resource(
    base_location: &ResourceLocation,
    resource: &ast::ZoglinResource,
  ) -> ResourceLocation {
    let base_location = base_location.clone();

    if let Some(mut namespace) = resource.namespace.clone() {
      if namespace.is_empty() {
        namespace.clone_from(&base_location.namespace);
      } else if namespace == "~" {
        let mut location = base_location.module();

        location.modules.extend(resource.modules.clone());
        let name = if resource.name.is_empty() {
          &location
            .modules
            .pop()
            .expect("TODO: Make this work for namespaces")
        } else {
          &resource.name
        };
        return location.with_name(&name);
      }

      let modules = resource.modules.clone();
      return ResourceLocation {
        namespace,
        modules,
        kind: ResourceKind::Module,
      }
      .with_name(&resource.name);
    }

    let mut location = base_location;
    location.modules.extend(resource.modules.clone());
    location.with_name(&resource.name)
  }

  pub fn join(&self, suffix: &str) -> EcoString {
    let mut prefix = self.to_string();
    if !self.modules.is_empty() {
      prefix.push('/');
    }
    prefix.push_str(suffix);

    prefix.into()
  }

  pub fn module(mut self) -> ResourceLocation {
    match self.kind {
      ResourceKind::Function => {
        self.modules.pop().expect("Should have a name");
        self.kind = ResourceKind::Module;
        self
      }
      ResourceKind::Module => self,
    }
  }

  pub fn try_split(mut self) -> Option<(ResourceLocation, EcoString)> {
    match self.kind {
      ResourceKind::Function => {
        let name = self.modules.pop().expect("Should have a name");
        self.kind = ResourceKind::Function;
        Some((self, name))
      }
      ResourceKind::Module => None,
    }
  }

  pub fn _name(&self) -> &EcoString {
    self.modules.last().expect("Should have a name")
  }

  pub fn with_name(mut self, name: &str) -> ResourceLocation {
    self.modules.push(name.into());
    ResourceLocation {
      namespace: self.namespace,
      modules: self.modules,
      kind: ResourceKind::Function,
    }
  }

  pub fn new_module(namespace: &str, modules: &[&str]) -> Self {
    Self {
      namespace: namespace.into(),
      modules: modules
        .into_iter()
        .map(|&module| EcoString::from(module))
        .collect(),
      kind: ResourceKind::Module,
    }
  }

  pub fn new_function(namespace: &str, modules: &[&str]) -> Self {
    if modules.is_empty() {
      panic!("Should not construct function locations with no name");
    }
    Self {
      namespace: namespace.into(),
      modules: modules
        .into_iter()
        .map(|&module| EcoString::from(module))
        .collect(),
      kind: ResourceKind::Function,
    }
  }
}

#[derive(Clone, Debug)]
pub struct StorageLocation {
  pub storage: ResourceLocation,
  pub name: EcoString,
}

impl Display for StorageLocation {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{} {}", self.storage, self.name)
  }
}

impl StorageLocation {
  pub fn new(storage: ResourceLocation, name: EcoString) -> StorageLocation {
    StorageLocation { storage, name }
  }

  pub fn from_zoglin_resource(
    fn_loc: &ResourceLocation,
    resource: &ZoglinResource,
  ) -> StorageLocation {
    StorageLocation::from_function_location(ResourceLocation::from_zoglin_resource(
      fn_loc, resource,
    ))
  }

  fn from_function_location(location: ResourceLocation) -> StorageLocation {
    let (module, name) = location.try_split().expect("Should have a name");
    StorageLocation {
      storage: module,
      name,
    }
  }
}

#[derive(Clone, Debug)]
pub struct ScoreboardLocation {
  pub scoreboard: ResourceLocation,
  pub name: EcoString,
}

impl Display for ScoreboardLocation {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{} {}.{}",
      self.name,
      self.scoreboard.namespace,
      self.scoreboard.modules.join(".")
    )
  }
}

impl ScoreboardLocation {
  pub fn scoreboard_string(&self) -> EcoString {
    eco_format!(
      "{}.{}",
      self.scoreboard.namespace,
      self.scoreboard.modules.join(".")
    )
  }

  pub fn from_zoglin_resource(
    fn_loc: &ResourceLocation,
    resource: &ZoglinResource,
  ) -> ScoreboardLocation {
    ScoreboardLocation::from_function_location(ResourceLocation::from_zoglin_resource(
      fn_loc, resource,
    ))
  }

  fn from_function_location(location: ResourceLocation) -> ScoreboardLocation {
    let (module, name) = location.try_split().expect("Function location");
    ScoreboardLocation {
      scoreboard: module,
      name: eco_format!("{name}"),
    }
  }

  pub fn new(location: ResourceLocation, name: &str) -> ScoreboardLocation {
    ScoreboardLocation {
      scoreboard: location,
      name: eco_format!("{name}"),
    }
  }

  pub fn of_internal(name: &str) -> ScoreboardLocation {
    ScoreboardLocation {
      scoreboard: ResourceLocation::new_function("zoglin", &["internal", "vars"]),
      name: eco_format!("{name}"),
    }
  }
}
