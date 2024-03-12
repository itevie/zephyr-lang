//use std::collections::HashMap;
use std::{
  collections::HashMap,
  fs,
  io::Write,
  sync::{atomic::Ordering, Arc},
};

use crate::{errors::ZephyrError, lexer::location::Location};

use super::{
  interpreter::Interpreter,
  values::{to_array, Null, Number, Object, ObjectContainer, RuntimeValue, StringValue},
};

type R = Result<RuntimeValue, ZephyrError>;

#[derive(Clone)]
pub struct CallOptions {
  pub args: Vec<RuntimeValue>,
  pub location: Location,
  pub interpreter: Interpreter,
}

pub fn error(options: CallOptions) -> R {
  let message: String;
  let mut data: RuntimeValue = RuntimeValue::Null(Null {});

  match &options.args[..] {
    [RuntimeValue::StringValue(str)] => message = str.value.clone(),
    [RuntimeValue::StringValue(str), d] => {
      message = str.value.clone();
      data = d.clone();
    }
    _ => {
      return Err(ZephyrError::runtime(
        "Invalid args".to_string(),
        options.location,
      ))
    }
  }

  let obj = RuntimeValue::Object(Object {
    items: HashMap::from([
      (
        "message".to_string(),
        RuntimeValue::StringValue(StringValue {
          value: message.clone(),
        }),
      ),
      ("data".to_string(), data),
      (
        "type".to_string(),
        RuntimeValue::StringValue(StringValue {
          value: message.clone(),
        }),
      ),
    ]),
  });

  Ok(RuntimeValue::ObjectContainer(ObjectContainer {
    location: crate::MEMORY.lock().unwrap().add_value(obj),
  }))
}

pub fn push_arr(options: CallOptions) -> R {
  match &options.args[..] {
    [RuntimeValue::ArrayContainer(container), value] => {
      let mut array = match crate::MEMORY
        .lock()
        .unwrap()
        .get_value(container.location)?
      {
        RuntimeValue::Array(arr) => arr,
        _ => unreachable!(),
      };

      array.items.push(Box::from(value.clone()));

      // Modify the value
      crate::MEMORY
        .lock()
        .unwrap()
        .set_value(container.location, RuntimeValue::Array(array))?;

      Ok(RuntimeValue::ArrayContainer(container.clone()))
    }
    _ => Err(ZephyrError::runtime(
      "Invalid args".to_string(),
      options.location,
    )),
  }
}

