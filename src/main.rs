use clap::{self, Arg, Command};
mod compiler;
mod lexer;
mod parser;

use std::{
  fs::{self},
  path::Path,
};

use lexer::Lexer;

use crate::{compiler::Compiler, parser::Parser};

fn main() {
  let matches = Command::new("zog")
    .subcommand(Command::new("build").args([
      Arg::new("file").short('f').default_value("main.zog"),
      Arg::new("output").short('o').default_value("build"),
    ]))
    .subcommand(Command::new("init").arg(Arg::new("name")))
    .get_matches();

  if let Some(matches) = matches.subcommand_matches("build") {
    let file: &String = matches.get_one("file").unwrap();
    let output: &String = matches.get_one("output").unwrap();
    build(file, output);
  } else if let Some(matches) = matches.subcommand_matches("init") {
    let name = matches.get_one("name");
    if let Some(name) = name {
      init(name);
    } else {
      init(&String::new());
    }
  }
}

fn build(file: &String, output: &String) {
  let mut lexer = Lexer::new(file);
  let tokens = lexer.tokenise();

  // println!("{:#?}", tokens)

  let mut parser = Parser::new(tokens);
  let ast = parser.parse();

  let compiler = Compiler::new(ast);
  compiler.compile(output);
}

const DEFAULT_PROJECT: &str = r#"namespace $name {
  fn tick() {

  }

  fn load() {
    tellraw @a "Loaded $name"
  }
}
"#;

fn init(name: &String) {
  if name == "" {
    let dir = std::env::current_dir().unwrap();
    let current_dir = Path::new(&dir).file_name().unwrap().to_str().unwrap();
    let is_empty = dir.read_dir().unwrap().next().is_none();
    if !is_empty {
      println!("No init'ing projects in non-empty directories, naughty naughty!");
      return;
    }
    let contents = DEFAULT_PROJECT.replace("$name", current_dir);
    fs::write("main.zog", contents).unwrap();
  } else {
    fs::create_dir(name).unwrap();
    let contents = DEFAULT_PROJECT.replace("$name", name);
    fs::write(name.clone() + "/main.zog", contents).unwrap();
  }
}
