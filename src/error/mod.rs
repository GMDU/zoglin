use std::process::exit;

#[derive(Debug, Clone)]
pub struct Location {
  pub line: usize,
  pub column: usize,
  pub file: String,
}

pub fn raise_error(location: &Location, message: &str) {
  eprintln!("{}:{}:{}:\x1b[31m {}", location.file, location.line, location.column, message);
  exit(1);
}