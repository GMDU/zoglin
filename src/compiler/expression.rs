use std::collections::HashMap;

use regex::Regex;

use crate::{
  error::{raise_error, Location, Result},
  parser::ast::ArrayType,
};

use super::{
  file_tree::{ScoreboardLocation, StorageLocation},
  Compiler,
};

pub(super) enum Expression {
  Void(Location),
  Byte(i8, Location),
  Short(i16, Location),
  Integer(i32, Location),
  Long(i64, Location),
  Float(f32, Location),
  Double(f64, Location),
  Storage(StorageLocation, Location),
  Scoreboard(ScoreboardLocation, Location),
  Boolean(bool, Location),
  String(String, Location),
  Array {
    values: Vec<Expression>,
    data_type: NbtType,
    location: Location,
  },
  ByteArray(Vec<Expression>, Location),
  IntArray(Vec<Expression>, Location),
  LongArray(Vec<Expression>, Location),
  Compound(HashMap<String, Expression>, Location),
  Condition(Condition, Location),
}

pub(super) enum Condition {
  Less(ScoreboardLocation, ScoreboardLocation),
  LessEq(ScoreboardLocation, ScoreboardLocation),
  Greater(ScoreboardLocation, ScoreboardLocation),
  GreaterEq(ScoreboardLocation, ScoreboardLocation),
  Match(ScoreboardLocation, String),
}

impl Condition {
  fn to_string(&self) -> String {
    match self {
      Condition::Less(a, b) => format!("if score {a} < {b}", a = a.to_string(), b = b.to_string()),
      Condition::LessEq(a, b) => {
        format!("if score {a} <= {b}", a = a.to_string(), b = b.to_string())
      }
      Condition::Greater(a, b) => {
        format!("if score {a} > {b}", a = a.to_string(), b = b.to_string())
      }
      Condition::GreaterEq(a, b) => {
        format!("if score {a} >= {b}", a = a.to_string(), b = b.to_string())
      }
      Condition::Match(score, range) => {
        format!(
          "if score {score} matches {range}",
          score = score.to_string()
        )
      }
    }
  }

  pub(super) fn from_operator(
    operator: &str,
    left: ScoreboardLocation,
    right: ScoreboardLocation,
  ) -> Self {
    match operator {
      "<" => Self::Less(left, right),
      "<=" => Self::LessEq(left, right),
      ">" => Self::Greater(left, right),
      ">=" => Self::GreaterEq(left, right),
      _ => unreachable!(),
    }
  }
}

pub(super) enum StorageKind {
  Direct,
  Indirect,
}

pub(super) enum ScoreKind {
  Direct(String),
  Indirect,
}

pub(super) enum ConditionKind {
  Check(String),
  Known(bool),
}

impl Expression {
  pub(super) fn to_storage(
    &self,
    state: &mut Compiler,
    code: &mut Vec<String>,
  ) -> Result<(String, StorageKind)> {
    Ok(match self {
      Expression::Void(location) => {
        return Err(raise_error(
          location.clone(),
          "Cannot assign void to a value",
        ))
      }
      Expression::Byte(b, _) => (format!("value {}b", *b), StorageKind::Direct),
      Expression::Short(s, _) => (format!("value {}s", *s), StorageKind::Direct),
      Expression::Integer(i, _) => (format!("value {}", *i), StorageKind::Direct),
      Expression::Long(l, _) => (format!("value {}l", *l), StorageKind::Direct),
      Expression::Float(f, _) => (format!("value {}f", *f), StorageKind::Direct),
      Expression::Double(d, _) => (format!("value {}d", *d), StorageKind::Direct),
      Expression::Boolean(b, _) => (format!("value {}", *b), StorageKind::Direct),
      Expression::String(s, _) => (
        format!("value \"{}\"", s.escape_default()),
        StorageKind::Direct,
      ),
      Expression::Array {
        values, data_type, ..
      } => array_to_storage(values, *data_type, "", state, code)?,
      Expression::ByteArray(a, _) => array_to_storage(a, NbtType::Byte, "B;", state, code)?,
      Expression::IntArray(a, _) => array_to_storage(a, NbtType::Int, "I;", state, code)?,
      Expression::LongArray(a, _) => array_to_storage(a, NbtType::Long, "L;", state, code)?,
      // TODO: optimise this, like a lot
      Expression::Compound(types, _) => {
        let storage = state.next_storage().to_string();
        code.push(format!("data modify storage {storage} set value {{}}"));
        for (key, value) in types {
          let unescaped_regex = Regex::new("^[A-Za-z_]\\w*$").expect("Regex is valid");
          let key = if unescaped_regex.is_match(key) {
            key
          } else {
            &format!(
              "\"{}\"",
              key.escape_default().to_string().replace("\\'", "'")
            )
          };
          match value.to_storage(state, code)? {
            (expr_code, StorageKind::Direct) => {
              code.push(format!(
                "data modify storage {storage}.{key} set {expr_code}"
              ));
            }
            (expr_code, StorageKind::Indirect) => {
              code.push(format!(
                "execute store result storage {storage}.{key} int 1 run {expr_code}"
              ));
            }
          }
        }
        (format!("from storage {storage}"), StorageKind::Direct)
      }
      Expression::Storage(location, _) => (
        format!("from storage {}", location.to_string()),
        StorageKind::Direct,
      ),
      Expression::Scoreboard(location, _) => (
        format!("scoreboard players get {}", location.to_string()),
        StorageKind::Indirect,
      ),
      Expression::Condition(condition, _) => (
        format!("execute {}", condition.to_string()),
        StorageKind::Indirect,
      ),
    })
  }

