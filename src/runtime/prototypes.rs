use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

use super::values;
use super::values::RuntimeValueUtils;

static PROTOTYPE_STORE: OnceLock<Mutex<HashMap<String, usize>>> = OnceLock::new();

pub struct PrototypeStore {}

macro_rules! make_proto {
    ($name:expr) => {
        (
            $name.to_string(),
            values::Object::new(HashMap::from([])).as_ref(),
        )
    };
}

impl PrototypeStore {
    pub fn init() {
        PROTOTYPE_STORE.get_or_init(|| {
            Mutex::from(HashMap::from([
                make_proto!("string"),
                make_proto!("array"),
                make_proto!("object"),
                make_proto!("event_emitter"),
                make_proto!("any"),
            ]))
        });
    }

    pub fn get<T: Into<String>>(name: T) -> usize {
        let name = name.into();

        *PROTOTYPE_STORE
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .get(&name)
            .unwrap_or_else(|| panic!("Tried to get prototype {}, but it does not exist.", name))
    }
}
