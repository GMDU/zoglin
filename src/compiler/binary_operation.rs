use std::borrow::Borrow;

use crate::parser::ast::{self, BinaryOperation, Operator};

use crate::error::{raise_error, Location, Result};

use super::{
  expression::{Condition, Expression, ScoreKind, StorageKind},
  file_tree::{FunctionLocation, ScoreboardLocation, StorageLocation},
  Compiler,
};

impl Compiler {
  pub(super) fn compile_binary_operation(
    &mut self,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> Result<Expression> {
    match binary_operation.operator {
      Operator::Plus => self.compile_plus(binary_operation, location, code),
      Operator::Minus => self.compile_minus(binary_operation, location, code),
      Operator::Divide => self.compile_divide(binary_operation, location, code),
      Operator::Multiply => self.compile_multiply(binary_operation, location, code),
      Operator::Modulo => self.compile_modulo(binary_operation, location, code),
      Operator::Power => todo!(),
      Operator::LeftShift => todo!(),
      Operator::RightShift => todo!(),
      Operator::LessThan => self.compile_less_than(code, binary_operation, location),
      Operator::GreaterThan => self.compile_greater_than(code, binary_operation, location),
      Operator::LessThanEquals => self.compile_less_than_equals(code, binary_operation, location),
      Operator::GreaterThanEquals => {
        self.compile_greater_than_equals(code, binary_operation, location)
      }
      Operator::Equal => todo!(),
      Operator::NotEqual => todo!(),
      Operator::LogicalAnd => todo!(),
      Operator::LogicalOr => todo!(),
      Operator::Assign => self.compile_assignment(binary_operation, location, code),
      Operator::OperatorAssign(_) => todo!(),
    }
  }

  fn compile_assignment(
    &mut self,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> Result<Expression> {
    let ast::Expression::Variable(variable) = binary_operation.left.borrow() else {
      return Err(raise_error(
        binary_operation.left.location(),
        "Can only assign to variables.",
      ));
    };

    let typ = self.compile_expression(*binary_operation.right, location, code)?;

    let (command, kind) = typ.to_storage(self, code)?;

    match kind {
      StorageKind::Direct => {
        code.push(format!(
          "data modify storage {} set {}",
          StorageLocation::from_zoglin_resource(location.clone(), variable).to_string(),
          command
        ));
      }
      StorageKind::Indirect => {
        code.push(format!(
          "execute store result storage {} int 1 run {}",
          StorageLocation::from_zoglin_resource(location.clone(), variable).to_string(),
          command
        ));
      }
    }

    Ok(typ)
  }

  fn compile_plus(
    &mut self,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code)?;
    let right = self.compile_expression(*binary_operation.right, location, code)?;

    match (left, right) {
      (Expression::Void(location), _) | (_, Expression::Void(location)) => Err(raise_error(
        location,
        "Cannot add type void to another value.",
      )),
      (Expression::Boolean(_, location), _) | (_, Expression::Boolean(_, location)) => {
        Err(raise_error(location, "Cannot perform plus with boolean."))
      }
      (Expression::String(_, location), _) | (_, Expression::String(_, location)) => {
        Err(raise_error(location, "Cannot perform plus with string."))
      }
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        Ok(Expression::Integer(
          left.numeric_value().unwrap() + right.numeric_value().unwrap(),
          binary_operation.location,
        ))
      }
      (num, other) | (other, num) if num.numeric_value().is_some() => {
        let scoreboard = self.copy_to_scoreboard(code, other)?;
        code.push(format!(
          "scoreboard players add {} {}",
          scoreboard.to_string(),
          num.numeric_value().unwrap(),
        ));
        Ok(Expression::Scoreboard(
          scoreboard,
          binary_operation.location,
        ))
      }
      (left, right) => {
        self.compile_basic_operator(left, right, binary_operation.location, '+', code)
      }
    }
  }

  fn compile_minus(
    &mut self,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code)?;
    let right = self.compile_expression(*binary_operation.right, location, code)?;

