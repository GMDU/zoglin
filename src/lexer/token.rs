#[derive(Debug, Clone)]

pub struct Token {
  pub kind: TokenKind,
  pub value: String,
  pub file: String,
  pub line: usize,
  pub column: usize,
}

#[derive(PartialEq, Debug, Clone)]
pub enum TokenKind {
  Invalid,
  EndOfFile,
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