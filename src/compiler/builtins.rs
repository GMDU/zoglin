use ecow::EcoString;
use crate::{
  error::{raise_error, Location, Result},
  parser::ast,
};

use super::{
  expression::{Expression, ExpressionKind}, Compiler, FunctionContext
};

impl Compiler {
  pub(super) fn compile_builtin_function(
    &mut self,
    name: &str,
    raw_arguments: Vec<ast::Expression>,
    location: Location,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    let mut arguments = Vec::new();
    for argument in raw_arguments {
      arguments.push(self.compile_expression(argument, context, false)?);
    }

    match name {
      "temp_score" => self.temp_score(arguments, location, context),
      "temp_storage" => self.temp_storage(arguments, location, context),
      "scoreboard" => self.def_scoreboard(arguments, location, context),
      _ => Err(raise_error(
        location,
        format!("Builtin function '@{name}' does not exist."),
      )),
    }
  }

  fn temp_score(
    &mut self,
    arguments: Vec<Expression>,
    location: Location,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    if arguments.len() > 1 {
      return Err(raise_error(
        location,
        format!(
          "Incorrect number of arguments. Expected 0 or 1, got {}",
          arguments.len()
        ),
      ));
    }

    let scoreboard = self.next_scoreboard(&context.location.namespace);

    match arguments.first() {
      None => {}
      Some(value) => self.set_scoreboard(&mut context.code, &scoreboard, value)?,
    }

    Ok(Expression::new(
      ExpressionKind::Scoreboard(scoreboard),
      location,
    ))
  }

  fn temp_storage(
    &mut self,
    arguments: Vec<Expression>,
    location: Location,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    if arguments.len() > 1 {
      return Err(raise_error(
        location,
        format!(
          "Incorrect number of arguments. Expected 0 or 1, got {}",
          arguments.len()
        ),
      ));
    }

    let storage = self.next_storage(&context.location.namespace);

    match arguments.first() {
      None => {}
      Some(value) => self.set_storage(
        &mut context.code,
        &storage,
        value,
        &context.location.namespace,
      )?,
    }

    Ok(Expression::new(ExpressionKind::Storage(storage), location))
  }

  fn def_scoreboard(
    &mut self,
    arguments: Vec<Expression>,
    location: Location,
    _context: &mut FunctionContext,
  ) -> Result<Expression> {
    if arguments.len() > 2 || arguments.len() < 1 {
      return Err(raise_error(
        location,
        format!(
          "Incorrect number of arguments. Expected 1 or 2, got {}",
          arguments.len()
        ),
      ));
    };
  
    let name: EcoString = match &arguments.first().unwrap().kind {
      ExpressionKind::Storage(storage_location) => {
        let storage = &storage_location.storage;
        let mut path = vec![storage.namespace.clone()];
        path.extend(storage.modules.clone());
        path.push(storage_location.name.clone());

        path.join(".").into()
      },
      _ => {return Err(raise_error(location, "Invalid argument. Expected zoglin path."))},
    };

    match arguments.get(1) {
      Some(expression) => {
        match &expression.kind {
          ExpressionKind::String(critera) => {
            self.use_scoreboard(name, critera.clone());
          }
          _ => {return Err(raise_error(location, "Invalid argument. Expected string."))}
        }
      },
      None => {
        self.use_scoreboard_dummy(name);
      }
    };
    
    Ok(Expression::new(ExpressionKind::Void, location))
  }
}

fn _check_args(location: &Location, expected: usize, got: usize) -> Result<()> {
  if expected == got {
    Ok(())
  } else {
    Err(raise_error(
      location.clone(),
      format!("Incorrect number of arguments. Expected {expected}, got {got}"),
    ))
  }
}
