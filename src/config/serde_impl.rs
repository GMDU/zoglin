use std::iter::Peekable;

use serde::{
  de::{self, Visitor},
  Deserialize, Serialize,
};

use crate::config::ConstraintKind;

use super::{Version, VersionConstraint};

impl Serialize for VersionConstraint {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    serializer.serialize_str(&self.to_string())
  }
}

struct VersionConstraintVisitor;

impl<'de> Visitor<'de> for VersionConstraintVisitor {
  type Value = VersionConstraint;

  fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    formatter.write_str("a string")
  }

  fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    parse_version_constraint(&mut v.chars().peekable())
  }
}

fn parse_version_constraint<E: de::Error>(
  chars: &mut Peekable<impl Iterator<Item = char>>,
) -> Result<VersionConstraint, E> {
  let constraint = do_parse_version_constraint(chars)?;
  if let Some(next) = chars.peek() {
    return Err(E::custom(format!(
      "Expected end of version constraint, got `{next}`"
    )));
  }
  Ok(constraint)
}

fn do_parse_version_constraint<E: de::Error>(
  chars: &mut Peekable<impl Iterator<Item = char>>,
) -> Result<VersionConstraint, E> {
  skip_whitespace(chars);
  let mut left = parse_single_constraint(chars)?;
  skip_whitespace(chars);

  while let Some(next) = chars.peek() {
    match next {
      '&' => {
        chars.next();
        if chars.peek() != Some(&'&') {
          return Err(E::custom("Expected `&&`"));
        }
        chars.next();

        skip_whitespace(chars);
        let right = do_parse_version_constraint(chars)?;
        left = VersionConstraint::And(Box::new(left), Box::new(right));
      }
      '|' => {
        chars.next();
        if chars.peek() != Some(&'|') {
          return Err(E::custom("Expected `||`"));
        }
        chars.next();

        skip_whitespace(chars);
        let right = do_parse_version_constraint(chars)?;
        left = VersionConstraint::Or(Box::new(left), Box::new(right));
      }
      _ => break,
    }
  }
  Ok(left)
}

fn parse_single_constraint<E: de::Error>(
  chars: &mut Peekable<impl Iterator<Item = char>>,
) -> Result<VersionConstraint, E> {
  let kind = match chars
    .peek()
    .ok_or(E::custom("Expected version constraint"))?
  {
    '>' => {
      chars.next();
      if chars
        .peek()
        .ok_or(E::custom("Expected version after `>`"))?
        == &'='
      {
        chars.next();
        ConstraintKind::GreaterOrEqual
      } else {
        ConstraintKind::Greater
      }
    }
    '<' => {
      chars.next();
      if chars
        .peek()
        .ok_or(E::custom("Expected version after `<`"))?
        == &'='
      {
        chars.next();
        ConstraintKind::LessOrEqual
      } else {
        ConstraintKind::Less
      }
    }
    '~' => {
      chars.next();
      if chars.peek() == Some(&'>') {
        chars.next();
        ConstraintKind::UntilNextSignificant
      } else {
        return Err(E::custom("Exprected `>` after `~`"));
      }
    }
    '(' => {
      chars.next();
      let constraint = do_parse_version_constraint(chars)?;
      if chars.peek() != Some(&')') {
        return Err(E::custom("Expected `)`"));
      }
      chars.next();
      return Ok(VersionConstraint::Grouped(Box::new(constraint)));
    }
    _ => ConstraintKind::Single,
  };

  skip_whitespace(chars);

  let version = parse_version(chars)?;

  Ok(VersionConstraint::Single {
    kind,
    constrained_to: version,
  })
}

fn skip_whitespace(chars: &mut Peekable<impl Iterator<Item = char>>) {
  while chars.peek().is_some_and(|c| c.is_whitespace()) {
    chars.next();
  }
}

impl<'de> Deserialize<'de> for VersionConstraint {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: de::Deserializer<'de>,
  {
    deserializer.deserialize_str(VersionConstraintVisitor)
  }
}

impl Serialize for Version {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    serializer.serialize_str(&self.to_string())
  }
}

struct VersionVisitor;

impl<'de> Visitor<'de> for VersionVisitor {
  type Value = Version;

  fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    formatter.write_str("a string")
  }

  fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    parse_version(&mut v.chars().peekable())
  }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum VersionParseState {
  Major,
  Minor,
  Patch,
  Extra,
}

fn parse_version<E: de::Error>(
  chars: &mut Peekable<impl Iterator<Item = char>>,
) -> Result<Version, E> {
  let mut version = Version {
    major: 0,
    minor: None,
    patch: None,
    extra: None,
  };
  let mut current = String::new();
  let mut state = VersionParseState::Major;

  while let Some(&char) = chars.peek() {
    if char.is_ascii_digit() {
      chars.next();
      current.push(char);
    } else {
      match state {
        VersionParseState::Major => {
          version.major = current.parse().map_err(E::custom)?;
          current.clear();
          if chars.peek() == Some(&'.') {
            chars.next();
            state = VersionParseState::Minor;
          } else {
            break;
          }
        }
        VersionParseState::Minor => {
          version.minor = Some(current.parse().map_err(E::custom)?);
          current.clear();
          if chars.peek() == Some(&'.') {
            chars.next();
            state = VersionParseState::Patch;
          } else {
            break;
          }
        }
        VersionParseState::Patch => {
          version.patch = Some(current.parse().map_err(E::custom)?);
          current.clear();
          state = VersionParseState::Extra;
          break;
        }
        VersionParseState::Extra => unreachable!("We always break when we reach here"),
      }
    }
  }

  match state {
    VersionParseState::Major => {
      return Err(E::custom("Expected version number"));
    }
    VersionParseState::Minor => {
      version.minor = Some(current.parse().map_err(E::custom)?);
    }
    VersionParseState::Patch => {
      version.patch = Some(current.parse().map_err(E::custom)?);
    }
    VersionParseState::Extra => {
      let mut rest = String::new();
      while let Some(c) = chars.peek() {
        if !matches!(c, '0'..='9' | 'a'..='z' | 'A'..='Z' | '-' | '_' | '.' ) {
          break;
        }
        rest.push(*c);
        chars.next();
      }
      version.extra = Some(rest)
    }
  };

  Ok(version)
}

impl<'de> Deserialize<'de> for Version {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    deserializer.deserialize_str(VersionVisitor)
  }
}
