use crate::runtime::scope::PrototypeStore;

use super::{RuntimeValue, RuntimeValueDetails, RuntimeValueUtils};

#[derive(Debug, Clone)]
pub struct ZString {
    pub options: RuntimeValueDetails,
    pub value: String,
}

impl ZString {
    pub fn new(value: String) -> RuntimeValue {
        RuntimeValue::ZString(ZString {
            value,
            options: RuntimeValueDetails::with_proto(PrototypeStore::get("string".to_string())),
        })
    }
}

impl RuntimeValueUtils for ZString {
    fn type_name(&self) -> &str {
        "string"
    }

    fn iter(&self) -> Result<Vec<RuntimeValue>, crate::errors::ZephyrError> {
        Ok(self
            .value
            .chars()
            .map(|v| ZString::new(v.to_string()))
            .collect::<Vec<RuntimeValue>>())
    }
}
