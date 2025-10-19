use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use uuid::Uuid;
use crate::{
    errors::{ErrorCode, ZephyrError},
    lexer::tokens::Location,
    runtime::zephyr_mspc::{Job, MspcChannel},
    util::colors,
};
use crate::runtime::native::NativeExecutionContext;
use crate::runtime::values::thread_crossing::ThreadRuntimeValueArray;
use super::{
    // thread_crossing::{ThreadRuntimeFunctionType, ThreadRuntimeValueArray},
    FunctionType, RuntimeValue, RuntimeValueDetails, RuntimeValueUtils,
};

#[derive(Debug, Clone)]
pub struct EventEmitterForThreads {
    pub listeners: Arc<Mutex<HashMap<String, Arc<Mutex<Vec<Uuid>>>>>>,
}

impl EventEmitterForThreads {
    pub fn new() -> Self {
        Self {
            listeners: Default::default(),
        }
    }

    pub fn emit_from_thread(
        &self,
        message: &str,
        args: ThreadRuntimeValueArray,
        sender: &mut MspcChannel,
    ) {
        if let Some(listeners) = self.listeners.lock().unwrap().get(&message.to_string()) {
            let parts = listeners.lock().unwrap();
            for part in parts.iter() {
                sender.thread_message(Job {
                    func: part.clone(),
                    args: args.clone(),
                });
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct EventEmitter {
    pub options: RuntimeValueDetails,
    pub defined_events: Vec<String>,
    pub thread_part: EventEmitterForThreads,
}

impl EventEmitter {
    pub fn new(events: Vec<&str>) -> Self {
        EventEmitter {
            options: RuntimeValueDetails::with_proto("event_emitter".to_string()),
            defined_events: events.iter().map(|x| x.to_string()).collect(),
            thread_part: EventEmitterForThreads::new(),
        }
    }

    pub fn add_listener(
        &self,
        message: String,
        func: FunctionType,
        ctx: NativeExecutionContext
    ) -> Result<(), ZephyrError> {
        if !self.defined_events.contains(&message) {
            return Err(ZephyrError {
                message: format!("Event emitter does not have a {} event", message),
                code: ErrorCode::UndefinedEventMessage,
                location: Some(ctx.location.clone()),
            });
        }

        let func_uuid = ctx.interpreter.insert_function(func);
        let mut lock = self.thread_part.listeners.lock().unwrap();

        if !lock.contains_key(&message) {
            lock.insert(message, Arc::from(Mutex::from(vec![func_uuid])));
        } else {
            lock.get(&message)
                .unwrap()
                .lock()
                .unwrap()
                .push(func_uuid);
        }

        Ok(())
    }
}

impl RuntimeValueUtils for EventEmitter {
    fn type_name(&self) -> &str {
        "event_emitter"
    }

    fn wrap(&self) -> RuntimeValue {
        RuntimeValue::EventEmitter(self.clone())
    }

    fn to_string(&self, _is_display: bool, color: bool) -> Result<String, ZephyrError> {
        let keys = self
            .defined_events
            .iter()
            .map(|x| format!("\"{}\"", x))
            .collect::<Vec<String>>()
            .join(", ");

        Ok(match color {
            true => format!(
                "{}EventEmitter<{}{}{}{}>{}",
                colors::FG_CYAN,
                colors::FG_YELLOW,
                keys,
                colors::COLOR_RESET,
                colors::FG_CYAN,
                colors::COLOR_RESET,
            ),
            false => format!("EventEmitter<{}>", keys),
        })
    }
}
