use super::{RuntimeValue, RuntimeValueDetails, RuntimeValueUtils};

#[derive(Debug, Clone)]
pub struct Null {
    pub options: RuntimeValueDetails,
}

impl Null {
    pub fn new() -> RuntimeValue {
        RuntimeValue::Null(Null {
            options: RuntimeValueDetails::default(),
        })
    }
}

impl RuntimeValueUtils for Null {
    fn type_name(&self) -> &str {
        "null"
    }
}
