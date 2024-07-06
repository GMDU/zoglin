use std::collections::HashMap;

use regex::Regex;

use crate::parser::ast::ArrayType;

use super::{
  file_tree::{ScoreboardLocation, StorageLocation},
  Compiler,
};

pub(super) enum ExpressionType {
  Void,
  Byte(i8),
  Short(i16),
  Integer(i32),
  Long(i64),
  Float(f32),
  Double(f64),
  Storage(StorageLocation),
  Scoreboard(ScoreboardLocation),
  Boolean(bool),
  String(String),
  Array(Vec<ExpressionType>),
  ByteArray(Vec<ExpressionType>),
  IntArray(Vec<ExpressionType>),
  LongArray(Vec<ExpressionType>),
  Compound(HashMap<String, ExpressionType>),
  Condition(Condition),
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

impl ExpressionType {
  pub(super) fn to_storage(
    &self,
    state: &mut Compiler,
    code: &mut Vec<String>,
  ) -> (String, StorageKind) {
    match self {
      ExpressionType::Void => panic!("Cannot assign void to a value"),
      ExpressionType::Byte(b) => (format!("value {}b", *b), StorageKind::Direct),
      ExpressionType::Short(s) => (format!("value {}s", *s), StorageKind::Direct),
      ExpressionType::Integer(i) => (format!("value {}", *i), StorageKind::Direct),
      ExpressionType::Long(l) => (format!("value {}l", *l), StorageKind::Direct),
      ExpressionType::Float(f) => (format!("value {}f", *f), StorageKind::Direct),
      ExpressionType::Double(d) => (format!("value {}d", *d), StorageKind::Direct),
      ExpressionType::Boolean(b) => (format!("value {}", *b), StorageKind::Direct),
      ExpressionType::String(s) => (
        format!("value \"{}\"", s.escape_default().to_string()),
        StorageKind::Direct,
      ),
      ExpressionType::Array(a) => array_to_storage(&a, "", state, code),
      ExpressionType::ByteArray(a) => array_to_storage(&a, "B;", state, code),
      ExpressionType::IntArray(a) => array_to_storage(&a, "I;", state, code),
      ExpressionType::LongArray(a) => array_to_storage(&a, "L;", state, code),
      // TODO: optimise this, like a lot
      ExpressionType::Compound(types) => {
        let storage = state.next_storage().to_string();
        code.push(format!("data modify storage {storage} set value {{}}"));
        for (key, value) in types {
          let unescaped_regex = Regex::new("^[A-Za-z_]\\w*$").unwrap();
          let key = if unescaped_regex.is_match(&key) {
            key
          } else {
            // TODO: Escape other stuff too (like `\`)
            &format!("\"{}\"", key.replace("\"", "\\\""))
          };
          match value.to_storage(state, code) {
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
      ExpressionType::Storage(location) => (
        format!("from storage {}", location.to_string()),
        StorageKind::Direct,
      ),
      ExpressionType::Scoreboard(location) => (
        format!("scoreboard players get {}", location.to_string()),
        StorageKind::Indirect,
      ),
      ExpressionType::Condition(condition) => (
        format!("execute {}", condition.to_string()),
        StorageKind::Indirect,
      ),
    }
  }

  pub(super) fn to_score(&self) -> (String, ScoreKind) {
    match self {
      ExpressionType::Void => panic!("Cannot assign void to a value"),
      ExpressionType::Byte(b) => (b.to_string(), ScoreKind::Direct("set".to_string())),
      ExpressionType::Short(s) => (s.to_string(), ScoreKind::Direct("set".to_string())),
      ExpressionType::Integer(i) => (i.to_string(), ScoreKind::Direct("set".to_string())),
      ExpressionType::Long(l) => (
        (*l as i32).to_string(),
        ScoreKind::Direct("set".to_string()),
      ),
      ExpressionType::Float(f) => (
        (f.floor() as i32).to_string(),
        ScoreKind::Direct("set".to_string()),
      ),
      ExpressionType::Double(d) => (
        (d.floor() as i32).to_string(),
        ScoreKind::Direct("set".to_string()),
      ),
      ExpressionType::Boolean(b) => (
        if *b { "1" } else { "0" }.to_string(),
        ScoreKind::Direct("set".to_string()),
      ),
      ExpressionType::String(_) => panic!("Cannot assign string to a scoreboard variable"),
      ExpressionType::Array(_)
      | ExpressionType::ByteArray(_)
      | ExpressionType::IntArray(_)
      | ExpressionType::LongArray(_) => panic!("Cannot assign array to a scoreboard variable"),
      ExpressionType::Compound(_) => panic!("Cannot assign compound to a scoreboard variable"),
      ExpressionType::Storage(location) => (
        format!("data get storage {}", location.to_string()),
        ScoreKind::Indirect,
      ),
      ExpressionType::Scoreboard(location) => (
        format!("= {}", location.to_string()),
        ScoreKind::Direct("operation".to_string()),
      ),
      ExpressionType::Condition(condition) => (
        format!("execute {}", condition.to_string()),
        ScoreKind::Indirect,
      ),
    }
  }

  pub(super) fn to_condition(&self) -> ConditionKind {
    match self {
      ExpressionType::Void => panic!("Cannot check void"),
      ExpressionType::Byte(b) => ConditionKind::Known(*b != 0),
      ExpressionType::Short(s) => ConditionKind::Known(*s != 0),
      ExpressionType::Integer(i) => ConditionKind::Known(*i != 0),
      ExpressionType::Long(l) => ConditionKind::Known(*l != 0),
      ExpressionType::Float(f) => ConditionKind::Known(*f != 0.0),
      ExpressionType::Double(d) => ConditionKind::Known(*d != 0.0),
      ExpressionType::Boolean(b) => ConditionKind::Known(*b),
      ExpressionType::String(_) => panic!("Cannot use string as a condition"),
      ExpressionType::Array(_)
      | ExpressionType::ByteArray(_)
      | ExpressionType::IntArray(_)
      | ExpressionType::LongArray(_) => panic!("Cannot use array as a condition"),
      ExpressionType::Compound(_) => panic!("Cannot use compound as a condition"),
      ExpressionType::Condition(condition) => ConditionKind::Check(condition.to_string()),
      ExpressionType::Scoreboard(scoreboard) => {
        ConditionKind::Check(format!("unless score {} matches 0", scoreboard.to_string()))
      }
      ExpressionType::Storage(_) => todo!(),
    }
  }

  pub(super) fn numeric_value(&self) -> Option<i32> {
    Some(match self {
      ExpressionType::Byte(b) => *b as i32,
      ExpressionType::Short(s) => *s as i32,
      ExpressionType::Integer(i) => *i,
      ExpressionType::Long(l) => *l as i32,
      ExpressionType::Float(f) => f.floor() as i32,
      ExpressionType::Double(d) => d.floor() as i32,
      _ => return None,
    })
  }
}

// TODO: optimise this, like a lot
fn array_to_storage(
  elements: &[ExpressionType],
  prefix: &str,
  state: &mut Compiler,
  code: &mut Vec<String>,
) -> (String, StorageKind) {
  let storage = state.next_storage().to_string();
  code.push(format!(
    "data modify storage {storage} set value [{prefix}]"
  ));
  for element in elements {
    match element.to_storage(state, code) {
      (expr_code, StorageKind::Direct) => {
        code.push(format!("data modify storage {storage} append {expr_code}"));
      }
      (expr_code, StorageKind::Indirect) => {
        let temp_storage = state.next_storage().to_string();
        // TODO: Make type known
        code.push(format!(
          "execute store result storage {temp_storage} int 1 run {expr_code}"
        ));
        code.push(format!(
          "data modify storage {storage} append from storage {temp_storage}"
        ));
      }
    }
  }
  (format!("from storage {storage}"), StorageKind::Direct)
}

pub fn verify_types(types: &[ExpressionType], typ: ArrayType) -> bool {
  let mut single_type = match typ {
    ArrayType::Any => &ExpressionType::Void,
    ArrayType::Byte => &ExpressionType::Byte(0),
    ArrayType::Int => &ExpressionType::Integer(0),
    ArrayType::Long => &ExpressionType::Long(0),
  };

  for typ in types {
    match (typ, single_type) {
      (ExpressionType::Void, _) => panic!("Cannot use void as a value"),
      (ExpressionType::Storage(_), ExpressionType::Void) => {}
      (ExpressionType::Scoreboard(_), ExpressionType::Void) => {
        single_type = &ExpressionType::Integer(0)
      }
      (ExpressionType::Condition(_), ExpressionType::Void) => {
        single_type = &ExpressionType::Byte(0)
      }
      (ExpressionType::Boolean(_), ExpressionType::Void) => single_type = &ExpressionType::Byte(0),
      (_, ExpressionType::Void) => single_type = typ,
      (ExpressionType::Byte(_), ExpressionType::Byte(_)) => {}
      (ExpressionType::Short(_), ExpressionType::Short(_)) => {}
      (ExpressionType::Integer(_), ExpressionType::Integer(_)) => {}
      (ExpressionType::Long(_), ExpressionType::Long(_)) => {}
      (ExpressionType::Float(_), ExpressionType::Float(_)) => {}
      (ExpressionType::Double(_), ExpressionType::Double(_)) => {}
      (ExpressionType::Storage(_), _) => {}
      (ExpressionType::Scoreboard(_), ExpressionType::Integer(_)) => {}
      (ExpressionType::Boolean(_), ExpressionType::Byte(_)) => {}
      (ExpressionType::String(_), ExpressionType::String(_)) => {}
      (ExpressionType::Array(_), ExpressionType::Array(_)) => {}
      (ExpressionType::Condition(_), ExpressionType::Byte(_)) => {}
      _ => return false,
    }
  }

  true
}
