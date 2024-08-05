use std::{collections::HashMap, fmt, sync::Arc};

use crate::{
  errors::ZephyrError,
  lexer::{location::Location, token::ComparisonTokenType},
  parser::nodes::{Block, ComparisonExpression, Identifier, WhereClause},
  util,
};

use super::{
  interpreter::Interpreter, memory::MemoryAddress, native_functions::CallOptions,
  scope::ScopeContainer,
};

// ----- Base -----
#[derive(Clone, Debug)]
pub enum RuntimeValue {
  Number(Number),
  Null(Null),
  Boolean(Boolean),
  StringValue(StringValue),
  Array(Array),
  ArrayContainer(ArrayContainer),
  Function(Function),
  NativeFunction(NativeFunction),
  NativeFunction2(NativeFunction2),
  Object(Object),
  ObjectContainer(ObjectContainer),
  ErrorValue(ErrorValue),
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
      RuntimeValue::Array(_) => "inner_array",
      RuntimeValue::ArrayContainer(_) => "array",
      RuntimeValue::Object(_) => "inner_object",
      RuntimeValue::ObjectContainer(_) => "object",
      RuntimeValue::Function(_) => "function",
      RuntimeValue::NativeFunction(_) => "native_function",
      RuntimeValue::NativeFunction2(_) => "native_function",
      RuntimeValue::ErrorValue(_) => "error",
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

  pub fn is_equal_to(
    &self,
    right: RuntimeValue,
    expr: Option<ComparisonExpression>,
  ) -> Result<bool, ZephyrError> {
    // Collect operator
    let operator = if let Some(expression) = expr.clone() {
      expression.operator
    } else {
      ComparisonTokenType::Equals
    };

    // Check if they are the same type
    if !util::varient_eq(&self.clone(), &right.clone()) {
      return Ok(false);
    }

    // Check for numbers
    if let (RuntimeValue::Number(left_number), RuntimeValue::Number(right_number)) = (self, &right)
    {
      return Ok(match operator {
        ComparisonTokenType::Equals => left_number.value == right_number.value,
        ComparisonTokenType::NotEquals => left_number.value != right_number.value,
        ComparisonTokenType::GreaterThan => left_number.value > right_number.value,
        ComparisonTokenType::GreaterThanOrEquals => left_number.value >= right_number.value,
        ComparisonTokenType::LessThan => left_number.value < right_number.value,
        ComparisonTokenType::LessThanOrEquals => left_number.value <= right_number.value,
      });
    } else if matches!(operator, ComparisonTokenType::Equals)
      || matches!(operator, ComparisonTokenType::NotEquals)
    {
      let result = match (self, &right) {
        // Booleans
        (RuntimeValue::Boolean(left_value), RuntimeValue::Boolean(right_value)) => {
          Some(left_value.value == right_value.value)
        }

        // Strings
        (
          RuntimeValue::StringValue(left_value),
          RuntimeValue::StringValue(right_value),
        ) => Some(left_value.value == right_value.value),

        // Null - this will always be true
        (&RuntimeValue::Null(_), _) => Some(true),

        // Arrays
        (
          RuntimeValue::ArrayContainer(left_value),
          RuntimeValue::ArrayContainer(right_value),
        ) => Some(left_value.location == right_value.location),

        // Objects
        (
          RuntimeValue::ObjectContainer(left_value),
          RuntimeValue::ObjectContainer(right_value),
        ) => Some(left_value.location == right_value.location),
        _ => None,
      };

      match result {
        Some(ok) => {
          return Ok(if matches!(operator, ComparisonTokenType::NotEquals) {
            !ok
          } else {
            ok
          })
        }
        _ => (),
      }
    }

    Err(ZephyrError::runtime(
      format!(
        "Cannot handle {} {} {}",
        self.type_name(),
        operator,
        right.type_name()
      ),
      if let Some(expression) = expr {
        expression.location
      } else {
        Location::no_location()
      },
    ))
  }

