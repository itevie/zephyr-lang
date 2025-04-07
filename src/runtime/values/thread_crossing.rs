use super::{
    FunctionInner, FunctionType, MspcSenderType, NativeFunctionType, RuntimeValue,
    RuntimeValueUtils,
};

#[derive(Debug, Clone)]
pub enum ThreadRuntimeValue {
    Number(f64),
    ZString(String),
}

impl From<&ThreadRuntimeValue> for RuntimeValue {
    fn from(value: &ThreadRuntimeValue) -> Self {
        match value {
            ThreadRuntimeValue::Number(v) => super::Number::new(*v).wrap(),
            ThreadRuntimeValue::ZString(v) => super::ZString::new(v.clone()).wrap(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ThreadRuntimeValueArray(Vec<ThreadRuntimeValue>);

impl ThreadRuntimeValueArray {
    pub fn new<T: Into<Vec<ThreadRuntimeValue>>>(values: T) -> Self {
        Self(values.into())
    }
}

impl From<ThreadRuntimeValueArray> for Vec<RuntimeValue> {
    fn from(value: ThreadRuntimeValueArray) -> Self {
        value.0.iter().map(|x| RuntimeValue::from(x)).collect()
    }
}

impl From<Vec<ThreadRuntimeValue>> for ThreadRuntimeValueArray {
    fn from(value: Vec<ThreadRuntimeValue>) -> Self {
        Self(value)
    }
}

/* ----- Functions ----- */
#[derive(Clone)]
pub enum ThreadRuntimeFunctionType {
    Native(NativeFunctionType),
    Function(FunctionInner),
    Mspc(MspcSenderType),
}

impl std::fmt::Debug for ThreadRuntimeFunctionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NativeFunction")
    }
}

impl From<FunctionType> for ThreadRuntimeFunctionType {
    fn from(value: FunctionType) -> Self {
        match value {
            FunctionType::Function(f) => Self::Function(f.inner),
            FunctionType::NativeFunction(f) => Self::Native(f.func),
            FunctionType::MspcSender(f) => Self::Mspc(f.sender),
        }
    }
}

impl From<ThreadRuntimeFunctionType> for FunctionType {
    fn from(value: ThreadRuntimeFunctionType) -> Self {
        match value {
            ThreadRuntimeFunctionType::Function(f) => FunctionType::Function(super::Function {
                inner: f,
                options: Default::default(),
            }),
            ThreadRuntimeFunctionType::Native(f) => {
                FunctionType::NativeFunction(super::NativeFunction {
                    func: f,
                    options: Default::default(),
                })
            }
            ThreadRuntimeFunctionType::Mspc(f) => FunctionType::MspcSender(super::MspcSender {
                sender: f,
                options: Default::default(),
            }),
        }
    }
}
