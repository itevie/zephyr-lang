//use std::collections::HashMap;
use std::io::Write;

use crate::{errors::ZephyrError, lexer::location::Location};

use super::values::{to_array, Null, Number, RuntimeValue, StringValue};

type R = Result<RuntimeValue, ZephyrError>;

pub struct CallOptions<'a> {
  pub args: &'a [RuntimeValue],
  pub location: Location,
}

pub fn unescape(options: CallOptions) -> R {
  match options.args {
    [RuntimeValue::StringValue(str)] => {
      let val = str.value.clone();
      let mut result = String::new();
      let chars = &mut val.chars();

      while let Some(c) = (*chars).next() {
        if c == '\\' {
          if let Some(escaped_char) = chars.next() {
            match escaped_char {
              'x' => {
                // Handle hexadecimal escape sequence
                let hex_digits: String = chars.take(2).collect();
                if let Ok(value) = u8::from_str_radix(&hex_digits, 16) {
                  result.push(value as char);
                } else {
                  result.push_str("\\x");
                  result.push_str(&hex_digits);
                }
              }
              // Add more cases for other escape sequences as needed
              _ => result.push(escaped_char),
            }
          } else {
            // If '\\' is at the end of the string
            result.push('\\');
          }
        } else {
          result.push(c);
        }
      }

      Ok(RuntimeValue::StringValue(StringValue { value: result }))
    }
    _ => Err(ZephyrError::runtime(
      "Invalid args".to_string(),
      options.location,
    )),
  }
}

pub fn print(options: CallOptions) -> R {
  for i in options.args {
    print!("{} ", i.stringify(true, true));
  }
  print!("\n");
  Ok(RuntimeValue::Null(Null {}))
}

pub fn read_line(options: CallOptions) -> R {
  let question = match options.args {
    [RuntimeValue::StringValue(ref str)] => str.value.clone(),
    _ => "".to_string(),
  };

  // Do stuff
  print!("{}", question);
  std::io::stdout().flush().unwrap();
  let mut input = String::new();
  let _ = std::io::stdin().read_line(&mut input);

  // Return answer
  Ok(RuntimeValue::StringValue(StringValue {
    value: input.replace("\n", "").replace("\r", ""),
  }))
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
