use std::borrow::Borrow;

use crate::parser::ast::{BinaryOperation, Expression, Operator};

use super::{
  file_tree::{FunctionLocation, ScoreboardLocation, StorageLocation},
  Compiler, ExpressionType, ScoreKind, StorageKind,
};

impl Compiler {
  pub(super) fn compile_binary_operation(
    &self,
    binary_operation: &BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>
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
      Operator::LessThan => todo!(),
      Operator::GreaterThan => todo!(),
      Operator::LessThanEquals => todo!(),
      Operator::GreaterThanEquals => todo!(),
      Operator::Equal => todo!(),
      Operator::NotEqual => todo!(),
      Operator::LogicalAnd => todo!(),
      Operator::LogicalOr => todo!(),
      Operator::Assign => self.compile_assignment(binary_operation, location, code),
      Operator::OperatorAssign(_) => todo!(),
    }
  }

  fn compile_assignment(
    &self,
    binary_operation: &BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>
  ) -> ExpressionType {
    let Expression::Variable(variable) = binary_operation.left.borrow() else {
      panic!("Can only assign to variables.")
    };

    let typ = self.compile_expression(&binary_operation.right, location, code);

    let (command, kind) = typ.to_storage();

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
    &self,
    binary_operation: &BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>
  ) -> ExpressionType {
    let left = self.compile_expression(&binary_operation.left, location, code);
    let right = self.compile_expression(&binary_operation.right, location, code);

    match (left, right) {
      (ExpressionType::Void, _) | (_, ExpressionType::Void) => {
        panic!("Cannot add type void to another value.")
      }
      (ExpressionType::Integer(a), ExpressionType::Integer(b)) => ExpressionType::Integer(a + b),
      (left, right) => {
        self.compile_basic_operator(left, right, '+', code)
      }
    }
  }

  fn compile_minus(
    &self,
    binary_operation: &BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>
  ) -> ExpressionType {
    let left = self.compile_expression(&binary_operation.left, location, code);
    let right = self.compile_expression(&binary_operation.right, location, code);

    match (left, right) {
      (ExpressionType::Void, _) | (_, ExpressionType::Void) => {
        panic!("Cannot perform subtraction with void.")
      }
      (ExpressionType::Integer(a), ExpressionType::Integer(b)) => ExpressionType::Integer(a - b),
      (left, right) => {
        self.compile_basic_operator(left, right, '-', code)
      }
    }
  }

  fn compile_multiply(
    &self,
    binary_operation: &BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>
  ) -> ExpressionType {
    let left = self.compile_expression(&binary_operation.left, location, code);
    let right = self.compile_expression(&binary_operation.right, location, code);

    match (left, right) {
      (ExpressionType::Void, _) | (_, ExpressionType::Void) => {
        panic!("Cannot perform multiplication with void.")
      }
      (ExpressionType::Integer(a), ExpressionType::Integer(b)) => ExpressionType::Integer(a * b),
      (left, right) => {
        self.compile_basic_operator(left, right, '*', code)
      }
    }
  }

  fn compile_divide(
    &self,
    binary_operation: &BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>
  ) -> ExpressionType {
    let left = self.compile_expression(&binary_operation.left, location, code);
    let right = self.compile_expression(&binary_operation.right, location, code);

    match (left, right) {
      (ExpressionType::Void, _) | (_, ExpressionType::Void) => {
        panic!("Cannot perform division with void.")
      }
      (ExpressionType::Integer(a), ExpressionType::Integer(b)) => ExpressionType::Integer(a / b),
      (left, right) => {
        self.compile_basic_operator(left, right, '/', code)
      }
    }
  }

  fn compile_modulo(
    &self,
    binary_operation: &BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>
  ) -> ExpressionType {
    let left = self.compile_expression(&binary_operation.left, location, code);
    let right = self.compile_expression(&binary_operation.right, location, code);

    match (left, right) {
      (ExpressionType::Void, _) | (_, ExpressionType::Void) => {
        panic!("Cannot perform modulo with void.")
      }
      (ExpressionType::Integer(a), ExpressionType::Integer(b)) => ExpressionType::Integer(a % b),
      (left, right) => {
        self.compile_basic_operator(left, right, '%', code)
      }
    }
  }

  fn compile_basic_operator(&self, left: ExpressionType, right: ExpressionType, operator: char, code: &mut Vec<String>) -> ExpressionType {
    let left_scoreboard = self.state.borrow_mut().next_scoreboard();
    let right_scoreboard: ScoreboardLocation = self.state.borrow_mut().next_scoreboard();
    code.push(self.copy_to_scoreboard(left, &left_scoreboard));
    code.push(self.copy_to_scoreboard(right, &right_scoreboard));
    code.push(format!(
      "scoreboard players operation {} {}= {}",
      left_scoreboard.to_string(),
      operator,
      right_scoreboard.to_string()
    ));
    ExpressionType::Scoreboard(left_scoreboard)
  }

  fn copy_to_scoreboard(&self, value: ExpressionType, scoreboard: &ScoreboardLocation) -> String {
    let (code, kind) = value.to_score();
    match kind {
      ScoreKind::Direct(operation) => format!(
        "scoreboard players {} {} {}",
        operation,
        scoreboard.to_string(),
        code
      ),
      ScoreKind::Indirect => format!(
        "execute store result score {} run {}",
        scoreboard.to_string(),
        code
      ),
    }
  }
}
