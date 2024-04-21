use crate::{
  lexer::{lexer::get_location_contents, location::Location},
  runtime::values::RuntimeValue,
  util::{self},
};
use std::cmp::max;

const REPL_OFFSET: usize = 2;

#[derive(Debug, Clone)]
pub struct ZephyrError {
  pub location: Location,
  pub error_message: String,
  pub error_type: ErrorType,
  pub reference: Option<Location>,
}

#[derive(Debug, Clone)]
pub enum ErrorType {
  Runtime,
  Break(Option<String>),
  Return(Box<RuntimeValue>),
  Continue(Option<String>),
  Parser,
  Lexer,
  UserDefined(Box<RuntimeValue>),
}

impl ZephyrError {
  pub fn runtime(message: String, location: Location) -> ZephyrError {
    ZephyrError {
      error_message: message,
      error_type: ErrorType::Runtime,
      reference: None,
      location,
    }
  }

  pub fn runtime_with_ref(message: String, location: Location, reference: Location) -> ZephyrError {
    ZephyrError {
      error_message: message,
      error_type: ErrorType::Runtime,
      reference: Some(reference),
      location,
    }
  }

  pub fn parser(message: String, location: Location) -> ZephyrError {
    ZephyrError {
      error_message: message,
      error_type: ErrorType::Parser,
      reference: None,
      location,
    }
  }

  pub fn parser_with_ref(message: String, location: Location, reference: Location) -> ZephyrError {
    ZephyrError {
      error_message: message,
      error_type: ErrorType::Parser,
      reference: Some(reference),
      location,
    }
  }

  pub fn lexer(message: String, location: Location) -> ZephyrError {
    ZephyrError {
      error_message: message,
      error_type: ErrorType::Lexer,
      reference: None,
      location,
    }
  }

  pub fn lexer_with_ref(message: String, location: Location, refer: Location) -> ZephyrError {
    ZephyrError {
      error_message: message,
      error_type: ErrorType::Lexer,
      reference: Some(refer),
      location,
    }
  }
  pub fn visualise(&self, _is_repl: bool) -> String {
    let mut result = String::new();

    // Add error
    let error_type = match self.error_type {
      ErrorType::UserDefined(ref _a) => "Userdefined".to_string(),
      _ => format!("{:?}", self.error_type),
    };
    result += &(util::colors::fg_red()
      + &format!("{} error: {}\n", error_type, self.error_message)
      + &util::colors::reset());

    // Add location
    result += &(ZephyrError::visualise_location(self.location, false, false));

    // Check if it has a reference
    if let Some(reference) = &self.reference {
      result += "\nIn reference to:\n";
      result += &(ZephyrError::visualise_location(*reference, false, true))
    }

    result
  }

  pub fn visualise_old(&self, is_repl: bool) -> String {
    format!(
      "{}\n{:?}: {}{}",
      ZephyrError::visualise_location_old(self.location, is_repl, false),
      self.error_type,
      self.error_message,
      if let Some(reference) = &self.reference {
        format!(
          "\n\n{}",
          ZephyrError::visualise_location_old(*reference, false, true)
        )
      } else {
        "".to_string()
      }
    )
  }

  pub fn visualise_location(location: Location, is_repl: bool, _is_ref: bool) -> String {
    let mut result = String::new();

    // Add file name and things
    let location_contents = &get_location_contents(location.location_contents);
    result += &(util::colors::fg_cyan()
      + &format!(
        "  -> {}:{}:{}",
        location_contents.file_name, location.line, location.char_start
      )
      + &util::colors::reset());

    // Add contents
    result += "\n\n  ";
    result += location_contents
      .contents
      .replace('\t', " ")
      .split('\n')
      .collect::<Vec<&str>>()[location.line as usize];

    // Calculate how long the arrow will be
    let offset = max(0, location.char_start);
    let arrow_length = max(location.char_end - location.char_start, 1);

    // Add arrow
    result += &("\n  ".to_string()
      + &(" ".repeat((if is_repl { REPL_OFFSET } else { 0 }) + offset as usize)));
    result +=
      &(util::colors::fg_red() + &("^".repeat(arrow_length as usize)) + &util::colors::reset());

    result
  }

  pub fn visualise_location_old(location: Location, is_repl: bool, is_ref: bool) -> String {
    let mut result = String::new();

    // Check if should add the contents
    if !is_repl {
      result += &(util::colors::fg_red()
        + &format!("{} at [char ", if is_ref { "Reference" } else { "Error" }));

      if location.char_end - location.char_start > 1 {
        result += &format!("{}-{}", location.char_start, location.char_end);
      } else {
        result += &format!("{}", location.char_start);
      }

      let location_contents = &get_location_contents(location.location_contents);
      result += &(format!(
        " line {} in {}]\n\n",
        location.line, location_contents.file_name
      ) + &util::colors::reset());

      result += location_contents
        .contents
        .replace('\t', " ")
        .split('\n')
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
