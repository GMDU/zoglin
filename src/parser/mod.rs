use ast::{ArrayType, Command, CommandPart, ElseStatement, KeyValue, StaticExpr};

use self::ast::{
  Expression, File, Function, FunctionCall, IfStatement, Import, Item, Module, Namespace, Resource,
  ResourceContent, Statement, ZoglinResource,
};
use crate::{
  error::{raise_error, raise_warning, Result},
  lexer::token::{Token, TokenKind},
};

pub mod ast;
mod binary_operation;
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
    self.peek(0)
  }

  fn peek(&self, mut offset: usize) -> Token {
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
        format!("Expected {:?}, got {:?}", kind, next.kind),
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
      TokenKind::ResourceKeyword | TokenKind::AssetKeyword => {
        Item::Resource(self.parse_resource()?)
      }
      TokenKind::FunctionKeyword => Item::Function(self.parse_function()?),
      _ => {
        return Err(raise_error(
          self.current().location,
          format!("Unexpected token kind: {:?}", self.current().kind),
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
    let location = self.current().location;

    let content: ResourceContent = if self.current().kind == TokenKind::Identifier {
      let name = self.expect(TokenKind::Identifier)?.value;
      let token = self.expect(TokenKind::Json)?;

      ResourceContent::Text(name, json::from_json5(&token.value, token.location)?)
    } else {
      let token = self.expect(TokenKind::String)?;
      let (base_path, path) = if token.value.starts_with('/') {
        (token.location.root, token.value[1..].to_string())
      } else {
        (token.location.file, token.value)
      };
      ResourceContent::File(path, base_path)
    };

    Ok(Resource {
      kind,
      content,
      is_asset,
      location,
    })
  }

  fn parse_block(&mut self) -> Result<Vec<Statement>> {
    self.expect(TokenKind::LeftBrace)?;
    let mut items = Vec::new();
    while self.current().kind != TokenKind::RightBrace {
      items.push(self.parse_statement()?);
    }
    self.expect(TokenKind::RightBrace)?;
    Ok(items)
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
    let Token {
      value: name,
      location,
      kind: _,
    } = self.expect(TokenKind::Identifier)?;

    self.expect(TokenKind::LeftParen)?;

    let arguments = self.parse_list(TokenKind::RightParen, |parser| {
      parser
        .expect(TokenKind::Identifier)
        .map(|token| token.value)
    })?;

    let items = self.parse_block()?;

    Ok(Function {
      name,
      location,
      arguments,
      items,
    })
  }

  fn parse_statement(&mut self) -> Result<Statement> {
    Ok(match self.current_including(&[TokenKind::Comment]).kind {
      TokenKind::CommandBegin => Statement::Command(self.parse_command()?),
      TokenKind::Comment => {
        let comment = self.consume_including(&[TokenKind::Comment]).value;
        Statement::Comment(comment)
      }
      TokenKind::IfKeyword => Statement::IfStatement(self.parse_if_statement()?),
      _ => Statement::Expression(self.parse_expression()?),
    })
  }

  fn parse_command(&mut self) -> Result<Command> {
    self.expect(TokenKind::CommandBegin)?;
    let mut parts = Vec::new();

    while self.current().kind != TokenKind::CommandEnd {
      match self.current().kind {
        TokenKind::CommandString => parts.push(CommandPart::Literal(self.consume().value)),
        _ => parts.push(CommandPart::Expression(self.parse_static_expr()?)),
      }
    }

    self.consume();

    Ok(Command { parts })
  }

  fn parse_number(&mut self) -> Result<Expression> {
    let current = self.consume();

    Ok(match current.kind {
      TokenKind::Byte => {
        let value = current.value.parse().map_err(|_| {
          raise_error(
            current.location.clone(),
            format!("Value {} is too large for a byte.", current.value),
          )
        })?;
        Expression::Byte(value, current.location)
      }
      TokenKind::Short => {
        let value = current.value.parse().map_err(|_| {
          raise_error(
            current.location.clone(),
            format!("Value {} is too large for a short.", current.value),
          )
        })?;
        Expression::Short(value, current.location)
      }
      TokenKind::Integer => match current.value.parse() {
        Ok(value) => Expression::Integer(value, current.location),
        Err(_) => {
          let value = current.value.parse().map_err(|_| {
            raise_error(
              current.location.clone(),
              format!("Value {} is too large for a int.", current.value),
            )
          })?;

          raise_warning(current.location.clone(), format!("Value {} is too large for an int, automatically converting to a long. If this is intentional, suffix it with 'l'.", current.value));
          Expression::Long(value, current.location)
        }
      },
      TokenKind::Long => {
        let value = current.value.parse().map_err(|_| {
          raise_error(
            current.location.clone(),
            format!("Value {} is too large for a long", current.value),
          )
        })?;
        Expression::Long(value, current.location)
      }
      TokenKind::Float => {
        let value = current.value.parse().map_err(|_| {
          raise_error(
            current.location.clone(),
            format!("Value {} is too large for a float.", current.value),
          )
        })?;
        Expression::Float(value, current.location)
      }
      TokenKind::Double => {
        let value = current.value.parse().map_err(|_| {
          raise_error(
            current.location.clone(),
            format!("Value {} is too large for a double.", current.value),
          )
        })?;
        Expression::Double(value, current.location)
      }
      _ => unreachable!(),
    })
  }

  fn parse_boolean(&mut self) -> Result<Expression> {
    let token = self.consume();
    Ok(Expression::Boolean(
      token.kind == TokenKind::TrueKeyword,
      token.location,
    ))
  }

  fn parse_string(&mut self) -> Result<Expression> {
    let token = self.consume();
    Ok(Expression::String(token.value, token.location))
  }

  fn parse_array(&mut self) -> Result<Expression> {
    let location = self.expect(TokenKind::LeftSquare)?.location;

    let array_type = if self.peek(1).kind == TokenKind::Semicolon
      && self.current().kind == TokenKind::Identifier
    {
      let array_type = match self.current().value.as_str() {
        "B" | "b" => ArrayType::Byte,
        "I" | "i" => ArrayType::Int,
        "L" | "l" => ArrayType::Long,
        _ => {
          return Err(raise_error(
            self.current().location,
            format!("\"{}\" is not a valid array type.", self.current().value),
          ))
        }
      };

      self.consume();
      self.consume();

      array_type
    } else {
      ArrayType::Any
    };

    let mut expressions = Vec::new();
    while !self.eof() && self.current().kind != TokenKind::LeftSquare {
      let expression = self.parse_expression()?;
      expressions.push(expression);

      if self.current().kind == TokenKind::Comma {
        self.consume();
      } else {
        break;
      }
    }
    self.expect(TokenKind::RightSquare)?;

    Ok(Expression::Array(array_type, expressions, location))
  }

  fn parse_compound(&mut self) -> Result<Expression> {
    let location = self.expect(TokenKind::LeftBrace)?.location;
    let mut key_values = Vec::new();

    while !self.eof() && self.current().kind != TokenKind::LeftBrace {
      if self.current().kind != TokenKind::Identifier && self.current().kind != TokenKind::String {
        return Err(raise_error(
          self.current().location,
          "Expected compound key.",
        ));
      }

      let Token {
        value: key,
        location,
        kind: _,
      } = self.consume();
      self.expect(TokenKind::Colon)?;
      let value = self.parse_expression()?;
      key_values.push(KeyValue {
        key,
        value,
        location,
      });

      if self.current().kind == TokenKind::Comma {
        self.consume();
      } else {
        break;
      }
    }

    self.expect(TokenKind::RightBrace)?;

    Ok(Expression::Compound(key_values, location))
  }

  fn parse_identifier(&mut self) -> Result<Expression> {
    let resource = self.parse_zoglin_resource()?;
    if self.current().kind == TokenKind::LeftParen {
      Ok(Expression::FunctionCall(
        self.parse_function_call(resource)?,
      ))
    } else {
      Ok(Expression::Variable(resource))
    }
  }

  fn parse_scoreboard_variable(&mut self) -> Result<Expression> {
    self.expect(TokenKind::Dollar)?;
    Ok(Expression::ScoreboardVariable(
      self.parse_zoglin_resource()?,
    ))
  }

  fn parse_static_expr(&mut self) -> Result<StaticExpr> {
    let is_fn = self.current().kind == TokenKind::FunctionKeyword;
    if is_fn {
      self.consume();
    }
    let resource = self.parse_zoglin_resource()?;
    if self.current().kind == TokenKind::LeftParen {
      if is_fn {
        return Err(raise_error(
          self.current().location,
          "`fn` keyword not required when calling a function.",
        ));
      }

      return Ok(StaticExpr::FunctionCall(
        self.parse_function_call(resource)?,
      ));
    }
    Ok(StaticExpr::ResourceRef { resource, is_fn })
  }

  fn parse_function_call(&mut self, path: ZoglinResource) -> Result<FunctionCall> {
    self.expect(TokenKind::LeftParen)?;
    let arguments = self.parse_list(TokenKind::RightParen, Self::parse_expression)?;

    Ok(FunctionCall { path, arguments })
  }

  fn parse_zoglin_resource(&mut self) -> Result<ZoglinResource> {
    let mut resource = ZoglinResource {
      namespace: None,
      location: self.current().location,
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

  fn parse_if_statement(&mut self) -> Result<IfStatement> {
    self.consume();
    let condition = self.parse_expression()?;
    let block = self.parse_block()?;

    let mut child = None;

    if self.current().kind == TokenKind::ElseKeyword {
      self.consume();

      if self.current().kind == TokenKind::IfKeyword {
        let if_statement = self.parse_if_statement()?;
        child = Some(ElseStatement::IfStatement(Box::new(if_statement)));
      } else {
        let block = self.parse_block()?;
        child = Some(ElseStatement::Block(block))
      }
    }

    Ok(IfStatement {
      condition,
      block,
      child,
    })
  }

  fn parse_list<T>(
    &mut self,
    delimiter: TokenKind,
    parse_fn: impl Fn(&mut Self) -> Result<T>,
  ) -> Result<Vec<T>> {
    let mut list = Vec::new();
    while self.current().kind != delimiter {
      list.push(parse_fn(self)?);

      if self.current().kind == TokenKind::Comma {
        self.consume();
      } else {
        break;
      }
    }
    self.expect(delimiter)?;

    Ok(list)
  }
}
