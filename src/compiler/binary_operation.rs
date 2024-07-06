use std::borrow::Borrow;

use crate::parser::ast::{BinaryOperation, Expression, Operator};

use super::{
  expression::{Condition, ExpressionType, ScoreKind, StorageKind},
  file_tree::{FunctionLocation, ScoreboardLocation, StorageLocation},
  Compiler,
};

impl Compiler {
  pub(super) fn compile_binary_operation(
    &mut self,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> ExpressionType {
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
  ) -> ExpressionType {
    let Expression::Variable(variable) = binary_operation.left.borrow() else {
      panic!("Can only assign to variables.")
    };

    let typ = self.compile_expression(*binary_operation.right, location, code);

    let (command, kind) = typ.to_storage(self, code);

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

    typ
  }

  fn compile_plus(
    &mut self,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> ExpressionType {
    let left = self.compile_expression(*binary_operation.left, location, code);
    let right = self.compile_expression(*binary_operation.right, location, code);

    match (left, right) {
      (ExpressionType::Void, _) | (_, ExpressionType::Void) => {
        panic!("Cannot add type void to another value.")
      }
      (ExpressionType::Boolean(_), _) | (_, ExpressionType::Boolean(_)) => {
        panic!("Cannot perform plus with boolean.")
      }
      (ExpressionType::String(_), _) | (_, ExpressionType::String(_)) => {
        panic!("Cannot perform plus with string.")
      }
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        ExpressionType::Integer(left.numeric_value().unwrap() + right.numeric_value().unwrap())
      }
      (num, other) | (other, num) if num.numeric_value().is_some() => {
        let scoreboard = self.copy_to_scoreboard(code, other);
        code.push(format!(
          "scoreboard players add {} {}",
          scoreboard.to_string(),
          num.numeric_value().unwrap(),
        ));
        ExpressionType::Scoreboard(scoreboard)
      }
      (left, right) => self.compile_basic_operator(left, right, '+', code),
    }
  }

  fn compile_minus(
    &mut self,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> ExpressionType {
    let left = self.compile_expression(*binary_operation.left, location, code);
    let right = self.compile_expression(*binary_operation.right, location, code);

    match (left, right) {
      (ExpressionType::Void, _) | (_, ExpressionType::Void) => {
        panic!("Cannot perform subtraction with void.")
      }
      (ExpressionType::Boolean(_), _) | (_, ExpressionType::Boolean(_)) => {
        panic!("Cannot perform subtraction with boolean.")
      }
      (ExpressionType::String(_), _) | (_, ExpressionType::String(_)) => {
        panic!("Cannot perform subtraction with string.")
      }
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        ExpressionType::Integer(left.numeric_value().unwrap() - right.numeric_value().unwrap())
      }
      (other, num) if num.numeric_value().is_some() => {
        let scoreboard = self.copy_to_scoreboard(code, other);
        code.push(format!(
          "scoreboard players remove {} {}",
          scoreboard.to_string(),
          num.numeric_value().unwrap(),
        ));
        ExpressionType::Scoreboard(scoreboard)
      }
      (left, right) => self.compile_basic_operator(left, right, '-', code),
    }
  }

  fn compile_multiply(
    &mut self,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> ExpressionType {
    let left = self.compile_expression(*binary_operation.left, location, code);
    let right = self.compile_expression(*binary_operation.right, location, code);

    match (left, right) {
      (ExpressionType::Void, _) | (_, ExpressionType::Void) => {
        panic!("Cannot perform multiplication with void.")
      }
      (ExpressionType::Boolean(_), _) | (_, ExpressionType::Boolean(_)) => {
        panic!("Cannot perform multiplication with boolean.")
      }
      (ExpressionType::String(_), _) | (_, ExpressionType::String(_)) => {
        panic!("Cannot perform multiplication with string.")
      }
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        ExpressionType::Integer(left.numeric_value().unwrap() * right.numeric_value().unwrap())
      }
      (left, right) => self.compile_basic_operator(left, right, '*', code),
    }
  }

  fn compile_divide(
    &mut self,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> ExpressionType {
    let left = self.compile_expression(*binary_operation.left, location, code);
    let right = self.compile_expression(*binary_operation.right, location, code);

    match (left, right) {
      (ExpressionType::Void, _) | (_, ExpressionType::Void) => {
        panic!("Cannot perform division with void.")
      }
      (ExpressionType::Boolean(_), _) | (_, ExpressionType::Boolean(_)) => {
        panic!("Cannot perform division with boolean.")
      }
      (ExpressionType::String(_), _) | (_, ExpressionType::String(_)) => {
        panic!("Cannot perform division with string.")
      }
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        ExpressionType::Integer(left.numeric_value().unwrap() / right.numeric_value().unwrap())
      }
      (left, right) => self.compile_basic_operator(left, right, '/', code),
    }
  }

  fn compile_modulo(
    &mut self,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> ExpressionType {
    let left = self.compile_expression(*binary_operation.left, location, code);
    let right = self.compile_expression(*binary_operation.right, location, code);

    match (left, right) {
      (ExpressionType::Void, _) | (_, ExpressionType::Void) => {
        panic!("Cannot perform modulo with void.")
      }
      (ExpressionType::Boolean(_), _) | (_, ExpressionType::Boolean(_)) => {
        panic!("Cannot perform modulo with boolean.")
      }
      (ExpressionType::String(_), _) | (_, ExpressionType::String(_)) => {
        panic!("Cannot perform modulo with string.")
      }
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        ExpressionType::Integer(left.numeric_value().unwrap() % right.numeric_value().unwrap())
      }
      (left, right) => self.compile_basic_operator(left, right, '%', code),
    }
  }

  fn compile_less_than(
    &mut self,
    code: &mut Vec<String>,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
  ) -> ExpressionType {
    let left = self.compile_expression(*binary_operation.left, location, code);
    let right = self.compile_expression(*binary_operation.right, location, code);

    match (left, right) {
      (ExpressionType::Void, _) | (_, ExpressionType::Void) => {
        panic!("Cannot compare with void.")
      }
      (ExpressionType::Boolean(_), _) | (_, ExpressionType::Boolean(_)) => {
        panic!("Cannot compare with boolean.")
      }
      (ExpressionType::String(_), _) | (_, ExpressionType::String(_)) => {
        panic!("Cannot compare with string.")
      }
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        ExpressionType::Boolean(left.numeric_value().unwrap() < right.numeric_value().unwrap())
      }
      (num, other) if num.numeric_value().is_some() => self.compile_match_comparison(
        code,
        other,
        format!("{}..", num.numeric_value().unwrap() + 1),
      ),
      (other, num) if num.numeric_value().is_some() => self.compile_match_comparison(
        code,
        other,
        format!("..{}", num.numeric_value().unwrap() - 1),
      ),
      (left, right) => self.compile_comparison_operator(code, left, right, "<"),
    }
  }

