use crate::lexer::token::{Token, TokenKind};
use self::ast::{File, Function, Item, Module, Namespace};

pub mod ast;

pub struct Parser {
  tokens: Vec<Token>,
  position: usize
}

impl Parser {
  pub fn new(tokens: Vec<Token>) -> Parser {
    Parser{tokens, position: 0}
  }

  pub fn parse(&mut self) -> File {
    let mut items = Vec::new();
    
    while !self.eof() {
      items.push(self.parse_namespace());
    }

    File{items}
  }

  fn eof(&self) -> bool {
    // self.position > self.tokens.len()
    self.current().kind == TokenKind::EndOfFile
  }

  fn current(&self) -> Token {
    self.tokens[self.position].clone()
  }

  fn consume(&mut self) -> Token {
    let current = self.current();
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
    Namespace{name, items}
  }
  
  fn parse_item(&mut self) -> ast::Item {
    if self.current().kind == TokenKind::ModuleKeyword {
      return Item::Module(self.parse_module());
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
    Module{name, items}
  }
    
  fn parse_function(&mut self) -> ast::Function {
    self.expect(TokenKind::FunctionKeyword);
    let name = self.expect(TokenKind::Identifier).value;
    self.expect(TokenKind::LeftParen);
    self.expect(TokenKind::RightParen);
    self.expect(TokenKind::LeftBrace);
    let mut items = Vec::new();
    while self.current().kind != TokenKind::RightBrace {
      items.push(self.parse_statement());
    }
    self.expect(TokenKind::RightBrace);
    Function{name, items}
  }
  
  fn parse_statement(&mut self) -> ast::Statement {
    let command = self.expect(TokenKind::Command).value;
    ast::Statement::Command(command)
  }
}