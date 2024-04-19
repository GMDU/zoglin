use std::fs;

#[derive(Debug)]
pub struct FileTree {
  pub namespaces: Vec<Namespace>
}

const PACK_MCMETA: &str = r#"{
  "pack": {
    "pack_format": 26,
    "description": ""
  }
}"#;

impl FileTree {
  pub fn generate(&self, root_path: String) {
    let working_path = root_path.clone() + "/generated/data";
    fs::create_dir_all(&working_path).unwrap();
    fs::write(root_path + "/generated/pack.mcmeta", PACK_MCMETA).unwrap();
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
    let namespace_path = path + "/" + &self.name + "/functions";
    fs::create_dir_all(&namespace_path).unwrap();
    for item in self.items.iter() {
      item.generate(namespace_path.clone());
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
  fn generate(&self, path: String) {
    match self {
      Item::Module(module) => module.generate(path),
      Item::Function(function) => function.generate(path),
      Item::Resource(resource) => resource.generate(path),
    }
  }
}

#[derive(Debug)]
pub struct Module {
  pub name: String,
  pub items: Vec<Item>,
}

impl Module {
  fn generate(&self, path: String) {
    let module_path = path + "/" + &self.name;
    fs::create_dir(&module_path).unwrap();
    for item in self.items.iter() {
      item.generate(module_path.clone());
    }
  }
}

#[derive(Debug)]
pub struct Function {
  pub name: String,
  pub commands: Vec<String>,
}

impl Function {
  fn generate(&self, path: String) {
    let file_path = path + "/" + &self.name + ".mcfunction";
    fs::write(file_path, self.commands.join("\n")).unwrap();
  }
}

#[derive(Debug)]
pub struct Resource {
  pub name: String,
  pub lines: Vec<String>,
}

impl Resource {
  fn generate(&self, path: String) {
    let file_path = path + "/" + &self.name + ".json";
    fs::write(file_path, self.lines.join("\n")).unwrap();
  }
}
