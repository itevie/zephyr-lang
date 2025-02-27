use crate::runtime::scope::PrototypeStore;

use super::{RuntimeValue, RuntimeValueDetails};

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
