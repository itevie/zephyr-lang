use crate::runtime::scope::PrototypeStore;

use super::{Reference, RuntimeValue, RuntimeValueDetails, RuntimeValueUtils};

#[derive(Debug, Clone)]
pub struct Array {
    pub options: RuntimeValueDetails,
    pub items: Vec<RuntimeValue>,
}

impl Array {
    pub fn new(items: Vec<RuntimeValue>) -> RuntimeValue {
        RuntimeValue::Array(Array {
            items,
            options: RuntimeValueDetails::with_proto(PrototypeStore::get("array".to_string())),
        })
    }

    pub fn new_ref(items: Vec<RuntimeValue>) -> RuntimeValue {
        Reference::new_from(Array::new(items))
    }
}

impl RuntimeValueUtils for Array {
    fn type_name(&self) -> &str {
        "array"
    }

    fn iter(&self) -> Result<Vec<RuntimeValue>, crate::errors::ZephyrError> {
        Ok(self.items.clone())
    }
}
