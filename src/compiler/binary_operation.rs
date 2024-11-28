use ecow::{eco_format, EcoString};

use crate::parser::ast::{self, BinaryOperation, Operator, UnaryExpression, UnaryOperator};

use crate::error::{raise_error, Result};

use super::expression::NbtType;
use super::utils::ToEcoString;
use super::FunctionContext;
use super::{
  expression::{Condition, ConditionKind, Expression, ExpressionKind, ScoreKind},
  file_tree::{ScoreboardLocation, StorageLocation},
  Compiler,
};

struct Operation {
  operator: &'static str,
  native_operation: Option<&'static str>,
  constant_operation: fn(i32, i32) -> i32,
  commutative: bool,
}

impl Operation {
  const ADD: Operation = Operation {
    operator: "+",
    native_operation: Some("add"),
    constant_operation: |a, b| a + b,
    commutative: true,
  };

  const SUB: Operation = Operation {
    operator: "-",
    native_operation: Some("remove"),
    constant_operation: |a, b| a - b,
    commutative: false,
  };

  const MUL: Operation = Operation {
    operator: "*",
    native_operation: None,
    constant_operation: |a, b| a * b,
    commutative: true,
  };

  const DIV: Operation = Operation {
    operator: "/",
    native_operation: None,
    constant_operation: |a, b| a / b,
    commutative: false,
  };

  const MOD: Operation = Operation {
    operator: "%",
    native_operation: None,
    constant_operation: |a, b| a % b,
    commutative: false,
  };

  fn from_operator(operator: Operator) -> Option<Operation> {
    match operator {
      Operator::Plus => Some(Operation::ADD),
      Operator::Minus => Some(Operation::SUB),
      Operator::Divide => Some(Operation::DIV),
      Operator::Multiply => Some(Operation::MUL),
      Operator::Modulo => Some(Operation::MOD),
      _ => None,
    }
  }
}

impl Compiler {
  pub(super) fn compile_binary_operation(
    &mut self,
    binary_operation: BinaryOperation,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    match binary_operation.operator {
      Operator::Plus => self.compile_numeric_operation(binary_operation, Operation::ADD, context),
      Operator::Minus => self.compile_numeric_operation(binary_operation, Operation::SUB, context),
      Operator::Divide => self.compile_numeric_operation(binary_operation, Operation::DIV, context),
      Operator::Multiply => {
        self.compile_numeric_operation(binary_operation, Operation::MUL, context)
      }
      Operator::Modulo => self.compile_numeric_operation(binary_operation, Operation::MOD, context),
      Operator::Power => todo!(),
      Operator::LeftShift => todo!(),
      Operator::RightShift => todo!(),
      Operator::LessThan => self.compile_less_than(binary_operation, context),
      Operator::GreaterThan => self.compile_greater_than(binary_operation, context),
      Operator::LessThanEquals => self.compile_less_than_equals(binary_operation, context),
      Operator::GreaterThanEquals => self.compile_greater_than_equals(binary_operation, context),
      Operator::Equal => self.compile_equals(binary_operation, context),
      Operator::NotEqual => self.compile_not_equals(binary_operation, context),
      Operator::LogicalAnd => self.compile_logical_and(binary_operation, context),
      Operator::LogicalOr => self.compile_logical_or(binary_operation, context),
      Operator::Assign => {
        self.compile_assignment(*binary_operation.left, *binary_operation.right, context)
      }
      Operator::AddAssign => {
        self.compile_operator_assignment(binary_operation, Operator::Plus, context)
      }
      Operator::SubAssign => {
        self.compile_operator_assignment(binary_operation, Operator::Minus, context)
      }
      Operator::MulAssign => {
        self.compile_operator_assignment(binary_operation, Operator::Multiply, context)
      }
      Operator::DivAssign => {
        self.compile_operator_assignment(binary_operation, Operator::Divide, context)
      }
      Operator::ModAssign => {
        self.compile_operator_assignment(binary_operation, Operator::Modulo, context)
      }
    }
  }

