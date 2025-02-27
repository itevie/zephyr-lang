pub mod zstring;
pub use zstring::*;

pub mod number;
pub use number::*;

pub mod null;
pub use null::*;

pub mod boolean;
pub use boolean::*;

pub mod functions;
pub use functions::*;

pub mod array;
pub use array::*;

pub mod event_emitter;
pub use event_emitter::*;

pub mod reference;
pub use reference::*;

pub mod range;
pub use range::*;

pub mod object;
pub use object::*;

pub mod details;
pub use details::*;

use crate::{
    errors::{ErrorCode, ZephyrError},
    lexer::tokens::{Comparison, Location},
};

use super::memory_store;

#[derive(Debug, Clone)]
pub enum RuntimeValue {
    Number(Number),
    Null(Null),
    ZString(ZString),
    Boolean(Boolean),
    Reference(Reference),
    Function(Function),
    NativeFunction(NativeFunction),
    Array(Array),
    Object(Object),
    EventEmitter(EventEmitter),
    RangeValue(RangeValue),
}

impl RuntimeValue {
    pub fn type_name(&self) -> &str {
        match self {
            RuntimeValue::Boolean(_) => "boolean",
            RuntimeValue::Null(_) => "null",
            RuntimeValue::Number(_) => "number",
            RuntimeValue::ZString(_) => "string",
            RuntimeValue::Reference(_) => "reference",
            RuntimeValue::Function(_) => "function",
            RuntimeValue::NativeFunction(_) => "native_function",
            RuntimeValue::Array(_) => "array",
            RuntimeValue::Object(_) => "object",
            RuntimeValue::EventEmitter(_) => "event_emitter",
            RuntimeValue::RangeValue(_) => "range",
        }
    }

    pub fn iter(&self) -> Result<Vec<RuntimeValue>, ZephyrError> {
        match self {
            RuntimeValue::ZString(str) => Ok(str
                .value
                .chars()
                .map(|v| ZString::new(v.to_string()))
                .collect::<Vec<RuntimeValue>>()),
            RuntimeValue::Array(a) => Ok(a.items.clone()),
            RuntimeValue::RangeValue(r) => Ok(r.iter()?.iter().map(|x| Number::new(*x)).collect()),
            v => Err(ZephyrError {
                message: format!("Cannot iter a {}", v.type_name()),
                code: ErrorCode::CannotIterate,
                location: None,
            }),
        }
    }

    /// Gets the options struct no matter what the underlying type is
    pub fn options(&self) -> &RuntimeValueDetails {
        match self {
            RuntimeValue::Array(v) => &v.options,
            RuntimeValue::Boolean(v) => &v.options,
            RuntimeValue::Function(v) => &v.options,
            RuntimeValue::NativeFunction(v) => &v.options,
            RuntimeValue::Null(v) => &v.options,
            RuntimeValue::Number(v) => &v.options,
            RuntimeValue::Object(v) => &v.options,
            RuntimeValue::Reference(v) => &v.options,
            RuntimeValue::ZString(v) => &v.options,
            RuntimeValue::EventEmitter(v) => &v.options,
            RuntimeValue::RangeValue(v) => &v.options,
        }
    }

    pub fn set_options(&mut self, new: RuntimeValueDetails) -> () {
        match self {
            RuntimeValue::Array(v) => v.options = new,
            RuntimeValue::Boolean(v) => v.options = new,
            RuntimeValue::Function(v) => v.options = new,
            RuntimeValue::NativeFunction(v) => v.options = new,
            RuntimeValue::Null(v) => v.options = new,
            RuntimeValue::Number(v) => v.options = new,
            RuntimeValue::Object(v) => v.options = new,
            RuntimeValue::Reference(v) => v.options = new,
            RuntimeValue::ZString(v) => v.options = new,
            RuntimeValue::EventEmitter(v) => v.options = new,
            RuntimeValue::RangeValue(v) => v.options = new,
        };
    }

    /// Converts the value into a string (not display)
    pub fn to_string(&self) -> Result<String, ZephyrError> {
        Ok(match self {
            RuntimeValue::Boolean(v) => v.value.to_string(),
            RuntimeValue::Null(_) => "null".to_string(),
            RuntimeValue::Number(v) => v.value.to_string(),
            RuntimeValue::Reference(v) => {
                return v.inner().unwrap().to_string();
            }
            RuntimeValue::ZString(v) => format!("{}", v.value),
            RuntimeValue::Array(a) => {
                format!(
                    "[{}]",
                    a.items
                        .iter()
                        .map(|x| x.to_string().unwrap())
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
            v => {
                format!("{:#?}", v)
                /*return Err(ZephyrError {
                    code: ErrorCode::CannotCoerce,
                    message: format!("Cannot coerce {} into a string", self.type_name()),
                    location: None,
                })*/
            }
        })
    }

    /// Checks whether or not the value is "truthy" following set rules
    pub fn is_truthy(&self) -> bool {
        match self {
            RuntimeValue::Boolean(v) => v.value,
            RuntimeValue::ZString(v) => v.value.len() > 0,
            RuntimeValue::Number(v) => v.value > 0f64,
            _ => false,
        }
    }

    /// Simply adds the value to the object store
    pub fn as_ref(&self) -> usize {
        memory_store::allocate(self.clone())
    }

    /// Used for returning a tuple containing the inner reference (or current value), along with the reference ID  
    /// Looks like: (value, ref)
    pub fn as_ref_tuple(&self) -> Result<(RuntimeValue, Option<Reference>), ZephyrError> {
        match self {
            RuntimeValue::Reference(r) => Ok(((*r.inner()?).clone(), Some(r.clone()))),
            _ => Ok((self.clone(), None)),
        }
    }

    pub fn compare_with(
        &self,
        right: RuntimeValue,
        t: Comparison,
        location: Option<Location>,
    ) -> Result<bool, ZephyrError> {
        if let Comparison::Eq = t {
            if self.type_name() != right.type_name() {
                return Ok(false);
            }
        }

        if let Comparison::Neq = t {
            if self.type_name() != right.type_name() {
                return Ok(true);
            }
        }

        return Ok(match (self, right, t) {
            (RuntimeValue::Number(l), RuntimeValue::Number(r), ref t) => match t {
                Comparison::Eq => l.value == r.value,
                Comparison::Neq => l.value != r.value,
                Comparison::Gt => l.value > r.value,
                Comparison::Lt => l.value < r.value,
                Comparison::GtEq => l.value >= r.value,
                Comparison::LtEq => l.value <= r.value,
            },
            (RuntimeValue::ZString(l), RuntimeValue::ZString(r), Comparison::Eq) => {
                l.value == r.value
            }
            (RuntimeValue::ZString(l), RuntimeValue::ZString(r), Comparison::Neq) => {
                l.value != r.value
            }
            (RuntimeValue::Null(_), RuntimeValue::Null(_), Comparison::Eq) => true,
            (_, ref r, ref t) => {
                return Err(ZephyrError {
                    code: ErrorCode::InvalidOperation,
                    message: format!(
                        "Cannot perform {} {} {}",
                        self.type_name(),
                        t,
                        r.type_name()
                    ),
                    location,
                })
            }
        });
    }
}
