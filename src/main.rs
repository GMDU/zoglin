mod lexer;
use std::fs::read_to_string;

use lexer::Lexer;
fn main() {
    let contents = read_to_string("input.zog").unwrap();
    let mut lexer = Lexer::new(&contents);
    let tokens = lexer.tokenise();
    println!("{:#?}", tokens);
}
