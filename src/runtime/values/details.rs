use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use super::RuntimeValue;

#[derive(Debug, Clone)]
pub struct RuntimeValueDetails {
    pub tags: Arc<Mutex<HashMap<String, String>>>,
    pub proto: Option<usize>,
    pub proto_value: Option<Box<RuntimeValue>>,
}

impl RuntimeValueDetails {
    pub fn with_proto(id: usize) -> Self {
        Self {
            proto: Some(id),
            ..Default::default()
        }
    }
}

impl Default for RuntimeValueDetails {
    fn default() -> Self {
        Self {
            tags: Arc::from(Mutex::from(HashMap::default())),
            proto: None,
            proto_value: None,
        }
    }
}
