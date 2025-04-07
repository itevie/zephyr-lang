use std::{cell::RefCell, collections::HashMap, rc::Rc};

use super::RuntimeValue;

#[derive(Debug, Clone)]
pub struct RuntimeValueDetails {
    pub tags: Rc<RefCell<HashMap<String, String>>>,
    pub proto: Rc<RefCell<Option<String>>>,
    pub proto_value: Option<Box<RuntimeValue>>,
}

impl RuntimeValueDetails {
    pub fn with_proto(id: String) -> Self {
        Self {
            proto: Rc::from(RefCell::from(Some(id))),
            ..Default::default()
        }
    }
}

impl Default for RuntimeValueDetails {
    fn default() -> Self {
        Self {
            tags: Rc::from(RefCell::from(HashMap::default())),
            proto: Rc::from(RefCell::from(None)),
            proto_value: None,
        }
    }
}
