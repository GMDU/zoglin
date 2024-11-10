use std::{collections::HashMap, fmt::Display};

use ecow::{eco_format, EcoString};

use crate::{
  error::{raise_error, Location, Result},
  parser::ast::ArrayType,
};

use super::{
  file_tree::{ScoreboardLocation, StorageLocation},
  utils::ToEcoString,
  Compiler,
};

#[derive(Clone)]
pub struct Expression {
  pub location: Location,
  pub needs_macro: bool,
  pub kind: ExpressionKind,
}

#[derive(Clone)]
pub enum ExpressionKind {
  Void,
  Byte(i8),
  Short(i16),
  Integer(i32),
  Long(i64),
  Float(f32),
  Double(f64),
  Boolean(bool),
  String(EcoString),
  Array {
    values: Vec<Expression>,
    data_type: NbtType,
  },
  ByteArray(Vec<Expression>),
  IntArray(Vec<Expression>),
  LongArray(Vec<Expression>),
  Compound(HashMap<EcoString, Expression>),

  Storage(StorageLocation),
  SubString(StorageLocation, i32, Option<i32>),
  Scoreboard(ScoreboardLocation),
  Macro(StorageLocation),
  Condition(Condition),
}

#[derive(Clone)]
pub enum Condition {
  Less(ScoreboardLocation, ScoreboardLocation),
  LessEq(ScoreboardLocation, ScoreboardLocation),
  Greater(ScoreboardLocation, ScoreboardLocation),
  GreaterEq(ScoreboardLocation, ScoreboardLocation),
  Eq(ScoreboardLocation, ScoreboardLocation),
  Match(ScoreboardLocation, EcoString),
  Check(EcoString),
  And(EcoString, EcoString),
  Inverted(Box<Condition>),
}

impl Display for Condition {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.do_to_string(false))
  }
}

impl Condition {
  fn do_to_string(&self, invert: bool) -> EcoString {
    let check_str = if invert { "unless" } else { "if" };
    match self {
      Condition::Less(a, b) => eco_format!("{check_str} score {a} < {b}",),
      Condition::LessEq(a, b) => {
        eco_format!("{check_str} score {a} <= {b}",)
      }
      Condition::Greater(a, b) => {
        eco_format!("{check_str} score {a} > {b}",)
      }
      Condition::GreaterEq(a, b) => {
        eco_format!("{check_str} score {a} >= {b}",)
      }
      Condition::Eq(a, b) => {
        eco_format!("{check_str} score {a} = {b}",)
      }
      Condition::Match(score, range) => {
        eco_format!("{check_str} score {score} matches {range}",)
      }
      Condition::Check(code) => {
        if invert {
          Self::invert_code(code)
        } else {
          code.clone()
        }
      }
      Condition::And(a, b) => {
        if invert {
          eco_format!("{} {}", Self::invert_code(a), Self::invert_code(b))
        } else {
          eco_format!("{a} {b}")
        }
      }
      Condition::Inverted(condition) => condition.do_to_string(!invert),
    }
  }

  fn invert_code(code: &str) -> EcoString {
    if let Some(condition) = code.strip_prefix("if") {
      eco_format!("unless {condition}")
    } else if let Some(condition) = code.strip_prefix("unless") {
      eco_format!("if {condition}")
    } else {
      code.into()
    }
  }

