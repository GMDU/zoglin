mod commands;
pub mod token;
use std::{fs, path::Path};
use token::{Token, TokenKind};
use self::commands::COMMANDS;

pub struct Lexer {
  file: String,
  src: String,
  position: usize,
  is_newline: bool,
  next_brace_json: bool,
  line: usize,
  column: usize,
}

static OPERATOR_REGISTRY: &[(&str, TokenKind)] = &[
  ("(", TokenKind::LeftParen),
  (")", TokenKind::RightParen),
  ("[", TokenKind::LeftSquare),
  ("]", TokenKind::RightSquare),
  ("{", TokenKind::LeftBrace),
  ("}", TokenKind::RightBrace),
  ("/", TokenKind::ForwardSlash),
  (":", TokenKind::Colon),
  (".", TokenKind::Dot),
];

static KEYWORD_REGISTRY: &[(&str, TokenKind)] = &[
  ("namespace", TokenKind::NamespaceKeyword),
  ("module", TokenKind::ModuleKeyword),
  ("fn", TokenKind::FunctionKeyword),
  ("res", TokenKind::ResourceKeyword),
  ("include", TokenKind::IncludeKeyword),
];

impl Lexer {
  pub fn new(src: &str) -> Lexer {
    let contents = fs::read_to_string(src).unwrap();
    Lexer {
      file: src.to_string(),
      src: contents,
      position: 0,
      is_newline: true,
      next_brace_json: false,
      line: 1,
      column: 1,
    }
  }

  pub fn tokenise(&mut self) -> Vec<Token> {
    let mut tokens = Vec::new();
    loop {
      let next = self.next_token();
      if next.kind == TokenKind::IncludeKeyword {
        tokens.extend(self.parse_include());
        tokens.pop();
      } else {
        tokens.push(next);
        if tokens.last().unwrap().kind == TokenKind::EndOfFile {
          break;
        }
      }
    }
    tokens
  }

  fn peek(&self, offset: usize) -> char {
    self.src.chars().nth(self.position + offset).unwrap_or('\0')
  }

  fn current(&self) -> char {
    self.peek(0)
  }

  fn current_is_delim(&self) -> bool {
    self.current() == '\n' || self.current() == '\0'
  }

  fn next_token(&mut self) -> Token {
    self.skip_whitespace();

    let mut kind = TokenKind::Invalid;
    let mut position = self.position;
    let line = self.line;
    let column = self.column;
    let mut value = String::new();

    if self.current() == '\0' {
      return Token {
        kind: TokenKind::EndOfFile,
        value: "\0".to_string(),
        file: self.file.clone(),
        line,
        column,
      };
    } else if self.current() == '{' && self.next_brace_json {
      kind = TokenKind::JSON;
      if !self.tokenise_json() {
        value = self.src[position + 1..self.position - 1].to_string();
      }
    } else if self.current() == '/' && self.is_newline {
      self.consume();
      kind = TokenKind::Command;
      position += 1;
    } else if self.current() == '#' {
      while !self.current_is_delim() {
        self.consume();
      }
      kind = TokenKind::Comment;
    } else if let Some(punctuation) = self.parse_punctuation() {
      kind = punctuation;
    } else if self.current().is_digit(10) {
      kind = TokenKind::Integer;
      while self.current().is_digit(10) {
        self.consume();
      }
    } else if self.current() == '"' || self.current() == '\'' {
      kind = TokenKind::String;
      value = self.tokenise_string();
      self.next_brace_json = false;
    } else if valid_identifier_start(self.current()) {
      kind = self.tokenise_identifier(position);
    } else {
      self.consume();
    }

    if kind == TokenKind::Command {
      while !self.current_is_delim() {
        self.consume();
      }
    }

    if kind == TokenKind::ResourceKeyword {
      self.next_brace_json = true;
    }

    self.is_newline = false;
    if &value == "" {
      value = self.src[position..self.position].to_string();
    }
    return Token {
      kind,
      value: value,
      file: self.file.clone(),
      line,
      column,
    };
  }

  fn parse_punctuation(&mut self) -> Option<TokenKind> {
    let mut index = 0;
    let mut exact = None;
    let mut matches = Vec::from(OPERATOR_REGISTRY);
    loop {
      let current = self.peek(index);
      matches = matches
        .into_iter()
        .filter(|(str, kind)| {
          if str.len() <= index {
            return false;
          }
          let is_match = str.chars().nth(index).unwrap() == current;
          if is_match && str.len() == index + 1 {
            exact = Some(kind.clone());
          }
          is_match
        })
        .collect();

      if matches.len() == 0 {
        break;
      };

      index += 1;
    }
    if exact.is_some() {
      self.consume_many(index);
    }
    exact
  }

  fn tokenise_identifier(&mut self, position: usize) -> TokenKind {
    let mut kind = TokenKind::Identifier;
    while valid_identifier_body(self.current()) {
      self.consume();
    }
    let identifier_value = &self.src[position..self.position];
    let keyword = KEYWORD_REGISTRY
      .iter()
      .find(|(text, _)| *text == identifier_value);
    if keyword.is_some() {
      kind = keyword.unwrap().1.clone();
    } else if self.is_newline && COMMANDS.contains(&identifier_value) && self.next_significant_char() != '(' {
      kind = TokenKind::Command;
    }
    kind
  }

  fn skip_whitespace(&mut self) {
    while self.current().is_whitespace() {
      self.consume();
    }
  }

  fn next_significant_char(&mut self) -> char {
    let position = self.position;
    self.skip_whitespace();
    let current = self.current();
    self.position = position;
    current
  }

  fn consume(&mut self) -> char {
    self.column += 1;
    let current = self.current();
    if current == '\n' {
      self.is_newline = true;
      self.line += 1;
      self.column = 1;
    }
    self.position += 1;
    current
  }

  fn consume_many(&mut self, count: usize) {
    self.position += count;
  }

  fn tokenise_json(&mut self) -> bool {
    self.consume();
    self.skip_whitespace();
    let char = self.current();
    let mut include_braces = false;

    if char == '"' || char == '\'' {
      self.tokenise_string();
      self.skip_whitespace();
      include_braces = self.current() == ':';
    } else if char.is_alphabetic() {
      while self.current().is_alphanumeric() {
        self.consume();
      }
      self.skip_whitespace();
      include_braces = self.current() == ':';
    } else if char == '}' {
      include_braces = true;
    }

    let mut count = 1;

    while count > 0 {
      if self.current() == '{' {
        count += 1;
      }

      if self.current() == '}' {
        count -= 1;
      }

      if self.current() == '"' || self.current() == '\'' {
        self.tokenise_string();
      } else {
        self.consume();
      }
    }

    include_braces
  }

  fn tokenise_string(&mut self) -> String {
    let char = self.current();
    let mut string = String::new();
    self.consume();
    while self.current() != char {
      if self.current() == '\\' {
        self.consume();
      }
      string.push(self.consume());
    }
    self.consume();
    string
  }

  fn parse_include(&mut self) -> Vec<Token> {
    let token = self.next_token();
    assert_eq!(token.kind, TokenKind::String);
    let mut path: String = token.value;
    if !path.ends_with(".zog") {
      path = path + ".zog";
    }
    let relative_path = Path::new(&self.file).parent().unwrap().join(path);
    let mut lexer = Lexer::new(relative_path.to_str().unwrap());
    lexer.tokenise()
  }
}

fn valid_identifier_start(character: char) -> bool {
  character.is_ascii_alphabetic() || character == '_'
}

fn valid_identifier_body(character: char) -> bool {
  character.is_ascii_alphanumeric() || character == '_'
}