  pub fn stringify(
    &self,
    is_alone: bool,
    pretty: bool,
    interpreter: Option<Interpreter>,
  ) -> String {
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
          res.push_str(
            &array.items[i]
              .stringify(false, pretty, interpreter)
              .to_string(),
          );

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

        // Check for PrettyPrint
        /*if object.items.contains_key(&get_symbol("PrettyPrint")) {
          if let Some(mut inter) = interpreter {
            let func = match object.items.get(&get_symbol("PrettyPrint")).unwrap() {
              RuntimeValue::Function(func) => func,
              _ => return "<Stringify Err>".to_string(),
            };

            return inter
              .evaluate_zephyr_function(
                func.clone(),
                vec![Box::from(RuntimeValue::ObjectContainer(obj.clone()))],
                Location::no_location(),
              )
              .unwrap()
              .stringify(is_alone, pretty, interpreter);
          }
        }*/

        let item_length = object.items.len();

        for (key, value) in object.items {
          if pretty && item_length > 1 {
            res.push_str("\n  ");
          }
          res.push_str(&format!(
            "\"{}\": {}",
            key,
            if pretty && item_length > 1 {
              let v = value.stringify(false, pretty, interpreter);
              v.replace('\n', "\n  ")
            } else {
              
              value.stringify(false, pretty, interpreter)
            }
          ));
        }

        if pretty && item_length > 1 {
          res.push('\n');
        }
        res.push('}');

        res
      }
      RuntimeValue::ErrorValue(_) => "error".to_string(),
      RuntimeValue::Object(_) => "object".to_string(),
      RuntimeValue::Function(_) => "function".to_string(),
      RuntimeValue::NativeFunction(_) => "native_function".to_string(),
      RuntimeValue::NativeFunction2(_) => "native_function".to_string(),
    }
  }
}

// ----- Util -----
impl fmt::Display for RuntimeValue {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.stringify(false, true, None))
  }
}

impl RuntimeValue {
  pub fn is_truthy(&self) -> bool {
    match self {
      RuntimeValue::Number(number) => number.value > 0.0,
      RuntimeValue::Boolean(boolean) => boolean.value,
      RuntimeValue::StringValue(string) => !string.value.is_empty(),
      _ => false,
    }
  }
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
  pub fn create_container(self) -> RuntimeValue {
    RuntimeValue::ArrayContainer(ArrayContainer {
      location: crate::MEMORY
        .lock()
        .unwrap()
        .add_value(RuntimeValue::Array(self.clone())),
    })
  }

  pub fn make(items: Vec<Box<RuntimeValue>>) -> Array {
    Array { items }
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

  pub fn create_container(self) -> RuntimeValue {
    RuntimeValue::ObjectContainer(ObjectContainer {
      location: crate::MEMORY
        .lock()
        .unwrap()
        .add_value(RuntimeValue::Object(self.clone())),
    })
  }
}

#[derive(Clone, Debug)]
pub struct ErrorValue {
  pub error: ZephyrError,
  pub data: Box<RuntimeValue>,
}

impl ErrorValue {
  pub fn make(error: ZephyrError, data: RuntimeValue) -> ErrorValue {
    ErrorValue {
      error,
      data: Box::from(data),
    }
  }
}

// ----- Function Mess -----

#[derive(Clone)]
pub struct Function {
  pub scope: ScopeContainer,
  pub body: Box<Block>,
  pub name: Option<String>,
  pub arguments: Vec<Identifier>,
  pub where_clause: Box<WhereClause>,
  pub type_call: Option<Box<RuntimeValue>>,
}

#[derive(Clone)]
pub struct NativeFunction {
  pub func: &'static dyn Fn(CallOptions) -> Result<RuntimeValue, ZephyrError>,
}

impl NativeFunction {
  pub fn make(
    func: &'static dyn Fn(CallOptions) -> Result<RuntimeValue, ZephyrError>,
  ) -> RuntimeValue {
    RuntimeValue::NativeFunction(Self { func })
  }
}

#[derive(Clone)]
pub struct NativeFunction2 {
  pub func: Arc<dyn Fn(CallOptions) -> Result<RuntimeValue, ZephyrError> + Send + Sync>,
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
