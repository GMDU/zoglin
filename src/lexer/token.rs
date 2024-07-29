use crate::error;
use error::Location;

#[derive(Debug, Clone)]

pub struct Token {
  pub kind: TokenKind,
  pub value: String,
  pub location: Location,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum TokenKind {
  // Special
  EndOfFile,
  EndOfInclude,

  // Keywords
  NamespaceKeyword,
  FunctionKeyword,
  ModuleKeyword,
  ResourceKeyword,
  AssetKeyword,
  IncludeKeyword,
  ImportKeyword,
  AsKeyword,
  IfKeyword,
  ElseKeyword,
  WhileKeyword,
  TrueKeyword,
  FalseKeyword,
  ReturnKeyword,

  // Non-zoglin
  CommandBegin,
  CommandString,
  CommandEnd,
  Json,
  Comment,

  // Symbols
  LeftBrace,
  RightBrace,
  LeftSquare,
  RightSquare,
  LeftParen,
  RightParen,
  ForwardSlash,
  Colon,
  Dot,
  Semicolon,
  Comma,
  Plus,
  Minus,
  Star,
  Percent,
  DoubleStar,
  LeftShift,
  RightShift,
  LessThan,
  GreaterThan,
  LessThanEquals,
  GreaterThanEquals,
  DoubleEquals,
  BangEquals,
  Ampersand,
  DoubleAmpersand,
  DoublePipe,
  Bang,
  Equals,
  PlusEquals,
  MinusEquals,
  StarEquals,
  ForwardSlashEquals,
  PercentEquals,
  Dollar,

  // Values
  Identifier,
  Byte,
  Short,
  Long,
  Integer,
  Float,
  Double,
  String,
}
