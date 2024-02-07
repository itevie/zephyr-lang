use std::collections::HashMap;

use crate::{errors::ZephyrError, lexer::location::Location};

use super::values::{to_array, to_object, Null, Number, RuntimeValue, StringValue};

type R = Result<RuntimeValue, ZephyrError>;

pub fn print(args: &[RuntimeValue]) -> R {
  for i in args {
    print!("{} ", i.stringify(true, true));
  }
  print!("\n");
  Ok(RuntimeValue::Null(Null {}))
}

pub fn iter(args: &[RuntimeValue]) -> R {
  if args.len() == 1 {
    Ok(to_array(args[0].iterate()?))
  } else {
    Err(ZephyrError::runtime(
      "Cannot iter provided args".to_string(),
      Location::no_location(),
    ))
  }
}

pub fn reverse(args: &[RuntimeValue]) -> R {
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

// ----- Network -----
pub fn response_to_object(result: reqwest::blocking::Response) -> R {
  let result_status = result.status().as_u16();

  Ok(to_object(HashMap::from([
    (
      "text".to_string(),
      match result.text() {
        Ok(ok) => RuntimeValue::StringValue(StringValue { value: ok }),
        Err(_) => {
          return Err(ZephyrError::runtime(
            "Failed to parse http response".to_string(),
            Location::no_location(),
          ))
        }
      },
    ),
    (
      "status_code".to_string(),
      RuntimeValue::Number(Number {
        value: result_status as f64,
      }),
    ),
  ])))
}

pub fn http_get(args: &[RuntimeValue]) -> R {
  match args {
    [RuntimeValue::StringValue(ref url)] => {
      let result = match reqwest::blocking::get(url.value.clone()) {
        Ok(ok) => ok,
        Err(err) => {
          return Err(ZephyrError::runtime(
            err.to_string(),
            Location::no_location(),
          ))
        }
      };

      Ok(response_to_object(result)?)
    }
    _ => Err(ZephyrError::runtime(
      "Cannot handle provided args".to_string(),
      Location::no_location(),
    )),
  }
}
