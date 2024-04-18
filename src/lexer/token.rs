#[derive(Debug, Clone)]

pub struct Token {
  pub kind: TokenKind,
  pub value: String
}

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub enum TokenKind {
  Invalid,
  EndOfFile,
  NamespaceKeyword,
  FunctionKeyword,
  ModuleKeyword,
  ResourceKeyword,
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
}