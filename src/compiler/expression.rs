use super::file_tree::{ScoreboardLocation, StorageLocation};

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
  pub(super) fn to_storage(&self) -> (String, StorageKind) {
    match self {
      ExpressionType::Void => panic!("Cannot assign void to a value"),
      ExpressionType::Byte(b) => (format!("value {}b", *b), StorageKind::Direct),
      ExpressionType::Short(s) => (format!("value {}s", *s), StorageKind::Direct),
      ExpressionType::Integer(i) => (format!("value {}", *i), StorageKind::Direct),
      ExpressionType::Long(l) => (format!("value {}l", *l), StorageKind::Direct),
      ExpressionType::Float(f) => (format!("value {}f", *f), StorageKind::Direct),
      ExpressionType::Double(d) => (format!("value {}d", *d), StorageKind::Direct),
      ExpressionType::Boolean(b) => (format!("value {}", *b), StorageKind::Direct),
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
