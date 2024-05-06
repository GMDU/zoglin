#[derive(Debug)]
pub struct File {
  pub items: Vec<Namespace>,
}

#[derive(Debug)]
pub struct Namespace {
  pub name: String,
  pub items: Vec<Item>,
}

#[derive(Debug)]
pub enum Item {
  Module(Module),
  Import(Import),
  Function(Function),
  Resource(Resource),
}

#[derive(Debug)]
pub struct Module {
  pub name: String,
  pub items: Vec<Item>,
}

#[derive(Debug)]
pub struct Import {
  pub path: ZoglinResource,
}

#[derive(Debug)]
pub struct Resource {
  pub kind: String,
  pub content: ResourceContent,
}

#[derive(Debug)]
pub enum ResourceContent {
  Text(String, String),
  File(String, String),
}

#[derive(Debug)]
pub struct Function {
  pub name: String,
  pub items: Vec<Statement>,
}

#[derive(Debug)]
pub enum Statement {
  Command(String),
  Comment(String),
  Expression(Expression),
}

#[derive(Debug)]
pub enum Expression {
  FunctionCall(FunctionCall),
}

#[derive(Debug)]
pub struct FunctionCall {
  pub path: ZoglinResource,
}

#[derive(Debug)]
pub struct ZoglinResource {
  pub namespace: Option<String>,
  pub modules: Vec<String>,
  pub name: String,
}
