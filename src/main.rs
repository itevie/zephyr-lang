#![allow(unused_variables)]
#![allow(dead_code)]

use errors::ZephyrError;
use lexer::lexer::lex;
use parser::Parser;
use runtime::{memory_store, values::RuntimeValue, Interpreter};
use std::{env, fs};

mod errors;
mod lexer;
mod parser;
mod runtime;
mod util;

fn main() {
    memory_store::initialise_store();

    let args = env::args().collect::<Vec<String>>();
    let file_name = args.get(3).ok_or_else(|| panic!("")).unwrap();

    println!(
        "{}",
        match run(&file_name) {
            Ok(ok) => match ok.to_string() {
                Ok(ok) => ok,
                Err(err) => err.visualise(),
            },

            Err(err) => err.visualise(),
        }
    );
}

fn run(file_name: &str) -> Result<RuntimeValue, ZephyrError> {
    let data = fs::read_to_string(file_name).unwrap();

    let result = lex(&data, file_name.to_string())?;
    let parsed = Parser::new(result, String::from(file_name.to_string())).produce_ast()?;
    Interpreter::new().run(parsed)
}
