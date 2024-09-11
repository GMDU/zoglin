use crate::error::Result;
use crate::lexer::token::Token;
use crate::parser::name::{validate, NameKind};
use crate::{error::raise_error, lexer::token::TokenKind};

use super::ast::{
  BinaryOperation, Index, Member, MemberKind, RangeIndex, UnaryExpression, UnaryOperator,
  ZoglinResource,
};
use super::name::validate_or_quote;
use super::{
  ast::{Expression, Operator},
  Parser,
};

#[derive(PartialEq, PartialOrd)]
enum Precedence {
  None,
  Assignment,
  Logical,
  Equality,
  Comparison,
  Bitshift,
  Addition,
  Multiplication,
  Exponentiation,
  Prefix,
  Postfix,
}

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

  fn match_precedence(kind: TokenKind) -> (Precedence, Precedence) {
    use Precedence::*;
    match kind {
      TokenKind::LeftSquare | TokenKind::Dot => (Postfix, Postfix),
      TokenKind::DoubleStar => (Exponentiation, Multiplication),
      TokenKind::ForwardSlash | TokenKind::Star | TokenKind::Percent => {
        (Multiplication, Multiplication)
      }
      TokenKind::Plus | TokenKind::Minus => (Addition, Addition),
      TokenKind::LeftShift | TokenKind::RightShift => (Bitshift, Bitshift),
      TokenKind::LessThan
      | TokenKind::GreaterThan
      | TokenKind::LessThanEquals
      | TokenKind::GreaterThanEquals => (Comparison, Comparison),
      TokenKind::DoubleEquals | TokenKind::BangEquals => (Equality, Equality),
      TokenKind::DoubleAmpersand | TokenKind::DoublePipe => (Logical, Logical),
      TokenKind::Equals
      | TokenKind::PlusEquals
      | TokenKind::MinusEquals
      | TokenKind::StarEquals
      | TokenKind::ForwardSlashEquals
      | TokenKind::PercentEquals => (Assignment, None),
      _ => (None, None),
    }
  }

  fn lookup_prefix(kind: TokenKind) -> Option<fn(&mut Parser) -> Result<Expression>> {
    use TokenKind::*;
    let function = match kind {
      TrueKeyword | FalseKeyword => Parser::parse_boolean,
      Identifier | Colon | Tilde => Parser::parse_identifier,
      Byte | Short | Integer | Long | Float | Double => Parser::parse_number,
      String => Parser::parse_string,
      LeftParen => Parser::parse_bracketed_expression,
      LeftSquare => Parser::parse_array,
      LeftBrace => Parser::parse_compound,
      Dollar => Parser::parse_scoreboard_variable,
      Percent => |parser: &mut Parser| {
        parser.consume();
        let name = parser.expect(TokenKind::Identifier)?.clone();
        validate(&name.value, &name.location, NameKind::MacroVariable)?;
        Ok(Expression::MacroVariable(name.value, name.location))
      },
      Ampersand => Parser::parse_comptime_variable,
      Minus | Bang => Parser::parse_prefix_expression,
      BuiltinName => |parser: &mut Parser| {
        let Token {
          value: name,
          location,
          ..
        } = parser.consume().clone();
        if parser.current().kind == TokenKind::LeftParen {
          parser.consume();
          let args = parser.parse_list(TokenKind::RightParen, Parser::parse_expression)?;
          Ok(Expression::BuiltinFunction(name, args, location))
        } else {
          Ok(Expression::BuiltinVariable(name, location))
        }
      },
      _ => return None,
    };
    Some(function)
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
      TokenKind::LeftSquare => Parser::parse_index_expr,
      TokenKind::Dot => Parser::parse_member_expr,
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
    self.parse_sub_expression(Precedence::None)
  }

  fn parse_sub_expression(&mut self, min_precedence: Precedence) -> Result<Expression> {
    let function = Parser::lookup_prefix(self.current().kind).ok_or(raise_error(
      self.current().location.clone(),
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
    } = self.consume().clone();
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

  fn parse_index_expr(&mut self, left: Expression) -> Result<Expression> {
    self.consume();
    if self.current().kind == TokenKind::DoubleDot {
      return self.parse_range_index(left, None);
    }

    let index = self.parse_expression()?;
    if self.current().kind == TokenKind::DoubleDot {
      return self.parse_range_index(left, Some(index));
    }

    self.expect(TokenKind::RightSquare)?;
    Ok(Expression::Index(Index {
      left: Box::new(left),
      index: Box::new(index),
    }))
  }

  fn parse_range_index(
    &mut self,
    left: Expression,
    start: Option<Expression>,
  ) -> Result<Expression> {
    let location = self.consume().location.clone();

    let end = if self.current().kind == TokenKind::RightSquare {
      None
    } else {
      Some(self.parse_expression()?)
    };

    if end.is_none() && start.is_none() {
      return Err(raise_error(
        location,
        "Range indexing must have a start or an end",
      ));
    }

    self.expect(TokenKind::RightSquare)?;
    Ok(Expression::RangeIndex(RangeIndex {
      left: Box::new(left),
      start: start.map(Box::new),
      end: end.map(Box::new),
    }))
  }

  fn parse_member_expr(&mut self, left: Expression) -> Result<Expression> {
    self.consume();
    let member = match self.current().kind {
      TokenKind::Identifier => {
        let token = self.consume();
        let member = validate_or_quote(
          token.value.clone(),
          &token.location,
          NameKind::NBTPathComponent,
        );
        MemberKind::Literal(member)
      }
      TokenKind::LeftSquare => {
        self.consume();
        let expr = self.parse_expression()?;
        self.expect(TokenKind::RightSquare)?;
        MemberKind::Dynamic(expr)
      }
      _ => {
        return Err(raise_error(
          self.current().location.clone(),
          "Expected an identifier or square-bracket after member access operator.",
        ))
      }
    };
    Ok(Expression::Member(Member {
      left: Box::new(left),
      member: Box::new(member),
    }))
  }

  fn parse_prefix_expression(&mut self) -> Result<Expression> {
    let token = self.consume();
    let location = token.location.clone();
    let operator = match token.kind {
      TokenKind::Bang => UnaryOperator::LogicalNot,
      TokenKind::Minus => UnaryOperator::Negation,
      _ => unreachable!(),
    };
    let operand = self.parse_sub_expression(Precedence::Prefix)?;
    Ok(Expression::UnaryOperation(UnaryExpression {
      location,
      operator,
      operand: Box::new(operand),
    }))
  }

  pub(super) fn parse_comptime_variable(&mut self) -> Result<Expression> {
    self.consume();
    let path = self.parse_zoglin_resource(NameKind::ComptimeVariable)?;
    if self.current().kind == TokenKind::LeftParen {
      let call = self.parse_function_call(path, true)?;
      Ok(Expression::FunctionCall(call))
    } else {
      match path {
        ZoglinResource {
          location,
          namespace: None,
          modules,
          name,
        } if modules.is_empty() => Ok(Expression::ComptimeVariable(name, location)),
        ZoglinResource { location, .. } => Err(raise_error(
          location,
          "Compile-time variables are not namespaced.",
        )),
      }
    }
  }
}

type InfixFn = fn(&mut Parser, Expression) -> Result<Expression>;
