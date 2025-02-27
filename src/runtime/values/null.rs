use super::{RuntimeValue, RuntimeValueDetails};

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
