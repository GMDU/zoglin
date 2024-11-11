use crate::{
  error::{raise_error, Location, Result},
  parser::ast,
};
use ecow::EcoString;

use super::{
  expression::{Expression, ExpressionKind},
  Compiler, FunctionContext,
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
      "set" => self.set(arguments, location, context),
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

    let name: EcoString = match &arguments
      .first()
      .expect("There must be at least one argument")
      .kind
    {
      ExpressionKind::Storage(storage_location) => {
        let storage = &storage_location.storage;
        let mut path = vec![storage.namespace.clone()];
        path.extend(storage.modules.clone());
        path.push(storage_location.name.clone());

        path.join(".").into()
      }
      _ => {
        return Err(raise_error(
          location,
          "Invalid argument. Expected zoglin path.",
        ))
      }
    };

    match arguments.get(1) {
      Some(expression) => match &expression.kind {
        ExpressionKind::String(critera) => {
          self.use_scoreboard(name, critera.clone());
        }
        _ => return Err(raise_error(location, "Invalid argument. Expected string.")),
      },
      None => {
        self.use_scoreboard_dummy(name);
      }
    };

    Ok(Expression::new(ExpressionKind::Void, location))
  }

  fn set(
    &mut self,
    arguments: Vec<Expression>,
    location: Location,
    context: &mut FunctionContext,
  ) -> Result<Expression> {
    check_args(&location, 2, arguments.len())?;

    let [dst, src]: [Expression; 2] = arguments
      .try_into()
      .unwrap_or_else(|_| panic!("There must be exactly two arguments"));

    match dst.kind {
      ExpressionKind::Void
      | ExpressionKind::Byte(_)
      | ExpressionKind::Short(_)
      | ExpressionKind::Integer(_)
      | ExpressionKind::Long(_)
      | ExpressionKind::Float(_)
      | ExpressionKind::Double(_)
      | ExpressionKind::Boolean(_)
      | ExpressionKind::String(_)
      | ExpressionKind::Array { .. }
      | ExpressionKind::ByteArray(_)
      | ExpressionKind::IntArray(_)
      | ExpressionKind::LongArray(_)
      | ExpressionKind::SubString(_, _, _)
      | ExpressionKind::Macro(_)
      | ExpressionKind::Condition(_)
      | ExpressionKind::Compound(_) => {
        return Err(raise_error(
          location,
          "`@set` can only be used on scoreboards and storages.",
        ))
      }
      ExpressionKind::Storage(storage_location) => self.set_storage(
        &mut context.code,
        &storage_location,
        &src,
      )?,
      ExpressionKind::Scoreboard(scoreboard_location) => {
        self.set_scoreboard(&mut context.code, &scoreboard_location, &src)?
      }
    }

    Ok(Expression::new(ExpressionKind::Void, location))
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
