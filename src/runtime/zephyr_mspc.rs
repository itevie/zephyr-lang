use std::sync::mpsc::Sender;

use super::values::thread_crossing::{ThreadRuntimeFunctionType, ThreadRuntimeValueArray};

#[derive(Debug, Clone)]
pub struct Job {
    pub func: ThreadRuntimeFunctionType,
    pub args: ThreadRuntimeValueArray,
}

#[derive(Debug, Clone)]
pub enum MspcSendType {
    ThreadCreate,
    ThreadDestroy,
    ThreadMessage(Job),
}

#[derive(Debug, Clone)]
pub struct MspcChannel {
    pub mspc: Sender<MspcSendType>,
}

impl MspcChannel {
    pub fn thread_start(&mut self) {
        self.mspc
            .send(MspcSendType::ThreadCreate)
            .unwrap_or_else(|e| panic!("Failed to send thread_start {:#?}", e.0))
    }

    pub fn thread_destroy(&mut self) {
        self.mspc
            .send(MspcSendType::ThreadDestroy)
            .unwrap_or_else(|e| panic!("Failed to send thread_destroy {:#?}", e.0))
    }

    pub fn thread_message(&mut self, job: Job) {
        self.mspc
            .send(MspcSendType::ThreadMessage(job))
            .unwrap_or_else(|e| panic!("Failed to send thread_message: {:?}", e))
    }
}
