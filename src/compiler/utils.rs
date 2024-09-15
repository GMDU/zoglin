use std::fmt::Display;

use ecow::{eco_format, EcoString};

pub trait ToEcoString {
  fn to_eco_string(&self) -> EcoString;
}

impl<T: Display + ?Sized> ToEcoString for T {
  fn to_eco_string(&self) -> EcoString {
    eco_format!("{}", self)
  }
}
