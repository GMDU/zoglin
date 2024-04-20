use std::{fs, path::Path};

use serde::Serialize;

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
    let working_path = root_path.clone() + "/data";
    fs::create_dir_all(&working_path).unwrap();
    let text = serde_json::to_string_pretty(&DEFAULT_MCMETA).unwrap();
    fs::write(root_path.clone() + "/pack.mcmeta", text).unwrap();
    for namespace in self.namespaces.iter() {
      namespace.generate(working_path.clone());
    }
  }
}

#[derive(Debug)]
pub struct Namespace {
  pub name: String,
  pub items: Vec<Item>,
}

impl Namespace {
  fn generate(&self, path: String) {
    for item in self.items.iter() {
      item.generate(
        path.clone(),
        ResourceLocation {
          namespace: self.name.clone(),
          modules: Vec::new(),
        },
      );
    }
  }
}

#[derive(Debug)]
pub enum Item {
  Module(Module),
  Function(Function),
  Resource(Resource),
}

impl Item {
  fn generate(&self, root_path: String, local_path: ResourceLocation) {
    match self {
      Item::Module(module) => module.generate(root_path, local_path),
      Item::Function(function) => function.generate(root_path, local_path),
      Item::Resource(resource) => resource.generate(root_path, local_path),
    }
  }
}

#[derive(Debug)]
pub struct Module {
  pub name: String,
  pub items: Vec<Item>,
}

impl Module {
  fn generate(&self, root_path: String, mut local_path: ResourceLocation) {
    local_path.modules.push(self.name.clone());
    for item in self.items.iter() {
      item.generate(root_path.clone(), local_path.clone());
    }
  }
}

#[derive(Debug)]
pub struct Function {
  pub name: String,
  pub commands: Vec<String>,
}

impl Function {
  fn generate(&self, root_path: String, local_path: ResourceLocation) {
    let dir_path = Path::new(&root_path)
      .join(local_path.namespace)
      .join("functions")
      .join(local_path.modules.join("/"));

    fs::create_dir_all(&dir_path).unwrap();
    let file_path = dir_path.join(self.name.clone() + ".mcfunction");
    fs::write(file_path, self.commands.join("\n")).unwrap();
  }
}

#[derive(Debug)]
pub struct Resource {
  pub name: String,
  pub kind: String,
  pub text: String,
}

impl Resource {
  fn generate(&self, root_path: String, local_path: ResourceLocation) {
    let dir_path = Path::new(&root_path)
      .join(local_path.namespace)
      .join(&self.kind)
      .join(local_path.modules.join("/"));

    fs::create_dir_all(&dir_path).unwrap();
    let file_path = dir_path.join(self.name.clone() + ".json");
    fs::write(file_path, self.text.clone()).unwrap();
  }
}

#[derive(Clone)]
struct ResourceLocation {
  pub namespace: String,
  pub modules: Vec<String>,
}
