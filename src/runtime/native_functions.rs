use crate::{errors::ZephyrError, lexer::location::Location};

use super::values::{to_array, Null, RuntimeValue};

pub fn print(args: &[RuntimeValue]) -> Result<RuntimeValue, ZephyrError> {
  for i in args {
    print!("{} ", i);
  }
  print!("\n");
  Ok(RuntimeValue::Null(Null {}))
}

pub fn iter(args: &[RuntimeValue]) -> Result<RuntimeValue, ZephyrError> {
  if args.len() == 1 {
    Ok(to_array(args[0].iterate()?))
  } else {
    Err(ZephyrError::runtime(
      "Cannot iter provided args".to_string(),
      Location::no_location(),
    ))
  }
}

pub fn reverse(args: &[RuntimeValue]) -> Result<RuntimeValue, ZephyrError> {
  if args.len() == 1 {
    Ok(to_array({
      let mut args = args[0].iterate()?;
      args.reverse();
      args
    }))
  } else {
    Err(ZephyrError::runtime(
      "Cannot reverse provided args".to_string(),
      Location::no_location(),
    ))
  }
}
