use crate::error::{raise_error, Error, Location};

use super::ast::{ParameterKind, ZoglinResource};

#[derive(Clone, Copy, PartialEq)]
pub enum NameKind {
  // Used where we don't have enough context yet and don't want to perform
  // validation on the name just yet
  Unknown,
  MacroVariable,
  NBTPathComponent,
  Namespace,
  Module,
  Function,
  Resource,
  ResourcePathComponent,
  Parameter(ParameterKind),
  StorageVariable,
  ScoreboardVariable,
  ComptimeVariable,
}

pub fn validate_or_quote(name: String, location: &Location, kind: NameKind) -> String {
  match validate(&name, location, kind) {
    Ok(_) => name,
    Err(_) => format!(
      "\"{}\"",
      name.escape_default().to_string().replace("\\'", "'")
    ),
  }
}

pub fn validate(name: &str, location: &Location, kind: NameKind) -> Result<(), Error> {
  match kind {
    NameKind::Unknown => Ok(()),
    NameKind::MacroVariable => verify(name, location, macro_variable, "macro variable"),
    NameKind::NBTPathComponent => verify(name, location, nbt_path_component, "compound member"),
    NameKind::Namespace => verify(name, location, resource_location_component, "namespace"),
    NameKind::Module => verify(name, location, resource_location_component, "module"),
    NameKind::Function => verify(name, location, resource_location_component, "function"),
    NameKind::Resource => verify(name, location, resource_location_component, "resource"),
    NameKind::ResourcePathComponent => verify(
      name,
      location,
      resource_location_component,
      "resource path component",
    ),
    NameKind::Parameter(ParameterKind::Storage) => {
      verify(name, location, nbt_path_component, "storage parameter")
    }
    NameKind::Parameter(ParameterKind::Scoreboard) => {
      verify_all(name, location, scoreboard_player, "scoreboard parameter")
    }
    NameKind::Parameter(ParameterKind::Macro) => {
      verify(name, location, macro_variable, "macro parameter")
    }
    // compile-time parameters aren't translated to mcfunction so they can use any name
    NameKind::Parameter(ParameterKind::CompileTime) => Ok(()),
    NameKind::StorageVariable => verify(name, location, nbt_path_component, "variable"),
    NameKind::ScoreboardVariable => {
      verify_all(name, location, scoreboard_player, "scoreboard variable")
    }
    // See the above comment on comptime parameters
    NameKind::ComptimeVariable => Ok(()),
  }
}

pub fn validate_zoglin_resource(resource: &ZoglinResource, kind: NameKind) -> Result<(), Error> {
  let location = &resource.location;
  match resource.namespace.as_deref() {
    // We allow the special case of `~` here
    Some("~") => {}
    Some(namespace) => validate(namespace, location, NameKind::Namespace)?,
    None => {}
  }
  for module in resource.modules.iter() {
    validate(module, location, NameKind::Module)?;
  }
  validate(&resource.name, location, kind)
}

fn verify(
  name: &str,
  location: &Location,
  valid_char_fn: impl Fn(char) -> bool,
  description: &str,
) -> Result<(), Error> {
  if name.chars().all(valid_char_fn) {
    Ok(())
  } else {
    Err(raise_error(
      location.clone(),
      format!("`{name}` is not a valid {description} name."),
    ))
  }
}

fn verify_all(
  name: &str,
  location: &Location,
  validate_fn: impl Fn(&str) -> bool,
  description: &str,
) -> Result<(), Error> {
  if validate_fn(name) {
    Ok(())
  } else {
    Err(raise_error(
      location.clone(),
      format!("`{name}` is not a valid {description} name."),
    ))
  }
}

fn macro_variable(c: char) -> bool {
  c.is_ascii_alphanumeric() || c == '_'
}

// TODO: Escape quotes if we encounter some in the middle (this isn't a problem for now)
// also, allow any characters if quoted with `'`
fn nbt_path_component(c: char) -> bool {
  !c.is_whitespace() && !matches!(c, '.' | '[' | ']' | '{' | '}' | '"')
}

fn resource_location_component(c: char) -> bool {
  c.is_numeric() || c.is_ascii_lowercase() || matches!(c, '_' | '.' | '-')
}

fn scoreboard_player(name: &str) -> bool {
  name.chars().next() != Some('@') && !name.chars().any(char::is_whitespace)
}
