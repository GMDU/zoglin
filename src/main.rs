use clap::{self, Arg, Command};
mod compiler;
mod error;
mod lexer;
mod parser;

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
    let file: &String = matches.get_one("file").unwrap();
    let output: &String = matches.get_one("output").unwrap();
    let debug_mode: &String = matches.get_one("debug_mode").unwrap();
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
    let file: &String = matches.get_one("file").unwrap();
    let output: &String = matches.get_one("output").unwrap();
    watch(file, output);
  }
}

fn build(file: &String, output: &String, debug_mode: &str) -> (HashSet<String>, Result<()>) {
  print!("Building {} into {}... ", file, output);
  let start = SystemTime::now();
  let mut lexer = Lexer::new(file);
  let result = lexer.tokenise();
  let tokens = match result {
    Ok(tokens) => tokens,
    Err(e) => return (lexer.dependent_files, Err(e)),
  };

  if debug_mode == "tokens" {
    println!(
      "Read tokens in {}ms",
      SystemTime::now().duration_since(start).unwrap().as_millis()
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
      SystemTime::now().duration_since(start).unwrap().as_millis()
    );
    println!("{:#?}", ast);
    return (lexer.dependent_files, Ok(()));
  }

  let compiler = Compiler::new(ast);
  compiler.compile(output);
  println!(
    "Built in {}ms",
    SystemTime::now().duration_since(start).unwrap().as_millis()
  );
  return (lexer.dependent_files, Ok(()));
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
      if !Path::new(name).exists() {
        continue;
      }
      let modified = fs::metadata(name)
        .and_then(|metadata| metadata.modified())
        .unwrap();
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

fn get_modification_times(files: HashSet<String>) -> HashMap<String, SystemTime> {
  files
    .into_iter()
    .map(|name| {
      let time = fs::metadata(&name).unwrap().modified().unwrap();
      (name, time)
    })
    .collect()
}
