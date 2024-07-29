use json_tools::{Lexer, Token, TokenType};
use std::collections::HashMap;

use crate::{
  errors::ZephyrError,
  lexer::location::Location,
  runtime::values::{self, Boolean, Null, ObjectContainer, RuntimeValue, StringValue},
};

macro_rules! gen_err {
  ($x: expr, $y: expr) => {
    Err(ZephyrError::runtime(
      format!(
        "{} at char {}-{}",
        $x,
        match &$y.buf {
          json_tools::Buffer::Span(span) => span.first,
          _ => unreachable!(),
        },
        match &$y.buf {
          json_tools::Buffer::Span(span) => span.end,
          _ => unreachable!(),
        }
      ),
      Location::no_location(),
    ))
  };
}
macro_rules! get_part {
  ($a: expr, $x: expr) => {
    match &$x.buf {
      json_tools::Buffer::Span(span) => {
        String::from_utf8(Vec::from(&$a[(span.first as usize)..(span.end as usize)])).unwrap()
      }
      _ => unreachable!(),
    }
  };
}

pub fn json_to_zephyr_object(data: &str) -> Result<RuntimeValue, ZephyrError> {
  let tokens = Lexer::new(data.bytes(), json_tools::BufferType::Span);
  let mut vec_tokens: Vec<Token> = vec![];
  for i in tokens {
    vec_tokens.push(i);
  }
  let result = parse_object(&mut vec_tokens, data.bytes().collect::<Vec<u8>>())?;

  Ok(result)
}

pub fn parse_object(tokens: &mut Vec<Token>, data: Vec<u8>) -> Result<RuntimeValue, ZephyrError> {
  if tokens.len() == 0 || tokens[0].kind != TokenType::CurlyOpen {
    if tokens.len() == 0 {
      return Err(ZephyrError::runtime(
        "Expected { for start of object".to_string(),
        Location::no_location(),
      ));
    }
    return gen_err!("Expected { for start of object", tokens[0]);
  }
  let mut old = tokens.remove(0);

  let mut keys: HashMap<String, RuntimeValue> = HashMap::new();

  while tokens.len() > 0 && tokens[0].kind != TokenType::CurlyClose {
    // Expect key
    if tokens[0].kind != TokenType::String {
      return gen_err!("Expected key", tokens[0]);
    }

    // Collect key
    let mut key = get_part!(data.clone(), tokens[0]);
    key.remove(0);
    key.pop();
    old = tokens.remove(0);

    // Expect :
    if tokens.len() == 0 || tokens[0].kind != TokenType::Colon {
      return gen_err!(
        "Expected colon between key-value pair",
        if tokens.len() == 0 {
          old.clone()
        } else {
          tokens[0].clone()
        }
      );
    }
    old = tokens.remove(0);

    // Make sure there is a value
    if tokens.len() == 0 {
      return gen_err!("Expected value after key", old);
    }

    // Parse the value
    let value = token_to_zephyr(tokens, data.clone())?;
    keys.insert(key, value);

    // Check for ,
    if tokens.len() > 0 && tokens[0].kind == TokenType::Comma {
      old = tokens.remove(0);
    }
  }

  if tokens.len() == 0 || tokens[0].kind != TokenType::CurlyClose {
    return gen_err!(
      "Expected { for start of object",
      if tokens.len() == 0 {
        old.clone()
      } else {
        tokens[0].clone()
      }
    );
  }
  tokens.remove(0);

  return Ok(values::Object::make(keys).create_container());
}

pub fn token_to_zephyr(
  tokens: &mut Vec<Token>,
  data: Vec<u8>,
) -> Result<RuntimeValue, ZephyrError> {
  let part = get_part!(data, tokens[0]);
  Ok(match tokens[0].kind {
    TokenType::Number => {
      let v = RuntimeValue::Number(values::Number {
        value: match part.parse() {
          Err(_) => return gen_err!("Failed to parse number", tokens[0]),
          Ok(ok) => ok,
        },
      });
      tokens.remove(0);
      v
    }
    TokenType::BooleanFalse => {
      tokens.remove(0);
      Boolean::make(false)
    }
    TokenType::BooleanTrue => {
      tokens.remove(0);
      Boolean::make(true)
    }
    TokenType::Null => {
      tokens.remove(0);
      Null::make()
    }
    TokenType::String => {
      let mut p = part;
      p.remove(0);
      p.pop();
      tokens.remove(0);
      StringValue::make(p)
    }
    TokenType::CurlyOpen => parse_object(tokens, data.clone())?,
    TokenType::BracketOpen => {
      let mut items = values::Array { items: vec![] };
      let old = tokens.remove(0);

      while tokens.len() > 0 && tokens[0].kind != TokenType::BracketClose {
        items
          .items
          .push(Box::from(token_to_zephyr(tokens, data.clone())?));

        if tokens.len() > 0 && tokens[0].kind != TokenType::Comma {
          break;
        }

        tokens.remove(0);
      }

      // Check end
      if tokens.len() == 0 {
        return gen_err!("Expected closing of array", old);
      } else if tokens[0].kind != TokenType::BracketClose {
        return gen_err!("Expected closing of array", tokens[0]);
      }

      tokens.remove(0);

      items.create_container()
    }
    _ => return gen_err!("Unexpected token", tokens[0]),
  })
}

pub fn zephyr_to_json(object: ObjectContainer) -> Result<String, ZephyrError> {
  zephyr_object_to_json(object)
}

pub fn zephyr_object_to_json(object: ObjectContainer) -> Result<String, ZephyrError> {
  let o = object.deref();
  let mut contents = String::from("{");

  for item in o.items {
    let value = zephyr_value_to_json(item.1)?;
    contents.push_str(&format!("\"{}\": {},", item.0, value));
  }

  // Remove last ,
  contents.pop();

  contents.push('}');

  Ok(contents)
}

pub fn zephyr_value_to_json(what: RuntimeValue) -> Result<String, ZephyrError> {
  Ok(match what {
    RuntimeValue::Number(number) => number.value.to_string(),
    RuntimeValue::StringValue(string) => format!("\"{}\"", string.value),
    RuntimeValue::Boolean(bool) => bool.value.to_string(),
    RuntimeValue::Null(_) => "null".to_string(),
    RuntimeValue::ObjectContainer(obj) => zephyr_object_to_json(obj)?,
    RuntimeValue::ArrayContainer(arr) => {
      let mut text = String::from("[");
      let a = arr.deref();

      for item in a.items {
        text.push_str(&zephyr_value_to_json(*item)?);
        text.push(',');
      }

      text.pop();
      text.push(']');

      text
    }
    _ => {
      return Err(ZephyrError::runtime(
        format!("Cannot convert a {} to a JSON value", what.type_name()),
        Location::no_location(),
      ))
    }
  })
}
