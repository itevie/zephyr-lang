// Majority of the functions are assumed to be type-checked within the lib
// so the given argument's types are usually trusted to be correct

use std::{
  collections::HashMap,
  fs,
  io::{self, Read, Write},
  net::TcpStream,
  path::PathBuf,
  sync::{atomic::Ordering, Arc, Mutex},
  time::Duration,
};

use rand::Rng;
use regex::Regex;

use crate::{
  errors::ZephyrError,
  lexer::location::Location,
  util::{
    self,
    json::{json_to_zephyr_object, zephyr_to_json},
  },
};

use super::{
  interpreter::Interpreter,
  values::{
    Array, NativeFunction2, Null, Number, Object, ObjectContainer, RuntimeValue, StringValue,
  },
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
        RuntimeValue::StringValue(StringValue { value: message }),
      ),
    ]),
  });

  Ok(RuntimeValue::ObjectContainer(ObjectContainer {
    location: crate::MEMORY.lock().unwrap().add_value(obj),
  }))
}

pub fn get_args(_: CallOptions) -> R {
  Ok(
    Array::make(
      crate::ZEPHYR_ARGS
        .read()
        .unwrap()
        .iter()
        .map(|v| {
          Box::from(RuntimeValue::StringValue(StringValue {
            value: v.to_string(),
          }))
        })
        .collect(),
    )
    .create_container(),
  )
}

// ----- Console -----
pub fn print(options: CallOptions) -> R {
  for i in options.args {
    println!("{} ", i.stringify(false, true, Some(options.interpreter)));
  }
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
    value: input.replace(['\n', '\r'], ""),
  }))
}

pub fn write(options: CallOptions) -> R {
  match &options.args[..] {
    [val] => {
      print!("{}", val.stringify(true, true, Some(options.interpreter)));
    }
    _ => unreachable!(),
  }
  Ok(RuntimeValue::Null(Null {}))
}

pub fn clear_console(_: CallOptions) -> R {
  print!("\x1B[2J\x1B[1;1H");
  Ok(RuntimeValue::Null(Null {}))
}

