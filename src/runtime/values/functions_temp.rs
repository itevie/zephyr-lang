use std::sync::{mpsc::Sender, Arc};

use crate::{
    lexer::tokens::Location,
    parser::nodes,
    runtime::{native::NativeExecutionContext, scope::ScopeInnerType, R},
};

use super::{thread_crossing::ThreadRuntimeValue, RuntimeValueDetails, RuntimeValueUtils};

pub type NativeFunctionType = Arc<dyn Fn(NativeExecutionContext) -> R + Send + Sync>;

#[derive(Clone)]
pub struct NativeFunction(NativeFunctionType);

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

#[derive(Debug, Clone)]
pub struct MspcSender(MspcSenderType);

#[derive(Debug, Clone)]
pub struct FunctionInner {
    pub body: nodes::Block,
    pub name: Option<String>,
    pub arguments: Vec<String>,
    pub scope: ScopeInnerType,
}

#[derive(Debug, Clone)]
pub enum FunctionType {
    Function(FunctionInner),
    Native(NativeFunction),
    MspcSender(MspcSender),
}

#[derive(Debug, Clone)]
pub struct Function {
    pub options: RuntimeValueDetails,
    pub func: FunctionType,
}
