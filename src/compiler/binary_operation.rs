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
      Operator::Equal => self.compile_equals(code, binary_operation, location),
      Operator::NotEqual => self.compile_not_equals(code, binary_operation, location),
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
    match *binary_operation.left {
      ast::Expression::Variable(variable) => {
        let typ = self.compile_expression(*binary_operation.right, location, code, false)?;
        let storage = StorageLocation::from_zoglin_resource(location.clone(), &variable);
        self.set_storage(code, &storage, &typ)?;

        Ok(typ)
      }
      ast::Expression::ScoreboardVariable(variable) => {
        let typ: Expression = self.compile_expression(*binary_operation.right, location, code, false)?;
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
          left.numeric_value().expect("Numeric value exists")
            + right.numeric_value().expect("Numeric value exists"),
          binary_operation.location,
        ))
      }
      (num, other) | (other, num) if num.numeric_value().is_some() => {
        let scoreboard = self.copy_to_scoreboard(code, &other, &location.module.namespace)?;
        code.push(format!(
          "scoreboard players add {} {}",
          scoreboard.to_string(),
          num.numeric_value().expect("Numeric value exists"),
        ));
        Ok(Expression::Scoreboard(
          scoreboard,
          binary_operation.location,
        ))
      }
      (left, right) => self.compile_basic_operator(
        left,
        right,
        binary_operation.location,
        '+',
        code,
        &location.module.namespace,
      ),
    }
  }

  fn compile_minus(
    &mut self,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code, false)?;
    let right = self.compile_expression(*binary_operation.right, location, code, false)?;

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
          left.numeric_value().expect("Numeric value exists")
            - right.numeric_value().expect("Numeric value exists"),
          binary_operation.location,
        ))
      }
      (other, num) if num.numeric_value().is_some() => {
        let scoreboard = self.copy_to_scoreboard(code, &other, &location.module.namespace)?;
        code.push(format!(
          "scoreboard players remove {} {}",
          scoreboard.to_string(),
          num.numeric_value().expect("Numeric value exists"),
        ));
        Ok(Expression::Scoreboard(
          scoreboard,
          binary_operation.location,
        ))
      }
      (left, right) => self.compile_basic_operator(
        left,
        right,
        binary_operation.location,
        '-',
        code,
        &location.module.namespace,
      ),
    }
  }

  fn compile_multiply(
    &mut self,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code, false)?;
    let right = self.compile_expression(*binary_operation.right, location, code, false)?;

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
          left.numeric_value().expect("Numeric value exists")
            * right.numeric_value().expect("Numeric value exists"),
          binary_operation.location,
        ))
      }
      (left, right) => self.compile_basic_operator(
        left,
        right,
        binary_operation.location,
        '*',
        code,
        &location.module.namespace,
      ),
    }
  }

  fn compile_divide(
    &mut self,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code, false)?;
    let right = self.compile_expression(*binary_operation.right, location, code, false)?;

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
          left.numeric_value().expect("Numeric value exists")
            / right.numeric_value().expect("Numeric value exists"),
          binary_operation.location,
        ))
      }
      (left, right) => self.compile_basic_operator(
        left,
        right,
        binary_operation.location,
        '/',
        code,
        &location.module.namespace,
      ),
    }
  }

  fn compile_modulo(
    &mut self,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
    code: &mut Vec<String>,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code, false)?;
    let right = self.compile_expression(*binary_operation.right, location, code, false)?;

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
          left.numeric_value().expect("Numeric value exists")
            % right.numeric_value().expect("Numeric value exists"),
          binary_operation.location,
        ))
      }
      (left, right) => self.compile_basic_operator(
        left,
        right,
        binary_operation.location,
        '%',
        code,
        &location.module.namespace,
      ),
    }
  }

  fn compile_less_than(
    &mut self,
    code: &mut Vec<String>,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code, false)?;
    let right = self.compile_expression(*binary_operation.right, location, code, false)?;

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
          left.numeric_value().expect("Numeric value exists")
            < right.numeric_value().expect("Numeric value exists"),
          binary_operation.location,
        ))
      }
      (num, other) if num.numeric_value().is_some() => self.compile_match_comparison(
        code,
        other,
        binary_operation.location,
        format!(
          "{}..",
          num.numeric_value().expect("Numeric value exists") + 1
        ),
        &location.module.namespace,
      ),
      (other, num) if num.numeric_value().is_some() => self.compile_match_comparison(
        code,
        other,
        binary_operation.location,
        format!(
          "..{}",
          num.numeric_value().expect("Numeric value exists") - 1
        ),
        &location.module.namespace,
      ),
      (left, right) => self.compile_comparison_operator(
        code,
        left,
        right,
        binary_operation.location,
        "<",
        &location.module.namespace,
      ),
    }
  }

  fn compile_greater_than(
    &mut self,
    code: &mut Vec<String>,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code, false)?;
    let right = self.compile_expression(*binary_operation.right, location, code, false)?;

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
          left.numeric_value().expect("Numeric value exists")
            > right.numeric_value().expect("Numeric value exists"),
          binary_operation.location,
        ))
      }
      (num, other) if num.numeric_value().is_some() => self.compile_match_comparison(
        code,
        other,
        binary_operation.location,
        format!(
          "..{}",
          num.numeric_value().expect("Numeric value exists") - 1
        ),
        &location.module.namespace,
      ),
      (other, num) if num.numeric_value().is_some() => self.compile_match_comparison(
        code,
        other,
        binary_operation.location,
        format!(
          "{}..",
          num.numeric_value().expect("Numeric value exists") + 1
        ),
        &location.module.namespace,
      ),
      (left, right) => self.compile_comparison_operator(
        code,
        left,
        right,
        binary_operation.location,
        ">",
        &location.module.namespace,
      ),
    }
  }

  fn compile_less_than_equals(
    &mut self,
    code: &mut Vec<String>,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code, false)?;
    let right = self.compile_expression(*binary_operation.right, location, code, false)?;

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
          left.numeric_value().expect("Numeric value exists")
            <= right.numeric_value().expect("Numeric value exists"),
          binary_operation.location,
        ))
      }
      (num, other) if num.numeric_value().is_some() => self.compile_match_comparison(
        code,
        other,
        binary_operation.location,
        format!("{}..", num.numeric_value().expect("Numeric value exists")),
        &location.module.namespace,
      ),
      (other, num) if num.numeric_value().is_some() => self.compile_match_comparison(
        code,
        other,
        binary_operation.location,
        format!("..{}", num.numeric_value().expect("Numeric value exists")),
        &location.module.namespace,
      ),
      (left, right) => self.compile_comparison_operator(
        code,
        left,
        right,
        binary_operation.location,
        "<=",
        &location.module.namespace,
      ),
    }
  }

  fn compile_greater_than_equals(
    &mut self,
    code: &mut Vec<String>,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code, false)?;
    let right = self.compile_expression(*binary_operation.right, location, code, false)?;

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
          left.numeric_value().expect("Numeric value exists")
            >= right.numeric_value().expect("Numeric value exists"),
          binary_operation.location,
        ))
      }
      (num, other) if num.numeric_value().is_some() => self.compile_match_comparison(
        code,
        other,
        binary_operation.location,
        format!("..{}", num.numeric_value().expect("Numeric value exists")),
        &location.module.namespace,
      ),
      (other, num) if num.numeric_value().is_some() => self.compile_match_comparison(
        code,
        other,
        binary_operation.location,
        format!("{}..", num.numeric_value().expect("Numeric value exists")),
        &location.module.namespace,
      ),
      (left, right) => self.compile_comparison_operator(
        code,
        left,
        right,
        binary_operation.location,
        ">=",
        &location.module.namespace,
      ),
    }
  }

  fn compile_equals(
    &mut self,
    code: &mut Vec<String>,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code, false)?;
    let right = self.compile_expression(*binary_operation.right, location, code, false)?;

    if let Some(equal) = left.equal(&right) {
      return Ok(Expression::Boolean(equal, binary_operation.location));
    }

    match (left, right) {
      (Expression::Void(location), _) | (_, Expression::Void(location)) => {
        Err(raise_error(location, "Cannot compare with void."))
      }
      (storage @ Expression::Storage(_, _), other)
      | (other, storage @ Expression::Storage(_, _)) => self.storage_comparison(
        code,
        binary_operation.location,
        other,
        storage,
        true,
        &location.module.namespace,
      ),
      (left, right) if left.to_type().is_numeric() && right.to_type().is_numeric() => self
        .compile_comparison_operator(
          code,
          left,
          right,
          binary_operation.location,
          "=",
          &location.module.namespace,
        ),
      (left, right) => self.storage_comparison(
        code,
        binary_operation.location,
        left,
        right,
        true,
        &location.module.namespace,
      ),
    }
  }

  fn compile_not_equals(
    &mut self,
    code: &mut Vec<String>,
    binary_operation: BinaryOperation,
    location: &FunctionLocation,
  ) -> Result<Expression> {
    let left = self.compile_expression(*binary_operation.left, location, code, false)?;
    let right = self.compile_expression(*binary_operation.right, location, code, false)?;

    if let Some(equal) = left.equal(&right) {
      return Ok(Expression::Boolean(!equal, binary_operation.location));
    }

    match (left, right) {
      (Expression::Void(location), _) | (_, Expression::Void(location)) => {
        Err(raise_error(location, "Cannot compare with void."))
      }
      (storage @ Expression::Storage(_, _), other)
      | (other, storage @ Expression::Storage(_, _)) => self.storage_comparison(
        code,
        binary_operation.location,
        other,
        storage,
        false,
        &location.module.namespace,
      ),
      (left, right) if left.to_type().is_numeric() && right.to_type().is_numeric() => self
        .compile_comparison_operator(
          code,
          left,
          right,
          binary_operation.location,
          "!=",
          &location.module.namespace,
        ),
      (left, right) => self.storage_comparison(
        code,
        binary_operation.location,
        left,
        right,
        false,
        &location.module.namespace,
      ),
    }
  }

  fn storage_comparison(
    &mut self,
    code: &mut Vec<String>,
    location: Location,
    left: Expression,
    right: Expression,
    check_equality: bool,
    namespace: &str,
  ) -> Result<Expression> {
    let right_storage = self.move_to_storage(code, right)?;
    let temp_storage = self.copy_to_storage(code, &left)?;
    let condition_scoreboard: ScoreboardLocation = self.next_scoreboard(namespace);
    code.push(format!(
      "execute store success score {score} run data modify storage {temp} set from storage {storage}",
      score = condition_scoreboard.to_string(),
      temp = temp_storage.to_string(),
      storage = right_storage.to_string()
    ));
    Ok(Expression::Condition(
      Condition::Match(
        condition_scoreboard,
        if check_equality { "0" } else { "1" }.to_string(),
      ),
      location,
    ))
  }

  fn compile_basic_operator(
    &mut self,
    left: Expression,
    right: Expression,
    location: Location,
    operator: char,
    code: &mut Vec<String>,
    namespace: &str,
  ) -> Result<Expression> {
    let left_scoreboard = self.copy_to_scoreboard(code, &left, namespace)?;
    let right_scoreboard = self.move_to_scoreboard(code, right, namespace)?;
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
    namespace: &str,
  ) -> Result<Expression> {
    let left_scoreboard = self.move_to_scoreboard(code, left, namespace)?;
    let right_scoreboard = self.move_to_scoreboard(code, right, namespace)?;
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
    namespace: &str,
  ) -> Result<Expression> {
    let scoreboard = self.move_to_scoreboard(code, value, namespace)?;
    Ok(Expression::Condition(
      Condition::Match(scoreboard, range),
      location,
    ))
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
    if let Expression::Scoreboard(scoreboard, _) = value {
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
        "scoreboard players {} {} {}",
        operation,
        scoreboard.to_string(),
        conversion_code
      )),
      ScoreKind::Macro => code.push(format!(
        "$scoreboard players set {} $({})",
        scoreboard.to_string(),
        conversion_code
      )),
      ScoreKind::Indirect => code.push(format!(
        "execute store result score {} run {}",
        scoreboard.to_string(),
        conversion_code
      )),
    }
    Ok(())
  }

  pub(super) fn copy_to_storage(
    &mut self,
    code: &mut Vec<String>,
    value: &Expression,
  ) -> Result<StorageLocation> {
    let storage = self.next_storage();
    self.set_storage(code, &storage, value)?;

    Ok(storage)
  }

  pub(super) fn move_to_storage(
    &mut self,
    code: &mut Vec<String>,
    value: Expression,
  ) -> Result<StorageLocation> {
    if let Expression::Storage(location, _) = value {
      Ok(location)
    } else {
      self.copy_to_storage(code, &value)
    }
  }

  pub(super) fn set_storage(
    &mut self,
    code: &mut Vec<String>,
    storage: &StorageLocation,
    value: &Expression,
  ) -> Result<()> {
    let (conversion_code, kind) = value.to_storage(self, code)?;
    match kind {
      StorageKind::Modify => code.push(format!(
        "data modify storage {storage} set {conversion_code}",
        storage = storage.to_string()
      )),
      StorageKind::Macro => code.push(format!(
        "$data modify storage {storage} set value $({conversion_code})",
        storage = storage.to_string()
      )),
      StorageKind::Store => code.push(format!(
        "execute store result storage {storage} int 1 run {conversion_code}",
        storage = storage.to_string()
      )),
    }
    Ok(())
  }
}
