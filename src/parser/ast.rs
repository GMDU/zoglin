use crate::error::Location;

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
  pub location: Location,
  pub kind: String,
  pub content: ResourceContent,
}

#[derive(Debug)]
pub enum ResourceContent {
  Text(String, String),
  File(String, String),
}

#[derive(Debug, Clone, Copy)]
pub enum ParameterKind {
  Storage,
  Scoreboard,
  Macro,
  CompileTime,
}

#[derive(Debug, Clone)]
pub struct Parameter {
  pub name: String,
  pub kind: ParameterKind,
}

#[derive(Debug)]
pub struct Function {
  pub location: Location,
  pub name: String,
  pub parameters: Vec<Parameter>,
  pub items: Vec<Statement>,
}

#[derive(Debug)]
pub enum Statement {
  Command(Command),
  Comment(String),
  Expression(Expression),
  IfStatement(IfStatement),
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
  Boolean(bool, Location),
  Byte(i8, Location),
  Short(i16, Location),
  Integer(i32, Location),
  Long(i64, Location),
  Float(f32, Location),
  Double(f64, Location),
  String(String, Location),
  Array(ArrayType, Vec<Expression>, Location),
  Compound(Vec<KeyValue>, Location),
  Variable(ZoglinResource),
  ScoreboardVariable(ZoglinResource),
  BinaryOperation(BinaryOperation),
}

impl Expression {
  pub fn location(&self) -> Location {
    match self {
      Expression::FunctionCall(FunctionCall {
        path: ZoglinResource { location, .. },
        ..
      })
      | Expression::Boolean(_, location)
      | Expression::Byte(_, location)
      | Expression::Short(_, location)
      | Expression::Integer(_, location)
      | Expression::Long(_, location)
      | Expression::Float(_, location)
      | Expression::Double(_, location)
      | Expression::String(_, location)
      | Expression::Array(_, _, location)
      | Expression::Compound(_, location)
      | Expression::Variable(ZoglinResource { location, .. })
      | Expression::ScoreboardVariable(ZoglinResource { location, .. })
      | Expression::BinaryOperation(BinaryOperation { location, .. }) => location.clone(),
    }
  }
}

#[derive(Debug)]
pub struct KeyValue {
  pub location: Location,
  pub key: String,
  pub value: Expression,
}

#[derive(Debug, Clone, Copy)]
pub enum ArrayType {
  Any,
  Byte,
  Int,
  Long,
}

#[derive(Debug)]
pub struct FunctionCall {
  pub path: ZoglinResource,
  pub arguments: Vec<Expression>,
}

#[derive(Debug)]
pub struct ZoglinResource {
  pub location: Location,
  pub namespace: Option<String>,
  pub modules: Vec<String>,
  pub name: String,
}

#[derive(Debug)]
pub struct BinaryOperation {
  pub location: Location,
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
  OperatorAssign(Box<Operator>),
}

#[derive(Debug)]
pub struct IfStatement {
  pub condition: Expression,
  pub block: Vec<Statement>,
  pub child: Option<ElseStatement>,
}

#[derive(Debug)]
pub enum ElseStatement {
  IfStatement(Box<IfStatement>),
  Block(Vec<Statement>),
}
