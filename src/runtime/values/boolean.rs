use super::{RuntimeValue, RuntimeValueDetails, RuntimeValueUtils};

#[derive(Debug, Clone)]
pub struct Boolean {
    pub options: RuntimeValueDetails,
    pub value: bool,
}

impl Boolean {
    pub fn new(value: bool) -> RuntimeValue {
        RuntimeValue::Boolean(Boolean {
            value,
            options: RuntimeValueDetails::default(),
        })
    }
}

impl RuntimeValueUtils for Boolean {
    fn type_name(&self) -> &str {
        "boolean"
    }
}