    match (left, right) {
      (Expression::Void(location), _) | (_, Expression::Void(location)) => Err(raise_error(
        location,
        "Cannot perform subtraction with void.",
      )),
      (Expression::Boolean(_, location), _) | (_, Expression::Boolean(_, location)) => Err(
        raise_error(location, "Cannot perform subtraction with boolean."),
      ),
      (Expression::String(_, location), _) | (_, Expression::String(_, location)) => Err(
        raise_error(location, "Cannot perform subtraction with string."),
      ),
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        Ok(Expression::Integer(
          left.numeric_value().unwrap() - right.numeric_value().unwrap(),
          binary_operation.location,
        ))
      }
      (other, num) if num.numeric_value().is_some() => {
        let scoreboard = self.copy_to_scoreboard(code, other)?;
        code.push(format!(
          "scoreboard players remove {} {}",
          scoreboard.to_string(),
          num.numeric_value().unwrap(),
        ));
        Ok(Expression::Scoreboard(
          scoreboard,
          binary_operation.location,
        ))
      }
      (left, right) => {
        self.compile_basic_operator(left, right, binary_operation.location, '-', code)
      }
    }
  }

  fn compile_multiply(
    &mut self,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code)?;
    let right = self.compile_expression(*binary_operation.right, location, code)?;

    match (left, right) {
      (Expression::Void(location), _) | (_, Expression::Void(location)) => Err(raise_error(
        location,
        "Cannot perform multiplication with void.",
      )),
      (Expression::Boolean(_, location), _) | (_, Expression::Boolean(_, location)) => Err(
        raise_error(location, "Cannot perform multiplication with boolean."),
      ),
      (Expression::String(_, location), _) | (_, Expression::String(_, location)) => Err(
        raise_error(location, "Cannot perform multiplication with string."),
      ),
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        Ok(Expression::Integer(
          left.numeric_value().unwrap() * right.numeric_value().unwrap(),
          binary_operation.location,
        ))
      }
      (left, right) => {
        self.compile_basic_operator(left, right, binary_operation.location, '*', code)
      }
    }
  }

  fn compile_divide(
    &mut self,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code)?;
    let right = self.compile_expression(*binary_operation.right, location, code)?;

    match (left, right) {
      (Expression::Void(location), _) | (_, Expression::Void(location)) => {
        Err(raise_error(location, "Cannot perform division with void."))
      }
      (Expression::Boolean(_, location), _) | (_, Expression::Boolean(_, location)) => Err(
        raise_error(location, "Cannot perform division with boolean."),
      ),
      (Expression::String(_, location), _) | (_, Expression::String(_, location)) => Err(
        raise_error(location, "Cannot perform division with string."),
      ),
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        Ok(Expression::Integer(
          left.numeric_value().unwrap() / right.numeric_value().unwrap(),
          binary_operation.location,
        ))
      }
      (left, right) => {
        self.compile_basic_operator(left, right, binary_operation.location, '/', code)
      }
    }
  }

  fn compile_modulo(
    &mut self,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code)?;
    let right = self.compile_expression(*binary_operation.right, location, code)?;

    match (left, right) {
      (Expression::Void(location), _) | (_, Expression::Void(location)) => {
        Err(raise_error(location, "Cannot perform modulo with void."))
      }
      (Expression::Boolean(_, location), _) | (_, Expression::Boolean(_, location)) => {
        Err(raise_error(location, "Cannot perform modulo with boolean."))
      }
      (Expression::String(_, location), _) | (_, Expression::String(_, location)) => {
        Err(raise_error(location, "Cannot perform modulo with string."))
      }
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        Ok(Expression::Integer(
          left.numeric_value().unwrap() % right.numeric_value().unwrap(),
          binary_operation.location,
        ))
      }
      (left, right) => {
        self.compile_basic_operator(left, right, binary_operation.location, '%', code)
      }
    }
  }

  fn compile_less_than(
    &mut self,
    code: &mut Vec<String>,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code)?;
    let right = self.compile_expression(*binary_operation.right, location, code)?;

    match (left, right) {
      (Expression::Void(location), _) | (_, Expression::Void(location)) => {
        Err(raise_error(location, "Cannot compare with void."))
      }
      (Expression::Boolean(_, location), _) | (_, Expression::Boolean(_, location)) => {
        Err(raise_error(location, "Cannot compare with boolean."))
      }
      (Expression::String(_, location), _) | (_, Expression::String(_, location)) => {
        Err(raise_error(location, "Cannot compare with string."))
      }
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        Ok(Expression::Boolean(
          left.numeric_value().unwrap() < right.numeric_value().unwrap(),
          binary_operation.location,
        ))
      }
      (num, other) if num.numeric_value().is_some() => self.compile_match_comparison(
        code,
        other,
        binary_operation.location,
        format!("{}..", num.numeric_value().unwrap() + 1),
      ),
      (other, num) if num.numeric_value().is_some() => self.compile_match_comparison(
        code,
        other,
        binary_operation.location,
        format!("..{}", num.numeric_value().unwrap() - 1),
      ),
      (left, right) => {
        self.compile_comparison_operator(code, left, right, binary_operation.location, "<")
      }
    }
  }

  fn compile_greater_than(
    &mut self,
    code: &mut Vec<String>,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code)?;
    let right = self.compile_expression(*binary_operation.right, location, code)?;

    match (left, right) {
      (Expression::Void(location), _) | (_, Expression::Void(location)) => {
        Err(raise_error(location, "Cannot compare with void."))
      }
      (Expression::Boolean(_, location), _) | (_, Expression::Boolean(_, location)) => {
        Err(raise_error(location, "Cannot compare with boolean."))
      }
      (Expression::String(_, location), _) | (_, Expression::String(_, location)) => {
        Err(raise_error(location, "Cannot compare with string."))
      }
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        Ok(Expression::Boolean(
          left.numeric_value().unwrap() > right.numeric_value().unwrap(),
          binary_operation.location,
        ))
      }
      (num, other) if num.numeric_value().is_some() => self.compile_match_comparison(
        code,
        other,
        binary_operation.location,
        format!("..{}", num.numeric_value().unwrap() - 1),
      ),
      (other, num) if num.numeric_value().is_some() => self.compile_match_comparison(
        code,
        other,
        binary_operation.location,
        format!("{}..", num.numeric_value().unwrap() + 1),
      ),
      (left, right) => {
        self.compile_comparison_operator(code, left, right, binary_operation.location, ">")
      }
    }
  }

  fn compile_less_than_equals(
    &mut self,
    code: &mut Vec<String>,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code)?;
    let right = self.compile_expression(*binary_operation.right, location, code)?;

    match (left, right) {
      (Expression::Void(location), _) | (_, Expression::Void(location)) => {
        Err(raise_error(location, "Cannot compare with void."))
      }
      (Expression::String(_, location), _) | (_, Expression::String(_, location)) => {
        Err(raise_error(location, "Cannot compare with string."))
      }
      (Expression::Boolean(_, location), _) | (_, Expression::Boolean(_, location)) => {
        Err(raise_error(location, "Cannot compare with boolean."))
      }
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        Ok(Expression::Boolean(
          left.numeric_value().unwrap() <= right.numeric_value().unwrap(),
          binary_operation.location,
        ))
      }
      (num, other) if num.numeric_value().is_some() => self.compile_match_comparison(
        code,
        other,
        binary_operation.location,
        format!("{}..", num.numeric_value().unwrap()),
      ),
      (other, num) if num.numeric_value().is_some() => self.compile_match_comparison(
        code,
        other,
        binary_operation.location,
        format!("..{}", num.numeric_value().unwrap()),
      ),
      (left, right) => {
        self.compile_comparison_operator(code, left, right, binary_operation.location, "<=")
      }
    }
  }

  fn compile_greater_than_equals(
    &mut self,
    code: &mut Vec<String>,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code)?;
    let right = self.compile_expression(*binary_operation.right, location, code)?;

    match (left, right) {
      (Expression::Void(location), _) | (_, Expression::Void(location)) => {
        Err(raise_error(location, "Cannot compare with void."))
      }
      (Expression::Boolean(_, location), _) | (_, Expression::Boolean(_, location)) => {
        Err(raise_error(location, "Cannot compare with boolean."))
      }
      (Expression::String(_, location), _) | (_, Expression::String(_, location)) => {
        Err(raise_error(location, "Cannot compare with string."))
      }
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        Ok(Expression::Boolean(
          left.numeric_value().unwrap() >= right.numeric_value().unwrap(),
          binary_operation.location,
        ))
      }
      (num, other) if num.numeric_value().is_some() => self.compile_match_comparison(
        code,
        other,
        binary_operation.location,
        format!("..{}", num.numeric_value().unwrap()),
      ),
      (other, num) if num.numeric_value().is_some() => self.compile_match_comparison(
        code,
        other,
        binary_operation.location,
        format!("{}..", num.numeric_value().unwrap()),
      ),
      (left, right) => {
        self.compile_comparison_operator(code, left, right, binary_operation.location, ">=")
      }
    }
  }

  fn compile_basic_operator(
    &mut self,
    left: Expression,
    right: Expression,
    location: Location,
    operator: char,
    code: &mut Vec<String>,
  ) -> Result<Expression> {
    let left_scoreboard = self.copy_to_scoreboard(code, left)?;
    let right_scoreboard = self.move_to_scoreboard(code, right)?;
    code.push(format!(
      "scoreboard players operation {} {}= {}",
      left_scoreboard.to_string(),
      operator,
      right_scoreboard.to_string()
    ));
    Ok(Expression::Scoreboard(left_scoreboard, location))
  }

  fn compile_comparison_operator(
    &mut self,
    code: &mut Vec<String>,
    left: Expression,
    right: Expression,
    location: Location,
    operator: &str,
  ) -> Result<Expression> {
    let left_scoreboard = self.move_to_scoreboard(code, left)?;
    let right_scoreboard = self.move_to_scoreboard(code, right)?;
    Ok(Expression::Condition(
      Condition::from_operator(operator, left_scoreboard, right_scoreboard),
      location,
    ))
  }

  fn compile_match_comparison(
    &mut self,
    code: &mut Vec<String>,
    value: Expression,
    location: Location,
    range: String,
  ) -> Result<Expression> {
    let scoreboard = self.move_to_scoreboard(code, value)?;
    Ok(Expression::Condition(
      Condition::Match(scoreboard, range),
      location,
    ))
  }

  pub(super) fn copy_to_scoreboard(
    &mut self,
    code: &mut Vec<String>,
    value: Expression,
  ) -> Result<ScoreboardLocation> {
    let scoreboard = self.next_scoreboard();
    let (conversion_code, kind) = value.to_score()?;
    match kind {
      ScoreKind::Direct(operation) => code.push(format!(
        "scoreboard players {} {} {}",
        operation,
        scoreboard.to_string(),
        conversion_code
      )),
      ScoreKind::Indirect => code.push(format!(
        "execute store result score {} run {}",
        scoreboard.to_string(),
        conversion_code
      )),
    }

    Ok(scoreboard)
  }

  pub(super) fn move_to_scoreboard(
    &mut self,
    code: &mut Vec<String>,
    value: Expression,
  ) -> Result<ScoreboardLocation> {
    if let Expression::Scoreboard(scoreboard, _) = value {
      return Ok(scoreboard);
    }

    let scoreboard = self.next_scoreboard();
    let (conversion_code, kind) = value.to_score()?;
    match kind {
      ScoreKind::Direct(operation) => code.push(format!(
        "scoreboard players {} {} {}",
        operation,
        scoreboard.to_string(),
        conversion_code
      )),
      ScoreKind::Indirect => code.push(format!(
        "execute store result score {} run {}",
        scoreboard.to_string(),
        conversion_code
      )),
    }

    Ok(scoreboard)
  }
}
