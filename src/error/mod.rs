#[derive(Debug, Clone)]
pub struct Location {
  pub line: usize,
  pub column: usize,
  pub file: String,
  pub root: String,
}

impl Location {
  pub fn blank() -> Location {
    Location {
      line: 0,
      column: 0,
      file: String::new(),
      root: String::new(),
    }
  }
}

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";

#[derive(Debug)]
pub struct Error {
  location: Option<Location>,
  message: String,
}

impl Error {
  pub fn print(&self) {
    if let Some(ref location) = self.location {
      eprintln!(
        "{}:{}:{}: {}{}{}",
        location.file, location.line, location.column, RED, self.message, RESET
      );
    } else {
      eprintln!("Error: {}{}{}", RED, self.message, RESET);
    }
  }
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn raise_error(location: Location, message: impl ToString) -> Error {
  Error {
    location: Some(location),
    message: message.to_string(),
  }
}

pub fn raise_floating_error(message: impl ToString) -> Error {
  Error {
    location: None,
    message: message.to_string(),
  }
}

pub fn raise_warning(location: Location, message: impl ToString) {
  eprintln!(
    "{}:{}:{}: {}{}{}",
    location.file,
    location.line,
    location.column,
    YELLOW,
    message.to_string(),
    RESET
  );
}
