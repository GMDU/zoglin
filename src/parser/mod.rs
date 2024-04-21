use self::ast::{File, Function, Item, Module, Namespace};
use crate::lexer::token::{Token, TokenKind};

pub mod ast;
mod json;

pub struct Parser {
  tokens: Vec<Token>,
  position: usize,
}

impl Parser {
  pub fn new(tokens: Vec<Token>) -> Parser {
    Parser {
      tokens,
      position: 0,
    }
  }

  pub fn parse(&mut self) -> File {
    let mut items = Vec::new();

    while !self.eof() {
      items.push(self.parse_namespace());
    }

    File { items }
  }

  fn eof(&mut self) -> bool {
    // self.position > self.tokens.len()
    self.current().kind == TokenKind::EndOfFile
  }

  fn current(&mut self) -> Token {
    while self.tokens[self.position].kind == TokenKind::Comment {
      self.position += 1;
    }
    self.current_with_comments()
  }
  
  fn current_with_comments(&self) -> Token {
    self.tokens[self.position].clone()
  }

  fn consume(&mut self) -> Token {
    let current = self.current();
    self.position += 1;
    current
  }

  fn consume_with_comments(&mut self) -> Token {
    let current = self.current_with_comments();
    self.position += 1;
    current
  }

  fn expect(&mut self, kind: TokenKind) -> Token {
    let next = self.consume();
    assert_eq!(next.kind, kind);
    next
  }

  fn parse_namespace(&mut self) -> ast::Namespace {
    self.expect(TokenKind::NamespaceKeyword);
    let name = self.expect(TokenKind::Identifier).value;
    self.expect(TokenKind::LeftBrace);
    let mut items = Vec::new();
    while self.current().kind != TokenKind::RightBrace {
      items.push(self.parse_item());
    }
    self.expect(TokenKind::RightBrace);
    Namespace { name, items }
  }

  fn parse_item(&mut self) -> ast::Item {
    if self.current().kind == TokenKind::ModuleKeyword {
      return Item::Module(self.parse_module());
    } else if self.current().kind == TokenKind::ResourceKeyword {
      return Item::Resource(self.parse_resource());
    } else {
      return Item::Function(self.parse_function());
    }
  }

  fn parse_module(&mut self) -> ast::Module {
    self.expect(TokenKind::ModuleKeyword);
    let name = self.expect(TokenKind::Identifier).value;
    self.expect(TokenKind::LeftBrace);
    let mut items = Vec::new();
    while self.current().kind != TokenKind::RightBrace {
      items.push(self.parse_item());
    }
    self.expect(TokenKind::RightBrace);
    Module { name, items }
  }

  fn parse_resource(&mut self) -> ast::Resource {
    self.expect(TokenKind::ResourceKeyword);
    let kind = self.parse_resource_path();
    let name = self.expect(TokenKind::Identifier).value;
    let json = self.expect(TokenKind::JSON).value;

    let text = json::from_json5(&json);

    ast::Resource { name, kind, text }
  }

  fn parse_resource_path(&mut self) -> String {
    let mut text = self.expect(TokenKind::Identifier).value;
    while self.current().kind == TokenKind::ForwardSlash {
      text.push('/');
      self.consume();
      text.push_str(&self.expect(TokenKind::Identifier).value);
    }
    text
  }

  fn parse_function(&mut self) -> ast::Function {
    self.expect(TokenKind::FunctionKeyword);
    let name = self.expect(TokenKind::Identifier).value;
    self.expect(TokenKind::LeftParen);
    self.expect(TokenKind::RightParen);
    self.expect(TokenKind::LeftBrace);
    let mut items = Vec::new();
    while self.current_with_comments().kind != TokenKind::RightBrace {
      items.push(self.parse_statement());
    }
    self.expect(TokenKind::RightBrace);
    Function { name, items }
  }

  fn parse_statement(&mut self) -> ast::Statement {
    match self.current_with_comments().kind {
      TokenKind::Command => {
        let command = self.consume_with_comments().value;
        ast::Statement::Command(command)
      }
      TokenKind::Comment => {
        let comment = self.consume_with_comments().value;
        ast::Statement::Comment(comment)
      }
      _ => unreachable!()
    }
  }
}
