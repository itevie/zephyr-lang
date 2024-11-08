use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::errors::{ErrorCode, ZephyrError};

use super::values::RuntimeValue;

pub struct Variable {
    pub is_const: bool,
    pub value: RuntimeValue,
}

pub struct Scope {
    pub parent: Option<Arc<Mutex<Scope>>>,
    pub variables: HashMap<String, Variable>,
}

impl Scope {
    pub fn new(parent: Option<Arc<Mutex<Scope>>>) -> Self {
        let scope = Scope {
            parent,
            variables: HashMap::new(),
        };

        scope
    }

    pub fn lookup(&self, name: String) -> Result<RuntimeValue, ZephyrError> {
        if let Some(variable) = self.variables.get(&name) {
            Ok(variable.value.clone())
        } else if let Some(ref parent) = self.parent {
            parent.lock().unwrap().lookup(name)
        } else {
            Err(ZephyrError {
                code: ErrorCode::UnknownReference,
                message: format!("Cannot find variable {} in the current scope", name),
                location: None,
            })
        }
    }

    pub fn insert(&mut self, name: String, variable: Variable) -> Result<(), ZephyrError> {
        if self.variables.contains_key(&name) {
            return Err(ZephyrError {
                code: ErrorCode::AlreadyDefined,
                message: format!("Variable {} already exists in the current scope", name),
                location: None,
            });
        }

        self.variables.insert(name, variable);
        Ok(())
    }

    pub fn modify(&mut self, name: String, value: RuntimeValue) -> Result<(), ZephyrError> {
        if let Some(variable) = self.variables.get_mut(&name) {
            if variable.is_const {
                return Err(ZephyrError {
                    code: ErrorCode::ConstantAssignment,
                    message: format!("Variable {} is constant", name),
                    location: None,
                });
            }
            variable.value = value;
        } else if let Some(ref parent) = self.parent {
            parent.lock().unwrap().modify(name, value)?;
        } else {
            return Err(ZephyrError {
                code: ErrorCode::UnknownReference,
                message: format!("Cannot find variable {} in the current scope", name),
                location: None,
            });
        }

        Ok(())
    }
}
