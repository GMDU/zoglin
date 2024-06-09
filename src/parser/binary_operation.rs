use crate::{error::raise_error, lexer::token::TokenKind};
use crate::error::Result;

use super::ast::BinaryOperation;
use super::{ast::{Expression, Operator}, Parser};

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
      _ => unreachable!("Invalid Operator")
    }
  }

  fn match_precedence(kind: TokenKind) -> u8 {
    match kind {
      TokenKind::DoubleStar => 8,
      TokenKind::ForwardSlash => 7,
      TokenKind::Star => 7,
      TokenKind::Percent => 7,
      TokenKind::Plus => 6,
      TokenKind::Minus => 6,
      TokenKind::LeftShift => 5,
      TokenKind::RightShift => 5,
      TokenKind::LessThan => 4,
      TokenKind::GreaterThan => 4,
      TokenKind::LessThanEquals => 4,
      TokenKind::GreaterThanEquals => 4,
      TokenKind::DoubleEquals => 3,
      TokenKind::BangEquals => 3,
      TokenKind::DoubleAmpersand => 2,
      TokenKind::DoublePipe => 2,
      TokenKind::Equals => 1,
      TokenKind::PlusEquals => 1,
      TokenKind::MinusEquals => 1,
      TokenKind::StarEquals => 1,
      TokenKind::ForwardSlashEquals => 1,
      TokenKind::PercentEquals => 1,
      _ => 0
    }
  }

  fn lookup_prefix(kind: TokenKind) -> Option<fn(&mut Parser) -> Result<Expression>> {
    let function = match kind {
      TokenKind::Identifier | TokenKind::Colon => Parser::parse_identifier,
      TokenKind::Integer => Parser::parse_integer,
      _ => return None
    };
    Some(function)
  }

  pub fn parse_expression(&mut self, min_precedence: u8) -> Result<Expression> {
    let function = Parser::lookup_prefix(self.current().kind).ok_or(raise_error(self.current().location, &format!("Expected expression, got {:?}.", self.current().kind)))?;
    let mut left = function(self)?;
    while let Some(function) = Parser::lookup_infix(self.current().kind) {
      let precedence = Parser::match_precedence(self.current().kind);
      if precedence <= min_precedence { break };
      left = function(self, left)?;
    }
    Ok(left)
  }

  fn parse_binary_operation(&mut self, left: Expression) -> Result<Expression> {
    let operator = Parser::match_operator(self.current().kind);
    let precedence = Parser::match_precedence(self.consume().kind);
    let right = self.parse_expression(precedence)?;
    return Ok(Expression::BinaryOperation(
      BinaryOperation{
        operator,
        left: Box::new(left),
        right: Box::new(right)
      }
    ))
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
      _ => return None
    };
    Some(function)
  }
}

type InfixFn = fn(&mut Parser, Expression) -> Result<Expression>;