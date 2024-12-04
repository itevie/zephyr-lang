use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::errors::{ErrorCode, ZephyrError};

use super::values::{self, RuntimeValue};

#[derive(Debug)]
pub struct Variable {
    pub is_const: bool,
    pub value: RuntimeValue,
}

impl Variable {
    pub fn from(value: RuntimeValue) -> Self {
        Self {
            is_const: false,
            value,
        }
    }
}

#[derive(Debug)]
pub enum ScopeType {
    Normal,
    Global,
    Package,
}

#[derive(Debug)]
pub struct Scope {
    pub parent: Option<Arc<Mutex<Scope>>>,
    pub variables: HashMap<String, Variable>,
    pub scope_type: ScopeType,
}

impl Scope {
    pub fn new(parent: Option<Arc<Mutex<Scope>>>) -> Self {
        if let Some(p) = parent {
            Scope {
                parent: Some(p),
                variables: HashMap::new(),
                scope_type: ScopeType::Normal,
            }
        } else {
            Scope {
                parent: None,
                variables: HashMap::from([
                    (
                        "true".to_string(),
                        Variable::from(values::Boolean::new(true)),
                    ),
                    (
                        "false".to_string(),
                        Variable::from(values::Boolean::new(false)),
                    ),
                    ("null".to_string(), Variable::from(values::Null::new())),
                ]),
                scope_type: ScopeType::Normal,
            }
        }
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
