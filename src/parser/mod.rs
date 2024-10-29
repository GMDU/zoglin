use ast::{
  ArrayType, Command, CommandPart, ComptimeFunction, ElseStatement, KeyValue, Parameter,
  ParameterKind, ReturnType, StaticExpr, WhileLoop,
};
use ecow::{eco_format, EcoString};
use name::{validate, validate_or_quote, NameKind};

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
mod name;
mod resource;

fn json5_to_json(text: &str, location: Location) -> Result<EcoString> {
  let map: serde_json::Value = json5::from_str(text).map_err(|e| raise_error(location, e))?;
  Ok(
    serde_json::to_string_pretty(&map)
      .expect("Json is valid, it was just parsed")
      .into(),
  )
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
    let name = self.expect(TokenKind::Identifier)?;
    validate(name.get_value(), &name.location, NameKind::Namespace)?;
    let name = name.get_value().clone();

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

  fn parse_block_namespace(&mut self, name: EcoString) -> Result<Namespace> {
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
      TokenKind::FunctionKeyword => self.parse_function()?,
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
    let name = self.expect(TokenKind::Identifier)?.get_value().clone();
    self.expect(TokenKind::Equals)?;
    let value = self.parse_expression()?;
    Ok(Item::ComptimeAssignment(name, value))
  }

  fn parse_module(&mut self) -> Result<Module> {
    self.expect(TokenKind::ModuleKeyword)?;
    let name = self.expect(TokenKind::Identifier)?;
    validate(&name.get_value(), &name.location, NameKind::Module)?;
    let name = name.get_value().clone();
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
    let path = self.parse_import_resource()?;
    let mut alias = None;
    if self.current().kind == TokenKind::AsKeyword {
      self.consume();
      // TODO: Maybe validate here? If we try to use a weird alias in other
      // places in the code, it will probably complain, so we might want to
      // catch that here
      alias = Some(self.expect(TokenKind::Identifier)?.get_value().clone());
    }
    Ok(Import { path, alias })
  }

  fn parse_resource(&mut self) -> Result<Resource> {
    let is_asset = self.consume().kind == TokenKind::AssetKeyword;
    let kind = self.parse_resource_path()?;
    let location = self.current().location.clone();

    let content: ResourceContent = if self.current().kind == TokenKind::Identifier {
      let name = self.consume();
      validate(&name.get_value(), &name.location, NameKind::Resource)?;
      let name = name.get_value().clone();
      let token = self.expect(TokenKind::Json)?;

      ResourceContent::Text(name, json5_to_json(&token.get_value(), token.location.clone())?)
    } else {
      let token = self.expect(TokenKind::String)?;
      let (base_path, path) = if token.get_value().starts_with('/') {
        (token.location.root.clone(), token.get_value()[1..].into())
      } else {
        (token.location.file.clone(), token.get_value().clone())
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

  fn parse_resource_path(&mut self) -> Result<EcoString> {
    if self.current().kind == TokenKind::Dot {
      return Ok(self.consume().get_value().clone());
    }

    let text = self.expect(TokenKind::Identifier)?;
    validate(&text.get_value(), &text.location, NameKind::ResourcePathComponent)?;
    let mut text = text.get_value().clone();

    while self.current().kind == TokenKind::ForwardSlash {
      text.push('/');
      self.consume();
      let next = self.expect(TokenKind::Identifier)?;
      validate(&next.get_value(), &next.location, NameKind::ResourcePathComponent)?;
      text.push_str(&next.get_value());
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
    let token = self.expect(TokenKind::Identifier)?.clone();
    let name = token.get_value().clone();
    let location = token.location;
    validate(&name, &location, NameKind::Parameter(kind))?;

    let default = if self.current().kind == TokenKind::Equals {
      self.consume();
      Some(self.parse_expression()?)
    } else {
      None
    };

    Ok(Parameter {
      name: name.clone(),
      location,
      kind,
      default,
    })
  }

  fn parse_comptime_parameter(&mut self) -> Result<Parameter> {
    if self.current().kind == TokenKind::Ampersand {
      self.consume();
    }

    let token = self.expect(TokenKind::Identifier)?.clone();
    let name = token.get_value().clone();
    let location = token.location;
    validate(
      &name,
      &location,
      NameKind::Parameter(ParameterKind::CompileTime),
    )?;

    let default = if self.current().kind == TokenKind::Equals {
      self.consume();
      Some(self.parse_expression()?)
    } else {
      None
    };

    Ok(Parameter {
      name: name.clone(),
      location,
      kind: ParameterKind::CompileTime,
      default,
    })
  }

  fn parse_function(&mut self) -> Result<Item> {
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
      TokenKind::Ampersand => {
        self.consume();
        return self.parse_comptime_function();
      }
      _ => ReturnType::Storage,
    };

    let token = self.expect(TokenKind::Identifier)?.clone();
    let name = token.get_value().clone();
    let location = token.location;
    validate(&name, &location, NameKind::Function)?;

    self.expect(TokenKind::LeftParen)?;

    let parameters = self.parse_list(TokenKind::RightParen, Parser::parse_parameter)?;

    let mut found_optional = false;
    for parameter in parameters.iter() {
      if parameter.default.is_some() {
        found_optional = true;
      } else if found_optional {
        return Err(raise_error(
          parameter.location.clone(),
          "Required parameters cannot come after optional parameters.",
        ));
      }
    }

    let items = self.parse_block()?;

    Ok(Item::Function(Function {
      name: name.clone(),
      return_type,
      location,
      parameters,
      items,
    }))
  }

  // Expects `fn &` already to be consumed
  fn parse_comptime_function(&mut self) -> Result<Item> {
    let name = self.expect(TokenKind::Identifier)?.get_value().clone();

    self.expect(TokenKind::LeftParen)?;

    let parameters = self.parse_list(TokenKind::RightParen, Parser::parse_comptime_parameter)?;

    let items: Vec<Statement> = self.parse_block()?;

    Ok(Item::ComptimeFunction(ComptimeFunction {
      name,
      parameters,
      items,
    }))
  }

  fn parse_statement(&mut self) -> Result<Statement> {
    Ok(match self.current_including(&[TokenKind::Comment]).kind {
      TokenKind::CommandBegin(_) => Statement::Command(self.parse_command()?),
      TokenKind::Comment => {
        let comment = self.consume_including(&[TokenKind::Comment]).get_value().clone();
        Statement::Comment(comment)
      }
      TokenKind::IfKeyword => Statement::If(self.parse_if_statement()?),
      TokenKind::WhileKeyword => Statement::WhileLoop(self.parse_while_loop()?),
      TokenKind::ReturnKeyword => Statement::Return(self.parse_return()?),
      _ => Statement::Expression(self.parse_expression()?),
    })
  }

  fn parse_command(&mut self) -> Result<Command> {
    let next = self.consume();
    if next.kind != TokenKind::CommandBegin(true) && next.kind != TokenKind::CommandBegin(false) {
      return Err(raise_error(
        next.location.clone(),
        format!("Expected {:?}, got {:?}", TokenKind::CommandBegin(true), next.kind),
      ));
    }
    
    let mut parts = Vec::new();

    while self.current().kind != TokenKind::CommandEnd {
      match self.current().kind {
        TokenKind::CommandString => parts.push(CommandPart::Literal(self.consume().get_value().clone())),
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
        let value = current.get_value().parse().map_err(|_| {
          raise_error(
            current.location.clone(),
            format!("Value {} is too large for a byte.", current.get_value()),
          )
        })?;
        Expression::Byte(value, current.location.clone())
      }
      TokenKind::Short => {
        let value = current.get_value().parse().map_err(|_| {
          raise_error(
            current.location.clone(),
            format!("Value {} is too large for a short.", current.get_value()),
          )
        })?;
        Expression::Short(value, current.location.clone())
      }
      TokenKind::Integer => match current.get_value().parse() {
        Ok(value) => Expression::Integer(value, current.location.clone()),
        Err(_) => {
          let value = current.get_value().parse().map_err(|_| {
            raise_error(
              current.location.clone(),
              format!("Value {} is too large for a int.", current.get_value()),
            )
          })?;

          raise_warning(current.location.clone(), format!("Value {} is too large for an int, automatically converting to a long. If this is intentional, suffix it with 'l'.", current.get_value()));
          Expression::Long(value, current.location.clone())
        }
      },
      TokenKind::Long => {
        let value = current.get_value().parse().map_err(|_| {
          raise_error(
            current.location.clone(),
            format!("Value {} is too large for a long", current.get_value()),
          )
        })?;
        Expression::Long(value, current.location.clone())
      }
      TokenKind::Float => {
        let value = current.get_value().parse().map_err(|_| {
          raise_error(
            current.location.clone(),
            format!("Value {} is too large for a float.", current.get_value()),
          )
        })?;
        Expression::Float(value, current.location.clone())
      }
      TokenKind::Double => {
        let value = current.get_value().parse().map_err(|_| {
          raise_error(
            current.location.clone(),
            format!("Value {} is too large for a double.", current.get_value()),
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
    Ok(Expression::String(token.get_value().clone(), token.location))
  }

  fn parse_array(&mut self) -> Result<Expression> {
    let location = self.expect(TokenKind::LeftSquare)?.location.clone();

    let array_type = if self.peek(1).kind == TokenKind::Semicolon
      && self.current().kind == TokenKind::Identifier
    {
      let array_type = match self.current().get_value().as_str() {
        "B" | "b" => ArrayType::Byte,
        "I" | "i" => ArrayType::Int,
        "L" | "l" => ArrayType::Long,
        _ => {
          return Err(raise_error(
            self.current().location.clone(),
            format!("\"{}\" is not a valid array type.", self.current().get_value()),
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
      let token = self.expect(TokenKind::Identifier)?.clone();
      let key = token.get_value().clone();
      let location = token.location;

      let key = validate_or_quote(key.clone(), &location, NameKind::NBTPathComponent);

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
    let resource = self.parse_zoglin_resource(NameKind::Unknown)?;
    if self.current().kind == TokenKind::LeftParen {
      validate(&resource.name, &resource.location, NameKind::Function)?;
      Ok(Expression::FunctionCall(
        self.parse_function_call(resource, false)?,
      ))
    } else {
      let mut resource = resource;

      resource.name =
        validate_or_quote(resource.name, &resource.location, NameKind::StorageVariable);
      Ok(Expression::Variable(resource))
    }
  }

  fn parse_scoreboard_variable(&mut self) -> Result<Expression> {
    self.expect(TokenKind::Dollar)?;

    let mut resource: ZoglinResource;

    if self.current().kind == TokenKind::LeftSquare {
      let name = self.parse_scoreboard_variable_name()?;
      resource = ZoglinResource{
        namespace: None, location: self.current().location.clone(), modules: Vec::new(), name
      }
    } else {
      resource = self.parse_zoglin_resource(NameKind::ScoreboardVariable)?;

      if self.current().kind == TokenKind::LeftSquare {
        let name = self.parse_scoreboard_variable_name()?;
        resource.modules.push(resource.name);
        resource.name = name;
      } else {
        resource.name = eco_format!("${}", resource.name);
      }
    }

    Ok(Expression::ScoreboardVariable(
      resource
    ))
  }

  fn parse_scoreboard_variable_name(&mut self) -> Result<EcoString> {
    self.expect(TokenKind::LeftSquare)?;
    let mut square_count = 0;
    let mut output = EcoString::new();

    while square_count > 0 || self.current().kind != TokenKind::RightSquare {
      if self.current().kind == TokenKind::LeftSquare { square_count += 1; }
      if self.current().kind == TokenKind::RightSquare { square_count -= 1; }

      output.push_str(&self.consume().raw);
    }

    self.expect(TokenKind::RightSquare)?;

    Ok(output)
  }

  fn parse_static_expr(&mut self) -> Result<StaticExpr> {
    match self.current().kind {
      TokenKind::Percent => {
        self.consume();
        let name = self.expect(TokenKind::Identifier)?;
        validate(&name.get_value(), &name.location, NameKind::MacroVariable)?;
        Ok(StaticExpr::MacroVariable(name.get_value().clone()))
      }
      TokenKind::Ampersand => match self.parse_comptime_variable()? {
        Expression::FunctionCall(call) => Ok(StaticExpr::FunctionCall(call)),
        Expression::ComptimeVariable(name, _) => Ok(StaticExpr::ComptimeVariable(name)),
        _ => unreachable!(),
      },
      TokenKind::FunctionKeyword => {
        self.consume();
        let path = match self.current().kind {
          TokenKind::CommandString => None,
          _ => Some(self.parse_zoglin_resource(NameKind::Function)?),
        };
        Ok(StaticExpr::FunctionRef { path })
      }
      _ => {
        let resource = self.parse_zoglin_resource(NameKind::Resource)?;
        if self.current().kind == TokenKind::LeftParen {
          return Ok(StaticExpr::FunctionCall(
            self.parse_function_call(resource, false)?,
          ));
        }
        Ok(StaticExpr::ResourceRef { resource })
      }
    }
  }

  fn parse_function_call(&mut self, path: ZoglinResource, comptime: bool) -> Result<FunctionCall> {
    self.expect(TokenKind::LeftParen)?;
    let arguments = self.parse_list(TokenKind::RightParen, Self::parse_expression)?;

    Ok(FunctionCall {
      path,
      arguments,
      comptime,
    })
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
