use clap::{self, Arg, Command};
mod compiler;
mod config;
mod error;
mod lexer;
mod parser;

use ecow::EcoString;
use error::Result;
use std::{
  collections::{HashMap, HashSet},
  fs,
  path::Path,
  process::exit,
  thread,
  time::{Duration, SystemTime},
};

use lexer::Lexer;

use crate::{compiler::Compiler, parser::Parser};

fn main() {
  let matches = Command::new("zog")
    .subcommand(Command::new("build").args([
      Arg::new("file").short('f').default_value("main.zog"),
      Arg::new("output").short('o').default_value("build"),
      Arg::new("debug_mode").long("debug").default_value("none"),
    ]))
    .subcommand(Command::new("init").arg(Arg::new("name")))
    .subcommand(Command::new("watch").args([
      Arg::new("file").short('f').default_value("main.zog"),
      Arg::new("output").short('o').default_value("build"),
    ]))
    .get_matches();

  if let Some(matches) = matches.subcommand_matches("build") {
    let file: &String = matches
      .get_one("file")
      .expect("Argument has a default value");
    let output: &String = matches
      .get_one("output")
      .expect("Argument has a default value");
    let debug_mode: &String = matches
      .get_one("debug_mode")
      .expect("Argument has a default value");
    if let Err(e) = build(file, output, debug_mode).1 {
      e.print();
      exit(1);
    }
  } else if let Some(matches) = matches.subcommand_matches("init") {
    let name = matches.get_one("name");
    if let Some(name) = name {
      init(name);
    } else {
      init(&String::new());
    }
  } else if let Some(matches) = matches.subcommand_matches("watch") {
    let file: &String = matches
      .get_one("file")
      .expect("Argument has a default value");
    let output: &String = matches
      .get_one("output")
      .expect("Argument has a default value");
    watch(file, output);
  }
}

fn build(file: &String, output: &String, debug_mode: &str) -> (HashSet<EcoString>, Result<()>) {
  print!("Building {} into {}... ", file, output);
  let start = SystemTime::now();
  let result = Lexer::new(file);
  let mut lexer = match result {
    Ok(lexer) => lexer,
    Err(e) => return (HashSet::new(), Err(e)),
  };
  let result = lexer.tokenise();
  let tokens = match result {
    Ok(tokens) => tokens,
    Err(e) => return (lexer.dependent_files, Err(e)),
  };

  if debug_mode == "tokens" {
    println!(
      "Read tokens in {}ms",
      SystemTime::now()
        .duration_since(start)
        .expect("Now is always later than previously")
        .as_millis()
    );
    println!("{:#?}", tokens);
    return (lexer.dependent_files, Ok(()));
  }

  let mut parser = Parser::new(tokens);
  let result = parser.parse();
  let ast = match result {
    Ok(ast) => ast,
    Err(e) => return (lexer.dependent_files, Err(e)),
  };

  if debug_mode == "ast" {
    println!(
      "Parsed AST in {}ms",
      SystemTime::now()
        .duration_since(start)
        .expect("Now is always later than previously")
        .as_millis()
    );
    println!("{:#?}", ast);
    return (lexer.dependent_files, Ok(()));
  }

  if let Err(e) = Compiler::compile(ast, output) {
    return (lexer.dependent_files, Err(e));
  }

  println!(
    "Built in {}ms",
    SystemTime::now()
      .duration_since(start)
      .expect("Now is always later than previously")
      .as_millis()
  );
  (lexer.dependent_files, Ok(()))
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
  if name.is_empty() {
    let dir = std::env::current_dir().expect("Current directory should be valid");
    let current_dir = Path::new(&dir)
      .file_name()
      .expect("Current directory cannot end in ..")
      .to_str()
      .expect("Path should be valid");
    let is_empty = dir
      .read_dir()
      .expect("Directory should be readable")
      .next()
      .is_none();
    if !is_empty {
      println!("No init'ing projects in non-empty directories, naughty naughty!");
      return;
    }
    let contents = DEFAULT_PROJECT.replace("$name", current_dir);
    fs::write("main.zog", contents).expect("Directory should be writable");
  } else {
    fs::create_dir(name).expect("Directory should be writable");
    let contents = DEFAULT_PROJECT.replace("$name", name);
    fs::write(name.clone() + "/main.zog", contents).expect("Directory should be writable");
  }
}

fn watch(file: &String, output: &String) {
  let (dep_files, result) = build(file, output, "none");
  if let Err(e) = result {
    e.print();
  }
  let mut files: HashMap<_, _> = get_modification_times(dep_files);
  loop {
    thread::sleep(Duration::from_secs(1));
    let mut dependent_files = None;
    for (name, last_modified) in files.iter() {
      if !Path::new(name.as_str()).exists() {
        continue;
      }
      let modified = fs::metadata(name.as_str())
        .and_then(|metadata| metadata.modified())
        .expect("Path must be valid and readable");
      if &modified != last_modified {
        let (dep_files, result) = build(file, output, "none");
        if let Err(e) = result {
          e.print();
        }
        dependent_files = Some(dep_files);
        break;
      }
    }

    if let Some(dep_files) = dependent_files {
      files = get_modification_times(dep_files);
    }
  }
}

fn get_modification_times(files: HashSet<EcoString>) -> HashMap<EcoString, SystemTime> {
  files
    .into_iter()
    .map(|name| {
      let time = fs::metadata(name.as_str())
        .and_then(|meta| meta.modified())
        .expect("Path must be valid and readable");
      (name, time)
    })
    .collect()
}
