use ast::{
  ArrayType, Command, CommandPart, ElseStatement, KeyValue, Parameter, ParameterKind, ReturnType,
  StaticExpr, WhileLoop,
};

use self::ast::{
  Expression, File, Function, FunctionCall, IfStatement, Import, Item, Module, Namespace, Resource,
  ResourceContent, Statement, ZoglinResource,
};
use crate::{
  error::{raise_error, raise_warning, Location, Result},
  lexer::token::{Token, TokenKind},
};

pub mod ast;
mod binary_operation;

fn json5_to_json(text: &str, location: Location) -> Result<String> {
  let map: serde_json::Value = json5::from_str(text).map_err(|e| raise_error(location, e))?;
  Ok(serde_json::to_string_pretty(&map).expect("Json is valid, it was just parsed"))
}

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

  fn current(&self) -> &Token {
    self.peek(0)
  }

  fn peek(&self, mut offset: usize) -> &Token {
    while self.should_skip(offset, &[]) {
      offset += 1;
    }
    &self.tokens[self.position + offset]
  }

  fn current_including(&self, kinds: &[TokenKind]) -> &Token {
    let mut offset = 0;
    while self.should_skip(offset, kinds) {
      offset += 1;
    }
    &self.tokens[self.position + offset]
  }

  fn consume(&mut self) -> &Token {
    while self.should_skip(0, &[]) {
      self.position += 1;
    }
    self.position += 1;
    &self.tokens[self.position - 1]
  }

  fn consume_including(&mut self, kinds: &[TokenKind]) -> &Token {
    while self.should_skip(0, kinds) {
      self.position += 1;
    }
    self.position += 1;
    &self.tokens[self.position - 1]
  }

  fn expect(&mut self, kind: TokenKind) -> Result<&Token> {
    let next = self.consume();
    if next.kind != kind {
      return Err(raise_error(
        next.location.clone(),
        format!("Expected {:?}, got {:?}", kind, next.kind),
      ));
    }
    Ok(next)
  }

  fn parse_namespace(&mut self) -> Result<Vec<Namespace>> {
    let file = self
      .expect(TokenKind::NamespaceKeyword)?
      .location
      .file
      .clone();
    let name = self.expect(TokenKind::Identifier)?.value.clone();

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

  fn is_namespace_end(&mut self, file: &str) -> bool {
    let next = self.current_including(&[TokenKind::EndOfInclude, TokenKind::EndOfFile]);
    if next.kind == TokenKind::EndOfFile {
      return true;
    }
    if next.kind == TokenKind::EndOfInclude {
      if &*next.location.file == file {
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
      TokenKind::Ampersand => self.parse_comptime_assignment()?,
      _ => {
        return Err(raise_error(
          self.current().location.clone(),
          format!("Unexpected token kind: {:?}", self.current().kind),
        ))
      }
    })
  }

  fn parse_comptime_assignment(&mut self) -> Result<Item> {
    self.consume();
    let name = self.expect(TokenKind::Identifier)?.value.clone();
    self.expect(TokenKind::Equals)?;
    let value = self.parse_expression()?;
    Ok(Item::ComptimeAssignment(name, value))
  }

  fn parse_module(&mut self) -> Result<Module> {
    self.expect(TokenKind::ModuleKeyword)?;
    let name = self.expect(TokenKind::Identifier)?.value.clone();
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
      alias = Some(self.expect(TokenKind::Identifier)?.value.clone());
    }
    Ok(Import { path, alias })
  }

  fn parse_resource(&mut self) -> Result<Resource> {
    let is_asset = self.consume().kind == TokenKind::AssetKeyword;
    let kind = self.parse_resource_path()?;
    let location = self.current().location.clone();

    let content: ResourceContent = if self.current().kind == TokenKind::Identifier {
      let name = self.expect(TokenKind::Identifier)?.value.clone();
      let token = self.expect(TokenKind::Json)?;

      ResourceContent::Text(name, json5_to_json(&token.value, token.location.clone())?)
    } else {
      let token = self.expect(TokenKind::String)?;
      let (base_path, path) = if token.value.starts_with('/') {
        (
          token.location.root.to_string(),
          token.value[1..].to_string(),
        )
      } else {
        (token.location.file.to_string(), token.value.clone())
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
      return Ok(self.consume().value.clone());
    }

    let mut text = self.expect(TokenKind::Identifier)?.value.clone();
    while self.current().kind == TokenKind::ForwardSlash {
      text.push('/');
      self.consume();
      text.push_str(&self.expect(TokenKind::Identifier)?.value);
    }
    Ok(text)
  }

  fn parse_parameter(&mut self) -> Result<Parameter> {
    let kind = match self.current().kind {
      TokenKind::Dollar => {
        self.consume();
        ParameterKind::Scoreboard
      }
      TokenKind::Percent => {
        self.consume();
        ParameterKind::Macro
      }
      TokenKind::Ampersand => todo!(),
      _ => ParameterKind::Storage,
    };
    let name = self.expect(TokenKind::Identifier)?.value.clone();
    Ok(Parameter { name, kind })
  }

  fn parse_function(&mut self) -> Result<Function> {
    self.expect(TokenKind::FunctionKeyword)?;

    let return_type = match self.current().kind {
      TokenKind::Dollar => {
        self.consume();
        ReturnType::Scoreboard
      }
      TokenKind::Percent => {
        self.consume();
        ReturnType::Direct
      }
      _ => ReturnType::Storage,
    };

    let Token {
      value: name,
      location,
      kind: _,
    } = self.expect(TokenKind::Identifier)?.clone();

    self.expect(TokenKind::LeftParen)?;

    let arguments = self.parse_list(TokenKind::RightParen, Parser::parse_parameter)?;

    let items = self.parse_block()?;

    Ok(Function {
      name,
      return_type,
      location,
      parameters: arguments,
      items,
    })
  }

  fn parse_statement(&mut self) -> Result<Statement> {
    Ok(match self.current_including(&[TokenKind::Comment]).kind {
      TokenKind::CommandBegin => Statement::Command(self.parse_command()?),
      TokenKind::Comment => {
        let comment = self.consume_including(&[TokenKind::Comment]).value.clone();
        Statement::Comment(comment)
      }
      TokenKind::IfKeyword => Statement::IfStatement(self.parse_if_statement()?),
      TokenKind::WhileKeyword => Statement::WhileLoop(self.parse_while_loop()?),
      TokenKind::ReturnKeyword => Statement::Return(self.parse_return()?),
      _ => Statement::Expression(self.parse_expression()?),
    })
  }

  fn parse_command(&mut self) -> Result<Command> {
    self.expect(TokenKind::CommandBegin)?;
    let mut parts = Vec::new();

    while self.current().kind != TokenKind::CommandEnd {
      match self.current().kind {
        TokenKind::CommandString => parts.push(CommandPart::Literal(self.consume().value.clone())),
        _ => parts.push(CommandPart::Expression(self.parse_static_expr()?)),
      }
    }

    self.consume();

    Ok(Command { parts })
  }

  fn parse_return(&mut self) -> Result<Option<Expression>> {
    self.consume();
    self.parse_optional_expression()
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
        Expression::Byte(value, current.location.clone())
      }
      TokenKind::Short => {
        let value = current.value.parse().map_err(|_| {
          raise_error(
            current.location.clone(),
            format!("Value {} is too large for a short.", current.value),
          )
        })?;
        Expression::Short(value, current.location.clone())
      }
      TokenKind::Integer => match current.value.parse() {
        Ok(value) => Expression::Integer(value, current.location.clone()),
        Err(_) => {
          let value = current.value.parse().map_err(|_| {
            raise_error(
              current.location.clone(),
              format!("Value {} is too large for a int.", current.value),
            )
          })?;

          raise_warning(current.location.clone(), format!("Value {} is too large for an int, automatically converting to a long. If this is intentional, suffix it with 'l'.", current.value));
          Expression::Long(value, current.location.clone())
        }
      },
      TokenKind::Long => {
        let value = current.value.parse().map_err(|_| {
          raise_error(
            current.location.clone(),
            format!("Value {} is too large for a long", current.value),
          )
        })?;
        Expression::Long(value, current.location.clone())
      }
      TokenKind::Float => {
        let value = current.value.parse().map_err(|_| {
          raise_error(
            current.location.clone(),
            format!("Value {} is too large for a float.", current.value),
          )
        })?;
        Expression::Float(value, current.location.clone())
      }
      TokenKind::Double => {
        let value = current.value.parse().map_err(|_| {
          raise_error(
            current.location.clone(),
            format!("Value {} is too large for a double.", current.value),
          )
        })?;
        Expression::Double(value, current.location.clone())
      }
      _ => unreachable!(),
    })
  }

  fn parse_boolean(&mut self) -> Result<Expression> {
    let token = self.consume();
    Ok(Expression::Boolean(
      token.kind == TokenKind::TrueKeyword,
      token.location.clone(),
    ))
  }

  fn parse_string(&mut self) -> Result<Expression> {
    let token = self.consume().clone();
    Ok(Expression::String(token.value, token.location))
  }

  fn parse_array(&mut self) -> Result<Expression> {
    let location = self.expect(TokenKind::LeftSquare)?.location.clone();

    let array_type = if self.peek(1).kind == TokenKind::Semicolon
      && self.current().kind == TokenKind::Identifier
    {
      let array_type = match self.current().value.as_str() {
        "B" | "b" => ArrayType::Byte,
        "I" | "i" => ArrayType::Int,
        "L" | "l" => ArrayType::Long,
        _ => {
          return Err(raise_error(
            self.current().location.clone(),
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
    while !self.eof() && self.current().kind != TokenKind::RightSquare {
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
    let location = self.expect(TokenKind::LeftBrace)?.location.clone();
    let mut key_values = Vec::new();

    while !self.eof() && self.current().kind != TokenKind::LeftBrace {
      if self.current().kind != TokenKind::Identifier && self.current().kind != TokenKind::String {
        return Err(raise_error(
          self.current().location.clone(),
          "Expected compound key.",
        ));
      }

      let Token {
        value: key,
        location,
        kind: _,
      } = self.consume().clone();
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
    match self.current().kind {
      TokenKind::Percent => {
        self.consume();
        let name = self.expect(TokenKind::Identifier)?.value.clone();
        Ok(StaticExpr::MacroVariable(name))
      }
      TokenKind::Ampersand => {
        self.consume();
        let name = self.expect(TokenKind::Identifier)?.value.clone();
        Ok(StaticExpr::ComptimeVariable(name))
      }
      TokenKind::FunctionKeyword => {
        self.consume();
        let path = match self.current().kind {
          TokenKind::CommandString => None,
          _ => Some(self.parse_zoglin_resource()?),
        };
        Ok(StaticExpr::FunctionRef { path })
      }
      _ => {
        let resource: ZoglinResource = self.parse_zoglin_resource()?;
        if self.current().kind == TokenKind::LeftParen {
          return Ok(StaticExpr::FunctionCall(
            self.parse_function_call(resource)?,
          ));
        }
        Ok(StaticExpr::ResourceRef { resource })
      }
    }
  }

  fn parse_function_call(&mut self, path: ZoglinResource) -> Result<FunctionCall> {
    self.expect(TokenKind::LeftParen)?;
    let arguments = self.parse_list(TokenKind::RightParen, Self::parse_expression)?;

    Ok(FunctionCall { path, arguments })
  }

  fn parse_zoglin_resource(&mut self) -> Result<ZoglinResource> {
    let mut resource = ZoglinResource {
      namespace: None,
      location: self.current().location.clone(),
      modules: Vec::new(),
      name: String::new(),
    };
    let mut allow_colon: bool = true;
    if self.current().kind == TokenKind::Colon {
      self.consume();
      allow_colon = false;
      resource.namespace = Some(String::new());
    } else if self.current().kind == TokenKind::Tilde {
      self.consume();
      allow_colon = false;
      resource.namespace = Some("~".to_string());
      if self.current().kind == TokenKind::ForwardSlash {
        self.consume();
      }
    }
    loop {
      let identifier = self.expect(TokenKind::Identifier)?.value.clone();
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

  fn parse_while_loop(&mut self) -> Result<WhileLoop> {
    self.consume();
    let condition = self.parse_expression()?;
    let block = self.parse_block()?;

    Ok(WhileLoop { condition, block })
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
