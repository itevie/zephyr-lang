use std::{collections::HashMap, fmt};

use crate::{
  errors::ZephyrError,
  lexer::location::Location,
  parser::nodes::{Block, Identifier, WhereClause},
};

use super::{memory::MemoryAddress, native_functions::CallOptions, scope::ScopeContainer};

// ----- Base -----
#[derive(Clone, Debug)]
pub enum RuntimeValue {
  Number(Number),
  Null(Null),
  Boolean(Boolean),
  StringValue(StringValue),
  Reference(Reference),
  Array(Array),
  ArrayContainer(ArrayContainer),
  Function(Function),
  NativeFunction(NativeFunction),
  Object(Object),
  ObjectContainer(ObjectContainer),
}

unsafe impl Send for RuntimeValue {}
unsafe impl Sync for RuntimeValue {}

impl RuntimeValue {
  pub fn type_name(&self) -> &str {
    match *self {
      RuntimeValue::Number(_) => "number",
      RuntimeValue::Null(_) => "null",
      RuntimeValue::Boolean(_) => "boolean",
      RuntimeValue::StringValue(_) => "string",
      RuntimeValue::Reference(_) => "reference",
      RuntimeValue::Array(_) => "inner_array",
      RuntimeValue::ArrayContainer(_) => "array",
      RuntimeValue::Object(_) => "inner_object",
      RuntimeValue::ObjectContainer(_) => "object",
      RuntimeValue::Function(_) => "function",
      RuntimeValue::NativeFunction(_) => "native_function",
    }
  }

  pub fn iterate(&self) -> Result<Vec<Box<RuntimeValue>>, ZephyrError> {
    let value = match self {
      RuntimeValue::StringValue(str) => str
        .value
        .chars()
        .map(|val| {
          Box::from(RuntimeValue::StringValue(StringValue {
            value: val.to_string(),
          }))
        })
        .collect::<Vec<Box<RuntimeValue>>>(),
      RuntimeValue::ArrayContainer(arr) => {
        match crate::MEMORY.read().unwrap().get_value(arr.location)? {
          RuntimeValue::Array(a) => a.items,
          _ => unreachable!(),
        }
      }
      RuntimeValue::ObjectContainer(obj) => {
        match crate::MEMORY.read().unwrap().get_value(obj.location)? {
          RuntimeValue::Object(o) => o
            .items
            .keys()
            .map(|val| {
              Box::from(RuntimeValue::StringValue(StringValue {
                value: val.clone(),
              }))
            })
            .collect::<Vec<Box<RuntimeValue>>>(),
          _ => unreachable!(),
        }
      }
      _ => {
        return Err(ZephyrError::runtime(
          format!("Cannot iter provided {}", self.type_name()),
          Location::no_location(),
        ))
      }
    };

    Ok(value)
  }

  pub fn stringify(&self, is_alone: bool, pretty: bool) -> String {
    match self {
      RuntimeValue::Number(number) => format!("{}", number.value),
      RuntimeValue::Null(_) => "null".to_string(),
      RuntimeValue::Boolean(boolean) => if boolean.value { "true" } else { "false" }.to_string(),
      RuntimeValue::StringValue(string) => {
        if is_alone {
          string.value.clone()
        } else {
          format!("\"{}\"", string.value.clone())
        }
      }
      RuntimeValue::Reference(refer) => format!("&{}", refer.value),
      RuntimeValue::Array(_) => "array".to_string(),
      RuntimeValue::ArrayContainer(arr) => {
        let mut res = String::from("[");
        let array = match crate::MEMORY
          .read()
          .unwrap()
          .get_value(arr.location)
          .unwrap()
        {
          RuntimeValue::Array(arr) => arr,
          _ => unreachable!(),
        };

        for i in 0..array.items.len() {
          res.push_str(&format!("{}", array.items[i].stringify(false, pretty)));

          // Check if should add comma
          if i < array.items.len() - 1 {
            res.push_str(", ");
          }
        }

        res.push_str("]");

        res
      }
      RuntimeValue::ObjectContainer(obj) => {
        let mut res = String::from("{");
        let object = match crate::MEMORY
          .read()
          .unwrap()
          .get_value(obj.location)
          .unwrap()
        {
          RuntimeValue::Object(arr) => arr,
          _ => unreachable!(),
        };

        let item_length = object.items.len();

        for (key, value) in object.items {
          if pretty && item_length > 1 {
            res.push_str("\n  ");
          }
          res.push_str(&format!(
            "\"{}\": {}",
            key,
            if pretty && item_length > 1 {
              value.stringify(false, pretty).replace('\n', "\n  ")
            } else {
              value.stringify(false, pretty)
            }
          ));
        }

        if pretty && item_length > 1 {
          res.push_str("\n");
        }
        res.push_str("}");

        res
      }
      RuntimeValue::Object(_) => "object".to_string(),
      RuntimeValue::Function(_) => "function".to_string(),
      RuntimeValue::NativeFunction(_) => "native_function".to_string(),
    }
  }
}

// ----- Util -----
impl fmt::Display for RuntimeValue {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.stringify(false, true))
  }
}

impl RuntimeValue {
  pub fn is_truthy(&self) -> bool {
    match self {
      RuntimeValue::Number(number) => number.value > 0.0,
      RuntimeValue::Boolean(boolean) => boolean.value,
      _ => false,
    }
  }
}

pub fn to_array(values: Vec<Box<RuntimeValue>>) -> RuntimeValue {
  let array = RuntimeValue::Array(Array { items: values });

  RuntimeValue::ArrayContainer(ArrayContainer {
    location: crate::MEMORY.write().unwrap().add_value(array),
  })
}

pub fn to_object(values: HashMap<String, RuntimeValue>) -> RuntimeValue {
  let object = RuntimeValue::Object(Object { items: values });

  RuntimeValue::ObjectContainer(ObjectContainer {
    location: crate::MEMORY.write().unwrap().add_value(object),
  })
}

// ----- Actual Values -----
#[derive(Clone, Debug)]
pub struct Number {
  pub value: f64,
}

#[derive(Clone, Debug)]
pub struct StringValue {
  pub value: String,
}

#[derive(Clone, Debug)]
pub struct Null {}

#[derive(Clone, Debug)]
pub struct Boolean {
  pub value: bool,
}

#[derive(Clone, Debug)]
pub struct Reference {
  pub value: MemoryAddress,
}

#[derive(Clone, Debug)]
pub struct ArrayContainer {
  pub location: MemoryAddress,
}

#[derive(Clone, Debug)]
pub struct Array {
  pub items: Vec<Box<RuntimeValue>>,
}

#[derive(Clone, Debug)]
pub struct ObjectContainer {
  pub location: MemoryAddress,
}

#[derive(Clone, Debug)]
pub struct Object {
  pub items: HashMap<String, RuntimeValue>,
}

#[derive(Clone)]
pub struct Function {
  pub scope: ScopeContainer,
  pub body: Box<Block>,
  pub name: Option<String>,
  pub arguments: Vec<Identifier>,
  pub where_clause: Box<WhereClause>,
  pub pure: bool,
}

#[derive(Clone)]
pub struct NativeFunction {
  pub func: &'static dyn Fn(CallOptions) -> Result<RuntimeValue, ZephyrError>,
}

impl fmt::Debug for NativeFunction {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", "NativeFunction")
  }
}

impl fmt::Debug for Function {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", "Function")
  }
}