  pub fn from_operator(
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

#[derive(Clone, Copy)]
pub enum StorageKind {
  Modify,
  Store,
  MacroModify,
  MacroStore,
}

pub enum ScoreKind {
  Direct(EcoString),
  DirectMacro(EcoString),
  Indirect,
  IndirectMacro,
}

pub enum ConditionKind {
  Check(EcoString),
  Known(bool),
}

impl Expression {
  pub fn new(kind: ExpressionKind, location: Location) -> Expression {
    Expression {
      location,
      needs_macro: false,
      kind,
    }
  }

  pub fn with_macro(kind: ExpressionKind, location: Location, needs_macro: bool) -> Expression {
    Expression {
      location,
      needs_macro: needs_macro && !kind.compile_time_known(),
      kind,
    }
  }

  pub fn to_storage(
    &self,
    state: &mut Compiler,
    code: &mut Vec<EcoString>,
    namespace: &str,
  ) -> Result<(EcoString, StorageKind)> {
    if let Some(string) = self.kind.to_comptime_string(false) {
      return Ok((eco_format!("value {string}"), StorageKind::Modify));
    }

    let (conversion_code, kind) = match &self.kind {
      ExpressionKind::Void => {
        return Err(raise_error(
          self.location.clone(),
          "Cannot assign void to a value",
        ))
      }
      ExpressionKind::Byte(b) => (eco_format!("value {}b", *b), StorageKind::Modify),
      ExpressionKind::Short(s) => (eco_format!("value {}s", *s), StorageKind::Modify),
      ExpressionKind::Integer(i) => (eco_format!("value {}", *i), StorageKind::Modify),
      ExpressionKind::Long(l) => (eco_format!("value {}l", *l), StorageKind::Modify),
      ExpressionKind::Float(f) => (eco_format!("value {}f", *f), StorageKind::Modify),
      ExpressionKind::Double(d) => (eco_format!("value {}d", *d), StorageKind::Modify),
      ExpressionKind::Boolean(b) => (eco_format!("value {}", *b), StorageKind::Modify),
      ExpressionKind::String(s) => (
        eco_format!("value \"{}\"", s.escape_default()),
        StorageKind::Modify,
      ),
      ExpressionKind::Array {
        values, data_type, ..
      } => array_to_storage(values, *data_type, "", state, code, namespace)?,
      ExpressionKind::ByteArray(a) => {
        array_to_storage(a, NbtType::Byte, "B; ", state, code, namespace)?
      }
      ExpressionKind::IntArray(a) => {
        array_to_storage(a, NbtType::Int, "I; ", state, code, namespace)?
      }
      ExpressionKind::LongArray(a) => {
        array_to_storage(a, NbtType::Long, "L; ", state, code, namespace)?
      }
      // TODO: optimise this, like a lot
      ExpressionKind::Compound(types) => {
        let storage = state.next_storage(namespace).to_eco_string();
        code.push(eco_format!("data modify storage {storage} set value {{}}"));
        for (key, value) in types {
          match value.to_storage(state, code, namespace)? {
            (expr_code, StorageKind::Modify) => {
              code.push(eco_format!(
                "data modify storage {storage}.{key} set {expr_code}"
              ));
            }
            (expr_code, StorageKind::Store) => {
              code.push(eco_format!(
                "execute store result storage {storage}.{key} int 1 run {expr_code}"
              ));
            }
            (expr_code, StorageKind::MacroModify) => {
              code.push(eco_format!(
                "$data modify storage {storage}.{key} set {expr_code}"
              ));
            }
            (expr_code, StorageKind::MacroStore) => {
              code.push(eco_format!(
                "$execute store result storage {storage}.{key} int 1 run {expr_code}"
              ));
            }
          }
        }
        (eco_format!("from storage {storage}"), StorageKind::Modify)
      }
      ExpressionKind::Storage(storage) => {
        (eco_format!("from storage {storage}"), StorageKind::Modify)
      }
      ExpressionKind::SubString(storage, start, end) => (
        eco_format!(
          "string storage {storage} {start}{}",
          if let Some(end) = end {
            eco_format!(" {end}")
          } else {
            EcoString::new()
          }
        ),
        StorageKind::Modify,
      ),
      ExpressionKind::Scoreboard(scoreboard) => (
        eco_format!("scoreboard players get {scoreboard}",),
        StorageKind::Store,
      ),
      ExpressionKind::Macro(storage) => (
        eco_format!("value $({})", storage.name),
        StorageKind::MacroModify,
      ),
      ExpressionKind::Condition(condition) => {
        (eco_format!("execute {}", condition), StorageKind::Store)
      }
    };

    let kind = match (kind, self.needs_macro) {
      (StorageKind::Modify, true) => StorageKind::MacroModify,
      (StorageKind::Store, true) => StorageKind::MacroStore,
      _ => kind,
    };
    Ok((conversion_code, kind))
  }

  pub fn to_score(&self) -> Result<(EcoString, ScoreKind)> {
    let (conversion_code, kind) = match &self.kind {
      ExpressionKind::Void => {
        return Err(raise_error(
          self.location.clone(),
          "Cannot assign void to a value",
        ))
      }
      ExpressionKind::Byte(b) => (b.to_eco_string(), ScoreKind::Direct("set".into())),
      ExpressionKind::Short(s) => (s.to_eco_string(), ScoreKind::Direct("set".into())),
      ExpressionKind::Integer(i) => (i.to_eco_string(), ScoreKind::Direct("set".into())),
      ExpressionKind::Long(l) => ((*l as i32).to_eco_string(), ScoreKind::Direct("set".into())),
      ExpressionKind::Float(f) => (
        (f.floor() as i32).to_eco_string(),
        ScoreKind::Direct("set".into()),
      ),
      ExpressionKind::Double(d) => (
        (d.floor() as i32).to_eco_string(),
        ScoreKind::Direct("set".into()),
      ),
      ExpressionKind::Boolean(b) => (
        if *b { "1" } else { "0" }.to_eco_string(),
        ScoreKind::Direct("set".into()),
      ),
      ExpressionKind::String(_) | ExpressionKind::SubString(_, _, _) => {
        return Err(raise_error(
          self.location.clone(),
          "Cannot assign string to a scoreboard variable",
        ))
      }
      ExpressionKind::Array { .. }
      | ExpressionKind::ByteArray(_)
      | ExpressionKind::IntArray(_)
      | ExpressionKind::LongArray(_) => {
        return Err(raise_error(
          self.location.clone(),
          "Cannot assign array to a scoreboard variable",
        ))
      }
      ExpressionKind::Compound(_) => {
        return Err(raise_error(
          self.location.clone(),
          "Cannot assign compound to a scoreboard variable",
        ))
      }
      ExpressionKind::Storage(storage) => (
        eco_format!("data get storage {storage}"),
        ScoreKind::Indirect,
      ),
      ExpressionKind::Scoreboard(scoreboard) => (
        eco_format!("= {scoreboard}"),
        ScoreKind::Direct("operation".into()),
      ),
      ExpressionKind::Macro(storage) => (
        eco_format!("$({})", storage.name),
        ScoreKind::DirectMacro("set".into()),
      ),
      ExpressionKind::Condition(condition) => {
        (eco_format!("execute {}", condition), ScoreKind::Indirect)
      }
    };

    let kind = match (kind, self.needs_macro) {
      (ScoreKind::Direct(code), true) => ScoreKind::DirectMacro(code),
      (ScoreKind::Indirect, true) => ScoreKind::IndirectMacro,
      (kind, _) => kind,
    };
    Ok((conversion_code, kind))
  }

  pub fn to_condition(
    &self,
    compiler: &mut Compiler,
    code: &mut Vec<EcoString>,
    namespace: &str,
    inverted: bool,
  ) -> Result<ConditionKind> {
    Ok(match &self.kind {
      ExpressionKind::Void => return Err(raise_error(self.location.clone(), "Cannot check void")),
      ExpressionKind::Byte(b) => ConditionKind::Known(*b != 0),
      ExpressionKind::Short(s) => ConditionKind::Known(*s != 0),
      ExpressionKind::Integer(i) => ConditionKind::Known(*i != 0),
      ExpressionKind::Long(l) => ConditionKind::Known(*l != 0),
      ExpressionKind::Float(f) => ConditionKind::Known(*f != 0.0),
      ExpressionKind::Double(d) => ConditionKind::Known(*d != 0.0),
      ExpressionKind::Boolean(b) => ConditionKind::Known(*b),
      ExpressionKind::String(_) | ExpressionKind::SubString(_, _, _) => {
        return Err(raise_error(
          self.location.clone(),
          "Cannot use string as a condition",
        ))
      }
      ExpressionKind::Array { .. }
      | ExpressionKind::ByteArray(_)
      | ExpressionKind::IntArray(_)
      | ExpressionKind::LongArray(_) => {
        return Err(raise_error(
          self.location.clone(),
          "Cannot use array as a condition",
        ))
      }
      ExpressionKind::Compound(_) => {
        return Err(raise_error(
          self.location.clone(),
          "Cannot use compound as a condition",
        ))
      }
      ExpressionKind::Condition(condition) => {
        ConditionKind::Check(condition.do_to_string(inverted))
      }
      ExpressionKind::Scoreboard(scoreboard) => ConditionKind::Check(eco_format!(
        "{} score {scoreboard} matches 0",
        if inverted { "if" } else { "unless" },
      )),
      ExpressionKind::Storage(_) => {
        let scoreboard = compiler.copy_to_scoreboard(code, self, namespace)?;
        ConditionKind::Check(eco_format!(
          "{} score {scoreboard} matches 0",
          if inverted { "if" } else { "unless" },
        ))
      }
      ExpressionKind::Macro(_) => {
        let scoreboard = compiler.copy_to_scoreboard(code, self, namespace)?;
        ConditionKind::Check(eco_format!(
          "{} score {scoreboard} matches 0",
          if inverted { "if" } else { "unless" },
        ))
      }
    })
  }

  pub fn to_return_command(&self) -> Result<EcoString> {
    Ok(match &self.kind {
      ExpressionKind::Void => return Err(raise_error(self.location.clone(), "Cannot return void")),
      ExpressionKind::Byte(value) => eco_format!("return {value}"),
      ExpressionKind::Short(value) => eco_format!("return {value}"),
      ExpressionKind::Integer(value) => eco_format!("return {value}"),
      ExpressionKind::Long(value) => eco_format!("return {}", *value as i32),
      ExpressionKind::Float(value) => eco_format!("return {}", value.floor() as i32),
      ExpressionKind::Double(value) => eco_format!("return {}", value.floor() as i32),
      ExpressionKind::Boolean(b) => {
        if *b {
          "return 1".into()
        } else {
          "return 0".into()
        }
      }
      ExpressionKind::String(_)
      | ExpressionKind::SubString(_, _, _)
      | ExpressionKind::Array { .. }
      | ExpressionKind::ByteArray(_)
      | ExpressionKind::IntArray(_)
      | ExpressionKind::LongArray(_)
      | ExpressionKind::Compound(_) => {
        return Err(raise_error(
          self.location.clone(),
          "Can only directly return numeric values",
        ))
      }
      ExpressionKind::Storage(storage) => {
        eco_format!("return run data get storage {storage}")
      }
      ExpressionKind::Scoreboard(scoreboard) => {
        eco_format!("return run scoreboard players get {scoreboard}")
      }
      ExpressionKind::Macro(name) => eco_format!("$return $({name})"),
      ExpressionKind::Condition(condition) => {
        eco_format!("return run execute {}", condition)
      }
    })
  }
}

#[allow(dead_code)]
pub enum NbtValue {
  Byte(i8),
  Short(i16),
  Int(i32),
  Long(i64),
  Float(f32),
  Double(f64),
  ByteArray(Vec<i8>),
  IntArray(Vec<i32>),
  LongArray(Vec<i64>),
  String(EcoString),
  List(Vec<NbtValue>),
  Compound(HashMap<EcoString, NbtValue>),
}

impl ExpressionKind {
  pub fn numeric_value(&self) -> Option<i32> {
    Some(match self {
      ExpressionKind::Byte(b) => *b as i32,
      ExpressionKind::Short(s) => *s as i32,
      ExpressionKind::Integer(i) => *i,
      ExpressionKind::Long(l) => *l as i32,
      ExpressionKind::Float(f) => f.floor() as i32,
      ExpressionKind::Double(d) => d.floor() as i32,
      _ => return None,
    })
  }

  pub fn compile_time_value(&self) -> Option<NbtValue> {
    if !self.compile_time_known() {
      return None;
    }

    Some(match self {
      ExpressionKind::Void => return None,
      ExpressionKind::Byte(b) => NbtValue::Byte(*b),
      ExpressionKind::Short(s) => NbtValue::Short(*s),
      ExpressionKind::Integer(i) => NbtValue::Int(*i),
      ExpressionKind::Long(l) => NbtValue::Long(*l),
      ExpressionKind::Float(f) => NbtValue::Float(*f),
      ExpressionKind::Double(d) => NbtValue::Double(*d),
      ExpressionKind::Boolean(b) => NbtValue::Byte(*b as i8),
      ExpressionKind::String(s) => NbtValue::String(s.clone()),
      ExpressionKind::Array { values, .. } => NbtValue::List(
        values
          .iter()
          .map(|value| {
            value
              .kind
              .compile_time_value()
              .expect("Value is comptime-known")
          })
          .collect(),
      ),
      ExpressionKind::ByteArray(values) => NbtValue::ByteArray(
        values
          .iter()
          .map(|value| value.kind.numeric_value().expect("Value is comptime-known") as i8)
          .collect(),
      ),
      ExpressionKind::IntArray(values) => NbtValue::IntArray(
        values
          .iter()
          .map(|value| value.kind.numeric_value().expect("Value is comptime-known"))
          .collect(),
      ),
      ExpressionKind::LongArray(values) => NbtValue::LongArray(
        values
          .iter()
          .map(|value| value.kind.numeric_value().expect("Value is comptime-known") as i64)
          .collect(),
      ),
      ExpressionKind::Compound(values) => NbtValue::Compound(
        values
          .iter()
          .map(|(key, value)| {
            (
              key.clone(),
              value
                .kind
                .compile_time_value()
                .expect("Value is comptime-known"),
            )
          })
          .collect(),
      ),
      _ => return None,
    })
  }

  pub fn to_type(&self) -> NbtType {
    match self {
      ExpressionKind::Void => NbtType::Unknown,
      ExpressionKind::Byte(_) => NbtType::Byte,
      ExpressionKind::Short(_) => NbtType::Short,
      ExpressionKind::Integer(_) => NbtType::Int,
      ExpressionKind::Long(_) => NbtType::Long,
      ExpressionKind::Float(_) => NbtType::Float,
      ExpressionKind::Double(_) => NbtType::Double,
      ExpressionKind::Storage(_) => NbtType::Unknown,
      ExpressionKind::Scoreboard(_) => NbtType::Numeric,
      ExpressionKind::Boolean(_) => NbtType::Byte,
      ExpressionKind::String(_) => NbtType::String,
      ExpressionKind::SubString(_, _, _) => NbtType::String,
      ExpressionKind::Array { .. } => NbtType::List,
      ExpressionKind::ByteArray(_) => NbtType::ByteArray,
      ExpressionKind::IntArray(_) => NbtType::IntArray,
      ExpressionKind::LongArray(_) => NbtType::LongArray,
      ExpressionKind::Compound(_) => NbtType::Compound,
      ExpressionKind::Macro(_) => NbtType::Unknown,
      ExpressionKind::Condition(_) => NbtType::Byte,
    }
  }

  pub fn compile_time_known(&self) -> bool {
    match self {
      ExpressionKind::Void
      | ExpressionKind::Byte(_)
      | ExpressionKind::Short(_)
      | ExpressionKind::Integer(_)
      | ExpressionKind::Long(_)
      | ExpressionKind::Float(_)
      | ExpressionKind::Double(_)
      | ExpressionKind::Boolean(_)
      | ExpressionKind::String(_) => true,
      ExpressionKind::Array { values, .. }
      | ExpressionKind::ByteArray(values)
      | ExpressionKind::IntArray(values)
      | ExpressionKind::LongArray(values) => values.iter().all(|e| e.kind.compile_time_known()),
      ExpressionKind::Compound(map) => map.iter().all(|(_, e)| e.kind.compile_time_known()),
      ExpressionKind::Storage(_)
      | ExpressionKind::SubString(_, _, _)
      | ExpressionKind::Scoreboard(_)
      | ExpressionKind::Condition(_)
      | ExpressionKind::Macro(_) => false,
    }
  }

  /*pub fn comptime_compatible(&self, top_level: bool) -> bool {
    match self {
      ExpressionKind::Void => false,
      ExpressionKind::Byte(_)
      | ExpressionKind::Short(_)
      | ExpressionKind::Integer(_)
      | ExpressionKind::Long(_)
      | ExpressionKind::Float(_)
      | ExpressionKind::Double(_)
      | ExpressionKind::Boolean(_)
      | ExpressionKind::String(_) => true,
      ExpressionKind::Array { values, .. }
      | ExpressionKind::ByteArray(values)
      | ExpressionKind::IntArray(values)
      | ExpressionKind::LongArray(values) => {
        values.iter().all(|v| v.kind.comptime_compatible(false))
      }
      ExpressionKind::Compound(map) => map.values().all(|v| v.kind.comptime_compatible(false)),
      ExpressionKind::SubString(_, _, _) => false,
      ExpressionKind::Storage(_) |
      ExpressionKind::Scoreboard(_) |
      ExpressionKind::Condition(_) => top_level,
      ExpressionKind::Macro(_) => false,
    }
  }*/

  pub fn to_comptime_string(&self, top_level: bool) -> Option<EcoString> {
    Some(match self {
      ExpressionKind::Void => return None,
      ExpressionKind::Byte(b) => eco_format!("{b}b"),
      ExpressionKind::Short(s) => eco_format!("{s}s"),
      ExpressionKind::Integer(i) => i.to_eco_string(),
      ExpressionKind::Long(l) => eco_format!("{l}l"),
      ExpressionKind::Float(f) => eco_format!("{f}f"),
      ExpressionKind::Double(d) => eco_format!("{d}d"),
      ExpressionKind::Boolean(b) => b.to_eco_string(),
      ExpressionKind::String(s) => s.clone(),
      ExpressionKind::Array { values, .. } => return array_to_string(values, ""),
      ExpressionKind::ByteArray(values) => return array_to_string(values, "B; "),
      ExpressionKind::IntArray(values) => return array_to_string(values, "I; "),
      ExpressionKind::LongArray(values) => return array_to_string(values, "L; "),
      ExpressionKind::Compound(values) => {
        let value_strings: Vec<_> = values
          .iter()
          .filter_map(|(key, value)| {
            value
              .kind
              .to_comptime_string(false)
              .map(|s| eco_format!("{key}: {s}"))
          })
          .collect();

        if value_strings.len() != values.len() {
          return None;
        }
        eco_format!("{{{}}}", value_strings.join(", "))
      }
      ExpressionKind::Storage(storage) => {
        if top_level {
          storage.to_eco_string()
        } else {
          return None;
        }
      }
      ExpressionKind::SubString(_, _, _) => return None,
      ExpressionKind::Scoreboard(scoreboard) => {
        if top_level {
          scoreboard.to_eco_string()
        } else {
          return None;
        }
      }
      ExpressionKind::Macro(_) => return None,
      ExpressionKind::Condition(c) => {
        if top_level {
          c.to_eco_string()
        } else {
          return None;
        }
      }
    })
  }
}

fn array_to_string(values: &[Expression], prefix: &str) -> Option<EcoString> {
  let value_strings: Vec<_> = values
    .iter()
    .filter_map(|value| value.kind.to_comptime_string(false))
    .collect();

  if value_strings.len() != values.len() {
    return None;
  }
  Some(eco_format!("[{prefix}{}]", value_strings.join(", ")))
}

impl Expression {
  pub fn equal(&self, other: &Self) -> Option<bool> {
    Some(match (&self.kind, &other.kind) {
      (ExpressionKind::Void, ExpressionKind::Void) => true,
      (ExpressionKind::Byte(l0), ExpressionKind::Byte(r0)) => l0 == r0,
      (ExpressionKind::Short(l0), ExpressionKind::Short(r0)) => l0 == r0,
      (ExpressionKind::Integer(l0), ExpressionKind::Integer(r0)) => l0 == r0,
      (ExpressionKind::Long(l0), ExpressionKind::Long(r0)) => l0 == r0,
      (ExpressionKind::Float(l0), ExpressionKind::Float(r0)) => l0 == r0,
      (ExpressionKind::Double(l0), ExpressionKind::Double(r0)) => l0 == r0,
      (ExpressionKind::Boolean(l0), ExpressionKind::Boolean(r0)) => l0 == r0,
      (ExpressionKind::String(l0), ExpressionKind::String(r0)) => l0 == r0,
      (
        ExpressionKind::Array {
          values: l_values, ..
        },
        ExpressionKind::Array {
          values: r_values, ..
        },
      ) => return compare_expr_array(l_values, r_values),

      (ExpressionKind::ByteArray(l0), ExpressionKind::ByteArray(r0)) => {
        return compare_expr_array(l0, r0)
      }
      (ExpressionKind::IntArray(l0), ExpressionKind::IntArray(r0)) => {
        return compare_expr_array(l0, r0)
      }
      (ExpressionKind::LongArray(l0), ExpressionKind::LongArray(r0)) => {
        return compare_expr_array(l0, r0)
      }
      (ExpressionKind::Compound(l0), ExpressionKind::Compound(r0)) => {
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
      (ExpressionKind::Scoreboard(_), other) | (other, ExpressionKind::Scoreboard(_))
        if !other.to_type().is_numeric() =>
      {
        false
      }
      (ExpressionKind::Storage(_), _)
      | (_, ExpressionKind::Storage(_))
      | (ExpressionKind::Scoreboard(_), _)
      | (_, ExpressionKind::Scoreboard(_))
      | (ExpressionKind::Condition(_), _)
      | (_, ExpressionKind::Condition(_)) => return None,
      _ => false,
    })
  }
}

fn compare_expr_array(l_values: &[Expression], r_values: &[Expression]) -> Option<bool> {
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

// TODO: Know the storage we are writing to, so we don't need a temp variable
fn array_to_storage(
  elements: &[Expression],
  data_type: NbtType,
  prefix: &str,
  state: &mut Compiler,
  code: &mut Vec<EcoString>,
  namespace: &str,
) -> Result<(EcoString, StorageKind)> {
  let storage = state.next_storage(namespace).to_eco_string();

  let mut constant_elements = Vec::new();
  let mut computed_elements_code = Vec::new();

  for (i, element) in elements.iter().enumerate() {
    if let Some(value) = element.kind.to_comptime_string(false) {
      constant_elements.push(value);
      continue;
    }

    match element.to_storage(state, &mut computed_elements_code, namespace)? {
      (expr_code, StorageKind::Modify) => {
        computed_elements_code.push(eco_format!(
          "data modify storage {storage} insert {i} {expr_code}"
        ));
      }
      (expr_code, StorageKind::MacroModify) => {
        computed_elements_code.push(eco_format!(
          "$data modify storage {storage} insert {i} {expr_code}"
        ));
      }
      (expr_code, StorageKind::Store) => {
        let temp_storage = state.next_storage(namespace).to_eco_string();
        computed_elements_code.push(eco_format!(
          "execute store result storage {temp_storage} {data_type} 1 run {expr_code}",
          data_type = data_type
            .to_store_string()
            .expect("Only numeric types have an indirect storage kind")
        ));
        computed_elements_code.push(eco_format!(
          "data modify storage {storage} insert {i} from storage {temp_storage}"
        ));
      }
      (expr_code, StorageKind::MacroStore) => {
        let temp_storage = state.next_storage(namespace).to_eco_string();
        computed_elements_code.push(eco_format!(
          "$execute store result storage {temp_storage} {data_type} 1 run {expr_code}",
          data_type = data_type
            .to_store_string()
            .expect("Only numeric types have an indirect storage kind")
        ));
        computed_elements_code.push(eco_format!(
          "data modify storage {storage} insert {i} from storage {temp_storage}"
        ));
      }
    }
  }

  code.push(eco_format!(
    "data modify storage {storage} set value [{prefix}{elements}]",
    elements = constant_elements.join(", ")
  ));
  code.extend(computed_elements_code);
  Ok((eco_format!("from storage {storage}"), StorageKind::Modify))
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
  pub fn to_store_string(self) -> Option<EcoString> {
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
      .to_eco_string(),
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
    match (&typ.kind, single_type) {
      (ExpressionKind::Void, _) => {
        return Err(raise_error(
          typ.location.clone(),
          "Cannot use void as a value",
        ))
      }
      (typ, NbtType::Unknown) => single_type = typ.to_type(),
      (t, NbtType::Numeric) if t.to_type().is_numeric() => single_type = t.to_type(),
      (ExpressionKind::Byte(_), NbtType::Byte) => {}
      (ExpressionKind::Short(_), NbtType::Short) => {}
      (ExpressionKind::Integer(_), NbtType::Int) => {}
      (ExpressionKind::Long(_), NbtType::Long) => {}
      (ExpressionKind::Float(_), NbtType::Float) => {}
      (ExpressionKind::Double(_), NbtType::Double) => {}
      (ExpressionKind::Storage(_), _) => {}
      (ExpressionKind::Scoreboard(_), t) if t.is_numeric() => {}
      (ExpressionKind::Boolean(_), NbtType::Byte) => {}
      (ExpressionKind::String(_), NbtType::String) => {}
      (ExpressionKind::Array { .. }, NbtType::List) => {}
      (ExpressionKind::Condition(_), NbtType::Byte) => {}
      _ => return Err(raise_error(typ.location.clone(), message)),
    }
  }

  if single_type == NbtType::Numeric {
    single_type = NbtType::Int;
  }

  Ok(single_type)
}
