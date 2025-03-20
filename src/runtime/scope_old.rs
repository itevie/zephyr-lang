use std::{
    collections::HashMap,
    os::unix::raw::time_t,
    rc::Rc,
    sync::{atomic::AtomicU128, Arc, LazyLock, Mutex, OnceLock, RwLock},
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

type ScopeId = u128;

static PROTOTYPE_STORE: OnceLock<Mutex<HashMap<String, usize>>> = OnceLock::new();

static NEXT_SCOPE_ID: AtomicU128 = AtomicU128::new(1);
static VARIABLE_STORE: LazyLock<RwLock<HashMap<ScopeId, ScopeData>>> =
    LazyLock::new(|| Default::default());

fn create_variable_store() -> ScopeId {
    let next_id = NEXT_SCOPE_ID.fetch_add();

    next_id
}

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
pub struct ScopeData {
    pub parent: Option<ScopeId>,
    pub variables: HashMap<String, Variable>,
}

#[derive(Debug, Clone)]
pub struct Scope {
    pub id: ScopeId,
    pub parent: Option<ScopeId>,
    pub exported: HashMap<String, Option<String>>,
    pub scope_type: ScopeType,
    pub file_name: String,
}

impl Scope {
    pub fn new(file_name: String) -> Self {
        Scope {
            id: create_variable_store(),
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

    pub fn new_from_parent(parent: Arc<Mutex<Scope>>) -> Self {
        let file_name = parent.lock().unwrap().file_name.clone();
        Self::new_from_parent_new_file_name(parent.lock().unwrap().id, file_name)
    }

    pub fn new_from_parent_new_file_name(parent: ScopeId, file_name: String) -> Self {
        Scope {
            id: create_variable_store(),
            parent: Some(parent),
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
                let lock = VARIABLE_STORE.read().unwrap();
                let mut scope = Some(lock.get(&self.id).unwrap());

                while let Some(s) = scope {
                    if let Some(val) = s.variables.get(&name) {
                        return Ok(val.value.clone());
                    }

                    if let Some(ref parent) = s.parent {
                        scope = Some(lock.get(&parent).unwrap());
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
                let mut scope = Some(Arc::from(Mutex::from(self.clone())));

                while let Some(s) = scope.clone() {
                    let mut lock = s.lock().unwrap();
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

    pub fn get(name: String) -> usize {
        if let Some(proto) = PROTOTYPE_STORE.get().unwrap().lock().unwrap().get(&name) {
            *proto
        } else {
            panic!("Tried to get prototype {}, but it does not exist.", name)
        }
    }
}
