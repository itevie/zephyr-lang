use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use super::{FunctionInner, FunctionType, MspcSenderType, NativeFunctionType, RuntimeValue, RuntimeValueDetails, RuntimeValueUtils};

#[derive(Debug, Clone)]
pub struct ThreadRuntimeValue {
    pub value: ThreadInnerValue,
}

impl ThreadRuntimeValue {
    pub fn new(inner: ThreadInnerValue) -> Self {
        ThreadRuntimeValue {
            value: inner,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ThreadInnerValue {
    Number(f64),
    ZString(String),
    Null,
}

impl From<&ThreadRuntimeValue> for RuntimeValue {
    fn from(value: &ThreadRuntimeValue) -> Self {
        match &value.value {
            ThreadInnerValue::Number(v) => super::Number::new(*v).wrap(),
            ThreadInnerValue::ZString(v) => super::ZString::new(v.clone()).wrap(),
            ThreadInnerValue::Null => super::Null::new().wrap(),
        }
    }
}

impl From<&RuntimeValue> for ThreadRuntimeValue {
    fn from(value: &RuntimeValue) -> Self {
        match value {
            _ => panic!("Not able to convert {} to a thread value yet", value.type_name())
        }
    }
}


// #[derive(Debug, Clone)]
// pub struct ThreadRuntimeValueDetails {
//     pub tags: Arc<Mutex<HashMap<String, String>>>,
//     pub proto: Arc<Mutex<Option<String>>>,
//     pub proto_value: Option<Box<ThreadRuntimeValue>>,
// }
//
// impl From<RuntimeValueDetails> for ThreadRuntimeValueDetails {
//     fn from(value: RuntimeValueDetails) -> Self {
//         ThreadRuntimeValueDetails {
//             tags: Arc::from(Mutex::from(value.tags.borrow().clone())),
//             proto: Arc::from(Mutex::from(value.proto.borrow().clone())),
//             proto_value: value.proto_value.clone(),
//         }
//     }
// }








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
