use std::{
    collections::HashMap,
    env::var,
    sync::{
        atomic::{AtomicUsize, Ordering},
        LazyLock, RwLock,
    },
};

use crate::{
    errors::{ErrorCode, ZephyrError},
    lexer::tokens::Location,
};

use super::values::{self, RuntimeValue, RuntimeValueUtils};

type ScopeId = usize;

static NEXT_SCOPE_ID: AtomicUsize = AtomicUsize::new(1);
static SCOPE_STORE: LazyLock<RwLock<HashMap<ScopeId, ScopeData>>> =
    LazyLock::new(|| Default::default());

#[derive(Debug)]
pub enum ScopeType {
    Normal,
    Global,
    Package,
}

pub struct ScopeData {
    pub parent: Option<ScopeId>,
    pub variables: RwLock<HashMap<String, Variable>>,
    pub exported: RwLock<HashMap<String, Option<String>>>,
    pub file_name: String,
    pub scope_type: ScopeType,
}

impl ScopeData {
    pub fn new(parent: Option<ScopeId>, file_name: String) -> ScopeId {
        let id = NEXT_SCOPE_ID.fetch_add(1, Ordering::Relaxed);
        let scope = ScopeData {
            parent,
            variables: RwLock::new(HashMap::from([
                (
                    "true".to_string(),
                    Variable::new(values::Boolean::new(true).wrap()),
                ),
                (
                    "false".to_string(),
                    Variable::new(values::Boolean::new(false).wrap()),
                ),
                (
                    "null".to_string(),
                    Variable::new(values::Null::new().wrap()),
                ),
            ])),
            file_name,
            scope_type: ScopeType::Normal,
            exported: RwLock::new(HashMap::new()),
        };

        SCOPE_STORE.write().unwrap().insert(id, scope);

        id
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Scope(ScopeId);

impl Scope {
    pub fn new(parent: Option<Scope>, file_name: String) -> Self {
        Self(ScopeData::new(parent.map(|x| x.0), file_name))
    }

    pub fn file_name(&self) -> String {
        SCOPE_STORE
            .read()
            .unwrap()
            .get(&self.0)
            .unwrap()
            .file_name
            .clone()
    }

    pub fn lookup(
        &self,
        name: String,
        location: Option<Location>,
    ) -> Result<RuntimeValue, ZephyrError> {
        let lock = SCOPE_STORE.read().unwrap();
        let mut current = Some(*self);

        while let Some(scope) = current {
            let scope = lock.get(&scope.0).unwrap();
            if let Some(value) = scope.variables.read().unwrap().get(&name) {
                return Ok(value.value.clone());
            }

            current = scope.parent.map(|x| Scope(x));
        }

        Err(ZephyrError {
            code: ErrorCode::UnknownReference,
            message: format!("Cannot find variable {} in the current scope", name),
            location,
        })
    }

    pub fn insert(
        &self,
        name: String,
        value: Variable,
        location: Option<Location>,
    ) -> Result<(), ZephyrError> {
        if name == "_" {
            return Ok(());
        }

        let lock = SCOPE_STORE.read().unwrap();
        let mut variables = lock.get(&self.0).unwrap().variables.write().unwrap();

        if variables.contains_key(&name) {
            return Err(ZephyrError {
                code: ErrorCode::AlreadyDefined,
                message: format!("Variable {} already exists in the current scope", name),
                location,
            });
        }

        variables.insert(name, value);

        Ok(())
    }

    pub fn modify(
        &self,
        name: String,
        value: RuntimeValue,
        location: Option<Location>,
    ) -> Result<(), ZephyrError> {
        let lock = SCOPE_STORE.read().unwrap();
        let mut current = Some(*self);

        while let Some(scope) = current {
            let scope = lock.get(&scope.0).unwrap();
            if let Some(v) = scope.variables.write().unwrap().get_mut(&name) {
                v.value = value;
                return Ok(());
            }

            current = scope.parent.map(|x| Scope(x));
        }

        Err(ZephyrError {
            code: ErrorCode::UnknownReference,
            message: format!("Cannot find variable {} in the current scope", name),
            location,
        })
    }

    pub fn get_exported_list(&self) -> HashMap<String, Option<String>> {
        SCOPE_STORE
            .read()
            .unwrap()
            .get(&self.0)
            .unwrap()
            .exported
            .read()
            .unwrap()
            .clone()
    }

    pub fn insert_exported(&self, name: String, other_name: Option<String>) -> () {
        SCOPE_STORE
            .read()
            .unwrap()
            .get(&self.0)
            .unwrap()
            .exported
            .write()
            .unwrap()
            .insert(name, other_name);
    }
}

#[derive(Debug, Clone)]
pub struct Variable {
    pub is_const: bool,
    pub value: RuntimeValue,
}

impl Variable {
    pub fn new(value: RuntimeValue) -> Self {
        Self {
            is_const: false,
            value,
        }
    }
}