// ----- String Manipulation -----
pub fn str_to_number(options: CallOptions) -> R {
  match &options.args[..] {
    [RuntimeValue::StringValue(str)] => {
      let val: f64 = str.value.parse().unwrap();
      return Ok(RuntimeValue::Number(Number { value: val }));
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

pub fn slice(options: CallOptions) -> R {
  match &options.args[..] {
    [RuntimeValue::StringValue(string), RuntimeValue::Number(start), RuntimeValue::Number(end)] => {
      Ok(StringValue::make(
        (&string.value[(start.value as usize)..(end.value as usize)]).to_string(),
      ))
    }
    _ => Err(ZephyrError::runtime(
      "Invalid args".to_string(),
      options.location,
    )),
  }
}

// ----- JSON -----
pub fn json_parse(options: CallOptions) -> R {
  match &options.args[..] {
    [RuntimeValue::StringValue(value)] => match json_to_zephyr_object(&value.value) {
      Err(mut err) => {
        err.location = options.location;
        Err(err)
      }
      Ok(ok) => Ok(ok),
    },
    _ => Err(ZephyrError::runtime(
      "Invalid args".to_string(),
      options.location,
    )),
  }
}

pub fn zephyr_to_json_n(options: CallOptions) -> R {
  match &options.args[..] {
    [RuntimeValue::ObjectContainer(value)] => match zephyr_to_json(value.clone()) {
      Err(mut err) => {
        err.location = options.location;
        Err(err)
      }
      Ok(ok) => Ok(StringValue::make(ok)),
    },
    _ => Err(ZephyrError::runtime(
      "Invalid args".to_string(),
      options.location,
    )),
  }
}

// ----- Regex -----
pub fn rg_is_match(options: CallOptions) -> R {
  match &options.args[..] {
    [RuntimeValue::StringValue(what), RuntimeValue::StringValue(reg)] => {
      let re = match Regex::new(&reg.value) {
        Ok(ok) => ok,
        Err(err) => {
          return Err(ZephyrError::runtime(
            format!("Failed to compile regular expression: {:?}", err),
            options.location,
          ))
        }
      };

      let captures: Vec<Box<RuntimeValue>> = re
        .find_iter(&what.value)
        .filter_map(|f| Some(f.as_str().to_string()))
        .collect::<Vec<String>>()
        .iter()
        .map(|x| Box::from(StringValue::make(x.to_string())))
        .collect::<Vec<Box<RuntimeValue>>>();

      Ok(Array::make(captures).create_container())
    }
    _ => Err(ZephyrError::runtime(
      "Invalid args".to_string(),
      options.location,
    )),
  }
}

pub fn rg_replace(options: CallOptions) -> R {
  match &options.args[..] {
    [RuntimeValue::StringValue(what), RuntimeValue::StringValue(reg), RuntimeValue::StringValue(with), RuntimeValue::Number(amount)] =>
    {
      let re = match Regex::new(&reg.value) {
        Ok(ok) => ok,
        Err(err) => {
          return Err(ZephyrError::runtime(
            format!("Failed to compile regular expression: {:?}", err),
            options.location,
          ))
        }
      };

      let new = re.replacen(&what.value, amount.value as usize, with.value.clone());

      Ok(StringValue::make(new.to_string()))
    }
    _ => Err(ZephyrError::runtime(
      "Invalid args".to_string(),
      options.location,
    )),
  }
}

// ----- Functions -----
pub fn call_zephyr_function(options: CallOptions) -> R {
  match &options.args[..] {
    [RuntimeValue::Function(func), RuntimeValue::ArrayContainer(array_container)] => {
      let arr = array_container.deref();
      options.interpreter.clone().evaluate_zephyr_function(
        func.clone(),
        arr.items,
        options.location,
      )
    }
    _ => Err(ZephyrError::runtime(
      "Invalid args".to_string(),
      options.location,
    )),
  }
}

// ----- Arrays -----
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

pub fn arr_ref_set(options: CallOptions) -> R {
  match &options.args[..] {
    [RuntimeValue::ArrayContainer(arr), RuntimeValue::ArrayContainer(new)] => {
      let narr = new.deref();
      crate::MEMORY
        .lock()
        .unwrap()
        .set_value(arr.location, RuntimeValue::Array(narr))?;
      Ok(RuntimeValue::ArrayContainer(arr.clone()))
    }
    _ => Err(ZephyrError::runtime(
      "Invalid args".to_string(),
      options.location,
    )),
  }
}

pub fn reverse(options: CallOptions) -> R {
  if options.args.len() == 1 {
    Ok(
      Array::make({
        let mut args = options.args[0].iterate()?;
        args.reverse();
        args
      })
      .create_container(),
    )
  } else {
    Err(ZephyrError::runtime(
      "Cannot reverse provided args".to_string(),
      options.location,
    ))
  }
}

pub fn iter(options: CallOptions) -> R {
  if options.args.len() == 1 {
    Ok(Array::make(options.args[0].iterate()?).create_container())
  } else {
    Err(ZephyrError::runtime(
      "Cannot iter provided args".to_string(),
      options.location,
    ))
  }
}

// ----- Buffers -----
pub fn buff_to_utf8(options: CallOptions) -> R {
  match &options.args[..] {
    [RuntimeValue::ArrayContainer(arr)] => {
      let arr = arr.deref();
      let bytes = arr
        .items
        .iter()
        .map(|x| match *x.clone() {
          RuntimeValue::Number(num) => num.value as u8,
          _ => panic!(),
        })
        .collect::<Vec<u8>>();

      return Ok(StringValue::make(String::from_utf8(bytes).unwrap()));
    }
    _ => {
      return Err(ZephyrError::runtime(
        "Invalid args".to_string(),
        options.location,
      ))
    }
  }
}

pub fn utf8_to_buff(options: CallOptions) -> R {
  match &options.args[..] {
    [RuntimeValue::StringValue(str)] => {
      let bytes = str
        .value
        .as_bytes()
        .iter()
        .map(|x| Box::from(RuntimeValue::Number(Number { value: *x as f64 })))
        .collect::<Vec<Box<RuntimeValue>>>();
      return Ok(Array::make(bytes).create_container());
    }
    _ => {
      return Err(ZephyrError::runtime(
        "Invalid args".to_string(),
        options.location,
      ))
    }
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

// ----- Random -----
pub fn random(_: CallOptions) -> R {
  Ok(RuntimeValue::Number(Number {
    value: rand::random(),
  }))
}

pub fn random_range(options: CallOptions) -> R {
  match &options.args[..] {
    [RuntimeValue::Number(min), RuntimeValue::Number(max)] => Ok(RuntimeValue::Number(Number {
      value: rand::thread_rng().gen_range(min.value..max.value),
    })),
    _ => Err(ZephyrError::runtime(
      "Invalid args".to_string(),
      options.location,
    )),
  }
}

pub fn random_item(options: CallOptions) -> R {
  match &options.args[..] {
    [RuntimeValue::ArrayContainer(array)] => {
      let items = match crate::MEMORY.lock().unwrap().get_value(array.location)? {
        RuntimeValue::Array(arr) => arr,
        _ => unreachable!(),
      };

      let index = rand::thread_rng().gen_range(0..(items.items.len()));
      Ok(*items.items[index].clone())
    }
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
        crate::debug(
          &format!(
            "Thread initiated, there are now {} thread(s), this thread's ID is: {:?}",
            crate::GLOBAL_THREAD_COUNT.load(Ordering::SeqCst),
            std::thread::current().id(),
          ),
          "threading",
        );
        op.interpreter
          .clone()
          .evaluate_zephyr_function(func, vec![], op.location)
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
    [RuntimeValue::StringValue(str)] => {
      // Construct the path
      let mut loc = PathBuf::from(options.interpreter.scope.clone().get_directory()?);
      loc.push(str.value.clone());
      let _thing = util::path_resolver::resolve(
        PathBuf::from(options.interpreter.scope.clone().get_directory().unwrap()),
        &str.value,
      );

      // Read and check if it was ok
      match fs::read_to_string(loc.display().to_string()) {
        Ok(ok) => Ok(RuntimeValue::StringValue(StringValue { value: ok })),
        Err(e) => Err(ZephyrError::runtime(
          format!("Failed to read file {}: {}", str.value, e),
          Location::no_location(),
        )),
      }
    }
    _ => Err(ZephyrError::runtime(
      "Invalid args".to_string(),
      options.location,
    )),
  }
}

// ----- Network -----
pub fn rust_lambda_test(o: CallOptions) -> R {
  let address = match &o.args[..] {
    [RuntimeValue::StringValue(str)] => &str.value,
    _ => {
      return Err(ZephyrError::runtime(
        "Expected an address in arguments".to_string(),
        o.location,
      ))
    }
  };

  // Connect to the TCP server
  let stream_result = TcpStream::connect(address);
  if let Err(err) = stream_result {
    return Err(ZephyrError::runtime(
      format!("Failed to connect to TCP server: {:?}", err.to_string()),
      o.location,
    ));
  }

  // Create the arc mutex of the ok stream
  let stream_pre_arc = stream_result.unwrap();
  stream_pre_arc
    .set_read_timeout(Some(Duration::from_millis(100)))
    .unwrap();

  let stream = Arc::from(Mutex::from(stream_pre_arc));

  // Create write lambda
  let write: Arc<(dyn Fn(CallOptions) -> R + Send + Sync)> = {
    let stream = Arc::clone(&stream);
    Arc::from(move |options: CallOptions| {
      // Check the arguments
      match &options.args[..] {
        // Expects integer array
        [RuntimeValue::ArrayContainer(container)] => {
          let array = container.deref();
          let mut bytes: Vec<u8> = vec![];

          // Loop through the given array
          for item in array.items {
            if !matches!(*item, RuntimeValue::Number(_)) {
              return Err(ZephyrError::runtime(
                "All items of buffer must be integer".to_string(),
                options.location,
              ));
            }

            bytes.push(match *item {
              RuntimeValue::Number(num) => num.value as u8,
              _ => unreachable!(),
            })
          }

          // Send the packet
          return match stream.lock().unwrap().write(&bytes) {
            Ok(_) => Ok(Null::make()),
            Err(err) => Err(ZephyrError::runtime(
              format!("Failed to send message to TCP server: {}", err),
              options.location,
            )),
          };
        }
        _ => {
          return Err(ZephyrError::runtime(
            "Expected a buffer".to_string(),
            options.location,
          ))
        }
      }
    })
  };

  let read: Arc<(dyn Fn(CallOptions) -> R + Send + Sync)> = {
    let stream = Arc::clone(&stream);
    Arc::from(move |options: CallOptions| {
      let bytes: &mut [u8; 128] = &mut [0; 128];
      let value = match { stream.lock().unwrap().read(bytes) } {
        Ok(0) => {
          return Err(ZephyrError::runtime(
            "The TCP connection has closed".to_string(),
            options.location,
          ));
        }
        Ok(ok) => ok,
        Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => return Ok(Null::make()),
        Err(err) => {
          return Err(ZephyrError::runtime(
            format!("Failed to read from TCP stream: {:?}", err.to_string()),
            options.location,
          ));
        }
      };

      let mut vec_bytes = Vec::from(bytes);
      vec_bytes.truncate(value);

      let arr = Array::make(
        vec_bytes
          .iter()
          .map(|x| Box::from(RuntimeValue::Number(Number { value: *x as f64 })))
          .collect::<Vec<Box<RuntimeValue>>>(),
      )
      .create_container();
      Ok(arr)
    })
  };

  Ok(
    Object::make(HashMap::from([
      (
        "write".to_string(),
        RuntimeValue::NativeFunction2(NativeFunction2 { func: write }),
      ),
      (
        "read".to_string(),
        RuntimeValue::NativeFunction2(NativeFunction2 { func: read }),
      ),
    ]))
    .create_container(),
  )
}
