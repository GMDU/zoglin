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
  ) -> (Vec<String>, ExpressionType) {
    match binary_operation.operator {
      Operator::Plus => self.compile_plus(binary_operation, location),
      Operator::Minus => todo!(),
      Operator::Divide => todo!(),
      Operator::Multiply => todo!(),
      Operator::Modulo => todo!(),
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
      Operator::Assign => self.compile_assignment(binary_operation, location),
      Operator::OperatorAssign(_) => todo!(),
    }
  }

  fn compile_assignment(
    &self,
    binary_operation: &BinaryOperation,
    location: &FunctionLocation,
  ) -> (Vec<String>, ExpressionType) {
    let Expression::Variable(variable) = binary_operation.left.borrow() else {
      panic!("Can only assign to variables")
    };

    let (mut lines, typ) = self.compile_expression(&binary_operation.right, location);

    let (code, kind) = typ.to_storage();

    match kind {
      StorageKind::Direct => {
        lines.push(format!(
          "data modify storage {} set {}",
          StorageLocation::from_zoglin_resource(location.clone(), variable).to_string(),
          code
        ));
      }
      StorageKind::Indirect => {
        lines.push(format!(
          "execute store result storage {} int 1 run {}",
          StorageLocation::from_zoglin_resource(location.clone(), variable).to_string(),
          code
        ));
      }
    }

    (lines, typ)
  }

  fn compile_plus(
    &self,
    binary_operation: &BinaryOperation,
    location: &FunctionLocation,
  ) -> (Vec<String>, ExpressionType) {
    let mut code = Vec::new();
    let (left_code, left_type) = self.compile_expression(&binary_operation.left, location);
    code.extend(left_code);
    let (right_code, right_type) = self.compile_expression(&binary_operation.right, location);
    code.extend(right_code);

    match (left_type, right_type) {
      (ExpressionType::Void, _) | (_, ExpressionType::Void) => {
        panic!("Cannot add type void to another value")
      }
      (ExpressionType::Integer(a), ExpressionType::Integer(b)) => {
        (code, ExpressionType::Integer(a + b))
      }
      (left, right) => {
        let left_scoreboard = self.state.borrow_mut().next_scoreboard();
        let right_scoreboard = self.state.borrow_mut().next_scoreboard();
        code.push(self.copy_to_scoreboard(left, &left_scoreboard));
        code.push(self.copy_to_scoreboard(right, &right_scoreboard));
        code.push(format!(
          "scoreboard players operation {} += {}",
          left_scoreboard.to_string(),
          right_scoreboard.to_string()
        ));
        (code, ExpressionType::Scoreboard(left_scoreboard))
      }
    }
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
