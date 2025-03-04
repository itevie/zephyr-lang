use std::sync::{Arc, Mutex};

use crate::{
    errors::{ErrorCode, ZephyrError},
    parser::nodes,
    runtime::{native::NativeExecutionContext, scope::Scope, R},
    util::colors,
};

use super::{RuntimeValue, RuntimeValueDetails, RuntimeValueUtils};

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

impl RuntimeValueUtils for Function {
    fn type_name(&self) -> &str {
        "function"
    }

    fn to_string(&self, is_display: bool, color: bool) -> Result<String, ZephyrError> {
        let string = format!(
            "{}",
            self.arguments
                .iter()
                .map(|x| format!("\"{}\"", x))
                .collect::<Vec<String>>()
                .join(", ")
        );

        Ok(match color {
            true => {
                format!(
                    "{}Function<{}{}{}>{}",
                    colors::FG_CYAN,
                    colors::FG_YELLOW,
                    string,
                    colors::FG_CYAN,
                    colors::COLOR_RESET
                )
            }
            false => format!("Function<{}>", string),
        })
    }
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

impl RuntimeValueUtils for NativeFunction {
    fn type_name(&self) -> &str {
        "native_function"
    }

    fn to_string(&self, is_display: bool, color: bool) -> Result<String, ZephyrError> {
        Ok(match color {
            true => format!(
                "{}{}{}",
                colors::FG_CYAN,
                "NativeFunction<>",
                colors::COLOR_RESET
            ),
            false => "NativeFunction<>".to_string(),
        })
    }
}

impl std::fmt::Debug for NativeFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NativeFunction")
    }
}
