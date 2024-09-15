use std::mem::replace;

use ecow::EcoString;

use crate::error::Location;

#[derive(Debug)]
pub struct File {
  pub items: Vec<Namespace>,
}

#[derive(Debug)]
pub struct Namespace {
  pub name: EcoString,
  pub items: Vec<Item>,
}

#[derive(Debug)]
pub enum Item {
  None,
  Module(Module),
  Import(Import),
  Function(Function),
  ComptimeFunction(ComptimeFunction),
  Resource(Resource),
  ComptimeAssignment(EcoString, Expression),
}

impl Item {
  pub fn take(&mut self) -> Item {
    replace(self, Item::None)
  }
}

#[derive(Debug)]
pub struct Module {
  pub name: EcoString,
  pub items: Vec<Item>,
}

#[derive(Debug)]
pub struct Import {
  pub path: ImportPath,
  pub alias: Option<EcoString>,
}

#[derive(Debug)]
pub struct Resource {
  pub is_asset: bool,
  pub location: Location,
  pub kind: EcoString,
  pub content: ResourceContent,
}

#[derive(Debug)]
pub enum ResourceContent {
  Text(EcoString, EcoString),
  File(EcoString, EcoString),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParameterKind {
  Storage,
  Scoreboard,
  Macro,
  CompileTime,
}

#[derive(Debug, Clone)]
pub struct Parameter {
  pub name: EcoString,
  pub kind: ParameterKind,
}

#[derive(Debug)]
pub struct Function {
  pub location: Location,
  pub return_type: ReturnType,
  pub name: EcoString,
  pub parameters: Vec<Parameter>,
  pub items: Vec<Statement>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReturnType {
  Storage,
  Scoreboard,
  Direct,
}

#[derive(Debug)]
pub struct ComptimeFunction {
  pub name: EcoString,
  pub parameters: Vec<EcoString>,
  pub items: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum Statement {
  Command(Command),
  Comment(EcoString),
  Expression(Expression),
  If(IfStatement),
  WhileLoop(WhileLoop),
  Return(Option<Expression>),
}

#[derive(Debug, Clone)]
pub struct Command {
  pub parts: Vec<CommandPart>,
}

#[derive(Debug, Clone)]
pub enum CommandPart {
  Literal(EcoString),
  Expression(StaticExpr),
}

#[derive(Debug, Clone)]
pub enum StaticExpr {
  MacroVariable(EcoString),
  ComptimeVariable(EcoString),
  FunctionCall(FunctionCall),
  ResourceRef { resource: ZoglinResource },
  FunctionRef { path: Option<ZoglinResource> },
}

#[derive(Debug, Clone)]
pub enum Expression {
  FunctionCall(FunctionCall),
  Boolean(bool, Location),
  Byte(i8, Location),
  Short(i16, Location),
  Integer(i32, Location),
  Long(i64, Location),
  Float(f32, Location),
  Double(f64, Location),
  String(EcoString, Location),
  Array(ArrayType, Vec<Expression>, Location),
  Compound(Vec<KeyValue>, Location),
  BuiltinVariable(EcoString, Location),
  BuiltinFunction(EcoString, Vec<Expression>, Location),
  Variable(ZoglinResource),
  ScoreboardVariable(ZoglinResource),
  MacroVariable(EcoString, Location),
  ComptimeVariable(EcoString, Location),
  BinaryOperation(BinaryOperation),
  UnaryOperation(UnaryExpression),
  Index(Index),
  RangeIndex(RangeIndex),
  Member(Member),
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
      | Expression::BuiltinVariable(_, location)
      | Expression::BuiltinFunction(_, _, location)
      | Expression::ScoreboardVariable(ZoglinResource { location, .. })
      | Expression::MacroVariable(_, location)
      | Expression::ComptimeVariable(_, location)
      | Expression::BinaryOperation(BinaryOperation { location, .. })
      | Expression::UnaryOperation(UnaryExpression { location, .. }) => location.clone(),
      Expression::Index(index) => index.left.location(),
      Expression::RangeIndex(index) => index.left.location(),
      Expression::Member(member) => member.left.location(),
    }
  }
}

#[derive(Debug, Clone)]
pub struct Index {
  pub left: Box<Expression>,
  pub index: Box<Expression>,
}

#[derive(Debug, Clone)]
pub struct RangeIndex {
  pub left: Box<Expression>,
  pub start: Option<Box<Expression>>,
  pub end: Option<Box<Expression>>,
}

#[derive(Debug, Clone)]
pub struct Member {
  pub left: Box<Expression>,
  pub member: Box<MemberKind>,
}

#[derive(Debug, Clone)]
pub enum MemberKind {
  Literal(EcoString),
  Dynamic(Expression),
}

#[derive(Debug, Clone)]
pub struct KeyValue {
  pub location: Location,
  pub key: EcoString,
  pub value: Expression,
}

#[derive(Debug, Clone, Copy)]
pub enum ArrayType {
  Any,
  Byte,
  Int,
  Long,
}

#[derive(Debug, Clone)]
pub struct FunctionCall {
  pub comptime: bool,
  pub path: ZoglinResource,
  pub arguments: Vec<Expression>,
}

#[derive(Debug, Clone)]
pub struct ZoglinResource {
  pub location: Location,
  pub namespace: Option<EcoString>,
  pub modules: Vec<EcoString>,
  pub name: EcoString,
}

#[derive(Debug, Clone)]
pub struct ImportPath {
  pub namespace: EcoString,
  pub path: Vec<EcoString>,
  pub is_comptime: bool,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct UnaryExpression {
  pub location: Location,
  pub operator: UnaryOperator,
  pub operand: Box<Expression>,
}

#[derive(Debug, Clone)]
pub enum UnaryOperator {
  LogicalNot,
  Negation,
}

#[derive(Debug, Clone)]
pub struct IfStatement {
  pub condition: Expression,
  pub block: Vec<Statement>,
  pub child: Option<ElseStatement>,
}

#[derive(Debug, Clone)]
pub struct WhileLoop {
  pub condition: Expression,
  pub block: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum ElseStatement {
  IfStatement(Box<IfStatement>),
  Block(Vec<Statement>),
}
