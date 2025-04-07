use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex, OnceLock},
    time::Instant,
};

use crate::{
    errors::{ErrorCode, ZephyrError},
    lexer::tokens::Location,
};

use super::{
    insert_node_timing, time_this,
    values::{self, RuntimeValue, RuntimeValueUtils},
};

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

pub type ScopeInnerType = Rc<RefCell<Scope>>;

#[derive(Debug, Clone)]
pub struct Scope {
    pub parent: Option<ScopeInnerType>,
    pub variables: HashMap<String, Variable>,
    pub exported: HashMap<String, Option<String>>,
    pub scope_type: ScopeType,
    pub file_name: String,
}

impl Scope {
    pub fn new(file_name: String) -> Self {
        Scope {
            exported: HashMap::new(),
            parent: None,
            variables: HashMap::from([
                (
                    "true".to_string(),
                    Variable::from(values::Boolean::new(true).wrap()),
                ),
                (
                    "false".to_string(),
                    Variable::from(values::Boolean::new(false).wrap()),
                ),
                (
                    "null".to_string(),
                    Variable::from(values::Null::new().wrap()),
                ),
            ]),
            scope_type: ScopeType::Normal,
            file_name,
        }
    }

    pub fn new_from_parent(parent: ScopeInnerType) -> Self {
        let file_name = parent.borrow().file_name.clone();
        Self::new_from_parent_new_file_name(parent, file_name)
    }

    pub fn new_from_parent_new_file_name(parent: ScopeInnerType, file_name: String) -> Self {
        Scope {
            parent: Some(parent.clone()),
            variables: HashMap::new(),
            scope_type: ScopeType::Normal,
            exported: HashMap::new(),
            file_name,
        }
    }

    pub fn lookup(
        &self,
        name: String,
        location: Option<Location>,
    ) -> Result<RuntimeValue, ZephyrError> {
        time_this!(
            "Mini:ScopeLookup".to_string(),
            (|| {
                let mut scope = Some(Rc::from(RefCell::from(self.clone())));

                while let Some(s) = scope.clone() {
                    let lock = s.borrow();
                    if let Some(val) = lock.variables.get(&name) {
                        return Ok(val.value.clone());
                    }

                    if let Some(ref parent) = lock.parent {
                        scope = Some(parent.clone());
                    }
                }

                Err(ZephyrError {
                    code: ErrorCode::UnknownReference,
                    message: format!("Cannot find variable {} in the current scope", name),
                    location,
                })
            })()
        )
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

        if self.variables.contains_key(&name) {
            return Err(ZephyrError {
                code: ErrorCode::AlreadyDefined,
                message: format!("Variable {} already exists in the current scope", name),
                location,
            });
        }

        self.variables.insert(name, variable);
        Ok(())
    }

    pub fn modify(
        &mut self,
        name: String,
        value: RuntimeValue,
        location: Option<Location>,
    ) -> Result<(), ZephyrError> {
        time_this!(
            "Mini:ScopeModify".to_string(),
            (|| {
                let mut scope = Some(Rc::from(RefCell::from(self.clone())));

                while let Some(s) = scope.clone() {
                    let mut lock = s.borrow_mut();
                    if let Some(val) = lock.variables.get_mut(&name) {
                        val.value = value;
                        return Ok(());
                    }

                    if let Some(ref parent) = lock.parent {
                        scope = Some(parent.clone());
                    }
                }

                Err(ZephyrError {
                    code: ErrorCode::UnknownReference,
                    message: format!("Cannot find variable {} in the current scope", name),
                    location,
                })
            })()
        )
    }
}
/*
pub struct PrototypeStore {}

macro_rules! make_proto {
    ($name:expr) => {
        (
            $name.to_string(),
            values::Object::new(HashMap::from([])).wrap(),
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

    pub fn get(name: String) -> usize {
        if let Some(proto) = PROTOTYPE_STORE.get().unwrap().lock().unwrap().get(&name) {
            *proto
        } else {
            panic!("Tried to get prototype {}, but it does not exist.", name)
        }
    }
}
*/
