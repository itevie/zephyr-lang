use std::{collections::HashMap, sync::Arc};

use crate::{
    errors::{ErrorCode, ZephyrError},
    parser::nodes,
};

use super::memory_store::{self, allocate};

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

    pub fn check_ref(&self) -> Result<(RuntimeValue, Option<usize>), ZephyrError> {
        match self {
            RuntimeValue::Reference(r) => Ok(((*r.get()?).clone(), Some(r.location))),
            _ => Ok((self.clone(), None)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Number {
    pub value: f64,
}

impl Number {
    pub fn new(value: f64) -> RuntimeValue {
        RuntimeValue::Number(Number { value })
    }
}

#[derive(Debug, Clone)]
pub struct ZString {
    pub value: String,
}

impl ZString {
    pub fn new(value: String) -> RuntimeValue {
        RuntimeValue::ZString(ZString { value })
    }
}

#[derive(Debug, Clone)]
pub struct Null {}

impl Null {
    pub fn new() -> RuntimeValue {
        RuntimeValue::Null(Null {})
    }
}

#[derive(Debug, Clone)]
pub struct Boolean {
    pub value: bool,
}

impl Boolean {
    pub fn new(value: bool) -> RuntimeValue {
        RuntimeValue::Boolean(Boolean { value })
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub body: nodes::Block,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Array {
    pub items: Vec<RuntimeValue>,
}

impl Array {
    pub fn new(items: Vec<RuntimeValue>) -> RuntimeValue {
        RuntimeValue::Array(Array { items })
    }

    pub fn new_ref(items: Vec<RuntimeValue>) -> RuntimeValue {
        Reference::new_from(Array::new(items))
    }
}

#[derive(Debug, Clone)]
pub struct Object {
    pub items: HashMap<String, RuntimeValue>,
}

impl Object {
    pub fn new(items: HashMap<String, RuntimeValue>) -> RuntimeValue {
        RuntimeValue::Object(Object { items })
    }

    pub fn new_ref(items: HashMap<String, RuntimeValue>) -> RuntimeValue {
        Reference::new_from(Object::new(items))
    }
}

#[derive(Debug, Clone)]
pub struct Reference {
    pub location: usize,
}

impl Reference {
    pub fn new(location: usize) -> RuntimeValue {
        RuntimeValue::Reference(Reference { location })
    }

    pub fn new_from(value: RuntimeValue) -> RuntimeValue {
        RuntimeValue::Reference(Reference {
            location: allocate(value),
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
