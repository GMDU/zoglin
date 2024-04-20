use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum Val {
  Null,
  Bool(bool),
  Number(f64),
  String(String),
  Array(Vec<Val>),
  Object(HashMap<String, Val>),
}

pub fn from_json5(text: &String) -> String {
  let map: Val = json5::from_str(&text).unwrap();
  return serde_json::to_string_pretty(&map).unwrap();
}
