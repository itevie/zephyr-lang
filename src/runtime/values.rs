use std::{collections::HashMap, fmt, sync::Arc};

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
  NativeFunction2(NativeFunction2),
  NativeClosureFunction(NativeClosureFunction),
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
      RuntimeValue::NativeClosureFunction(_) => "native_function",
      RuntimeValue::NativeFunction2(_) => "native_function",
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
        match crate::MEMORY.lock().unwrap().get_value(arr.location)? {
          RuntimeValue::Array(a) => a.items,
          _ => unreachable!(),
        }
      }
      RuntimeValue::ObjectContainer(obj) => {
        match crate::MEMORY.lock().unwrap().get_value(obj.location)? {
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
          let v = format!("{:?}", string.value.clone());
          v
        }
      }
      RuntimeValue::Reference(refer) => format!("&{}", refer.value),
      RuntimeValue::Array(_) => "array".to_string(),
      RuntimeValue::ArrayContainer(arr) => {
        let mut res = String::from("[");
        let array = match crate::MEMORY
          .lock()
          .unwrap()
          .get_value(arr.location)
          .unwrap()
        {
          RuntimeValue::Array(arr) => arr,
          x => {
            println!("{:?}", x);
            unreachable!()
          }
        };

        for i in 0..array.items.len() {
          res.push_str(&array.items[i].stringify(false, pretty).to_string());

          // Check if should add comma
          if i < array.items.len() - 1 {
            res.push_str(", ");
          }
        }

        res.push(']');

        res
      }
      RuntimeValue::ObjectContainer(obj) => {
        let mut res = String::from("{");
        let object = match crate::MEMORY
          .lock()
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
              let v = value.stringify(false, pretty);
              v.replace('\n', "\n  ")
            } else {
              let v = value.stringify(false, pretty);
              v
            }
          ));
        }

        if pretty && item_length > 1 {
          res.push('\n');
        }
        res.push('}');

        res
      }
      RuntimeValue::Object(_) => "object".to_string(),
      RuntimeValue::Function(_) => "function".to_string(),
      RuntimeValue::NativeFunction(_) => "native_function".to_string(),
      RuntimeValue::NativeFunction2(_) => "native_function".to_string(),
      RuntimeValue::NativeClosureFunction(_) => "native_function".to_string(),
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
      RuntimeValue::StringValue(string) => string.value.len() != 0,
      _ => false,
    }
  }
}

pub fn to_array(values: Vec<Box<RuntimeValue>>) -> RuntimeValue {
  let array = RuntimeValue::Array(Array { items: values });

  RuntimeValue::ArrayContainer(ArrayContainer {
    location: crate::MEMORY.lock().unwrap().add_value(array),
  })
}

pub fn to_object(values: HashMap<String, RuntimeValue>) -> RuntimeValue {
  let object = RuntimeValue::Object(Object { items: values });

  RuntimeValue::ObjectContainer(ObjectContainer {
    location: crate::MEMORY.lock().unwrap().add_value(object),
  })
}

// ----- Actual Values -----
#[derive(Clone, Debug)]
pub struct Number {
  pub value: f64,
}
impl Number {
  pub fn make(number: f64) -> RuntimeValue {
    RuntimeValue::Number(Number { value: number })
  }
}

#[derive(Clone, Debug)]
pub struct StringValue {
  pub value: String,
}

impl StringValue {
  pub fn make(value: String) -> RuntimeValue {
    RuntimeValue::StringValue(StringValue { value })
  }
}

#[derive(Clone, Debug)]
pub struct Null {}
impl Null {
  pub fn make() -> RuntimeValue {
    RuntimeValue::Null(Null {})
  }
}

#[derive(Clone, Debug)]
pub struct Boolean {
  pub value: bool,
}

impl Boolean {
  pub fn make(value: bool) -> RuntimeValue {
    RuntimeValue::Boolean(Boolean { value })
  }
}

#[derive(Clone, Debug)]
pub struct Reference {
  pub value: MemoryAddress,
}

#[derive(Clone, Debug)]
pub struct ArrayContainer {
  pub location: MemoryAddress,
}

impl ArrayContainer {
  pub fn deref(&self) -> Array {
    let data = crate::MEMORY
      .lock()
      .unwrap()
      .get_value(self.location)
      .unwrap();
    match data {
      RuntimeValue::Array(arr) => arr,
      _ => unreachable!(),
    }
  }
}

#[derive(Clone, Debug)]
pub struct Array {
  pub items: Vec<Box<RuntimeValue>>,
}

impl Array {
  pub fn create_container(self) -> ArrayContainer {
    ArrayContainer {
      location: crate::MEMORY
        .lock()
        .unwrap()
        .add_value(RuntimeValue::Array(self.clone())),
    }
  }
}

#[derive(Clone, Debug)]
pub struct ObjectContainer {
  pub location: MemoryAddress,
}

impl ObjectContainer {
  pub fn deref(&self) -> Object {
    let data = crate::MEMORY
      .lock()
      .unwrap()
      .get_value(self.location)
      .unwrap();
    match data {
      RuntimeValue::Object(obj) => obj,
      _ => unreachable!(),
    }
  }
}

#[derive(Clone, Debug)]
pub struct Object {
  pub items: HashMap<String, RuntimeValue>,
}

impl Object {
  pub fn make(items: HashMap<String, RuntimeValue>) -> Object {
    Object { items }
  }

  pub fn create_container(self) -> ObjectContainer {
    ObjectContainer {
      location: crate::MEMORY
        .lock()
        .unwrap()
        .add_value(RuntimeValue::Object(self.clone())),
    }
  }
}

#[derive(Clone)]
pub struct Function {
  pub scope: ScopeContainer,
  pub body: Box<Block>,
  pub name: Option<String>,
  pub arguments: Vec<Identifier>,
  pub where_clause: Box<WhereClause>,
  pub pure: bool,
  pub type_call: Option<Box<RuntimeValue>>,
}

#[derive(Clone)]
pub struct NativeFunction {
  pub func: &'static dyn Fn(CallOptions) -> Result<RuntimeValue, ZephyrError>,
}

#[derive(Clone)]
pub struct NativeFunction2 {
  pub func: Arc<dyn Fn(CallOptions) -> Result<RuntimeValue, ZephyrError> + Send + Sync>,
}

#[derive(Clone, Debug)]
pub struct NativeClosureFunction {
  pub id: u128,
}
impl NativeClosureFunction {
  pub fn make(id: u128) -> RuntimeValue {
    RuntimeValue::NativeClosureFunction(NativeClosureFunction { id })
  }
}

impl fmt::Debug for NativeFunction {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "NativeFunction")
  }
}

impl fmt::Debug for NativeFunction2 {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "NativeFunction")
  }
}

impl fmt::Debug for Function {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Function")
  }
}
