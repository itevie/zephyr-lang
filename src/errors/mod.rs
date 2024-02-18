use std::cmp::max;

use crate::{
  lexer::{lexer::get_location_contents, location::Location},
  runtime::values::RuntimeValue,
};

const REPL_OFFSET: usize = 2;

#[derive(Debug)]
pub struct ZephyrError {
  pub location: Location,
  pub error_message: String,
  pub error_type: ErrorType,
}

#[derive(Debug)]
pub enum ErrorType {
  Runtime,
  Break,
  Return(Box<RuntimeValue>),
  Continue,
  Parser,
  Lexer,
}

impl ZephyrError {
  pub fn runtime(message: String, location: Location) -> ZephyrError {
    ZephyrError {
      error_message: message,
      error_type: ErrorType::Runtime,
      location,
    }
  }

  pub fn parser(message: String, location: Location) -> ZephyrError {
    ZephyrError {
      error_message: message,
      error_type: ErrorType::Parser,
      location,
    }
  }

  pub fn lexer(message: String, location: Location) -> ZephyrError {
    ZephyrError {
      error_message: message,
      error_type: ErrorType::Lexer,
      location,
    }
  }

  pub fn visualise(&self, is_repl: bool) -> String {
    format!(
      "{}\n{:?}: {}",
      ZephyrError::visualise_location(self.location.clone(), is_repl),
      self.error_type,
      self.error_message
    )
  }

  pub fn visualise_location(location: Location, is_repl: bool) -> String {
    let mut result = String::new();

    // Check if should add the contents
    if !is_repl {
      result += "Error at [char ";

      if location.char_end - location.char_start > 1 {
        result += &format!("{}-{}", location.char_start, location.char_end);
      } else {
        result += &format!("{}", location.char_start);
      }

      result += &format!(" line {}]\n\n", location.line);

      result += (&get_location_contents(location.location_contents))
        .replace("\t", " ")
        .split("\n")
        .collect::<Vec<&str>>()[location.line as usize];
      result += "\n";
    }

    // Calculate how long the arrow will be
    let offset = max(0, location.char_start);
    let arrow_length = max(location.char_end - location.char_start, 1);

    // Add arrow
    result += &(" ".repeat((if is_repl { REPL_OFFSET } else { 0 }) + offset as usize));
    result += &("^".repeat(arrow_length as usize));

    // Done
    result
  }
}

macro_rules! runtime_error {
  ($message:expr) => {
    ZephyrError::runtime($message, Location::no_location())
  };
}
pub(crate) use runtime_error;

macro_rules! parser_error {
  ($message:expr, $location:expr) => {
    ZephyrError::parser($message, $location)
  };
}
pub(crate) use parser_error;

macro_rules! lexer_error {
  ($message:expr) => {
    ZephyrError::lexer($message, Location::no_location())
  };
}
pub(crate) use lexer_error;
