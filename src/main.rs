#![allow(dead_code)]

use errors::ZephyrError;
use lexer::lexer::lex;
use parser::Parser;
use runtime::{values::RuntimeValue, Interpreter};
use std::{env, fs, thread};
use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;
use std::sync::mpsc;
use std::time::Duration;
use crate::runtime::values::thread_crossing::{ThreadInnerValue, ThreadRuntimeValue};

mod errors;
mod lexer;
mod parser;
mod runtime;
mod util;

fn main() {
    let args = env::args().collect::<Vec<String>>();

    if let Some(file_name) = args.get(2) {
        println!(
            "{}",
            match run(&file_name) {
                Ok(ok) => ok.to_string(true, true, true).unwrap_or_else(|err| err.visualise()),

                Err(err) => err.visualise(),
            }
        );
    } else {
        let mut interpreter = Interpreter::new("<REPL>".to_string());
        loop {
            let mut line = String::new();
            std::io::stdin().read_line(&mut line).unwrap();

            let mut parser = Parser::new(
                match lex(&line, "<REPL>".to_string()) {
                    Ok(ok) => ok,
                    Err(err) => {
                        eprintln!("{}", err.visualise());
                        continue;
                    }
                },
                "<REPL>".to_string(),
            );
            let parsed = match parser.produce_ast() {
                Ok(ok) => ok,
                Err(err) => {
                    eprintln!("{}", err.visualise());
                    continue;
                }
            };
            match interpreter.run(parsed) {
                Ok(ok) => println!("{}", ok.to_string(true, true, true).unwrap()),
                Err(err) => eprintln!("{}", err.visualise()),
            }
        }
    }
}

fn run(file_name: &str) -> Result<RuntimeValue, ZephyrError> {
    let data = fs::read_to_string(file_name).unwrap();

    let result = lex(&data, file_name.to_string())?;
    let parsed = Parser::new(result, String::from(file_name.to_string())).produce_ast()?;
    Interpreter::new(fs::canonicalize(file_name).unwrap().display().to_string()).base_run(parsed)
}
