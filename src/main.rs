#![allow(unused_variables)]
#![allow(dead_code)]

use errors::ZephyrError;
use lexer::lexer::lex;
use parser::Parser;
use runtime::{memory_store, values::RuntimeValue, Interpreter};
use std::fs;

mod errors;
mod lexer;
mod parser;
mod runtime;
mod util;

fn main() {
    memory_store::initialise_store();

    println!(
        "{}",
        match run(
            &fs::read_to_string("/home/isabella/Documents/projects/rust/zephyr/test.zr").unwrap()
        ) {
            Ok(ok) => match ok.to_string() {
                Ok(ok) => ok,
                Err(err) => err.visualise(),
            },
            Err(err) => err.visualise(),
        }
    );
}

fn run(stuff: &str) -> Result<RuntimeValue, ZephyrError> {
    let result = lex(
        &stuff,
        String::from("/home/isabella/Documents/projects/rust/zephyr/test.zr"),
    )?;
    let parsed = Parser::new(
        result,
        String::from("/home/isabella/Documents/projects/rust/zephyr/test.zr"),
    )
    .produce_ast()?;
    Interpreter::new().run(parsed)
}
