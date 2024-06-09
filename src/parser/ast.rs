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
  pub alias: Option<String>,
}

#[derive(Debug)]
pub struct Resource {
  pub is_asset: bool,
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
  Command(Command),
  Comment(String),
  Expression(Expression),
}

#[derive(Debug)]
pub struct Command {
  pub parts: Vec<CommandPart>,
}

#[derive(Debug)]
pub enum CommandPart {
  Literal(String),
  Expression(StaticExpr),
}

#[derive(Debug)]
pub enum StaticExpr {
  FunctionCall(FunctionCall),
  ResourceRef {
    resource: ZoglinResource,
    is_fn: bool,
  },
}

#[derive(Debug)]
pub enum Expression {
  FunctionCall(FunctionCall),
  Integer(i32),
  Variable(ZoglinResource),
  BinaryOperation(BinaryOperation),
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

#[derive(Debug)]
pub struct BinaryOperation {
  pub left: Box<Expression>,
  pub right: Box<Expression>,
  pub operator: Operator,
}


#[derive(Debug, Clone)]
pub enum Operator {
  Plus,
  Minus,
  Divide,
  Multiply,
  Modulo,
  Power,
  LeftShift,
  RightShift,
  LessThan,
  GreaterThan,
  LessThanEquals,
  GreaterThanEquals,
  Equal,
  NotEqual,
  LogicalAnd,
  LogicalOr,
  Assign,
  OperatorAssign(Box<Operator>)
}