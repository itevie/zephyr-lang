use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{
    errors::{ErrorCode, ZephyrError},
    lexer::tokens::{Comparison, Location},
    parser::nodes,
    parser::nodes::{self, Symbol},
};

use super::{
    memory_store::{self, allocate},
    scope::Scope,
};

#[derive(Debug, Clone)]
pub struct RuntimeValueDetails {
    tags: Arc<Mutex<HashMap<String, String>>>,
}

impl Default for RuntimeValueDetails {
    fn default() -> Self {
        Self {
            tags: Arc::from(Mutex::from(HashMap::default())),
        }
    }
}

#[derive(Debug, Clone)]
pub enum RuntimeValue {
    Number(Number),
    Null(Null),
    ZString(ZString),
    Boolean(Boolean),
    Reference(Reference),
    Function(Function),
    Array(Array),
    Object(Object),
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
            RuntimeValue::Array(_) => "array",
            RuntimeValue::Object(_) => "object",
        }
    }

    pub fn get_options(&self) -> RuntimeValueDetails {
        match self {
            RuntimeValue::Array(v) => v.options.clone(),
            RuntimeValue::Boolean(v) => v.options.clone(),
            RuntimeValue::Function(v) => v.options.clone(),
            RuntimeValue::Null(v) => v.options.clone(),
            RuntimeValue::Number(v) => v.options.clone(),
            RuntimeValue::Object(v) => v.options.clone(),
            RuntimeValue::Reference(v) => v.options.clone(),
            RuntimeValue::ZString(v) => v.options.clone(),
        }
    }

    #[allow(unreachable_patterns)]
    pub fn to_string(&self) -> Result<String, ZephyrError> {
        Ok(match self {
            RuntimeValue::Boolean(v) => v.value.to_string(),
            RuntimeValue::Null(_) => "null".to_string(),
            RuntimeValue::Number(v) => v.value.to_string(),
            RuntimeValue::ZString(v) => v.value.clone(),
            _ => {
                return Err(ZephyrError {
                    code: ErrorCode::CannotCoerce,
                    message: format!("Cannot coerce {} into a string", self.type_name()),
                    location: None,
                })
            }
        })
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            RuntimeValue::Boolean(v) => v.value,
            RuntimeValue::ZString(v) => v.value.len() > 0,
            RuntimeValue::Number(v) => v.value > 0f64,
            _ => false,
        }
    }

    pub fn check_ref(&self) -> Result<(RuntimeValue, Option<usize>), ZephyrError> {
        match self {
            RuntimeValue::Reference(r) => Ok(((*r.get()?).clone(), Some(r.location))),
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

        return Ok(match (self.clone(), right.clone(), t.clone()) {
            (RuntimeValue::Number(l), RuntimeValue::Number(r), _) => match t {
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
            _ => {
                return Err(ZephyrError {
                    code: ErrorCode::InvalidOperation,
                    message: format!(
                        "Cannot perform {} {} {}",
                        self.type_name(),
                        t,
                        right.type_name()
                    ),
                    location,
                })
            }
        });
    }
}

#[derive(Debug, Clone)]
pub struct Number {
    pub options: RuntimeValueDetails,
    pub value: f64,
}

impl Number {
    pub fn new(value: f64) -> RuntimeValue {
        RuntimeValue::Number(Number {
            value,
            options: RuntimeValueDetails::default(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ZString {
    pub options: RuntimeValueDetails,
    pub value: String,
}

impl ZString {
    pub fn new(value: String) -> RuntimeValue {
        RuntimeValue::ZString(ZString {
            value,
            options: RuntimeValueDetails::default(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Null {
    pub options: RuntimeValueDetails,
}

impl Null {
    pub fn new() -> RuntimeValue {
        RuntimeValue::Null(Null {
            options: RuntimeValueDetails::default(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Boolean {
    pub options: RuntimeValueDetails,
    pub value: bool,
}

impl Boolean {
    pub fn new(value: bool) -> RuntimeValue {
        RuntimeValue::Boolean(Boolean {
            value,
            options: RuntimeValueDetails::default(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub options: RuntimeValueDetails,
    pub body: nodes::Block,
    pub name: Option<String>,
    pub arguments: Vec<String>,
    pub scope: Arc<Mutex<Scope>>,
    pub args: Vec<Symbol>,
}

#[derive(Debug, Clone)]
pub struct Array {
    pub options: RuntimeValueDetails,
    pub items: Vec<RuntimeValue>,
}

impl Array {
    pub fn new(items: Vec<RuntimeValue>) -> RuntimeValue {
        RuntimeValue::Array(Array {
            items,
            options: RuntimeValueDetails::default(),
        })
    }

    pub fn new_ref(items: Vec<RuntimeValue>) -> RuntimeValue {
        Reference::new_from(Array::new(items))
    }
}

#[derive(Debug, Clone)]
pub struct Object {
    pub options: RuntimeValueDetails,
    pub items: HashMap<String, RuntimeValue>,
}

impl Object {
    pub fn new(items: HashMap<String, RuntimeValue>) -> RuntimeValue {
        RuntimeValue::Object(Object {
            items,
            options: RuntimeValueDetails::default(),
        })
    }

    pub fn new_ref(items: HashMap<String, RuntimeValue>) -> RuntimeValue {
        Reference::new_from(Object::new(items))
    }
}

#[derive(Debug, Clone)]
pub struct Reference {
    pub options: RuntimeValueDetails,
    pub location: usize,
}

impl Reference {
    pub fn new(location: usize) -> RuntimeValue {
        RuntimeValue::Reference(Reference {
            location,
            options: RuntimeValueDetails::default(),
        })
    }

    pub fn new_from(value: RuntimeValue) -> RuntimeValue {
        RuntimeValue::Reference(Reference {
            location: allocate(value),
            options: RuntimeValueDetails::default(),
        })
    }

    pub fn get(&self) -> Result<Arc<RuntimeValue>, ZephyrError> {
        match memory_store::OBJECT_STORE
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .get(self.location)
        {
            Some(ok) => {
                let res = ok.as_ref().and_then(|x| Some(x.clone()));

                Ok(Arc::clone(&res.unwrap()))
            }
            None => Err(ZephyrError {
                code: ErrorCode::UnknownReference,
                message: format!("Cannot find refernce &{}", self.location),
                location: None,
            }),
        }
    }
}
