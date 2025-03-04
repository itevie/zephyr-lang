use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{
    errors::{ErrorCode, ZephyrError},
    lexer::tokens::Location,
    runtime::{scope::PrototypeStore, Job, MspcChannel},
};

use super::{FunctionType, RuntimeValue, RuntimeValueDetails, RuntimeValueUtils};

#[derive(Debug, Clone)]
pub struct EventEmitter {
    pub options: RuntimeValueDetails,
    pub defined_events: Vec<String>,
    pub listeners: Arc<Mutex<HashMap<String, Arc<Mutex<Vec<FunctionType>>>>>>,
}

impl EventEmitter {
    pub fn new(events: Vec<String>) -> Self {
        EventEmitter {
            options: RuntimeValueDetails::with_proto(PrototypeStore::get(
                "event_emitter".to_string(),
            )),
            defined_events: events,
            listeners: Arc::from(Mutex::from(HashMap::new())),
        }
    }

    pub fn emit_from_thread(
        &self,
        message: String,
        args: Vec<RuntimeValue>,
        sender: &mut MspcChannel,
    ) -> () {
        if let Some(listeners) = self.listeners.lock().unwrap().get(&message) {
            let parts = listeners.lock().unwrap();
            for part in parts.iter() {
                sender.thread_message(Job {
                    func: part.clone(),
                    args: args.clone(),
                });
            }
        }
    }

    pub fn add_listener(
        &self,
        message: String,
        func: FunctionType,
        location: Location,
    ) -> Result<(), ZephyrError> {
        if !self.defined_events.contains(&message) {
            return Err(ZephyrError {
                message: format!("Event emitter does not have a {} event", message),
                code: ErrorCode::UndefinedEventMessage,
                location: Some(location),
            });
        }

        let mut lock = self.listeners.lock().unwrap();

        if !lock.contains_key(&message) {
            lock.insert(message, Arc::from(Mutex::from(vec![func])));
        } else {
            lock.get(&message).unwrap().lock().unwrap().push(func);
        }

        Ok(())
    }
}

impl RuntimeValueUtils for EventEmitter {
    fn type_name(&self) -> &str {
        "event_emitter"
    }
}
