use std::{cell::RefCell, collections::HashMap, rc::Rc};

use super::values::Object;

#[derive(Debug, Clone)]
pub struct PrototypeStore {
    pub prototype_mapping: Rc<RefCell<HashMap<String, Object>>>,
}

impl PrototypeStore {
    pub fn new() -> Self {
        Self {
            prototype_mapping: Rc::from(RefCell::from(HashMap::from(
                [
                    "any",
                    "event_emitter",
                    "string",
                    "array",
                    "number",
                    "enum",
                    "object",
                ]
                .iter()
                .map(|x| (x.to_string(), Object::new_empty()))
                .collect::<HashMap<String, Object>>(),
            ))),
        }
    }

    pub fn get<T: Into<String>>(&self, name: T) -> Object {
        let inner = name.into();
        self.prototype_mapping
            .borrow()
            .get(&inner)
            .unwrap_or_else(|| panic!("Couldn't find prototype for {}", inner))
            .clone()
    }

    pub fn set<T: Into<String>>(&self, name: T, value: Object) -> Object {
        self.prototype_mapping
            .borrow_mut()
            .insert(name.into(), value.clone());
        value
    }
}