pub fn unescape(options: CallOptions) -> R {
  match &options.args[..] {
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
  let question = match options.args[..] {
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
  match &options.args[..] {
    [val] => {
      print!("{}", val.stringify(true, true));
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
  match &options.args[..] {
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
  match &options.args[..] {
    [RuntimeValue::Number(num)] => Ok(RuntimeValue::Number(Number {
      value: num.value.ceil(),
    })),
    _ => Err(ZephyrError::runtime(
      "Invalid args".to_string(),
      options.location,
    )),
  }
}

// ----- Threads -----
pub fn spawn_thread(options: CallOptions) -> R {
  match &options.args[..] {
    [RuntimeValue::Function(func)] => {
      let op = Arc::new(options.clone());
      let func = func.clone();
      crate::GLOBAL_THREAD_COUNT.fetch_add(1, Ordering::SeqCst);
      std::thread::spawn(move || {
        op.interpreter
          .clone()
          .evaluate_zephyr_function(func, vec![], op.location.clone())
          .unwrap();
        crate::GLOBAL_THREAD_COUNT.fetch_sub(1, Ordering::SeqCst);
      });
      Ok(RuntimeValue::Null(Null {}))
    }
    _ => Err(ZephyrError::runtime(
      "Invalid args".to_string(),
      options.location,
    )),
  }
}

// ----- File Management -----
pub fn read_file(options: CallOptions) -> R {
  match &options.args[..] {
    [RuntimeValue::StringValue(str)] => match fs::read_to_string(str.value.clone()) {
      Ok(ok) => Ok(RuntimeValue::StringValue(StringValue { value: ok })),
      Err(e) => Err(ZephyrError::runtime(
        format!("Failed to read file {}: {}", str.value, e),
        Location::no_location(),
      )),
    },
    _ => Err(ZephyrError::runtime(
      "Invalid args".to_string(),
      options.location,
    )),
  }
}

// ----- Network -----
pub fn ureq_to_object(response: ureq::Response) -> R {
  // Create object
  let object = Object {
    items: HashMap::from([
      (
        "status_text".to_string(),
        RuntimeValue::StringValue(StringValue {
          value: response.status_text().to_string(),
        }),
      ),
      (
        "status".to_string(),
        RuntimeValue::Number(Number {
          value: response.status() as f64,
        }),
      ),
      (
        "contents".to_string(),
        match response.into_string() {
          Ok(ok) => RuntimeValue::StringValue(StringValue { value: ok }),
          _ => {
            return Err(ZephyrError::runtime(
              "Failed to parse response".to_string(),
              Location::no_location(),
            ))
          }
        },
      ),
    ]),
  };
  Ok(RuntimeValue::ObjectContainer(
    super::values::ObjectContainer {
      location: crate::MEMORY
        .lock()
        .unwrap()
        .add_value(RuntimeValue::Object(object)),
    },
  ))
}

pub fn http_get(options: CallOptions) -> R {
  match &options.args[..] {
    [RuntimeValue::StringValue(url), RuntimeValue::ObjectContainer(headers)] => {
      let request = ureq::get(&url.value);

      // Add headers
      let _object = match crate::MEMORY.lock().unwrap().get_value(headers.location)? {
        RuntimeValue::Object(obj) => obj,
        _ => unreachable!(),
      };
      /*for i in object.items {
        request.set(
          &i.0,
          match i.1 {
            RuntimeValue::StringValue(ref str) => &str.value,
            _ => unreachable!(),
          },
        );
      }*/

      let result = match request.call() {
        Ok(ok) => ureq_to_object(ok),
        Err(ureq::Error::Status(_, response)) => ureq_to_object(response),
        _ => unreachable!(),
      };

      result
    }
    _ => Err(ZephyrError::runtime(
      "Invalid args".to_string(),
      options.location,
    )),
  }
}

// Don't know rust enough for this shit
/*pub fn open_ws(options: CallOptions) -> R {
  let location = &options.location;
  let interpreter = Arc::from(Mutex::from(options.interpreter));
  match &options.args[..] {
    [RuntimeValue::StringValue(str), RuntimeValue::Function(func)] => {
      let msg_func = Arc::from(Mutex::from(0 as u128));
      thread::scope(|s| {
        let options = options.clone();

        s.spawn(move || {
          ws::connect(str.value.clone(), |_out| {
            fn test(_options: CallOptions) -> R {
              Ok(RuntimeValue::Null(Null {}))
            }

            let x: dyn Fn() = || {} as dyn Fn();

            let value = options
              .clone()
              .interpreter
              .evaluate_zephyr_function(
                func.clone(),
                vec![Box::from(RuntimeValue::NativeFunction(NativeFunction {
                  func: &test,
                }))],
                options.location,
              )
              .unwrap();
            *msg_func.lock().unwrap() = crate::MEMORY.lock().unwrap().add_value(value);
            |msg: ws::Message| {
              let func = match crate::MEMORY
                .lock()
                .unwrap()
                .get_value(msg_func.lock().unwrap().clone())
                .unwrap()
              {
                RuntimeValue::Function(func) => {
                  println!("{:?}", func);
                  func
                }
                _ => panic!("Expected a function to run"),
              };
              interpreter
                .lock()
                .unwrap()
                .evaluate_zephyr_function(
                  func,
                  vec![Box::from(RuntimeValue::StringValue(StringValue {
                    value: msg.to_string(),
                  }))],
                  location.clone(),
                )
                .unwrap();
              Ok(())
            }
          })
          .unwrap();
        });
      });
    }
    _ => {
      return Err(ZephyrError::runtime(
        "Invalid args".to_string(),
        options.location,
      ))
    }
  };
  Ok(RuntimeValue::Null(Null {}))
}*/

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
