use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::{raise_error, Error, Location};

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

pub fn from_json5(text: &str, location: Location) -> Result<String, Error> {
  let map: Val = json5::from_str(text).map_err(|e| raise_error(location, e))?;
  Ok(serde_json::to_string_pretty(&map).expect("Json is valid, it was just parsed"))
}
