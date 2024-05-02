use error::Location;
use crate::error;

#[derive(Debug, Clone)]

pub struct Token {
  pub kind: TokenKind,
  pub value: String,
  pub location: Location,
}

#[derive(PartialEq, Debug, Clone)]
pub enum TokenKind {
  Invalid,
  EndOfFile,
  EndOfInclude,
  NamespaceKeyword,
  FunctionKeyword,
  ModuleKeyword,
  ResourceKeyword,
  IncludeKeyword,
  Command,
  Comment,
  Identifier,
  LeftBrace,
  RightBrace,
  LeftSquare,
  RightSquare,
  LeftParen,
  RightParen,
  ForwardSlash,
  Colon,
  Dot,
  Integer,
  JSON,
  String,
}