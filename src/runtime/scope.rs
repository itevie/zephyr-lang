use std::{
    cell::RefCell,
    collections::HashMap,
    env::var,
    rc::Rc,
    sync::{
        atomic::{AtomicUsize, Ordering},
        LazyLock, RwLock,
    },
};

use crate::{
    errors::{ErrorCode, ZephyrError},
    lexer::tokens::Location,
};

use super::{
    values::{self, RuntimeValue, RuntimeValueUtils},
    R,
};

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

pub struct ScopeDataNew {
    pub parent: Option<Box<ScopeDataNew>>,
    pub variables: RefCell<HashMap<String, Variable>>,
}

impl ScopeDataNew {
    pub fn new(parent: Option<Box<ScopeDataNew>>, file_name: String) -> Self {
        Self {
            parent,
            variables: RefCell::from(HashMap::new()),
        }
    }

    pub fn lookup(
        &self,
        name: String,
        location: Option<Location>,
    ) -> Result<RuntimeValue, ZephyrError> {
        if let Some(var) = self.variables.borrow().get(&name) {
            Ok(var.value.clone())
        } else if let Some(parent) = self.parent.as_ref() {
            parent.lookup(name, location)
        } else {
            Err(ZephyrError {
                message: format!("Cannot find variable {} in current scope", name),
                code: ErrorCode::UnknownReference,
                location,
            })
        }
    }

    pub fn insert(
        &self,
        name: String,
        value: Variable,
        location: Option<Location>,
    ) -> Result<(), ZephyrError> {
        if self.variables.borrow().contains_key(&name) {
            return Err(ZephyrError {
                message: format!("Variable {} already defined in current scope", name),
                code: ErrorCode::AlreadyDefined,
                location,
            });
        }

        self.variables.borrow_mut().insert(name, value);

        Ok(())
    }
}

struct IntTest {
    scope: Box<ScopeDataNew>,
}

impl IntTest {
    pub fn new() -> Self {
        Self {
            scope: Box::from(ScopeDataNew::new(None, "test".to_string())),
        }
    }

    pub fn run(&mut self) -> R {
        self.scope.insert(
            "test".to_string(),
            Variable {
                value: values::Null::new().wrap(),
                is_const: false,
            },
            None,
        )?;
        todo!()
    }

    pub fn _scope(&self) -> R {
        let temp = *self.scope;
        let scope = ScopeDataNew::new(Some(Box::from(temp)), "test".to_string());
        todo!()
    }
}

fn test() {
    let scope = ScopeDataNew::new(&None, "ur mom".to_string());
    scope
        .insert(
            "test".to_string(),
            Variable {
                value: values::Null::new().wrap(),
                is_const: false,
            },
            None,
        )
        .unwrap();
    let wrapped = Some(scope);
    let new_scope = ScopeDataNew::new(&wrapped, "test".to_string());
    new_scope.lookup("test".to_string(), None);
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
        let rc = Rc::new(2);

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
