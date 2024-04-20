use clap::{self, Arg, Command};
mod compiler;
mod lexer;
mod parser;

use std::fs::read_to_string;

use lexer::Lexer;

use crate::{compiler::Compiler, parser::Parser};

fn main() {
  let matches = Command::new("zog")
    .subcommand(Command::new("build").args([
      Arg::new("file").short('f').default_value("main.zog"),
      Arg::new("output").short('o').default_value("build"),
    ]))
    .get_matches();

  if let Some(matches) = matches.subcommand_matches("build") {
    let file: &String = matches.get_one("file").unwrap();
    let output: &String = matches.get_one("output").unwrap();
    build(file, output);
  }
}

fn build(file: &String, output: &String) {
    let contents = read_to_string(file).unwrap();
    let mut lexer = Lexer::new(&contents);
    let tokens = lexer.tokenise();
  
    let mut parser = Parser::new(tokens);
    let ast = parser.parse();
  
    let compiler = Compiler::new(ast);
    compiler.compile(output);
}
