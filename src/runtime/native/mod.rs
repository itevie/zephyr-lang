use crate::{
    errors::{ErrorCode, ZephyrError},
    lexer::tokens::Location,
};

use super::{values::RuntimeValue, Interpreter};

pub mod events;
pub mod proto;
pub mod test;

pub struct NativeExecutionContext {
    pub interpreter: Interpreter,
    pub args: Vec<RuntimeValue>,
    pub location: Location,
}

pub fn make_no_args_error(location: Location) -> ZephyrError {
    ZephyrError {
        message: "Invalid args".to_string(),
        code: ErrorCode::TypeError,
        location: Some(location),
    }
}
