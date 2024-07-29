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

  Storage(StorageLocation, Location),
  Scoreboard(ScoreboardLocation, Location),
  Macro(String, Location),
  Condition(Condition, Location),
}

pub(super) enum Condition {
  Less(ScoreboardLocation, ScoreboardLocation),
  LessEq(ScoreboardLocation, ScoreboardLocation),
  Greater(ScoreboardLocation, ScoreboardLocation),
  GreaterEq(ScoreboardLocation, ScoreboardLocation),
  Eq(ScoreboardLocation, ScoreboardLocation),
  Match(ScoreboardLocation, String),
  Inverted(Box<Condition>),
}

impl Condition {
  fn to_string(&self) -> String {
    self.do_to_string(false)
  }

  fn do_to_string(&self, invert: bool) -> String {
    let check_str = if invert { "unless" } else { "if" };
    match self {
      Condition::Less(a, b) => format!(
        "{check_str} score {a} < {b}",
        a = a.to_string(),
        b = b.to_string()
      ),
      Condition::LessEq(a, b) => {
        format!(
          "{check_str} score {a} <= {b}",
          a = a.to_string(),
          b = b.to_string()
        )
      }
      Condition::Greater(a, b) => {
        format!(
          "{check_str} score {a} > {b}",
          a = a.to_string(),
          b = b.to_string()
        )
      }
      Condition::GreaterEq(a, b) => {
        format!(
          "{check_str} score {a} >= {b}",
          a = a.to_string(),
          b = b.to_string()
        )
      }
      Condition::Eq(a, b) => {
        format!(
          "{check_str} score {a} = {b}",
          a = a.to_string(),
          b = b.to_string()
        )
      }
      Condition::Match(score, range) => {
        format!(
          "{check_str} score {score} matches {range}",
          score = score.to_string()
        )
      }
      Condition::Inverted(condition) => condition.do_to_string(!invert),
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
      "=" => Self::Eq(left, right),
      "!=" => Self::Inverted(Box::new(Self::Eq(left, right))),
      _ => unreachable!(),
    }
  }
}

pub(super) enum StorageKind {
  Modify,
  Store,
  Macro,
}