  fn compile_operator_assignment(
    &mut self,
    mut binary_operation: BinaryOperation,
    operator: Operator,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    binary_operation.operator = operator;

    match binary_operation.left.as_ref() {
      ast::Expression::ScoreboardVariable(variable) => {
        let right = self.compile_expression(*binary_operation.right, context, false)?;
        let scoreboard = ScoreboardLocation::from_zoglin_resource(&context.location, variable);
        self.use_scoreboard_dummy(scoreboard.scoreboard_string());
        self.scoreboard_operation(
          &scoreboard,
          right.clone(),
          Operation::from_operator(operator).expect("Operator must be numeric"),
          context,
        )?;

        Ok(right)
      }
      left => self.compile_assignment(
        left.clone(),
        ast::Expression::BinaryOperation(binary_operation),
        context,
      ),
    }
  }

  fn compile_assignment(
    &mut self,
    left: ast::Expression,
    right: ast::Expression,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    match left {
      ast::Expression::Variable(variable) => {
        let right = self.compile_expression(right, context, false)?;
        let storage = StorageLocation::from_zoglin_resource(&context.location, &variable);
        self.set_storage(&mut context.code, &storage, &right)?;

        Ok(right)
      }
      ast::Expression::ScoreboardVariable(variable) => {
        let right = self.compile_expression(right, context, false)?;
        let scoreboard = ScoreboardLocation::from_zoglin_resource(&context.location, &variable);
        self.set_scoreboard(&mut context.code, &scoreboard, &right)?;
        self.use_scoreboard_dummy(scoreboard.scoreboard_string());

        Ok(right)
      }
      ast::Expression::ComptimeVariable(name, _) => {
        let right = self.compile_expression(right, context, false)?;
        self
          .comptime_scopes
          .last_mut()
          .expect("The must be at least one scope")
          .insert(name, right.clone());
        Ok(right)
      }
      _ => Err(raise_error(
        left.location(),
        "Can only assign to variables.",
      )),
    }
  }

  fn compile_numeric_operation(
    &mut self,
    binary_operation: BinaryOperation,
    operation: Operation,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, context, false)?;
    let right = self.compile_expression(*binary_operation.right, context, false)?;
    let needs_macro = left.needs_macro || right.needs_macro;

