use std::borrow::Borrow;

use crate::parser::ast::{BinaryOperation, Expression, Operator};

use super::{
  file_tree::{FunctionLocation, StorageLocation},
  Compiler, ExpressionType,
};

impl Compiler {
  pub(super) fn compile_binary_operation(
    &self,
    binary_operation: &BinaryOperation,
    location: &FunctionLocation,
  ) -> (Vec<String>, ExpressionType) {
    match binary_operation.operator {
      Operator::Plus => todo!(),
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

    lines.push(format!(
      "data modify storage {} set {}",
      StorageLocation::from_zoglin_resource(location.clone(), variable).to_string(),
      typ.to_storage()
    ));

    (lines, typ)
  }
}