pub(super) enum ScoreKind {
  Direct(String),
  Indirect,
  Macro,
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
      Expression::Byte(b, _) => (format!("value {}b", *b), StorageKind::Modify),
      Expression::Short(s, _) => (format!("value {}s", *s), StorageKind::Modify),
      Expression::Integer(i, _) => (format!("value {}", *i), StorageKind::Modify),
      Expression::Long(l, _) => (format!("value {}l", *l), StorageKind::Modify),
      Expression::Float(f, _) => (format!("value {}f", *f), StorageKind::Modify),
      Expression::Double(d, _) => (format!("value {}d", *d), StorageKind::Modify),
      Expression::Boolean(b, _) => (format!("value {}", *b), StorageKind::Modify),
      Expression::String(s, _) => (
        format!("value \"{}\"", s.escape_default()),
        StorageKind::Modify,
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
            (expr_code, StorageKind::Modify) => {
              code.push(format!(
                "data modify storage {storage}.{key} set {expr_code}"
              ));
            }
            (expr_code, StorageKind::Store) => {
              code.push(format!(
                "execute store result storage {storage}.{key} int 1 run {expr_code}"
              ));
            }
            (name, StorageKind::Macro) => {
              code.push(format!(
                "$data modify storage {storage}.{key} set value $({name})"
              ));
            }
          }
        }
        (format!("from storage {storage}"), StorageKind::Modify)
      }
      Expression::Storage(location, _) => (
        format!("from storage {}", location.to_string()),
        StorageKind::Modify,
      ),
      Expression::Scoreboard(location, _) => (
        format!("scoreboard players get {}", location.to_string()),
        StorageKind::Store,
      ),
      Expression::Macro(name, _) => (name.clone(), StorageKind::Macro),
      Expression::Condition(condition, _) => (
        format!("execute {}", condition.to_string()),
        StorageKind::Store,
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
      Expression::Macro(name, _) => (name.clone(), ScoreKind::Macro),
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
    namespace: &str,
    inverted: bool,
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
      Expression::Condition(condition, _) => ConditionKind::Check(condition.do_to_string(inverted)),
      Expression::Scoreboard(scoreboard, _) => {
        ConditionKind::Check(format!("{} score {} matches 0", if inverted {"if"} else {"unless"}, scoreboard.to_string()))
      }
      Expression::Storage(_, _) => {
        let scoreboard = compiler.copy_to_scoreboard(code, self, namespace)?;
        ConditionKind::Check(format!("{} score {} matches 0", if inverted {"if"} else {"unless"}, scoreboard.to_string()))
      }
      Expression::Macro(_, _) => {
        let scoreboard = compiler.copy_to_scoreboard(code, self, namespace)?;
        ConditionKind::Check(format!("{} score {} matches 0", if inverted {"if"} else {"unless"}, scoreboard.to_string()))
      }
    })
  }

  pub(super) fn to_return_command(&self) -> Result<String> {
    Ok(match self {
      Expression::Void(location) => {
        return Err(raise_error(location.clone(), "Cannot return void"))
      }
      Expression::Byte(value, _) => format!("return {value}"),
      Expression::Short(value, _) => format!("return {value}"),
      Expression::Integer(value, _) => format!("return {value}"),
      Expression::Long(value, _) => format!("return {}", *value as i32),
      Expression::Float(value, _) => format!("return {}", value.floor() as i32),
      Expression::Double(value, _) => format!("return {}", value.floor() as i32),
      Expression::Boolean(b, _) => {
        if *b {
          format!("return 1")
        } else {
          format!("return 0")
        }
      }
      Expression::String(_, location)
      | Expression::Array { location, .. }
      | Expression::ByteArray(_, location)
      | Expression::IntArray(_, location)
      | Expression::LongArray(_, location)
      | Expression::Compound(_, location) => {
        return Err(raise_error(
          location.clone(),
          "Can only directly return numeric values",
        ))
      }
      Expression::Storage(storage, _) => {
        format!("return run data get storage {}", storage.to_string())
      }
      Expression::Scoreboard(scoreboard, _) => format!(
        "return run scoreboard players get {}",
        scoreboard.to_string()
      ),
      Expression::Macro(name, _) => format!("$return $({name})"),
      Expression::Condition(condition, _) => format!("return run execute {}", condition.to_string()),
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
      | Expression::Macro(_, location)
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
      Expression::Scoreboard(_, _) => NbtType::Numeric,
      Expression::Boolean(_, _) => NbtType::Byte,
      Expression::String(_, _) => NbtType::String,
      Expression::Array { .. } => NbtType::List,
      Expression::ByteArray(_, _) => NbtType::ByteArray,
      Expression::IntArray(_, _) => NbtType::IntArray,
      Expression::LongArray(_, _) => NbtType::LongArray,
      Expression::Compound(_, _) => NbtType::Compound,
      Expression::Macro(_, _) => NbtType::Unknown,
      Expression::Condition(_, _) => NbtType::Byte,
    }
  }
}

