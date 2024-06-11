use glob::glob;
use serde::Serialize;
use std::{fs, path::Path};

use crate::parser::ast::{self, ZoglinResource};

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
    pack_format: 26,
    description: "",
  },
};

impl FileTree {
  pub fn generate(&self, root_path: &String) {
    let _ = fs::remove_dir_all(root_path);
    let working_path = Path::new(root_path).join("data");
    fs::create_dir_all(&working_path).unwrap();

    let text = serde_json::to_string_pretty(&DEFAULT_MCMETA).unwrap();
    fs::write(Path::new(root_path).join("pack.mcmeta"), text).unwrap();

    for namespace in self.namespaces.iter() {
      namespace.generate(&root_path);
    }
  }
}

#[derive(Debug)]
pub struct Namespace {
  pub name: String,
  pub items: Vec<Item>,
}

impl Namespace {
  fn generate(&self, path: &str) {
    for item in self.items.iter() {
      item.generate(
        path,
        &ResourceLocation {
          namespace: self.name.clone(),
          modules: Vec::new(),
        },
      );
    }
  }
}

#[derive(Debug)]
pub enum Item {
  Ignored,
  Module(Module),
  Function(Function),
  TextResource(TextResource),
  FileResource(FileResource),
}

impl Item {
  fn generate(&self, root_path: &str, local_path: &ResourceLocation) {
    match self {
      Item::Ignored => {}
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
  fn generate(&self, root_path: &str, local_path: &ResourceLocation) {
    let mut local_path = local_path.clone();
    local_path.modules.push(self.name.clone());
    for item in self.items.iter() {
      item.generate(root_path, &mut local_path);
    }
  }
}

#[derive(Debug)]
pub struct Function {
  pub name: String,
  pub commands: Vec<String>,
}

impl Function {
  fn generate(&self, root_path: &str, local_path: &ResourceLocation) {
    let dir_path = Path::new(root_path)
      .join("data")
      .join(&local_path.namespace)
      .join("function")
      .join(local_path.modules.join("/"));

    fs::create_dir_all(&dir_path).unwrap();
    let file_path = dir_path.join(self.name.clone() + ".mcfunction");
    fs::write(file_path, self.commands.join("\n")).unwrap();
  }
}

#[derive(Debug)]
pub struct TextResource {
  pub name: String,
  pub kind: String,
  pub is_asset: bool,
  pub text: String,
}

impl TextResource {
  fn generate(&self, root_path: &str, local_path: &ResourceLocation) {
    let resource_dir = if self.is_asset { "assets" } else { "data" };
    let dir_path = Path::new(root_path)
      .join(resource_dir)
      .join(&local_path.namespace)
      .join(&self.kind)
      .join(local_path.modules.join("/"));

    fs::create_dir_all(&dir_path).unwrap();
    let file_path = dir_path.join(self.name.clone() + ".json");
    fs::write(file_path, self.text.clone()).unwrap();
  }
}

#[derive(Debug)]
pub struct FileResource {
  pub kind: String,
  pub is_asset: bool,
  pub path: String,
}

impl FileResource {
  fn generate(&self, root_path: &str, local_path: &ResourceLocation) {
    let resource_dir = if self.is_asset { "assets" } else { "data" };
    let dir_path = Path::new(&root_path)
      .join(resource_dir)
      .join(&local_path.namespace)
      .join(&self.kind)
      .join(local_path.modules.join("/"));

    fs::create_dir_all(&dir_path).unwrap();
    for entry in glob(&self.path).unwrap() {
      match entry {
        Ok(path) => {
          let filename = path.file_name().unwrap();
          if Path::new(&path).is_file() {
            fs::copy(&path, &dir_path.join(filename)).unwrap();
          }
        }
        Err(e) => panic!("{:?}", e),
      };
    }
  }
}

#[derive(Clone, Debug)]
pub struct ResourceLocation {
  pub namespace: String,
  pub modules: Vec<String>,
}

// foo:bar = "foo"
// foo/bar = None
// :foo/bar = ""

impl ResourceLocation {
  pub fn from_zoglin_resource(
    base_location: &ResourceLocation,
    resource: &ast::ZoglinResource,
  ) -> ResourceLocation {
    if let Some(mut namespace) = resource.namespace.clone() {
      if namespace.is_empty() {
        namespace = base_location.namespace.clone();
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

  pub fn to_string(&self) -> String {
    self.namespace.clone() + ":" + &self.modules.join("/")
  }

  pub fn join(&self, suffix: &String) -> String {
    let mut prefix = self.to_string();
    if self.modules.len() > 0 {
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

impl FunctionLocation {
  pub fn flatten(self) -> ResourceLocation {
    let mut result = self.module;
    result.modules.push(self.name);
    result
  }
}

#[derive(Clone, Debug)]
pub struct StorageLocation {
  pub storage: ResourceLocation,
  pub name: String,
}

impl StorageLocation {
  pub fn from_zoglin_resource(
    fn_loc: FunctionLocation,
    resource: &ZoglinResource,
  ) -> StorageLocation {
    StorageLocation::from_resource_location(ResourceLocation::from_zoglin_resource(
      &fn_loc.flatten(),
      resource,
    ))
  }

  fn from_resource_location(mut location: ResourceLocation) -> StorageLocation {
    let name = location.modules.pop().unwrap();
    StorageLocation {
      storage: location,
      name,
    }
  }

  pub fn to_string(&self) -> String {
    format!("{} {}", self.storage.to_string(), self.name)
  }
}