  fn compile_greater_than(
    &mut self,
    code: &mut Vec<String>,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
  ) -> ExpressionType {
    let left = self.compile_expression(*binary_operation.left, location, code);
    let right = self.compile_expression(*binary_operation.right, location, code);

    match (left, right) {
      (ExpressionType::Void, _) | (_, ExpressionType::Void) => {
        panic!("Cannot compare with void.")
      }
      (ExpressionType::Boolean(_), _) | (_, ExpressionType::Boolean(_)) => {
        panic!("Cannot compare with boolean.")
      }
      (ExpressionType::String(_), _) | (_, ExpressionType::String(_)) => {
        panic!("Cannot compare with string.")
      }
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        ExpressionType::Boolean(left.numeric_value().unwrap() > right.numeric_value().unwrap())
      }
      (num, other) if num.numeric_value().is_some() => self.compile_match_comparison(
        code,
        other,
        format!("..{}", num.numeric_value().unwrap() - 1),
      ),
      (other, num) if num.numeric_value().is_some() => self.compile_match_comparison(
        code,
        other,
        format!("{}..", num.numeric_value().unwrap() + 1),
      ),
      (left, right) => self.compile_comparison_operator(code, left, right, ">"),
    }
  }

  fn compile_less_than_equals(
    &mut self,
    code: &mut Vec<String>,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
  ) -> ExpressionType {
    let left = self.compile_expression(*binary_operation.left, location, code);
    let right = self.compile_expression(*binary_operation.right, location, code);

    match (left, right) {
      (ExpressionType::Void, _) | (_, ExpressionType::Void) => {
        panic!("Cannot compare with void.")
      }
      (ExpressionType::String(_), _) | (_, ExpressionType::String(_)) => {
        panic!("Cannot compare with string.")
      }
      (ExpressionType::Boolean(_), _) | (_, ExpressionType::Boolean(_)) => {
        panic!("Cannot compare with boolean.")
      }
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        ExpressionType::Boolean(left.numeric_value().unwrap() <= right.numeric_value().unwrap())
      }
      (num, other) if num.numeric_value().is_some() => {
        self.compile_match_comparison(code, other, format!("{}..", num.numeric_value().unwrap()))
      }
      (other, num) if num.numeric_value().is_some() => {
        self.compile_match_comparison(code, other, format!("..{}", num.numeric_value().unwrap()))
      }
      (left, right) => self.compile_comparison_operator(code, left, right, "<="),
    }
  }

  fn compile_greater_than_equals(
    &mut self,
    code: &mut Vec<String>,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
  ) -> ExpressionType {
    let left = self.compile_expression(*binary_operation.left, location, code);
    let right = self.compile_expression(*binary_operation.right, location, code);

    match (left, right) {
      (ExpressionType::Void, _) | (_, ExpressionType::Void) => {
        panic!("Cannot compare with void.")
      }
      (ExpressionType::Boolean(_), _) | (_, ExpressionType::Boolean(_)) => {
        panic!("Cannot compare with boolean.")
      }
      (ExpressionType::String(_), _) | (_, ExpressionType::String(_)) => {
        panic!("Cannot compare with string.")
      }
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        ExpressionType::Boolean(left.numeric_value().unwrap() >= right.numeric_value().unwrap())
      }
      (num, other) if num.numeric_value().is_some() => {
        self.compile_match_comparison(code, other, format!("..{}", num.numeric_value().unwrap()))
      }
      (other, num) if num.numeric_value().is_some() => {
        self.compile_match_comparison(code, other, format!("{}..", num.numeric_value().unwrap()))
      }
      (left, right) => self.compile_comparison_operator(code, left, right, ">="),
    }
  }

  fn compile_basic_operator(
    &mut self,
    left: ExpressionType,
    right: ExpressionType,
    operator: char,
    code: &mut Vec<String>,
  ) -> ExpressionType {
    let left_scoreboard = self.copy_to_scoreboard(code, left);
    let right_scoreboard = self.move_to_scoreboard(code, right);
    code.push(format!(
      "scoreboard players operation {} {}= {}",
      left_scoreboard.to_string(),
      operator,
      right_scoreboard.to_string()
    ));
    ExpressionType::Scoreboard(left_scoreboard)
  }

  fn compile_comparison_operator(
    &mut self,
    code: &mut Vec<String>,
    left: ExpressionType,
    right: ExpressionType,
    operator: &str,
  ) -> ExpressionType {
    let left_scoreboard = self.move_to_scoreboard(code, left);
    let right_scoreboard = self.move_to_scoreboard(code, right);
    ExpressionType::Condition(Condition::from_operator(
      operator,
      left_scoreboard,
      right_scoreboard,
    ))
  }

  fn compile_match_comparison(
    &mut self,
    code: &mut Vec<String>,
    value: ExpressionType,
    range: String,
  ) -> ExpressionType {
    let scoreboard = self.move_to_scoreboard(code, value);
    ExpressionType::Condition(Condition::Match(scoreboard, range))
  }

  pub(super) fn copy_to_scoreboard(
    &mut self,
    code: &mut Vec<String>,
    value: ExpressionType,
  ) -> ScoreboardLocation {
    let scoreboard = self.next_scoreboard();
    let (conversion_code, kind) = value.to_score();
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

    scoreboard
  }

  pub(super) fn move_to_scoreboard(
    &mut self,
    code: &mut Vec<String>,
    value: ExpressionType,
  ) -> ScoreboardLocation {
    if let ExpressionType::Scoreboard(scoreboard) = value {
      return scoreboard;
    }

    let scoreboard = self.next_scoreboard();
    let (conversion_code, kind) = value.to_score();
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

    scoreboard
  }
}
