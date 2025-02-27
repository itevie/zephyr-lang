use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{Mutex, OnceLock},
};

use crate::{
    errors::{ErrorCode, ZephyrError},
    lexer::tokens::Location,
};

use super::values::{self, RuntimeValue};

static PROTOTYPE_STORE: OnceLock<Mutex<HashMap<String, usize>>> = OnceLock::new();

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub enum ScopeType {
    Normal,
    Global,
    Package,
}

#[derive(Debug, Clone)]
pub struct Scope {
    pub parent: Option<Box<Scope>>,
    pub variables: RefCell<HashMap<String, Variable>>,
    pub exported: RefCell<HashMap<String, Option<String>>>,
    pub scope_type: ScopeType,
    pub file_name: String,
}

impl Scope {
    pub fn new(file_name: String) -> Self {
        Scope {
            exported: HashMap::new().into(),
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
            ])
            .into(),
            scope_type: ScopeType::Normal,
            file_name: file_name,
        }
    }

    pub fn new_from_parent(parent: Box<Scope>) -> Self {
        let file_name = parent.file_name.clone();
        Self::new_from_parent_new_file_name(parent, file_name)
    }

    pub fn new_from_parent_new_file_name(parent: Box<Scope>, file_name: String) -> Self {
        Scope {
            parent: Some(parent.clone()),
            variables: HashMap::new().into(),
            scope_type: ScopeType::Normal,
            exported: HashMap::new().into(),
            file_name,
        }
    }

    pub fn lookup(
        &self,
        name: String,
        location: Option<Location>,
    ) -> Result<RuntimeValue, ZephyrError> {
        if let Some(variable) = self.variables.borrow().get(&name) {
            Ok(variable.value.clone())
        } else if let Some(ref parent) = self.parent {
            parent.lookup(name, location)
        } else {
            Err(ZephyrError {
                code: ErrorCode::UnknownReference,
                message: format!("Cannot find variable {} in the current scope", name),
                location,
            })
        }
    }

    pub fn insert(
        &mut self,
        name: String,
        variable: Variable,
        location: Option<Location>,
    ) -> Result<(), ZephyrError> {
        if name == "_" {
            return Ok(());
        }

        if self.variables.borrow().contains_key(&name) {
            return Err(ZephyrError {
                code: ErrorCode::AlreadyDefined,
                message: format!("Variable {} already exists in the current scope", name),
                location,
            });
        }

        self.variables.borrow_mut().insert(name, variable);
        Ok(())
    }

    pub fn modify(
        &mut self,
        name: String,
        value: RuntimeValue,
        location: Option<Location>,
    ) -> Result<(), ZephyrError> {
        if let Some(variable) = self.variables.borrow_mut().get_mut(&name) {
            if variable.is_const {
                return Err(ZephyrError {
                    code: ErrorCode::ConstantAssignment,
                    message: format!("Variable {} is constant", name),
                    location,
                });
            }
            variable.value = value;
        } else if let Some(ref mut parent) = self.parent {
            parent.modify(name, value, location)?;
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
            ]))
        });
    }

    pub fn get(name: String) -> usize {
        if let Some(proto) = PROTOTYPE_STORE.get().unwrap().lock().unwrap().get(&name) {
            *proto
        } else {
            panic!("Tried to get prototype {}, but it does not exist.", name)
        }
    }
}