    match (&left.kind, &right.kind) {
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        Ok(ExpressionKind::Integer((operation.constant_operation)(
          left.numeric_value().expect("Numeric value exists"),
          right.numeric_value().expect("Numeric value exists"),
        )))
      }
      // It's more efficient to have a constant value on the right-hand-side,
      // So if the left side is constant, we rearrange it. However, that only works
      // if the operator is commutative.
      (num, _) if operation.commutative && num.numeric_value().is_some() => {
        let scoreboard =
          self.copy_to_scoreboard(&mut context.code, &right, &context.location.namespace)?;
        self.scoreboard_operation(&scoreboard, left, operation, context)?;
        Ok(ExpressionKind::Scoreboard(scoreboard))
      }
      _ => {
        let scoreboard =
          self.copy_to_scoreboard(&mut context.code, &left, &context.location.namespace)?;
        self.scoreboard_operation(&scoreboard, right, operation, context)?;
        Ok(ExpressionKind::Scoreboard(scoreboard))
      }
    }
    .map(|kind| Expression::with_macro(kind, binary_operation.location, needs_macro))
  }

  fn scoreboard_operation(
    &mut self,
    scoreboard: &ScoreboardLocation,
    value: Expression,
    operation: Operation,
    context: &mut FunctionContext,
  ) -> Result<()> {
    match &value.kind {
      ExpressionKind::Void
      | ExpressionKind::String(_)
      | ExpressionKind::Array { .. }
      | ExpressionKind::ByteArray(_)
      | ExpressionKind::IntArray(_)
      | ExpressionKind::LongArray(_)
      | ExpressionKind::SubString(_, _, _)
      | ExpressionKind::Compound(_) => {
        return Err(raise_error(
          value.location,
          "Can only perform operations on numbers.",
        ))
      }
      num if num.numeric_value().is_some() => {
        let number = num.numeric_value().expect("Numeric value exists");
        if let Some(native_operation) = operation.native_operation {
          context.code.push(eco_format!(
            "scoreboard players {native_operation} {scoreboard} {number}",
          ));
        } else {
          let constant_scoreboard = self.constant_scoreboard(number);
          context.code.push(eco_format!(
            "scoreboard players operation {scoreboard} {}= {constant_scoreboard}",
            operation.operator
          ));
        }
      }
      _ => {
        let other_scoreboard =
          self.move_to_scoreboard(&mut context.code, value, &context.location.namespace)?;
        context.code.push(eco_format!(
          "scoreboard players operation {scoreboard} {}= {other_scoreboard}",
          operation.operator
        ));
      }
    }
    Ok(())
  }

  fn compile_less_than(
    &mut self,
    binary_operation: BinaryOperation,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, context, false)?;
    let right = self.compile_expression(*binary_operation.right, context, false)?;
    let needs_macro = left.needs_macro || right.needs_macro;

    match (&left.kind, &right.kind) {
      (ExpressionKind::Void, _) | (_, ExpressionKind::Void) => {
        Err(raise_error(left.location, "Cannot compare with void."))
      }
      (ExpressionKind::Boolean(_), _) | (_, ExpressionKind::Boolean(_)) => {
        Err(raise_error(left.location, "Cannot compare with boolean."))
      }
      (ExpressionKind::String(_), _) | (_, ExpressionKind::String(_)) => {
        Err(raise_error(left.location, "Cannot compare with string."))
      }
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        Ok(ExpressionKind::Boolean(
          left.numeric_value().expect("Numeric value exists")
            < right.numeric_value().expect("Numeric value exists"),
        ))
      }
      (num, _) if num.numeric_value().is_some() => self.compile_match_comparison(
        &mut context.code,
        right,
        eco_format!(
          "{}..",
          num.numeric_value().expect("Numeric value exists") + 1
        ),
        &context.location.namespace,
      ),
      (_, num) if num.numeric_value().is_some() => self.compile_match_comparison(
        &mut context.code,
        left,
        eco_format!(
          "..{}",
          num.numeric_value().expect("Numeric value exists") - 1
        ),
        &context.location.namespace,
      ),
      _ => self.compile_comparison_operator(
        &mut context.code,
        left,
        right,
        "<",
        &context.location.namespace,
      ),
    }
    .map(|kind| Expression::with_macro(kind, binary_operation.location, needs_macro))
  }

  fn compile_greater_than(
    &mut self,
    binary_operation: BinaryOperation,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, context, false)?;
    let right = self.compile_expression(*binary_operation.right, context, false)?;
    let needs_macro = left.needs_macro || right.needs_macro;

    match (&left.kind, &right.kind) {
      (ExpressionKind::Void, _) | (_, ExpressionKind::Void) => {
        Err(raise_error(left.location, "Cannot compare with void."))
      }
      (ExpressionKind::Boolean(_), _) | (_, ExpressionKind::Boolean(_)) => {
        Err(raise_error(left.location, "Cannot compare with boolean."))
      }
      (ExpressionKind::String(_), _) | (_, ExpressionKind::String(_)) => {
        Err(raise_error(left.location, "Cannot compare with string."))
      }
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        Ok(ExpressionKind::Boolean(
          left.numeric_value().expect("Numeric value exists")
            > right.numeric_value().expect("Numeric value exists"),
        ))
      }
      (num, _) if num.numeric_value().is_some() => self.compile_match_comparison(
        &mut context.code,
        right,
        eco_format!(
          "..{}",
          num.numeric_value().expect("Numeric value exists") - 1
        ),
        &context.location.namespace,
      ),
      (_, num) if num.numeric_value().is_some() => self.compile_match_comparison(
        &mut context.code,
        left,
        eco_format!(
          "{}..",
          num.numeric_value().expect("Numeric value exists") + 1
        ),
        &context.location.namespace,
      ),
      _ => self.compile_comparison_operator(
        &mut context.code,
        left,
        right,
        ">",
        &context.location.namespace,
      ),
    }
    .map(|kind| Expression::with_macro(kind, binary_operation.location, needs_macro))
  }

  fn compile_less_than_equals(
    &mut self,
    binary_operation: BinaryOperation,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, context, false)?;
    let right = self.compile_expression(*binary_operation.right, context, false)?;
    let needs_macro = left.needs_macro || right.needs_macro;

    match (&left.kind, &right.kind) {
      (ExpressionKind::Void, _) | (_, ExpressionKind::Void) => {
        Err(raise_error(left.location, "Cannot compare with void."))
      }
      (ExpressionKind::String(_), _) | (_, ExpressionKind::String(_)) => {
        Err(raise_error(left.location, "Cannot compare with string."))
      }
      (ExpressionKind::Boolean(_), _) | (_, ExpressionKind::Boolean(_)) => {
        Err(raise_error(left.location, "Cannot compare with boolean."))
      }
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        Ok(ExpressionKind::Boolean(
          left.numeric_value().expect("Numeric value exists")
            <= right.numeric_value().expect("Numeric value exists"),
        ))
      }
      (num, _) if num.numeric_value().is_some() => self.compile_match_comparison(
        &mut context.code,
        right,
        eco_format!("{}..", num.numeric_value().expect("Numeric value exists")),
        &context.location.namespace,
      ),
      (_, num) if num.numeric_value().is_some() => self.compile_match_comparison(
        &mut context.code,
        left,
        eco_format!("..{}", num.numeric_value().expect("Numeric value exists")),
        &context.location.namespace,
      ),
      _ => self.compile_comparison_operator(
        &mut context.code,
        left,
        right,
        "<=",
        &context.location.namespace,
      ),
    }
    .map(|kind| Expression::with_macro(kind, binary_operation.location, needs_macro))
  }

  fn compile_greater_than_equals(
    &mut self,
    binary_operation: BinaryOperation,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, context, false)?;
    let right = self.compile_expression(*binary_operation.right, context, false)?;
    let needs_macro = left.needs_macro || right.needs_macro;

    match (&left.kind, &right.kind) {
      (ExpressionKind::Void, _) | (_, ExpressionKind::Void) => {
        Err(raise_error(left.location, "Cannot compare with void."))
      }
      (ExpressionKind::Boolean(_), _) | (_, ExpressionKind::Boolean(_)) => {
        Err(raise_error(left.location, "Cannot compare with boolean."))
      }
      (ExpressionKind::String(_), _) | (_, ExpressionKind::String(_)) => {
        Err(raise_error(left.location, "Cannot compare with string."))
      }
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        Ok(ExpressionKind::Boolean(
          left.numeric_value().expect("Numeric value exists")
            >= right.numeric_value().expect("Numeric value exists"),
        ))
      }
      (num, _) if num.numeric_value().is_some() => self.compile_match_comparison(
        &mut context.code,
        right,
        eco_format!("..{}", num.numeric_value().expect("Numeric value exists")),
        &context.location.namespace,
      ),
      (_, num) if num.numeric_value().is_some() => self.compile_match_comparison(
        &mut context.code,
        left,
        eco_format!("{}..", num.numeric_value().expect("Numeric value exists")),
        &context.location.namespace,
      ),
      _ => self.compile_comparison_operator(
        &mut context.code,
        left,
        right,
        ">=",
        &context.location.namespace,
      ),
    }
    .map(|kind| Expression::with_macro(kind, binary_operation.location, needs_macro))
  }

  fn compile_equals(
    &mut self,
    binary_operation: BinaryOperation,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, context, false)?;
    let right = self.compile_expression(*binary_operation.right, context, false)?;
    let needs_macro = left.needs_macro || right.needs_macro;

    if let Some(equal) = left.equal(&right) {
      return Ok(Expression::new(
        ExpressionKind::Boolean(equal),
        binary_operation.location,
      ));
    }

    match (&left.kind, &right.kind) {
      (ExpressionKind::Void, _) | (_, ExpressionKind::Void) => {
        Err(raise_error(left.location, "Cannot compare with void."))
      }
      (ExpressionKind::Storage(_), _) | (_, ExpressionKind::Storage(_)) => self.storage_comparison(
        &mut context.code,
        left,
        right,
        true,
        &context.location.namespace,
      ),
      (left_kind, right_kind)
        if left_kind.to_type().is_numeric() && right_kind.to_type().is_numeric() =>
      {
        self.compile_comparison_operator(
          &mut context.code,
          left,
          right,
          "=",
          &context.location.namespace,
        )
      }
      _ => self.storage_comparison(
        &mut context.code,
        left,
        right,
        true,
        &context.location.namespace,
      ),
    }
    .map(|kind| Expression::with_macro(kind, binary_operation.location, needs_macro))
  }

  fn compile_not_equals(
    &mut self,
    binary_operation: BinaryOperation,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, context, false)?;
    let right = self.compile_expression(*binary_operation.right, context, false)?;
    let needs_macro = left.needs_macro || right.needs_macro;

    if let Some(equal) = left.equal(&right) {
      return Ok(Expression::new(
        ExpressionKind::Boolean(!equal),
        binary_operation.location,
      ));
    }

    match (&left.kind, &right.kind) {
      (ExpressionKind::Void, _) | (_, ExpressionKind::Void) => {
        Err(raise_error(left.location, "Cannot compare with void."))
      }
      (ExpressionKind::Storage(_), _) | (_, ExpressionKind::Storage(_)) => self.storage_comparison(
        &mut context.code,
        left,
        right,
        false,
        &context.location.namespace,
      ),
      (left_kind, right_kind)
        if left_kind.to_type().is_numeric() && right_kind.to_type().is_numeric() =>
      {
        self.compile_comparison_operator(
          &mut context.code,
          left,
          right,
          "!=",
          &context.location.namespace,
        )
      }
      _ => self.storage_comparison(
        &mut context.code,
        left,
        right,
        false,
        &context.location.namespace,
      ),
    }
    .map(|kind| Expression::with_macro(kind, binary_operation.location, needs_macro))
  }

  fn compile_logical_and(
    &mut self,
    binary_operation: BinaryOperation,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, context, false)?;
    let right = self.compile_expression(*binary_operation.right, context, false)?;
    let needs_macro = left.needs_macro || right.needs_macro;

    let left_condition =
      left.to_condition(self, &mut context.code, &context.location.namespace, false)?;
    let right_condition =
      right.to_condition(self, &mut context.code, &context.location.namespace, false)?;

    match (left_condition, right_condition) {
      (ConditionKind::Known(false), _) | (_, ConditionKind::Known(false)) => {
        Ok(ExpressionKind::Boolean(false))
      }
      (ConditionKind::Known(true), ConditionKind::Known(true)) => Ok(ExpressionKind::Boolean(true)),
      (ConditionKind::Known(true), ConditionKind::Check(other))
      | (ConditionKind::Check(other), ConditionKind::Known(true)) => {
        Ok(ExpressionKind::Condition(Condition::Check(other)))
      }
      (ConditionKind::Check(a), ConditionKind::Check(b)) => {
        Ok(ExpressionKind::Condition(Condition::And(a, b)))
      }
    }
    .map(|kind| Expression::with_macro(kind, binary_operation.location, needs_macro))
  }

  fn compile_logical_or(
    &mut self,
    binary_operation: BinaryOperation,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, context, false)?;
    let right = self.compile_expression(*binary_operation.right, context, false)?;
    let needs_macro = left.needs_macro || right.needs_macro;

    let left_condition =
      left.to_condition(self, &mut context.code, &context.location.namespace, false)?;
    let right_condition =
      right.to_condition(self, &mut context.code, &context.location.namespace, false)?;

    match (left_condition, right_condition) {
      (ConditionKind::Known(true), _) | (_, ConditionKind::Known(true)) => {
        Ok(ExpressionKind::Boolean(true))
      }
      (ConditionKind::Known(false), ConditionKind::Known(false)) => {
        Ok(ExpressionKind::Boolean(false))
      }
      (ConditionKind::Known(false), ConditionKind::Check(other))
      | (ConditionKind::Check(other), ConditionKind::Known(false)) => {
        Ok(ExpressionKind::Condition(Condition::Check(other)))
      }
      (ConditionKind::Check(a), ConditionKind::Check(b)) => {
        let scoreboard = self.next_scoreboard(&context.location.namespace);
        context.code.push(eco_format!(
          "execute {a} run scoreboard players set {scoreboard} 1",
        ));
        context.code.push(eco_format!(
          "execute {b} run scoreboard players set {scoreboard} 1",
        ));

        Ok(ExpressionKind::Condition(Condition::Match(
          scoreboard,
          "1".to_eco_string(),
        )))
      }
    }
    .map(|kind| Expression::with_macro(kind, binary_operation.location, needs_macro))
  }

  fn storage_comparison(
    &mut self,
    code: &mut Vec<EcoString>,
    left: Expression,
    right: Expression,
    check_equality: bool,
    namespace: &str,
  ) -> Result<ExpressionKind> {
    let right_storage = self.move_to_storage(code, right, namespace)?;
    let temp_storage = self.copy_to_storage(code, &left, namespace)?;
    let condition_scoreboard = self.next_scoreboard(namespace);
    code.push(eco_format!(
      "execute store success score {condition_scoreboard} run data modify storage {temp_storage} set from storage {right_storage}",
    ));
    Ok(ExpressionKind::Condition(Condition::Match(
      condition_scoreboard,
      if check_equality { "0" } else { "1" }.to_eco_string(),
    )))
  }

  fn compile_comparison_operator(
    &mut self,
    code: &mut Vec<EcoString>,
    left: Expression,
    right: Expression,
    operator: &str,
    namespace: &str,
  ) -> Result<ExpressionKind> {
    let left_scoreboard = self.move_to_scoreboard(code, left, namespace)?;
    let right_scoreboard = self.move_to_scoreboard(code, right, namespace)?;
    Ok(ExpressionKind::Condition(Condition::from_operator(
      operator,
      left_scoreboard,
      right_scoreboard,
    )))
  }

  fn compile_match_comparison(
    &mut self,
    code: &mut Vec<EcoString>,
    value: Expression,
    range: EcoString,
    namespace: &str,
  ) -> Result<ExpressionKind> {
    let scoreboard = self.move_to_scoreboard(code, value, namespace)?;
    Ok(ExpressionKind::Condition(Condition::Match(
      scoreboard, range,
    )))
  }

  pub(super) fn compile_unary_expression(
    &mut self,
    unary_expression: UnaryExpression,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    match unary_expression.operator {
      UnaryOperator::LogicalNot => self.compile_logical_not(unary_expression, context),
      UnaryOperator::Negation => self.compile_negation(unary_expression, context),
    }
  }

  fn compile_logical_not(
    &mut self,
    unary_expression: UnaryExpression,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    let operand = self.compile_expression(*unary_expression.operand, context, false)?;
    let needs_macro = operand.needs_macro;

    let condition =
      operand.to_condition(self, &mut context.code, &context.location.namespace, true)?;

    let kind = match condition {
      ConditionKind::Known(b) => ExpressionKind::Boolean(!b),
      ConditionKind::Check(condition) => ExpressionKind::Condition(Condition::Check(condition)),
    };

    Ok(Expression::with_macro(
      kind,
      unary_expression.location,
      needs_macro,
    ))
  }

  fn compile_negation(
    &mut self,
    unary_expression: UnaryExpression,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    let operand = self.compile_expression(*unary_expression.operand, context, false)?;
    let needs_macro = operand.needs_macro;

    let kind = match &operand.kind {
      ExpressionKind::Void
      | ExpressionKind::String(_)
      | ExpressionKind::Array { .. }
      | ExpressionKind::ByteArray(_)
      | ExpressionKind::IntArray(_)
      | ExpressionKind::LongArray(_)
      | ExpressionKind::Compound(_)
      | ExpressionKind::SubString(_, _, _)
      | ExpressionKind::Condition(_)
      | ExpressionKind::Boolean(_) => {
        return Err(raise_error(
          unary_expression.location,
          "Can only negate numbers",
        ))
      }

      ExpressionKind::Byte(b) => ExpressionKind::Byte(-*b),
      ExpressionKind::Short(s) => ExpressionKind::Short(-*s),
      ExpressionKind::Integer(i) => ExpressionKind::Integer(-*i),
      ExpressionKind::Long(l) => ExpressionKind::Long(-*l),
      ExpressionKind::Float(f) => ExpressionKind::Float(-*f),
      ExpressionKind::Double(d) => ExpressionKind::Double(-*d),

      ExpressionKind::Storage(storage) => {
        let temp_storage = self.next_storage(&context.location.namespace);
        context.code.push(eco_format!(
          "{}execute store result storage {temp_storage} int -1 run data get storage {storage}",
          if needs_macro { "$" } else { "" }
        ));
        ExpressionKind::Storage(temp_storage)
      }

      ExpressionKind::Scoreboard(scoreboard) => {
        let temp_storage = self.next_storage(&context.location.namespace);
        context.code.push(eco_format!(
          "{}execute store result storage {temp_storage} int -1 run scoreboard players get {scoreboard}",
          if needs_macro { "$" } else { "" }
        ));
        ExpressionKind::Storage(temp_storage)
      }

      ExpressionKind::Macro(_) => {
        let temp_storage =
          self.copy_to_storage(&mut context.code, &operand, &context.location.namespace)?;
        context.code.push(eco_format!(
          "execute store result storage {temp_storage} int -1 run data get storage {temp_storage}"
        ));
        ExpressionKind::Storage(temp_storage)
      }
    };

    Ok(Expression::new(kind, unary_expression.location))
  }

  pub(super) fn copy_to_scoreboard(
    &mut self,
    code: &mut Vec<EcoString>,
    value: &Expression,
    namespace: &str,
  ) -> Result<ScoreboardLocation> {
    let scoreboard = self.next_scoreboard(namespace);
    self.set_scoreboard(code, &scoreboard, value)?;
    Ok(scoreboard)
  }

  pub(super) fn move_to_scoreboard(
    &mut self,
    code: &mut Vec<EcoString>,
    value: Expression,
    namespace: &str,
  ) -> Result<ScoreboardLocation> {
    if let ExpressionKind::Scoreboard(scoreboard) = value.kind {
      Ok(scoreboard)
    } else {
      self.copy_to_scoreboard(code, &value, namespace)
    }
  }

  pub(super) fn set_scoreboard(
    &mut self,
    code: &mut Vec<EcoString>,
    scoreboard: &ScoreboardLocation,
    value: &Expression,
  ) -> Result<()> {
    let (conversion_code, kind) = value.to_score()?;
    match kind {
      ScoreKind::Direct(operation) => code.push(eco_format!(
        "scoreboard players {operation} {scoreboard} {conversion_code}",
      )),
      ScoreKind::DirectMacro(operation) => code.push(eco_format!(
        "$scoreboard players {operation} {scoreboard} {conversion_code}",
      )),
      ScoreKind::Indirect => code.push(eco_format!(
        "execute store result score {scoreboard} run {conversion_code}",
      )),
      ScoreKind::IndirectMacro => code.push(eco_format!(
        "$execute store result score {scoreboard} run {conversion_code}",
      )),
    }
    Ok(())
  }

  pub(super) fn copy_to_storage(
    &mut self,
    code: &mut Vec<EcoString>,
    value: &Expression,
    namespace: &str,
  ) -> Result<StorageLocation> {
    let storage = self.next_storage(namespace);
    self.set_storage(code, &storage, value)?;

    Ok(storage)
  }

  pub(super) fn move_to_storage(
    &mut self,
    code: &mut Vec<EcoString>,
    value: Expression,
    namespace: &str,
  ) -> Result<StorageLocation> {
    if let ExpressionKind::Storage(location) = value.kind {
      Ok(location)
    } else {
      self.copy_to_storage(code, &value, namespace)
    }
  }

  pub(super) fn set_storage(
    &mut self,
    code: &mut Vec<EcoString>,
    storage: &StorageLocation,
    value: &Expression,
  ) -> Result<()> {
    value.to_storage(self, code, storage, "set", NbtType::Unknown)
  }
}
