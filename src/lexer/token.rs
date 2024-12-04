use crate::error;
use ecow::EcoString;
use error::Location;

#[derive(Debug, Clone)]
pub struct Token {
  pub kind: TokenKind,
  pub value: Option<EcoString>,
  pub raw: EcoString,
  pub location: Location,
}

impl Token {
  pub fn get_value(&self) -> &EcoString {
    self.value.as_ref().unwrap_or(&self.raw)
  }

  pub fn take_value(self) -> EcoString {
    self.value.unwrap_or(self.raw)
  }
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
  CommandBegin(bool),
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
  DoubleDot,
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
  Tilde,
  Equals,
  PlusEquals,
  MinusEquals,
  StarEquals,
  ForwardSlashEquals,
  PercentEquals,
  Dollar,

  // Values
  Identifier,
  BuiltinName,
  Byte,
  Short,
  Long,
  Integer,
  Float,
  Double,
  String,
}
