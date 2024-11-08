#![allow(unused_variables)]
#![allow(dead_code)]

use std::fs;

use errors::ZephyrError;
use lexer::lexer::lex;
use parser::Parser;
use runtime::{memory_store, values::RuntimeValue, Interpreter};

mod errors;
mod lexer;
mod parser;
mod runtime;
mod util;

fn main() {
    memory_store::initialise_store();

    println!(
        "{}",
        match run("/home/isabella/Documents/projects/rust/zephyr/test.zr") {
            Ok(ok) => ok.to_string().unwrap(),
            Err(err) => err.visualise(),
        }
    );
}

fn run(file_path: &str) -> Result<RuntimeValue, ZephyrError> {
    let text = fs::read_to_string(file_path).unwrap();
    let result = lex(&text, String::from(file_path))?;
    let parsed = Parser::new(result, String::from(file_path)).produce_ast()?;
    Interpreter::new().run(parsed)
}
