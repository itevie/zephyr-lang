use super::{RuntimeValue, RuntimeValueDetails};

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
