use crate::runtime::scope::PrototypeStore;

use super::{RuntimeValue, RuntimeValueDetails};

#[derive(Debug, Clone)]
pub struct Number {
    pub options: RuntimeValueDetails,
    pub value: f64,
}

impl Number {
    pub fn new(value: f64) -> RuntimeValue {
        RuntimeValue::Number(Number {
            value,
            options: RuntimeValueDetails::with_proto(PrototypeStore::get("object".to_string())),
        })
    }
}
