pub mod token;
use token::{Token, TokenKind};

pub struct Lexer {
  src: String,
  position: usize,
  is_newline: bool,
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
  ("execute", TokenKind::Command),
  ("effect", TokenKind::Command),
  ("time", TokenKind::Command),
  ("say", TokenKind::Command),
  ("give", TokenKind::Command),
];

impl Lexer {
  pub fn new(src: &str) -> Lexer {
    Lexer{src: src.to_string(), position: 0, is_newline: true}
  }

  pub fn tokenise(&mut self) -> Vec<Token> {
    let mut tokens = Vec::new();
    loop {
      let next = self.next_token();
      tokens.push(next);
      if tokens.last().unwrap().kind == TokenKind::EndOfFile {
        break;
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

  fn next_token(&mut self) -> Token {
    self.skip_whitespace();

    let mut kind = TokenKind::Invalid;
    let mut position = self.position;

    if self.current() == '\0' {
      return Token{kind: TokenKind::EndOfFile, value: "\0".to_string()};
    } else if self.current() == '/' && self.is_newline {
      self.consume();
      kind = TokenKind::Command;
      position += 1;
    } else if self.current() == '#' {
      while self.current() != '\n' {
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
    } else if valid_identifier_start(self.current()) {
      kind = TokenKind::Identifier;
      while valid_identifier_body(self.current()) {
        self.consume();
      }
      let identifier_value = &self.src[position..self.position];
      let keyword = KEYWORD_REGISTRY.iter().find(
        |(text, _)| *text == identifier_value
      );
      if keyword.is_some() {
        kind = keyword.unwrap().1.clone();
      }
    } else {
      self.consume();
    }

    if kind == TokenKind::Command {
      while self.current() != '\n' {
        self.consume();
      }
    }

    self.is_newline = false;
    return Token{kind, value: self.src[position..self.position].to_string()};
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
            return false
          }
          let is_match = str.chars().nth(index).unwrap() == current;
          if is_match && str.len() == index + 1 {
            exact = Some(kind.clone());
          }
          is_match
        }).collect();
        
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
  
  fn skip_whitespace(&mut self) {
    while self.current().is_whitespace() {
      self.consume();
    }
  }

  fn consume(&mut self) -> char {
    let current = self.current();
    if current == '\n' {
      self.is_newline = true;
    }
    self.position += 1;
    current
  }

  fn consume_many(&mut self, count: usize) {
    self.position += count;
  }
}

fn valid_identifier_start(character: char) -> bool {
  character.is_ascii_alphabetic() || character == '_'
}

fn valid_identifier_body(character: char) -> bool {
  character.is_ascii_alphanumeric() || character == '_'
}