use std::collections::HashMap;

use super::{Reference, RuntimeValue, RuntimeValueDetails};

#[derive(Debug, Clone)]
pub struct Object {
    pub options: RuntimeValueDetails,
    pub items: HashMap<String, RuntimeValue>,
}

impl Object {
    pub fn new(items: HashMap<String, RuntimeValue>) -> RuntimeValue {
        RuntimeValue::Object(Object {
            items,
            options: RuntimeValueDetails::default(),
        })
    }

    pub fn new_ref(items: HashMap<String, RuntimeValue>) -> RuntimeValue {
        Reference::new_from(Object::new(items))
    }
}
