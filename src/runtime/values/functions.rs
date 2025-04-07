use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc, Mutex,
};

use crate::{
    errors::{ErrorCode, ZephyrError},
    lexer::tokens::Location,
    parser::nodes,
    runtime::{
        native::NativeExecutionContext,
        scope::{Scope, ScopeInnerType},
        R,
    },
    util::colors,
};

use super::{
    thread_crossing::{ThreadRuntimeValue, ThreadRuntimeValueArray},
    RuntimeValue, RuntimeValueDetails, RuntimeValueUtils,
};

#[derive(Debug, Clone)]
pub enum FunctionType {
    Function(Function),
    NativeFunction(NativeFunction),
    MspcSender(MspcSender),
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
pub struct FunctionInner {
    pub body: nodes::Block,
    pub name: Option<String>,
    pub arguments: Vec<String>,
    pub scope: ScopeInnerType,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub options: RuntimeValueDetails,
    pub inner: FunctionInner,
}

impl RuntimeValueUtils for Function {
    fn type_name(&self) -> &str {
        "function"
    }

    fn wrap(&self) -> RuntimeValue {
        RuntimeValue::Function(self.clone())
    }

    fn to_string(&self, is_display: bool, color: bool) -> Result<String, ZephyrError> {
        let string = format!(
            "{}",
            self.inner
                .arguments
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

pub type NativeFunctionType = Arc<dyn Fn(NativeExecutionContext) -> R + Send + Sync>;

#[derive(Clone)]
pub struct NativeFunction {
    pub options: RuntimeValueDetails,
    pub func: NativeFunctionType,
}

impl NativeFunction {
    pub fn new(f: NativeFunctionType) -> Self {
        NativeFunction {
            func: f,
            options: RuntimeValueDetails::default(),
        }
    }
}

impl RuntimeValueUtils for NativeFunction {
    fn type_name(&self) -> &str {
        "native_function"
    }

    fn wrap(&self) -> RuntimeValue {
        RuntimeValue::NativeFunction(self.clone())
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

pub struct MspcSenderOptions {
    pub args: Vec<ThreadRuntimeValue>,
    pub location: Location,
}

pub type MspcSenderType = Sender<MspcSenderOptions>;

#[derive(Clone, Debug)]
pub struct MspcSender {
    pub options: RuntimeValueDetails,
    pub sender: MspcSenderType,
}

impl MspcSender {
    pub fn new(sender: MspcSenderType) -> Self {
        Self {
            sender,
            options: RuntimeValueDetails::default(),
        }
    }

    pub fn new_handled() -> (Receiver<MspcSenderOptions>, Self) {
        let (tx, rx) = channel();
        (rx, Self::new(tx))
    }
}

impl RuntimeValueUtils for MspcSender {
    fn type_name(&self) -> &str {
        "mspc_sender"
    }

    fn wrap(&self) -> RuntimeValue {
        RuntimeValue::MspcSender(self.clone())
    }

    fn to_string(&self, is_display: bool, color: bool) -> Result<String, ZephyrError> {
        Ok(match color {
            true => format!(
                "{}{}{}",
                colors::FG_CYAN,
                "MspcSender<>",
                colors::COLOR_RESET
            ),
            false => "MspcSender<>".to_string(),
        })
    }
}