impl Expression {
  pub fn equal(&self, other: &Self) -> Option<bool> {
    Some(match (self, other) {
      (Self::Void(_), Self::Void(_)) => true,
      (Self::Byte(l0, _), Self::Byte(r0, _)) => l0 == r0,
      (Self::Short(l0, _), Self::Short(r0, _)) => l0 == r0,
      (Self::Integer(l0, _), Self::Integer(r0, _)) => l0 == r0,
      (Self::Long(l0, _), Self::Long(r0, _)) => l0 == r0,
      (Self::Float(l0, _), Self::Float(r0, _)) => l0 == r0,
      (Self::Double(l0, _), Self::Double(r0, _)) => l0 == r0,
      (Self::Boolean(l0, _), Self::Boolean(r0, _)) => l0 == r0,
      (Self::String(l0, _), Self::String(r0, _)) => l0 == r0,
      (
        Self::Array {
          values: l_values, ..
        },
        Self::Array {
          values: r_values, ..
        },
      ) => return compare_expr_array(l_values, r_values),

      (Self::ByteArray(l0, _), Self::ByteArray(r0, _)) => return compare_expr_array(l0, r0),
      (Self::IntArray(l0, _), Self::IntArray(r0, _)) => return compare_expr_array(l0, r0),
      (Self::LongArray(l0, _), Self::LongArray(r0, _)) => return compare_expr_array(l0, r0),
      (Self::Compound(l0, _), Self::Compound(r0, _)) => {
        if l0.len() != r0.len() {
          return Some(false);
        } else {
          let mut equal = Some(true);
          for (key, a) in l0 {
            if let Some(b) = r0.get(key) {
              match a.equal(b) {
                Some(true) => {}
                Some(false) => {
                  equal = Some(false);
                  break;
                }
                None => equal = None,
              }
            } else {
              equal = Some(false);
              break;
            }
          }
          return equal;
        }
      }
      (Self::Scoreboard(_, _), other) | (other, Self::Scoreboard(_, _))
        if !other.to_type().is_numeric() =>
      {
        false
      }
      (Self::Storage(_, _), _)
      | (_, Self::Storage(_, _))
      | (Self::Scoreboard(_, _), _)
      | (_, Self::Scoreboard(_, _))
      | (Self::Condition(_, _), _)
      | (_, Self::Condition(_, _)) => return None,
      _ => false,
    })
  }
}

fn compare_expr_array(l_values: &Vec<Expression>, r_values: &Vec<Expression>) -> Option<bool> {
  if l_values.len() != r_values.len() {
    Some(false)
  } else {
    let mut equal = Some(true);
    for (a, b) in l_values.iter().zip(r_values) {
      match a.equal(b) {
        Some(true) => {}
        Some(false) => {
          equal = Some(false);
          break;
        }
        None => equal = None,
      }
    }
    equal
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
      (expr_code, StorageKind::Modify) => {
        code.push(format!("data modify storage {storage} append {expr_code}"));
      }
      (name, StorageKind::Macro) => {
        code.push(format!(
          "$data modify storage {storage} append value $({name})"
        ));
      }
      (expr_code, StorageKind::Store) => {
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
  Ok((format!("from storage {storage}"), StorageKind::Modify))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NbtType {
  Unknown,
  Numeric,
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

        NbtType::Unknown
        | NbtType::Numeric
        | NbtType::ByteArray
        | NbtType::IntArray
        | NbtType::LongArray
        | NbtType::String
        | NbtType::List
        | NbtType::Compound => return None,
      }
      .to_string(),
    )
  }

  pub fn is_numeric(self) -> bool {
    match self {
      NbtType::Numeric
      | NbtType::Byte
      | NbtType::Short
      | NbtType::Int
      | NbtType::Long
      | NbtType::Float
      | NbtType::Double => true,

      NbtType::Unknown
      | NbtType::ByteArray
      | NbtType::IntArray
      | NbtType::LongArray
      | NbtType::String
      | NbtType::List
      | NbtType::Compound => false,
    }
  }
}

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
      (t, NbtType::Numeric) if t.to_type().is_numeric() => single_type = t.to_type(),
      (Expression::Byte(_, _), NbtType::Byte) => {}
      (Expression::Short(_, _), NbtType::Short) => {}
      (Expression::Integer(_, _), NbtType::Int) => {}
      (Expression::Long(_, _), NbtType::Long) => {}
      (Expression::Float(_, _), NbtType::Float) => {}
      (Expression::Double(_, _), NbtType::Double) => {}
      (Expression::Storage(_, _), _) => {}
      (Expression::Scoreboard(_, _), t) if t.is_numeric() => {}
      (Expression::Boolean(_, _), NbtType::Byte) => {}
      (Expression::String(_, _), NbtType::String) => {}
      (Expression::Array { .. }, NbtType::List) => {}
      (Expression::Condition(_, _), NbtType::Byte) => {}
      _ => return Err(raise_error(typ.location(), message)),
    }
  }

  if single_type == NbtType::Numeric {
    single_type = NbtType::Int;
  }

  Ok(single_type)
}