  pub(super) fn to_score(&self) -> Result<(String, ScoreKind)> {
    Ok(match self {
      Expression::Void(location) => {
        return Err(raise_error(
          location.clone(),
          "Cannot assign void to a value",
        ))
      }
      Expression::Byte(b, _) => (b.to_string(), ScoreKind::Direct("set".to_string())),
      Expression::Short(s, _) => (s.to_string(), ScoreKind::Direct("set".to_string())),
      Expression::Integer(i, _) => (i.to_string(), ScoreKind::Direct("set".to_string())),
      Expression::Long(l, _) => (
        (*l as i32).to_string(),
        ScoreKind::Direct("set".to_string()),
      ),
      Expression::Float(f, _) => (
        (f.floor() as i32).to_string(),
        ScoreKind::Direct("set".to_string()),
      ),
      Expression::Double(d, _) => (
        (d.floor() as i32).to_string(),
        ScoreKind::Direct("set".to_string()),
      ),
      Expression::Boolean(b, _) => (
        if *b { "1" } else { "0" }.to_string(),
        ScoreKind::Direct("set".to_string()),
      ),
      Expression::String(_, location) => {
        return Err(raise_error(
          location.clone(),
          "Cannot assign string to a scoreboard variable",
        ))
      }
      Expression::Array { location, .. }
      | Expression::ByteArray(_, location)
      | Expression::IntArray(_, location)
      | Expression::LongArray(_, location) => {
        return Err(raise_error(
          location.clone(),
          "Cannot assign array to a scoreboard variable",
        ))
      }
      Expression::Compound(_, location) => {
        return Err(raise_error(
          location.clone(),
          "Cannot assign compound to a scoreboard variable",
        ))
      }
      Expression::Storage(location, _) => (
        format!("data get storage {}", location.to_string()),
        ScoreKind::Indirect,
      ),
      Expression::Scoreboard(location, _) => (
        format!("= {}", location.to_string()),
        ScoreKind::Direct("operation".to_string()),
      ),
      Expression::Condition(condition, _) => (
        format!("execute {}", condition.to_string()),
        ScoreKind::Indirect,
      ),
    })
  }

  pub(super) fn to_condition(
    &self,
    compiler: &mut Compiler,
    code: &mut Vec<String>,
  ) -> Result<ConditionKind> {
    Ok(match self {
      Expression::Void(location) => return Err(raise_error(location.clone(), "Cannot check void")),
      Expression::Byte(b, _) => ConditionKind::Known(*b != 0),
      Expression::Short(s, _) => ConditionKind::Known(*s != 0),
      Expression::Integer(i, _) => ConditionKind::Known(*i != 0),
      Expression::Long(l, _) => ConditionKind::Known(*l != 0),
      Expression::Float(f, _) => ConditionKind::Known(*f != 0.0),
      Expression::Double(d, _) => ConditionKind::Known(*d != 0.0),
      Expression::Boolean(b, _) => ConditionKind::Known(*b),
      Expression::String(_, location) => {
        return Err(raise_error(
          location.clone(),
          "Cannot use string as a condition",
        ))
      }
      Expression::Array { location, .. }
      | Expression::ByteArray(_, location)
      | Expression::IntArray(_, location)
      | Expression::LongArray(_, location) => {
        return Err(raise_error(
          location.clone(),
          "Cannot use array as a condition",
        ))
      }
      Expression::Compound(_, location) => {
        return Err(raise_error(
          location.clone(),
          "Cannot use compound as a condition",
        ))
      }
      Expression::Condition(condition, _) => ConditionKind::Check(condition.to_string()),
      Expression::Scoreboard(scoreboard, _) => {
        ConditionKind::Check(format!("unless score {} matches 0", scoreboard.to_string()))
      }
      Expression::Storage(_, _) => {
        let scoreboard = compiler.copy_to_scoreboard(code, self)?;
        ConditionKind::Check(format!("unless score {} matches 0", scoreboard.to_string()))
      }
    })
  }

  pub(super) fn numeric_value(&self) -> Option<i32> {
    Some(match self {
      Expression::Byte(b, _) => *b as i32,
      Expression::Short(s, _) => *s as i32,
      Expression::Integer(i, _) => *i,
      Expression::Long(l, _) => *l as i32,
      Expression::Float(f, _) => f.floor() as i32,
      Expression::Double(d, _) => d.floor() as i32,
      _ => return None,
    })
  }

