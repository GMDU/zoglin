use crate::{
  error::{raise_error, Location, Result},
  parser::ast,
};

use super::{
  expression::{Expression, ExpressionKind},
  Compiler, FunctionContext,
};

impl Compiler {
  pub(super) fn compile_builtin_function(
    &mut self,
    name: String,
    arguments: Vec<ast::Expression>,
    location: Location,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    let mut args = Vec::new();
    for arg in arguments {
      args.push(self.compile_expression(arg, context, false)?);
    }

    match name.as_str() {
      "temp_score" => {
        check_args(&location, 0, args.len())?;
        let scoreboard = self.next_scoreboard(&context.location.namespace);
        Ok(Expression::new(
          ExpressionKind::Scoreboard(scoreboard),
          location,
        ))
      }
      "temp_storage" => {
        check_args(&location, 0, args.len())?;
        let storage = self.next_storage(&context.location.namespace);
        Ok(Expression::new(ExpressionKind::Storage(storage), location))
      }
      _ => Err(raise_error(
        location,
        format!("Builtin function '@{name}' does not exist."),
      )),
    }
  }
}

fn check_args(location: &Location, expected: usize, got: usize) -> Result<()> {
  if expected == got {
    Ok(())
  } else {
    Err(raise_error(
      location.clone(),
      format!("Incorrect number of arguments. Expected {expected}, got {got}"),
    ))
  }
}
