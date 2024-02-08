use std::io::ErrorKind;

use once_cell::sync::Lazy;
use runtime::{interpreter::Interpreter, memory::Memory};
use structopt::StructOpt;

#[path = "./repl.rs"]
mod repl;

//use std::io::Write;

pub mod errors;
pub mod lexer;
pub mod parser;
pub mod runtime;
pub mod util;

#[derive(StructOpt, Debug, Clone)]
pub struct Args {
  #[structopt(
    long = "file",
    empty_values = false,
    short = "f",
    value_name = "PATH_FLAG"
  )]
  pub file_flag: Option<String>,

  #[structopt(
    value_name = "PATH",
    empty_values = false,
    conflicts_with = "file-flag"
  )]
  pub file_pos: Option<String>,

  #[structopt(
    long = "directory",
    empty_values = false,
    short = "dir",
    value_name = "WORKING_DIRECTORY"
  )]
  pub directory: Option<String>,

  #[structopt(long = "debug", value_name = "DEBUG_MODE")]
  pub debug_mode: Option<bool>,
}

static mut MEMORY: Lazy<Memory> = Lazy::new(|| Memory::new());
static ARGS: Lazy<Args> = Lazy::new(|| Args::from_args());

pub fn debug(contents: &str, what: &str) {
  if let Some(debug_mode) = ARGS.debug_mode {
    if debug_mode {
      println!("[DEBUG:{}]: {}", what, contents);
    }
  }
}

fn main() {
  let args = ARGS.clone();

  // Get the directory to run in
  let dir = if let Some(directory) = args.directory.clone() {
    let mut path = std::path::PathBuf::new();
    path.push(&directory);
    path
  } else {
    std::env::current_dir().unwrap()
  };

  // Check if the provided dir exists
  if dir.exists() == false {
    return die(format!(
      "The directory ({}) provided with --directory does not exist",
      dir.display()
    ));
  }

  debug(
    &format!(
      "The current directory is set to: {}",
      dir.display().to_string()
    ),
    "main",
  );

  // Check if should run in repl mode
  if matches!(args.file_flag, None) && matches!(args.file_pos, None) {
    repl::repl(args, dir.display().to_string());
  } else {
    // Collect the file
    let file_name = &if let Some(f) = args.file_flag {
      f
    } else if let Some(f) = args.file_pos {
      f
    } else {
      panic!()
    };

    let input = match std::fs::read_to_string(file_name) {
      Ok(ok) => ok,
      Err(err) => {
        return die(match err.kind() {
          ErrorKind::NotFound => format!("File {} does not exist", file_name),
          ErrorKind::PermissionDenied => format!("Failed to read {}: permission denied", file_name),
          _ => format!("Failed to open {}: {}", file_name, err),
        })
      }
    };
    let mut interpreter = Interpreter::new(dir.display().to_string());

    let result = match lexer::lexer::lex(input) {
      Ok(val) => val,
      Err(err) => {
        println!("{}", err.visualise(false));
        return;
      }
    };

    let mut parser = parser::parser::Parser::new(result);
    let ast = match parser.produce_ast() {
      Ok(val) => val,
      Err(err) => {
        println!("{}", err.visualise(false));
        return;
      }
    };

    let value = interpreter.evaluate(parser::nodes::Expression::Program(ast));
    match value {
      Err(err) => println!("{}", err.visualise(false)),
      Ok(_) => return,
    }
  }
}

fn die(err: String) -> () {
  println!("Fatal Error: {}", err);
}