  pub(super) fn location(&self) -> Location {
    match self {
      Expression::Void(location)
      | Expression::Byte(_, location)
      | Expression::Short(_, location)
      | Expression::Integer(_, location)
      | Expression::Long(_, location)
      | Expression::Float(_, location)
      | Expression::Double(_, location)
      | Expression::Storage(_, location)
      | Expression::Scoreboard(_, location)
      | Expression::Boolean(_, location)
      | Expression::String(_, location)
      | Expression::Array { location, .. }
      | Expression::ByteArray(_, location)
      | Expression::IntArray(_, location)
      | Expression::LongArray(_, location)
      | Expression::Compound(_, location)
      | Expression::Condition(_, location) => location.clone(),
    }
  }

  pub fn to_type(&self) -> NbtType {
    match self {
      Expression::Void(_) => NbtType::Unknown,
      Expression::Byte(_, _) => NbtType::Byte,
      Expression::Short(_, _) => NbtType::Short,
      Expression::Integer(_, _) => NbtType::Int,
      Expression::Long(_, _) => NbtType::Long,
      Expression::Float(_, _) => NbtType::Float,
      Expression::Double(_, _) => NbtType::Double,
      Expression::Storage(_, _) => NbtType::Unknown,
      Expression::Scoreboard(_, _) => NbtType::Int,
      Expression::Boolean(_, _) => NbtType::Byte,
      Expression::String(_, _) => NbtType::String,
      Expression::Array { .. } => NbtType::List,
      Expression::ByteArray(_, _) => NbtType::ByteArray,
      Expression::IntArray(_, _) => NbtType::IntArray,
      Expression::LongArray(_, _) => NbtType::LongArray,
      Expression::Compound(_, _) => NbtType::Compound,
      Expression::Condition(_, _) => NbtType::Byte,
    }
  }
}

// TODO: optimise this, like a lot
fn array_to_storage(
  elements: &[Expression],
  data_type: NbtType,
  prefix: &str,
  state: &mut Compiler,
  code: &mut Vec<String>,
) -> Result<(String, StorageKind)> {
  let storage = state.next_storage().to_string();
  code.push(format!(
    "data modify storage {storage} set value [{prefix}]"
  ));
  for element in elements {
    match element.to_storage(state, code)? {
      (expr_code, StorageKind::Direct) => {
        code.push(format!("data modify storage {storage} append {expr_code}"));
      }
      (expr_code, StorageKind::Indirect) => {
        let temp_storage = state.next_storage().to_string();
        code.push(format!(
          "execute store result storage {temp_storage} {data_type} 1 run {expr_code}",
          data_type = data_type
            .to_store_string()
            .expect("Only numeric types have an indirect storage kind")
        ));
        code.push(format!(
          "data modify storage {storage} append from storage {temp_storage}"
        ));
      }
    }
  }
  Ok((format!("from storage {storage}"), StorageKind::Direct))
}

#[derive(Debug, Clone, Copy)]
pub enum NbtType {
  Unknown,
  Byte,
  Short,
  Int,
  Long,
  Float,
  Double,
  ByteArray,
  IntArray,
  LongArray,
  String,
  List,
  Compound,
}

impl NbtType {
  pub fn to_store_string(self) -> Option<String> {
    Some(
      match self {
        NbtType::Byte => "byte",
        NbtType::Short => "short",
        NbtType::Int => "int",
        NbtType::Long => "long",
        NbtType::Float => "float",
        NbtType::Double => "double",
        _ => return None,
      }
      .to_string(),
    )
  }
}

// TODO: Allow casting scoreboards to any numeric type
pub fn verify_types(types: &[Expression], typ: ArrayType, message: &str) -> Result<NbtType> {
  let mut single_type = match typ {
    ArrayType::Any => NbtType::Unknown,
    ArrayType::Byte => NbtType::Byte,
    ArrayType::Int => NbtType::Int,
    ArrayType::Long => NbtType::Long,
  };

  for typ in types {
    match (typ, single_type) {
      (Expression::Void(location), _) => {
        return Err(raise_error(location.clone(), "Cannot use void as a value"))
      }
      (_, NbtType::Unknown) => single_type = typ.to_type(),
      (Expression::Byte(_, _), NbtType::Byte) => {}
      (Expression::Short(_, _), NbtType::Short) => {}
      (Expression::Integer(_, _), NbtType::Int) => {}
      (Expression::Long(_, _), NbtType::Long) => {}
      (Expression::Float(_, _), NbtType::Float) => {}
      (Expression::Double(_, _), NbtType::Double) => {}
      (Expression::Storage(_, _), _) => {}
      (Expression::Scoreboard(_, _), NbtType::Int) => {}
      (Expression::Boolean(_, _), NbtType::Byte) => {}
      (Expression::String(_, _), NbtType::String) => {}
      (Expression::Array { .. }, NbtType::List) => {}
      (Expression::Condition(_, _), NbtType::Byte) => {}
      _ => return Err(raise_error(typ.location(), message)),
    }
  }

  Ok(single_type)
}
