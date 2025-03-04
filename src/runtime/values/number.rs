use crate::runtime::scope::PrototypeStore;

use super::{RuntimeValue, RuntimeValueDetails, RuntimeValueUtils};

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

impl RuntimeValueUtils for Number {
    fn type_name(&self) -> &str {
        "number"
    }
}
