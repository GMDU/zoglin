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
  pub fn generate(&self, root_path: &String) -> Result<()> {
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
  pub name: String,
  pub items: Vec<Item>,
}

impl Namespace {
  fn generate(&self, path: &str) -> Result<()> {
    for item in self.items.iter() {
      item.generate(
        path,
        &ResourceLocation {
          namespace: self.name.clone(),
          modules: Vec::new(),
        },
      )?;
    }
    Ok(())
  }

  pub fn get_module(&mut self, mut path: Vec<String>) -> &mut Vec<Item> {
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
  pub name: String,
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

  fn get_module(&mut self, mut path: Vec<String>) -> &mut Vec<Item> {
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
  pub name: String,
  pub commands: Vec<String>,
  pub location: Location,
}

impl Function {
  fn generate(&self, root_path: &str, local_path: &ResourceLocation) -> Result<()> {
    let dir_path = Path::new(root_path)
      .join("data")
      .join(&local_path.namespace)
      .join("function")
      .join(local_path.modules.join("/"));

    fs::create_dir_all(&dir_path).map_err(raise_floating_error)?;
    let file_path = dir_path.join(self.name.clone() + ".mcfunction");
    fs::write(file_path, self.commands.join("\n")).map_err(raise_floating_error)
  }
}

#[derive(Debug)]
pub struct TextResource {
  pub name: String,
  pub kind: String,
  pub is_asset: bool,
  pub text: String,
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
    let dir_path = Path::new(root_path)
      .join(resource_dir)
      .join(&local_path.namespace)
      .join(&self.kind)
      .join(local_path.modules.join("/"));

    fs::create_dir_all(&dir_path).map_err(raise_floating_error)?;
    let file_path = dir_path.join(self.name.clone() + ".json");
    fs::write(file_path, self.text.clone()).map_err(raise_floating_error)
  }
}

#[derive(Debug)]
pub struct FileResource {
  pub kind: String,
  pub is_asset: bool,
  pub path: String,
  pub location: Location,
}

impl FileResource {
  fn generate(&self, root_path: &str, local_path: &ResourceLocation) -> Result<()> {
    let resource_dir = if self.is_asset { "assets" } else { "data" };
    let dir_path = Path::new(&root_path)
      .join(resource_dir)
      .join(&local_path.namespace)
      .join(&self.kind)
      .join(local_path.modules.join("/"));

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

#[derive(Clone, Debug)]
pub struct ResourceLocation {
  pub namespace: String,
  pub modules: Vec<String>,
}

impl Display for ResourceLocation {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}:{}", self.namespace, self.modules.join("/"))
  }
}

impl ResourceLocation {
  pub fn new(namespace: &str, modules: &[&str]) -> ResourceLocation {
    ResourceLocation {
      namespace: namespace.to_string(),
      modules: modules.iter().map(|module| module.to_string()).collect(),
    }
  }

  pub fn from_zoglin_resource(
    base_location: &ResourceLocation,
    resource: &ast::ZoglinResource,
    function_scoped: bool,
  ) -> ResourceLocation {
    if let Some(mut namespace) = resource.namespace.clone() {
      if namespace.is_empty() {
        namespace.clone_from(&base_location.namespace);
      } else if namespace == "~" {
        let mut location = base_location.clone();
        if function_scoped {
          location.modules.pop();
        }

        location.modules.extend(resource.modules.clone());
        location.modules.push(resource.name.clone());
        return location;
      }

      let mut modules = resource.modules.clone();
      modules.push(resource.name.clone());
      return ResourceLocation { namespace, modules };
    }
    let mut location = base_location.clone();

    location.modules.extend(resource.modules.clone());
    location.modules.push(resource.name.clone());
    location
  }

  pub fn join(&self, suffix: &str) -> String {
    let mut prefix = self.to_string();
    if !self.modules.is_empty() {
      prefix.push('/');
    }
    prefix.push_str(suffix);
    prefix
  }
}

#[derive(Clone, Debug)]
pub struct FunctionLocation {
  pub module: ResourceLocation,
  pub name: String,
}

impl Display for FunctionLocation {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.module.join(&self.name))
  }
}

impl FunctionLocation {
  pub fn new(module: ResourceLocation, name: &str) -> FunctionLocation {
    FunctionLocation {
      module,
      name: name.to_string(),
    }
  }

  pub fn flatten(self) -> ResourceLocation {
    let mut result = self.module;
    result.modules.push(self.name);
    result
  }

  pub fn from_zoglin_resource(
    base_location: &ResourceLocation,
    resource: &ZoglinResource,
    function_scoped: bool,
  ) -> FunctionLocation {
    let mut resource_location = ResourceLocation::from_zoglin_resource(base_location, resource, function_scoped);
    let name = resource_location
      .modules
      .pop()
      .expect("There will be at least one module");
    FunctionLocation {
      module: resource_location,
      name,
    }
  }
}

#[derive(Clone, Debug)]
pub struct StorageLocation {
  pub storage: ResourceLocation,
  pub name: String,
}

impl Display for StorageLocation {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{} {}", self.storage, self.name)
  }
}

impl StorageLocation {
  pub fn new(storage: ResourceLocation, name: String) -> StorageLocation {
    StorageLocation { storage, name }
  }

  pub fn from_zoglin_resource(
    fn_loc: FunctionLocation,
    resource: &ZoglinResource,
  ) -> StorageLocation {
    StorageLocation::from_resource_location(ResourceLocation::from_zoglin_resource(
      &fn_loc.flatten(),
      resource,
      true,
    ))
  }

  fn from_resource_location(mut location: ResourceLocation) -> StorageLocation {
    let name = location
      .modules
      .pop()
      .expect("There must be at least one module");
    StorageLocation {
      storage: location,
      name,
    }
  }
}

#[derive(Clone, Debug)]
pub struct ScoreboardLocation {
  pub scoreboard: Vec<String>,
  pub name: String,
}

impl Display for ScoreboardLocation {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{} {}", self.name, self.scoreboard.join("."))
  }
}

impl ScoreboardLocation {
  pub fn scoreboard_string(&self) -> String {
    self.scoreboard.join(".")
  }

  pub fn from_zoglin_resource(
    fn_loc: FunctionLocation,
    resource: &ZoglinResource,
  ) -> ScoreboardLocation {
    ScoreboardLocation::from_resource_location(ResourceLocation::from_zoglin_resource(
      &fn_loc.flatten(),
      resource,
      true,
    ))
  }

  fn from_resource_location(mut location: ResourceLocation) -> ScoreboardLocation {
    let name = location
      .modules
      .pop()
      .expect("There must be at least one module");
    let mut scoreboard = location.modules;
    scoreboard.insert(0, location.namespace);
    ScoreboardLocation {
      scoreboard,
      name: format!("${name}"),
    }
  }

  pub fn new(location: ResourceLocation, name: &str) -> ScoreboardLocation {
    let mut scoreboard = location.modules;
    scoreboard.insert(0, location.namespace);
    ScoreboardLocation {
      scoreboard,
      name: format!("${name}"),
    }
  }

  pub fn of_internal(name: &str) -> ScoreboardLocation {
    ScoreboardLocation {
      scoreboard: vec![
        "zoglin".to_string(),
        "internal".to_string(),
        "vars".to_string(),
      ],

      name: format!("${name}"),
    }
  }
}
