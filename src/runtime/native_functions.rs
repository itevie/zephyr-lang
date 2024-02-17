//use std::collections::HashMap;

use crate::{errors::ZephyrError, lexer::location::Location};

use super::values::{to_array, Null, Number, RuntimeValue};

type R = Result<RuntimeValue, ZephyrError>;

pub struct CallOptions<'a> {
  pub args: &'a [RuntimeValue],
  pub location: Location,
}

pub fn print(options: CallOptions) -> R {
  for i in options.args {
    print!("{} ", i.stringify(true, true));
  }
  print!("\n");
  Ok(RuntimeValue::Null(Null {}))
}

pub fn write(options: CallOptions) -> R {
  match options.args {
    [RuntimeValue::StringValue(str)] => {
      print!("{}", str.value);
    }
    _ => unreachable!(),
  }
  Ok(RuntimeValue::Null(Null {}))
}

pub fn clear_console(_: CallOptions) -> R {
  print!("\x1B[2J\x1B[1;1H");
  Ok(RuntimeValue::Null(Null {}))
}

pub fn iter(options: CallOptions) -> R {
  if options.args.len() == 1 {
    Ok(to_array(options.args[0].iterate()?))
  } else {
    Err(ZephyrError::runtime(
      "Cannot iter provided args".to_string(),
      options.location,
    ))
  }
}

pub fn reverse(options: CallOptions) -> R {
  if options.args.len() == 1 {
    Ok(to_array({
      let mut args = options.args[0].iterate()?;
      args.reverse();
      args
    }))
  } else {
    Err(ZephyrError::runtime(
      "Cannot reverse provided args".to_string(),
      options.location,
    ))
  }
}

// ----- Time & Date -----
pub fn get_time_nanos(_: CallOptions) -> R {
  let time = std::time::SystemTime::now();
  Ok(RuntimeValue::Number(Number {
    value: time
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_nanos() as f64,
  }))
}

// ----- Math -----
pub fn floor(options: CallOptions) -> R {
  match options.args {
    [RuntimeValue::Number(num)] => Ok(RuntimeValue::Number(Number {
      value: num.value.floor(),
    })),
    _ => Err(ZephyrError::runtime(
      "Invalid args".to_string(),
      options.location,
    )),
  }
}

pub fn ceil(options: CallOptions) -> R {
  match options.args {
    [RuntimeValue::Number(num)] => Ok(RuntimeValue::Number(Number {
      value: num.value.ceil(),
    })),
    _ => Err(ZephyrError::runtime(
      "Invalid args".to_string(),
      options.location,
    )),
  }
}

// ----- Network -----
/*pub fn response_to_object(result: reqwest::blocking::Response) -> R {
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
*/
