use std::sync::{Arc, Mutex};

use crate::{
    errors::{ErrorCode, ZephyrError},
    parser::nodes,
    runtime::{native::NativeExecutionContext, scope::Scope, R},
};

use super::{RuntimeValue, RuntimeValueDetails};

#[derive(Debug, Clone)]
pub enum FunctionType {
    Function(Function),
    NativeFunction(NativeFunction),
}

impl FunctionType {
    pub fn from(val: RuntimeValue) -> Result<FunctionType, ZephyrError> {
        match val {
            RuntimeValue::Function(f) => Ok(FunctionType::Function(f)),
            RuntimeValue::NativeFunction(f) => Ok(FunctionType::NativeFunction(f)),
            _ => Err(ZephyrError {
                message: format!("{} is not a function", val.type_name()),
                code: ErrorCode::TypeError,
                location: None,
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub options: RuntimeValueDetails,
    pub body: nodes::Block,
    pub name: Option<String>,
    pub arguments: Vec<String>,
    pub scope: Arc<Mutex<Scope>>,
}

#[derive(Clone)]
pub struct NativeFunction {
    pub options: RuntimeValueDetails,
    pub func: Arc<dyn Fn(NativeExecutionContext) -> R + Send + Sync>,
}

impl NativeFunction {
    pub fn new(f: Arc<dyn Fn(NativeExecutionContext) -> R + Send + Sync>) -> RuntimeValue {
        RuntimeValue::NativeFunction(NativeFunction {
            func: f,
            options: RuntimeValueDetails::default(),
        })
    }
}

impl std::fmt::Debug for NativeFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NativeFunction")
    }
}
