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

pub mod range;
pub use range::*;

pub mod object;
pub use object::*;

pub mod details;
pub use details::*;

pub mod enum_variant;
pub use enum_variant::*;

pub mod export;
pub use export::*;

pub mod struct_mapping;
pub mod thread_crossing;

use crate::{
    errors::{ErrorCode, ZephyrError},
    lexer::tokens::{Comparison, Location},
    util::colors,
};

pub trait RuntimeValueUtils {
    fn wrap(&self) -> RuntimeValue;
    fn type_name(&self) -> &str;

    fn to_string(&self, _is_display: bool, _color: bool) -> Result<String, ZephyrError> {
        return Err(ZephyrError {
            message: format!("Cannot stringify {}", self.type_name()),
            code: ErrorCode::TypeError,
            location: None,
        });
    }

    fn iter(&self) -> Result<Vec<RuntimeValue>, ZephyrError> {
        return Err(ZephyrError {
            message: format!("Cannot iter a {}", self.type_name()),
            code: ErrorCode::CannotIterate,
            location: None,
        });
    }

    fn len(&self) -> Result<usize, ZephyrError> {
        Ok(self.iter()?.len())
    }

    /*fn as_ref(&self) -> usize {
        memory_store::allocate(self.wrap())
    }

    fn as_ref_val(&self) -> RuntimeValue {
        Reference::new(self.as_ref()).wrap()
    }*/
}

#[derive(Debug, Clone)]
pub enum RuntimeValue {
    Number(Number),
    Null(Null),
    ZString(ZString),
    Boolean(Boolean),
    Function(Function),
    NativeFunction(NativeFunction),
    MspcSender(MspcSender),
    Array(Array),
    Object(Object),
    EventEmitter(EventEmitter),
    RangeValue(RangeValue),
    EnumVariant(EnumVariant),
    Export(Export),
}

macro_rules! run_as_any {
    ($s:ident, $i:ident, $e:expr) => {
        match $s {
            RuntimeValue::Boolean($i) => $e,
            RuntimeValue::Null($i) => $e,
            RuntimeValue::Number($i) => $e,
            RuntimeValue::ZString($i) => $e,
            RuntimeValue::Function($i) => $e,
            RuntimeValue::NativeFunction($i) => $e,
            RuntimeValue::MspcSender($i) => $e,
            RuntimeValue::Array($i) => $e,
            RuntimeValue::Object($i) => $e,
            RuntimeValue::EventEmitter($i) => $e,
            RuntimeValue::RangeValue($i) => $e,
            RuntimeValue::EnumVariant($i) => $e,
            RuntimeValue::Export($i) => $e,
        }
    };
}

impl RuntimeValue {
    pub fn type_name(&self) -> &str {
        run_as_any!(self, v, v.type_name())
    }

    pub fn iter(&self) -> Result<Vec<RuntimeValue>, ZephyrError> {
        run_as_any!(self, v, v.iter())
    }

    pub fn len(&self) -> Result<usize, ZephyrError> {
        run_as_any!(self, v, v.len())
    }

    /// Gets the options struct no matter what the underlying type is
    pub fn options(&self) -> &RuntimeValueDetails {
        run_as_any!(self, v, &v.options)
    }

    pub fn set_options(&mut self, new: RuntimeValueDetails) {
        run_as_any!(self, v, v.options = new);
    }

    pub fn unwrap(&mut self) -> Box<dyn RuntimeValueUtils> {
        run_as_any!(self, v, Box::from(v.clone()))
    }

    pub fn set_proto(self, id: String) -> Self {
        let mut old_options = self.options().proto.borrow_mut();
        *old_options = Some(id);
        self.clone()
    }

    /// Converts the value into a string (not display)
    pub fn to_string(
        &self,
        is_display: bool,
        color: bool,
        full: bool,
    ) -> Result<String, ZephyrError> {
        let mut string = run_as_any!(self, v, v.to_string(is_display, color))?;

        if full {
            string.push_str(&match color {
                true => format!(
                    "\n{}# {:?}{}",
                    colors::FG_GRAY,
                    self.options().tags.borrow(),
                    colors::COLOR_RESET
                ),
                false => format!("\n# {:?}", self.options().tags.borrow()),
            });
        }

        Ok(string)
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

    /*/// Simply adds the value to the object store
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
    }*/

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
