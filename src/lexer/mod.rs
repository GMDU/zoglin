mod commands;
pub mod token;
use crate::error::{raise_error, raise_warning, Location, Result};

use self::commands::COMMANDS;
use glob::glob;
use std::{collections::HashSet, fs, path::Path};
use token::{Token, TokenKind};

pub struct Lexer {
  file: String,
  root: String,
  src: String,
  pub dependent_files: HashSet<String>,
  position: usize,
  is_newline: bool,
  next_brace_json: bool,
  line: usize,
  column: usize,
  include_chain: Vec<String>,
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
  (";", TokenKind::Semicolon),
  (",", TokenKind::Comma),
  ("+", TokenKind::Plus),
  ("-", TokenKind::Minus),
  ("*", TokenKind::Star),
  ("%", TokenKind::Percent),
  ("**", TokenKind::DoubleStar),
  ("<<", TokenKind::LeftShift),
  (">>", TokenKind::RightShift),
  ("<", TokenKind::LessThan),
  (">", TokenKind::GreaterThan),
  ("<=", TokenKind::LessThanEquals),
  (">=", TokenKind::GreaterThanEquals),
  ("==", TokenKind::DoubleEquals),
  ("!=", TokenKind::BangEquals),
  ("&&", TokenKind::DoubleAmpersand),
  ("||", TokenKind::DoublePipe),
  ("!", TokenKind::Bang),
  ("=", TokenKind::Equals),
  ("+=", TokenKind::PlusEquals),
  ("-=", TokenKind::MinusEquals),
  ("*=", TokenKind::StarEquals),
  ("/=", TokenKind::ForwardSlashEquals),
  ("%=", TokenKind::PercentEquals),
];

static KEYWORD_REGISTRY: &[(&str, TokenKind)] = &[
  ("namespace", TokenKind::NamespaceKeyword),
  ("module", TokenKind::ModuleKeyword),
  ("fn", TokenKind::FunctionKeyword),
  ("res", TokenKind::ResourceKeyword),
  ("asset", TokenKind::AssetKeyword),
  ("include", TokenKind::IncludeKeyword),
  ("import", TokenKind::ImportKeyword),
  ("as", TokenKind::AsKeyword),
  ("if", TokenKind::IfKeyword),
  ("else", TokenKind::ElseKeyword),
  ("true", TokenKind::TrueKeyword),
  ("false", TokenKind::FalseKeyword),
];

impl Lexer {
  pub fn new(file: &str) -> Lexer {
    let contents = fs::read_to_string(file).unwrap();
    Lexer {
      file: file.to_string(),
      root: file.to_string(),
      src: contents,
      position: 0,
      is_newline: true,
      next_brace_json: false,
      line: 1,
      column: 1,
      dependent_files: HashSet::new(),
      include_chain: vec![file.to_string()],
    }
  }

  fn child(file: &str, root_path: &str, mut include_chain: Vec<String>) -> Lexer {
    include_chain.push(file.to_string());
    let contents = fs::read_to_string(file).unwrap();
    Lexer {
      file: file.to_string(),
      root: root_path.to_string(),
      src: contents,
      position: 0,
      is_newline: true,
      next_brace_json: false,
      line: 1,
      column: 1,
      dependent_files: HashSet::new(),
      include_chain: include_chain,
    }
  }

  pub fn tokenise(&mut self) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    self.dependent_files.insert(self.file.clone());
    loop {
      let next = self.next_token()?;
      if next.kind == TokenKind::IncludeKeyword {
        tokens.extend(self.parse_include()?);
      } else if next.kind == TokenKind::CommandBegin {
        tokens.push(next);
        tokens.extend(self.parse_command()?);
      } else {
        tokens.push(next);
        if tokens.last().unwrap().kind == TokenKind::EndOfFile {
          break;
        }
      }
    }
    Ok(tokens)
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

