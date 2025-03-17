use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use super::RuntimeValue;

#[derive(Debug, Clone)]
pub struct RuntimeValueDetails {
    pub tags: Arc<Mutex<HashMap<String, String>>>,
    pub proto: Arc<Mutex<Option<usize>>>,
    pub proto_value: Option<Box<RuntimeValue>>,
}

impl RuntimeValueDetails {
    pub fn with_proto(id: usize) -> Self {
        Self {
            proto: Arc::from(Mutex::from(Some(id))),
            ..Default::default()
        }
    }
}

impl Default for RuntimeValueDetails {
    fn default() -> Self {
        Self {
            tags: Arc::from(Mutex::from(HashMap::default())),
            proto: Arc::from(Mutex::from(None)),
            proto_value: None,
        }
    }
}
