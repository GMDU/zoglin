mod compiler;
mod lexer;
mod parser;

use std::fs::read_to_string;

use lexer::Lexer;

use crate::{compiler::Compiler, parser::Parser};

fn main() {
    let contents = read_to_string("input.zog").unwrap();
    let mut lexer = Lexer::new(&contents);
    let tokens = lexer.tokenise();

    let mut parser = Parser::new(tokens);
    let ast = parser.parse();

    let compiler = Compiler::new(ast);
    compiler.compile();

    // println!("{:#?}", ast);
}
