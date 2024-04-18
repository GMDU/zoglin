#[derive(Debug)]
pub struct File {
  pub items: Vec<Namespace>
}

#[derive(Debug)]
pub struct Namespace {
  pub name: String,
  pub items: Vec<Item>
}

#[derive(Debug)]
pub enum Item {
  Module(Module),
  Function(Function)
}

#[derive(Debug)]
pub struct Module {
  pub name: String,
  pub items: Vec<Item>
}

#[derive(Debug)]
pub struct Function {
  pub name: String,
  pub items: Vec<Statement>
}

#[derive(Debug)]
pub enum Statement {
  Command(String)
}