use crate::parser::ast::{self, BinaryOperation, Operator, UnaryExpression, UnaryOperator};

use crate::error::{raise_error, Result};

use super::{
  expression::{Condition, ConditionKind, Expression, ExpressionKind, ScoreKind, StorageKind},
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
      Operator::Equal => self.compile_equals(code, binary_operation, location),
      Operator::NotEqual => self.compile_not_equals(code, binary_operation, location),
      Operator::LogicalAnd => self.compile_logical_and(code, binary_operation, location),
      Operator::LogicalOr => self.compile_logical_or(code, binary_operation, location),
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
    match *binary_operation.left {
      ast::Expression::Variable(variable) => {
        let typ = self.compile_expression(*binary_operation.right, location, code, false)?;
        let storage = StorageLocation::from_zoglin_resource(location.clone(), &variable);
        self.set_storage(code, &storage, &typ, &location.module.namespace)?;

        Ok(typ)
      }
      ast::Expression::ScoreboardVariable(variable) => {
        let typ: Expression =
          self.compile_expression(*binary_operation.right, location, code, false)?;
        let scoreboard = ScoreboardLocation::from_zoglin_resource(location.clone(), &variable);
        self.set_scoreboard(code, &scoreboard, &typ)?;
        self.used_scoreboards.insert(scoreboard.scoreboard_string());

        Ok(typ)
      }
      _ => Err(raise_error(
        binary_operation.left.location(),
        "Can only assign to variables.",
      )),
    }
  }

  fn compile_plus(
    &mut self,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code, false)?;
    let right = self.compile_expression(*binary_operation.right, location, code, false)?;
    let needs_macro = left.needs_macro || right.needs_macro;

    match (&left.kind, &right.kind) {
      (ExpressionKind::Void, _) | (_, ExpressionKind::Void) => Err(raise_error(
        left.location,
        "Cannot add type void to another value.",
      )),
      (ExpressionKind::Boolean(_), _) | (_, ExpressionKind::Boolean(_)) => Err(raise_error(
        left.location,
        "Cannot perform plus with boolean.",
      )),
      (ExpressionKind::String(_), _) | (_, ExpressionKind::String(_)) => Err(raise_error(
        left.location,
        "Cannot perform plus with string.",
      )),
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        Ok(ExpressionKind::Integer(
          left.numeric_value().expect("Numeric value exists")
            + right.numeric_value().expect("Numeric value exists"),
        ))
      }
      (num, _) if num.numeric_value().is_some() => {
        let scoreboard = self.copy_to_scoreboard(code, &right, &location.module.namespace)?;
        code.push(format!(
          "scoreboard players add {scoreboard} {}",
          num.numeric_value().expect("Numeric value exists"),
        ));
        Ok(ExpressionKind::Scoreboard(scoreboard))
      }
      (_, num) if num.numeric_value().is_some() => {
        let scoreboard = self.copy_to_scoreboard(code, &left, &location.module.namespace)?;
        code.push(format!(
          "scoreboard players add {scoreboard} {}",
          num.numeric_value().expect("Numeric value exists"),
        ));
        Ok(ExpressionKind::Scoreboard(scoreboard))
      }
      _ => self.compile_basic_operator(left, right, '+', code, &location.module.namespace),
    }
    .map(|kind| Expression::with_macro(kind, binary_operation.location, needs_macro))
  }

  fn compile_minus(
    &mut self,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code, false)?;
    let right = self.compile_expression(*binary_operation.right, location, code, false)?;
    let needs_macro = left.needs_macro || right.needs_macro;

    match (&left.kind, &right.kind) {
      (ExpressionKind::Void, _) | (_, ExpressionKind::Void) => Err(raise_error(
        left.location,
        "Cannot perform subtraction with void.",
      )),
      (ExpressionKind::Boolean(_), _) | (_, ExpressionKind::Boolean(_)) => Err(raise_error(
        left.location,
        "Cannot perform subtraction with boolean.",
      )),
      (ExpressionKind::String(_), _) | (_, ExpressionKind::String(_)) => Err(raise_error(
        left.location,
        "Cannot perform subtraction with string.",
      )),
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        Ok(ExpressionKind::Integer(
          left.numeric_value().expect("Numeric value exists")
            - right.numeric_value().expect("Numeric value exists"),
        ))
      }
      (_, num) if num.numeric_value().is_some() => {
        let scoreboard = self.copy_to_scoreboard(code, &left, &location.module.namespace)?;
        code.push(format!(
          "scoreboard players remove {scoreboard} {}",
          num.numeric_value().expect("Numeric value exists"),
        ));
        Ok(ExpressionKind::Scoreboard(scoreboard))
      }
      _ => self.compile_basic_operator(left, right, '-', code, &location.module.namespace),
    }
    .map(|kind| Expression::with_macro(kind, binary_operation.location, needs_macro))
  }

  fn compile_multiply(
    &mut self,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code, false)?;
    let right = self.compile_expression(*binary_operation.right, location, code, false)?;
    let needs_macro = left.needs_macro || right.needs_macro;

    match (&left.kind, &right.kind) {
      (ExpressionKind::Void, _) | (_, ExpressionKind::Void) => Err(raise_error(
        left.location,
        "Cannot perform multiplication with void.",
      )),
      (ExpressionKind::Boolean(_), _) | (_, ExpressionKind::Boolean(_)) => Err(raise_error(
        left.location,
        "Cannot perform multiplication with boolean.",
      )),
      (ExpressionKind::String(_), _) | (_, ExpressionKind::String(_)) => Err(raise_error(
        left.location,
        "Cannot perform multiplication with string.",
      )),
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        Ok(ExpressionKind::Integer(
          left.numeric_value().expect("Numeric value exists")
            * right.numeric_value().expect("Numeric value exists"),
        ))
      }
      _ => self.compile_basic_operator(left, right, '*', code, &location.module.namespace),
    }
    .map(|kind| Expression::with_macro(kind, binary_operation.location, needs_macro))
  }

  fn compile_divide(
    &mut self,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code, false)?;
    let right = self.compile_expression(*binary_operation.right, location, code, false)?;
    let needs_macro = left.needs_macro || right.needs_macro;

    match (&left.kind, &right.kind) {
      (ExpressionKind::Void, _) | (_, ExpressionKind::Void) => Err(raise_error(
        left.location,
        "Cannot perform division with void.",
      )),
      (ExpressionKind::Boolean(_), _) | (_, ExpressionKind::Boolean(_)) => Err(raise_error(
        left.location,
        "Cannot perform division with boolean.",
      )),
      (ExpressionKind::String(_), _) | (_, ExpressionKind::String(_)) => Err(raise_error(
        left.location,
        "Cannot perform division with string.",
      )),
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        Ok(ExpressionKind::Integer(
          left.numeric_value().expect("Numeric value exists")
            / right.numeric_value().expect("Numeric value exists"),
        ))
      }
      _ => self.compile_basic_operator(left, right, '/', code, &location.module.namespace),
    }
    .map(|kind| Expression::with_macro(kind, binary_operation.location, needs_macro))
  }

  fn compile_modulo(
    &mut self,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code, false)?;
    let right = self.compile_expression(*binary_operation.right, location, code, false)?;
    let needs_macro = left.needs_macro || right.needs_macro;

    match (&left.kind, &right.kind) {
      (ExpressionKind::Void, _) | (_, ExpressionKind::Void) => Err(raise_error(
        left.location,
        "Cannot perform modulo with void.",
      )),
      (ExpressionKind::Boolean(_), _) | (_, ExpressionKind::Boolean(_)) => Err(raise_error(
        left.location,
        "Cannot perform modulo with boolean.",
      )),
      (ExpressionKind::String(_), _) | (_, ExpressionKind::String(_)) => Err(raise_error(
        left.location,
        "Cannot perform modulo with string.",
      )),
      (left, right) if left.numeric_value().is_some() && right.numeric_value().is_some() => {
        Ok(ExpressionKind::Integer(
          left.numeric_value().expect("Numeric value exists")
            % right.numeric_value().expect("Numeric value exists"),
        ))
      }
      _ => self.compile_basic_operator(left, right, '%', code, &location.module.namespace),
    }
    .map(|kind| Expression::with_macro(kind, binary_operation.location, needs_macro))
  }

  fn compile_less_than(
    &mut self,
    code: &mut Vec<String>,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code, false)?;
    let right = self.compile_expression(*binary_operation.right, location, code, false)?;
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
        code,
        right,
        format!(
          "{}..",
          num.numeric_value().expect("Numeric value exists") + 1
        ),
        &location.module.namespace,
      ),
      (_, num) if num.numeric_value().is_some() => self.compile_match_comparison(
        code,
        left,
        format!(
          "..{}",
          num.numeric_value().expect("Numeric value exists") - 1
        ),
        &location.module.namespace,
      ),
      _ => self.compile_comparison_operator(code, left, right, "<", &location.module.namespace),
    }
    .map(|kind| Expression::with_macro(kind, binary_operation.location, needs_macro))
  }

  fn compile_greater_than(
    &mut self,
    code: &mut Vec<String>,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code, false)?;
    let right = self.compile_expression(*binary_operation.right, location, code, false)?;
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
        code,
        right,
        format!(
          "..{}",
          num.numeric_value().expect("Numeric value exists") - 1
        ),
        &location.module.namespace,
      ),
      (_, num) if num.numeric_value().is_some() => self.compile_match_comparison(
        code,
        left,
        format!(
          "{}..",
          num.numeric_value().expect("Numeric value exists") + 1
        ),
        &location.module.namespace,
      ),
      _ => self.compile_comparison_operator(code, left, right, ">", &location.module.namespace),
    }
    .map(|kind| Expression::with_macro(kind, binary_operation.location, needs_macro))
  }

  fn compile_less_than_equals(
    &mut self,
    code: &mut Vec<String>,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code, false)?;
    let right = self.compile_expression(*binary_operation.right, location, code, false)?;
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
        code,
        right,
        format!("{}..", num.numeric_value().expect("Numeric value exists")),
        &location.module.namespace,
      ),
      (_, num) if num.numeric_value().is_some() => self.compile_match_comparison(
        code,
        left,
        format!("..{}", num.numeric_value().expect("Numeric value exists")),
        &location.module.namespace,
      ),
      _ => self.compile_comparison_operator(code, left, right, "<=", &location.module.namespace),
    }
    .map(|kind| Expression::with_macro(kind, binary_operation.location, needs_macro))
  }

  fn compile_greater_than_equals(
    &mut self,
    code: &mut Vec<String>,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code, false)?;
    let right = self.compile_expression(*binary_operation.right, location, code, false)?;
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
        code,
        right,
        format!("..{}", num.numeric_value().expect("Numeric value exists")),
        &location.module.namespace,
      ),
      (_, num) if num.numeric_value().is_some() => self.compile_match_comparison(
        code,
        left,
        format!("{}..", num.numeric_value().expect("Numeric value exists")),
        &location.module.namespace,
      ),
      _ => self.compile_comparison_operator(code, left, right, ">=", &location.module.namespace),
    }
    .map(|kind| Expression::with_macro(kind, binary_operation.location, needs_macro))
  }

  fn compile_equals(
    &mut self,
    code: &mut Vec<String>,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code, false)?;
    let right = self.compile_expression(*binary_operation.right, location, code, false)?;
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
      (ExpressionKind::Storage(_), _) | (_, ExpressionKind::Storage(_)) => {
        self.storage_comparison(code, left, right, true, &location.module.namespace)
      }
      (left_kind, right_kind)
        if left_kind.to_type().is_numeric() && right_kind.to_type().is_numeric() =>
      {
        self.compile_comparison_operator(code, left, right, "=", &location.module.namespace)
      }
      _ => self.storage_comparison(code, left, right, true, &location.module.namespace),
    }
    .map(|kind| Expression::with_macro(kind, binary_operation.location, needs_macro))
  }

  fn compile_not_equals(
    &mut self,
    code: &mut Vec<String>,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code, false)?;
    let right = self.compile_expression(*binary_operation.right, location, code, false)?;
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
      (ExpressionKind::Storage(_), _) | (_, ExpressionKind::Storage(_)) => {
        self.storage_comparison(code, left, right, false, &location.module.namespace)
      }
      (left_kind, right_kind)
        if left_kind.to_type().is_numeric() && right_kind.to_type().is_numeric() =>
      {
        self.compile_comparison_operator(code, left, right, "!=", &location.module.namespace)
      }
      _ => self.storage_comparison(code, left, right, false, &location.module.namespace),
    }
    .map(|kind| Expression::with_macro(kind, binary_operation.location, needs_macro))
  }

  fn compile_logical_and(
    &mut self,
    code: &mut Vec<String>,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code, false)?;
    let right = self.compile_expression(*binary_operation.right, location, code, false)?;
    let needs_macro = left.needs_macro || right.needs_macro;

    let left_condition = left.to_condition(self, code, &location.module.namespace, false)?;
    let right_condition = right.to_condition(self, code, &location.module.namespace, false)?;

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
    code: &mut Vec<String>,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code, false)?;
    let right = self.compile_expression(*binary_operation.right, location, code, false)?;
    let needs_macro = left.needs_macro || right.needs_macro;

    let left_condition = left.to_condition(self, code, &location.module.namespace, false)?;
    let right_condition = right.to_condition(self, code, &location.module.namespace, false)?;

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
        let scoreboard = self.next_scoreboard(&location.module.namespace);
        code.push(format!(
          "execute {a} run scoreboard players set {scoreboard} 1",
        ));
        code.push(format!(
          "execute {b} run scoreboard players set {scoreboard} 1",
        ));

        Ok(ExpressionKind::Condition(Condition::Match(
          scoreboard,
          "1".to_string(),
        )))
      }
    }
    .map(|kind| Expression::with_macro(kind, binary_operation.location, needs_macro))
  }

  fn storage_comparison(
    &mut self,
    code: &mut Vec<String>,
    left: Expression,
    right: Expression,
    check_equality: bool,
    namespace: &str,
  ) -> Result<ExpressionKind> {
    let right_storage = self.move_to_storage(code, right, namespace)?;
    let temp_storage = self.copy_to_storage(code, &left, namespace)?;
    let condition_scoreboard = self.next_scoreboard(namespace);
    code.push(format!(
      "execute store success score {condition_scoreboard} run data modify storage {temp_storage} set from storage {right_storage}",
    ));
    Ok(ExpressionKind::Condition(Condition::Match(
      condition_scoreboard,
      if check_equality { "0" } else { "1" }.to_string(),
    )))
  }

  fn compile_basic_operator(
    &mut self,
    left: Expression,
    right: Expression,
    operator: char,
    code: &mut Vec<String>,
    namespace: &str,
  ) -> Result<ExpressionKind> {
    let left_scoreboard = self.copy_to_scoreboard(code, &left, namespace)?;
    let right_scoreboard = self.move_to_scoreboard(code, right, namespace)?;
    code.push(format!(
      "scoreboard players operation {left_scoreboard} {operator}= {right_scoreboard}"
    ));
    Ok(ExpressionKind::Scoreboard(left_scoreboard))
  }

  fn compile_comparison_operator(
    &mut self,
    code: &mut Vec<String>,
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
    code: &mut Vec<String>,
    value: Expression,
    range: String,
    namespace: &str,
  ) -> Result<ExpressionKind> {
    let scoreboard = self.move_to_scoreboard(code, value, namespace)?;
    Ok(ExpressionKind::Condition(Condition::Match(
      scoreboard, range,
    )))
  }

  pub fn compile_unary_expression(
    &mut self,
    unary_expression: UnaryExpression,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> Result<Expression> {
    match unary_expression.operator {
      UnaryOperator::LogicalNot => self.compile_logical_not(unary_expression, location, code),
      UnaryOperator::Negation => self.compile_negation(unary_expression, location, code),
    }
  }

  fn compile_logical_not(
    &mut self,
    unary_expression: UnaryExpression,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> Result<Expression> {
    let operand = self.compile_expression(*unary_expression.operand, location, code, false)?;
    let needs_macro = operand.needs_macro;

    let condition = operand.to_condition(self, code, &location.module.namespace, true)?;

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
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> Result<Expression> {
    let operand = self.compile_expression(*unary_expression.operand, location, code, false)?;
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
        let temp_storage = self.next_storage(&location.module.namespace);
        code.push(format!(
          "{}execute store result storage {temp_storage} int -1 run data get storage {storage}",
          if needs_macro { "$" } else { "" }
        ));
        ExpressionKind::Storage(temp_storage)
      }

      ExpressionKind::Scoreboard(scoreboard) => {
        let temp_storage = self.next_storage(&location.module.namespace);
        code.push(format!(
          "{}execute store result storage {temp_storage} int -1 run scoreboard players get {scoreboard}",
          if needs_macro { "$" } else { "" }
        ));
        ExpressionKind::Storage(temp_storage)
      }

      ExpressionKind::Macro(_) => {
        let temp_storage = self.copy_to_storage(code, &operand, &location.module.namespace)?;
        code.push(format!(
          "execute store result storage {temp_storage} int -1 run data get storage {temp_storage}"
        ));
        ExpressionKind::Storage(temp_storage)
      }
    };

    Ok(Expression::new(kind, unary_expression.location))
  }

  pub(super) fn copy_to_scoreboard(
    &mut self,
    code: &mut Vec<String>,
    value: &Expression,
    namespace: &str,
  ) -> Result<ScoreboardLocation> {
    let scoreboard = self.next_scoreboard(namespace);
    self.set_scoreboard(code, &scoreboard, value)?;
    Ok(scoreboard)
  }

  pub(super) fn move_to_scoreboard(
    &mut self,
    code: &mut Vec<String>,
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
    code: &mut Vec<String>,
    scoreboard: &ScoreboardLocation,
    value: &Expression,
  ) -> Result<()> {
    let (conversion_code, kind) = value.to_score()?;
    match kind {
      ScoreKind::Direct(operation) => code.push(format!(
        "scoreboard players {operation} {scoreboard} {conversion_code}",
      )),
      ScoreKind::DirectMacro(operation) => code.push(format!(
        "$scoreboard players {operation} {scoreboard} {conversion_code}",
      )),
      ScoreKind::Indirect => code.push(format!(
        "execute store result score {scoreboard} run {conversion_code}",
      )),
      ScoreKind::IndirectMacro => code.push(format!(
        "$execute store result score {scoreboard} run {conversion_code}",
      )),
    }
    Ok(())
  }

  pub(super) fn copy_to_storage(
    &mut self,
    code: &mut Vec<String>,
    value: &Expression,
    namespace: &str,
  ) -> Result<StorageLocation> {
    let storage = self.next_storage(namespace);
    self.set_storage(code, &storage, value, namespace)?;

    Ok(storage)
  }

  pub(super) fn move_to_storage(
    &mut self,
    code: &mut Vec<String>,
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
    code: &mut Vec<String>,
    storage: &StorageLocation,
    value: &Expression,
    namespace: &str,
  ) -> Result<()> {
    let (conversion_code, kind) = value.to_storage(self, code, namespace)?;
    match kind {
      StorageKind::Modify => code.push(format!(
        "data modify storage {storage} set {conversion_code}",
      )),
      StorageKind::MacroModify => code.push(format!(
        "$data modify storage {storage} set {conversion_code}",
      )),
      StorageKind::Store => code.push(format!(
        "execute store result storage {storage} int 1 run {conversion_code}",
      )),
      StorageKind::MacroStore => code.push(format!(
        "$execute store result storage {storage} int 1 run {conversion_code}",
      )),
    }
    Ok(())
  }
}