  fn next_token(&mut self) -> Result<Token> {
    self.skip_whitespace();

    let kind;
    let position = self.position;
    let line = self.line;
    let column = self.column;
    let mut value = String::new();

    if self.current() == '\0' {
      kind = TokenKind::EndOfFile;
      value.push('\0');
    } else if self.current() == '{' && self.next_brace_json {
      kind = TokenKind::JSON;
      self.next_brace_json = false;
      if !self.tokenise_json() {
        value = self.src[position + 1..self.position - 1].to_string();
      }
    } else if self.current() == '/' && self.is_newline {
      self.consume();
      kind = TokenKind::CommandBegin;
    } else if self.current() == '#' {
      while !self.current_is_delim() {
        self.consume();
      }
      kind = TokenKind::Comment;
    } else if let Some(punctuation) = self.parse_punctuation() {
      kind = punctuation;
    } else if self.current().is_digit(10) {
      (kind, value) = self.parse_number();
    } else if self.current() == '"' || self.current() == '\'' {
      kind = TokenKind::String;
      value = self.tokenise_string();
      self.next_brace_json = false;
    } else if valid_identifier_start(self.current()) {
      kind = self.tokenise_identifier(position);
    } else {
      return Err(raise_error(
        self.location(line, column),
        &format!("Unexpected character: {}", self.current()),
      ));
    }

    if kind == TokenKind::ResourceKeyword || kind == TokenKind::AssetKeyword {
      self.next_brace_json = true;
    }

    self.is_newline = false;
    if value.len() == 0 {
      value = self.src[position..self.position].to_string();
    }

    return Ok(Token {
      kind,
      value,
      location: self.location(line, column),
    });
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
    let identifier_value: &str = &self.src[position..self.position];
    let keyword = KEYWORD_REGISTRY
      .iter()
      .find(|(text, _)| *text == identifier_value);
    if keyword.is_some() {
      kind = keyword.unwrap().1.clone();
    } else if self.is_newline
      && COMMANDS.contains(&identifier_value)
      && self.next_significant_char() != '('
    {
      kind = TokenKind::CommandBegin;
      self.position = position;
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

  fn parse_number(&mut self) -> (TokenKind, String) {
    let mut kind = TokenKind::Integer;
    let mut str_value = String::new();

    while self.current().is_digit(10) {
      str_value.push(self.consume());
    }

    match self.current() {
      'b' | 'B' => {
        self.consume();
        kind = TokenKind::Byte
      }
      's' | 'S' => {
        self.consume();
        kind = TokenKind::Short
      }
      'l' | 'L' => {
        self.consume();
        kind = TokenKind::Long
      }
      'f' | 'F' => {
        self.consume();
        kind = TokenKind::Float;
      }
      'd' | 'D' => {
        self.consume();
        kind = TokenKind::Double
      }
      '.' => {
        str_value.push(self.consume());
        kind = TokenKind::Double;

        while self.current().is_digit(10) {
          str_value.push(self.consume());
        }
        match self.current() {
          'f' | 'F' => {
            self.consume();
            kind = TokenKind::Float
          }
          'd' | 'D' => {
            self.consume();
          }
          _ => {}
        }
      }

      _ => {}
    }

    (kind, str_value)
  }

  fn parse_include(&mut self) -> Result<Vec<Token>> {
    let token = self.next_token()?;

    if token.kind != TokenKind::String {
      return Err(raise_error(token.location, "Expected file name."));
    }

    let mut path: String = token.value;
    if !path.ends_with(".zog") {
      path.push_str(".zog");
    }
    let relative_path = if path.starts_with('/') {
      Path::new(&self.root).parent().unwrap().join(&path[1..])
    } else {
      Path::new(&self.file).parent().unwrap().join(path)
    };
    let mut tokens = Vec::new();

    for entry in glob(relative_path.to_str().unwrap()).unwrap() {
      match entry {
        Ok(path) => {
          let path_str = path.to_str().unwrap();
          let path_string = path_str.to_string();
          if let Some(index) = self
            .include_chain
            .iter()
            .position(|file| &path_string == file)
          {
            if index != (self.include_chain.len() - 1) {
              raise_warning(
                token.location.clone(),
                "Circular dependency detected, not including file.",
              );
            }
            continue;
          }
          self.dependent_files.insert(path_string);

          let mut lexer = Lexer::child(path_str, &self.root, self.include_chain.clone());

          tokens.extend(lexer.tokenise()?);
          self.dependent_files.extend(lexer.dependent_files);
          tokens.last_mut().unwrap().kind = TokenKind::EndOfInclude;
        }
        Err(e) => {
          return Err(raise_error(token.location, &e.to_string()));
        }
      }
    }
    Ok(tokens)
  }

  fn parse_command(&mut self) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();

    let mut current_part = String::new();
    let mut line = self.line;
    let mut column = self.column;

    while !self.current_is_delim() {
      if self.current() == '\\' && self.peek(1) == '&' {
        self.consume();
        current_part.push(self.current());
        self.consume();
        continue;
      }

      if self.current() == '&' && self.peek(1) == '{' {
        tokens.push(Token {
          kind: TokenKind::CommandString,
          value: current_part,
          location: self.location(line, column),
        });
        current_part = String::new();

        self.consume();
        self.consume();
        let mut brace_level = 0;
        while self.current() != '}' || brace_level > 0 {
          let next = self.next_token()?;
          if next.kind == TokenKind::LeftBrace {
            brace_level += 1;
          } else if next.kind == TokenKind::RightBrace {
            brace_level -= 1;
          }
          tokens.push(next);
        }
        self.consume();

        line = self.line;
        column = self.column;
        continue;
      }

      current_part.push(self.consume());
    }

    tokens.push(Token {
      kind: TokenKind::CommandString,
      value: current_part,
      location: self.location(line, column),
    });
    tokens.push(Token {
      kind: TokenKind::CommandEnd,
      value: String::new(),
      location: self.location(self.line, self.column),
    });

    Ok(tokens)
  }

  fn location(&self, line: usize, column: usize) -> Location {
    Location {
      file: self.file.clone(),
      root: self.root.clone(),
      line,
      column,
    }
  }
}

fn valid_identifier_start(character: char) -> bool {
  character.is_ascii_alphabetic() || character == '_'
}

fn valid_identifier_body(character: char) -> bool {
  character.is_ascii_alphanumeric() || character == '_'
}
