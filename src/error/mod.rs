#[derive(Debug, Clone)]
pub struct Location {
  pub line: usize,
  pub column: usize,
  pub file: String,
  pub root: String,
}

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";

pub struct Error {
  location: Location,
  message: String,
}

impl Error {
  pub fn print(&self) {
    eprintln!(
      "{}:{}:{}: {}{}{}",
      self.location.file, self.location.line, self.location.column, RED, self.message, RESET
    );
  }
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn raise_error(location: Location, message: &str) -> Error {
  Error {
    location: location,
    message: message.to_string(),
  }
}

pub fn raise_warning(location: Location, message: &str) {
  eprintln!(
    "{}:{}:{}: {}{}{}",
    location.file, location.line, location.column, YELLOW, message, RESET
  );
}
