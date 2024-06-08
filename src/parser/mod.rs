use self::ast::{
  Expression, File, Function, FunctionCall, Import, Item, Module, Namespace, Resource,
  ResourceContent, Statement, ZoglinResource,
};
use crate::{
  error::{raise_error, Result},
  lexer::token::{Token, TokenKind},
};

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

  pub fn parse(&mut self) -> Result<File> {
    let mut items = Vec::new();

    while !self.eof() {
      items.extend(self.parse_namespace()?);
    }

    Ok(File { items })
  }

  fn eof(&mut self) -> bool {
    // self.position > self.tokens.len()
    self.current().kind == TokenKind::EndOfFile
  }

  fn should_skip(&self, offset: usize, ignore: &[TokenKind]) -> bool {
    if ignore.contains(&self.tokens[self.position + offset].kind) {
      return false;
    }

    self.tokens[self.position + offset].kind == TokenKind::Comment
      || self.tokens[self.position + offset].kind == TokenKind::EndOfInclude
  }

  fn is(&self, kinds: &[TokenKind]) -> bool {
    kinds.contains(&self.current_including(kinds).kind)
  }

  fn current(&self) -> Token {
    let mut offset = 0;
    while self.should_skip(offset, &[]) {
      offset += 1;
    }
    self.tokens[self.position + offset].clone()
  }

  fn current_including(&self, kinds: &[TokenKind]) -> Token {
    let mut offset = 0;
    while self.should_skip(offset, kinds) {
      offset += 1;
    }
    self.tokens[self.position + offset].clone()
  }

  fn consume(&mut self) -> Token {
    let current = self.current();
    while self.should_skip(0, &[]) {
      self.position += 1;
    }
    self.position += 1;
    current
  }

  fn consume_including(&mut self, kinds: &[TokenKind]) -> Token {
    let current = self.current_including(kinds);
    while self.should_skip(0, kinds) {
      self.position += 1;
    }
    self.position += 1;
    current
  }

  fn expect(&mut self, kind: TokenKind) -> Result<Token> {
    let next = self.consume();
    if next.kind != kind {
      return Err(raise_error(
        next.location,
        &format!("Expected {:?}, got {:?}", kind, next.kind),
      ));
    }
    Ok(next)
  }

  fn parse_namespace(&mut self) -> Result<Vec<Namespace>> {
    let file = self.expect(TokenKind::NamespaceKeyword)?.location.file;
    let name = self.expect(TokenKind::Identifier)?.value;

    if self.current().kind == TokenKind::LeftBrace {
      return Ok(vec![self.parse_block_namespace(name)?]);
    }
    let mut namespaces = Vec::new();

    let mut items = Vec::new();
    while !self.is_namespace_end(&file) {
      if self.current().kind == TokenKind::NamespaceKeyword {
        namespaces.extend(self.parse_namespace()?);
      } else {
        items.push(self.parse_item()?);
      }
    }

    if self.is(&[TokenKind::EndOfInclude]) {
      self.consume_including(&[TokenKind::EndOfInclude]);
    }

    namespaces.push(Namespace { items, name });

    Ok(namespaces)
  }

  fn is_namespace_end(&mut self, file: &String) -> bool {
    let next = self.current_including(&[TokenKind::EndOfInclude, TokenKind::EndOfFile]);
    if next.kind == TokenKind::EndOfFile {
      return true;
    }
    if next.kind == TokenKind::EndOfInclude {
      if next.location.file == *file {
        return true;
      }
      self.consume_including(&[TokenKind::EndOfInclude]);
      return self.is_namespace_end(file);
    }
    false
  }

  fn parse_block_namespace(&mut self, name: String) -> Result<Namespace> {
    self.expect(TokenKind::LeftBrace)?;

    let mut items = Vec::new();
    while self.current().kind != TokenKind::RightBrace {
      items.push(self.parse_item()?);
    }
    self.expect(TokenKind::RightBrace)?;

    Ok(Namespace { name, items })
  }

  fn parse_item(&mut self) -> Result<Item> {
    Ok(match self.current().kind {
      TokenKind::ModuleKeyword => Item::Module(self.parse_module()?),
      TokenKind::ImportKeyword => Item::Import(self.parse_import()?),
      TokenKind::ResourceKeyword | TokenKind::AssetKeyword => Item::Resource(self.parse_resource()?),
      TokenKind::FunctionKeyword => Item::Function(self.parse_function()?),
      _ => {
        return Err(raise_error(
          self.current().location,
          &format!("Unexpected token kind: {:?}", self.current().kind),
        ))
      }
    })
  }

  fn parse_module(&mut self) -> Result<Module> {
    self.expect(TokenKind::ModuleKeyword)?;
    let name = self.expect(TokenKind::Identifier)?.value;
    self.expect(TokenKind::LeftBrace)?;

    let mut items = Vec::new();
    while self.current().kind != TokenKind::RightBrace {
      items.push(self.parse_item()?);
    }
    self.expect(TokenKind::RightBrace)?;

    Ok(Module { name, items })
  }

  fn parse_import(&mut self) -> Result<Import> {
    self.expect(TokenKind::ImportKeyword)?;
    let path = self.parse_zoglin_resource()?;
    let mut alias = None;
    if self.current().kind == TokenKind::AsKeyword {
      self.consume();
      alias = Some(self.expect(TokenKind::Identifier)?.value);
    }
    Ok(Import { path, alias })
  }

  fn parse_resource(&mut self) -> Result<Resource> {
    let is_asset = self.consume().kind == TokenKind::AssetKeyword;
    let kind = self.parse_resource_path()?;
    let content: ResourceContent;

    if self.current().kind == TokenKind::Identifier {
      let name = self.expect(TokenKind::Identifier)?.value;
      let json = self.expect(TokenKind::JSON)?.value;

      content = ResourceContent::Text(name, json::from_json5(&json));
    } else {
      let token = self.expect(TokenKind::String)?;
      content = ResourceContent::File(token.value, token.location.file);
    }

    Ok(Resource { kind, content, is_asset })
  }

  fn parse_resource_path(&mut self) -> Result<String> {
    if self.current().kind == TokenKind::Dot {
      return Ok(self.consume().value);
    }

    let mut text = self.expect(TokenKind::Identifier)?.value;
    while self.current().kind == TokenKind::ForwardSlash {
      text.push('/');
      self.consume();
      text.push_str(&self.expect(TokenKind::Identifier)?.value);
    }
    Ok(text)
  }

  fn parse_function(&mut self) -> Result<Function> {
    self.expect(TokenKind::FunctionKeyword)?;
    let name = self.expect(TokenKind::Identifier)?.value;

    self.expect(TokenKind::LeftParen)?;
    self.expect(TokenKind::RightParen)?;

    self.expect(TokenKind::LeftBrace)?;
    let mut items = Vec::new();
    while self.current().kind != TokenKind::RightBrace {
      items.push(self.parse_statement()?);
    }
    self.expect(TokenKind::RightBrace)?;

    Ok(Function { name, items })
  }

  fn parse_statement(&mut self) -> Result<Statement> {
    Ok(match self.current_including(&[TokenKind::Comment]).kind {
      TokenKind::Command => {
        let command = self.consume().value;
        Statement::Command(command)
      }
      TokenKind::Comment => {
        let comment = self.consume_including(&[TokenKind::Comment]).value;
        Statement::Comment(comment)
      }
      _ => Statement::Expression(self.parse_expression()?),
    })
  }

  fn parse_expression(&mut self) -> Result<Expression> {
    Ok(Expression::FunctionCall(self.parse_function_call()?))
  }

  fn parse_function_call(&mut self) -> Result<FunctionCall> {
    let path = self.parse_zoglin_resource()?;
    self.expect(TokenKind::LeftParen)?;
    self.expect(TokenKind::RightParen)?;
    Ok(FunctionCall { path })
  }

  fn parse_zoglin_resource(&mut self) -> Result<ZoglinResource> {
    let mut resource = ZoglinResource {
      namespace: None,
      modules: Vec::new(),
      name: String::new(),
    };
    let mut allow_colon: bool = true;
    if self.current().kind == TokenKind::Colon {
      self.consume();
      allow_colon = false;
      resource.namespace = Some(String::new());
    }
    loop {
      let identifier = self.expect(TokenKind::Identifier)?.value;
      match self.current().kind {
        TokenKind::Colon => {
          self.consume();
          if allow_colon && self.current().kind == TokenKind::Identifier {
            resource.namespace = Some(identifier);
            allow_colon = false;
          } else {
            resource.name = identifier;
            break;
          }
        }
        TokenKind::ForwardSlash => {
          resource.modules.push(identifier);
          allow_colon = false;
          self.consume();
        }
        _ => {
          resource.name = identifier;
          break;
        }
      }
    }
    Ok(resource)
  }
}
