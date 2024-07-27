use crate::error::Result;
use crate::lexer::token::Token;
use crate::{error::raise_error, lexer::token::TokenKind};

use super::ast::BinaryOperation;
use super::{
  ast::{Expression, Operator},
  Parser,
};

impl Parser {
  fn match_operator(kind: TokenKind) -> Operator {
    match kind {
      TokenKind::Plus => Operator::Plus,
      TokenKind::Minus => Operator::Minus,
      TokenKind::ForwardSlash => Operator::Divide,
      TokenKind::Star => Operator::Multiply,
      TokenKind::Percent => Operator::Modulo,
      TokenKind::DoubleStar => Operator::Power,
      TokenKind::LeftShift => Operator::LeftShift,
      TokenKind::RightShift => Operator::RightShift,
      TokenKind::LessThan => Operator::LessThan,
      TokenKind::GreaterThan => Operator::GreaterThan,
      TokenKind::LessThanEquals => Operator::LessThanEquals,
      TokenKind::GreaterThanEquals => Operator::GreaterThanEquals,
      TokenKind::DoubleEquals => Operator::Equal,
      TokenKind::BangEquals => Operator::NotEqual,
      TokenKind::DoubleAmpersand => Operator::LogicalAnd,
      TokenKind::DoublePipe => Operator::LogicalOr,
      TokenKind::Equals => Operator::Assign,
      TokenKind::PlusEquals => Operator::OperatorAssign(Box::new(Operator::Plus)),
      TokenKind::MinusEquals => Operator::OperatorAssign(Box::new(Operator::Minus)),
      TokenKind::StarEquals => Operator::OperatorAssign(Box::new(Operator::Multiply)),
      TokenKind::ForwardSlashEquals => Operator::OperatorAssign(Box::new(Operator::Divide)),
      TokenKind::PercentEquals => Operator::OperatorAssign(Box::new(Operator::Modulo)),
      _ => unreachable!("Invalid Operator"),
    }
  }

  fn match_precedence(kind: TokenKind) -> (u8, u8) {
    match kind {
      TokenKind::DoubleStar => (8, 7),
      TokenKind::ForwardSlash | TokenKind::Star | TokenKind::Percent => (7, 7),
      TokenKind::Plus | TokenKind::Minus => (6, 6),
      TokenKind::LeftShift | TokenKind::RightShift => (5, 5),
      TokenKind::LessThan
      | TokenKind::GreaterThan
      | TokenKind::LessThanEquals
      | TokenKind::GreaterThanEquals => (4, 4),
      TokenKind::DoubleEquals | TokenKind::BangEquals => (3, 3),
      TokenKind::DoubleAmpersand | TokenKind::DoublePipe => (2, 2),
      TokenKind::Equals
      | TokenKind::PlusEquals
      | TokenKind::MinusEquals
      | TokenKind::StarEquals
      | TokenKind::ForwardSlashEquals
      | TokenKind::PercentEquals => (1, 0),
      _ => (0, 0),
    }
  }

  fn lookup_prefix(kind: TokenKind) -> Option<fn(&mut Parser) -> Result<Expression>> {
    use TokenKind::*;
    let function = match kind {
      TrueKeyword | FalseKeyword => Parser::parse_boolean,
      Identifier | Colon => Parser::parse_identifier,
      Byte | Short | Integer | Long | Float | Double => Parser::parse_number,
      String => Parser::parse_string,
      LeftParen => Parser::parse_bracketed_expression,
      LeftSquare => Parser::parse_array,
      LeftBrace => Parser::parse_compound,
      Dollar => Parser::parse_scoreboard_variable,
      Percent => |parser: &mut Parser| {
        parser.consume();
        let name = parser.expect(TokenKind::Identifier)?;
        Ok(Expression::MacroVariable(name.value, name.location))
      },
      _ => return None,
    };
    Some(function)
  }

  pub fn parse_bracketed_expression(&mut self) -> Result<Expression> {
    self.expect(TokenKind::LeftParen)?;
    let expression = self.parse_expression()?;
    self.expect(TokenKind::RightParen)?;
    Ok(expression)
  }

  pub fn parse_optional_expression(&mut self) -> Result<Option<Expression>> {
    if Parser::lookup_prefix(self.current().kind).is_some() {
      self.parse_expression().map(Some)
    } else {
      Ok(None)
    }
  }

  pub fn parse_expression(&mut self) -> Result<Expression> {
    self.parse_sub_expression(0)
  }

  pub fn parse_sub_expression(&mut self, min_precedence: u8) -> Result<Expression> {
    let function = Parser::lookup_prefix(self.current().kind).ok_or(raise_error(
      self.current().location,
      format!("Expected expression, got {:?}.", self.current().kind),
    ))?;
    let mut left = function(self)?;
    while let Some(function) = Parser::lookup_infix(self.current().kind) {
      let precedence = Parser::match_precedence(self.current().kind).0;
      if precedence <= min_precedence {
        break;
      };
      left = function(self, left)?;
    }
    Ok(left)
  }

  fn parse_binary_operation(&mut self, left: Expression) -> Result<Expression> {
    let Token {
      location,
      kind,
      value: _,
    } = self.consume();
    let operator = Parser::match_operator(kind);
    let precedence = Parser::match_precedence(kind);
    let right = self.parse_sub_expression(precedence.1)?;
    Ok(Expression::BinaryOperation(BinaryOperation {
      operator,
      location,
      left: Box::new(left),
      right: Box::new(right),
    }))
  }

  fn lookup_infix(kind: TokenKind) -> Option<InfixFn> {
    let function = match kind {
      TokenKind::Plus
      | TokenKind::Minus
      | TokenKind::ForwardSlash
      | TokenKind::Star
      | TokenKind::DoubleStar
      | TokenKind::LeftShift
      | TokenKind::RightShift
      | TokenKind::LessThan
      | TokenKind::GreaterThan
      | TokenKind::LessThanEquals
      | TokenKind::GreaterThanEquals
      | TokenKind::DoubleEquals
      | TokenKind::BangEquals
      | TokenKind::DoubleAmpersand
      | TokenKind::DoublePipe
      | TokenKind::Percent
      | TokenKind::Equals
      | TokenKind::PlusEquals
      | TokenKind::MinusEquals
      | TokenKind::StarEquals
      | TokenKind::ForwardSlashEquals
      | TokenKind::PercentEquals => Parser::parse_binary_operation,
      _ => return None,
    };
    Some(function)
  }
}

type InfixFn = fn(&mut Parser, Expression) -> Result<Expression>;
