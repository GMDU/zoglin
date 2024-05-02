use std::process::exit;

#[derive(Debug, Clone)]
pub struct Location {
  pub line: usize,
  pub column: usize,
  pub file: String,
}

const RED: &str = "\x1b[31m";

pub fn raise_error(location: &Location, message: &str) -> ! {
  eprintln!(
    "{}:{}:{}: {}{}",
    location.file, location.line, location.column, RED, message
  );
  exit(1);
}
