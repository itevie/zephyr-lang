use std::sync::{Arc, Mutex};

use crate::{
    errors::{ErrorCode, ZephyrError},
    runtime::{memory_store, scope::Scope},
};

use super::{RuntimeValue, RuntimeValueDetails, RuntimeValueUtils};

#[derive(Debug, Clone)]
pub enum ReferenceType {
    Basic(usize),
    ModuleExport((Arc<Mutex<Scope>>, Option<String>)),
}

impl ReferenceType {
    pub fn as_basic(&self) -> Option<usize> {
        match self {
            ReferenceType::Basic(loc) => Some(*loc),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Reference {
    pub options: RuntimeValueDetails,
    pub location: ReferenceType,
}

impl Reference {
    pub fn new(location: usize) -> Self {
        Reference {
            location: ReferenceType::Basic(location),
            options: RuntimeValueDetails::default(),
        }
    }

    pub fn new_export(scope: Arc<Mutex<Scope>>, ident: Option<String>) -> Self {
        Reference {
            location: ReferenceType::ModuleExport((scope, ident)),
            options: RuntimeValueDetails::default(),
        }
    }

    pub fn new_from(value: RuntimeValue) -> RuntimeValue {
        RuntimeValue::Reference(Reference {
            location: ReferenceType::Basic(memory_store::allocate(value)),
            options: RuntimeValueDetails::default(),
        })
    }

    pub fn inner(&self) -> Result<Arc<RuntimeValue>, ZephyrError> {
        match self.location {
            ReferenceType::Basic(loc) => match memory_store::get_lock().get(loc) {
                Some(ok) => {
                    let res = ok.as_ref().and_then(|x| Some(x.clone()));

                    Ok(Arc::clone(&res.unwrap()))
                }
                None => Err(ZephyrError {
                    code: ErrorCode::UnknownReference,
                    message: format!("Cannot find refernce &{}", loc),
                    location: None,
                }),
            },
            ReferenceType::ModuleExport((ref scope, ref name)) => {
                if let Some(name) = name {
                    match scope.lock().unwrap().lookup(name.clone(), None) {
                        Ok(ok) => Ok(Arc::from(ok)),
                        Err(err) => Err(ZephyrError {
                            message: format!("Exported variable {} has not been resolved. Please move this expression to the init block, or fix the cyclic dependency.", name),
                            code: ErrorCode::Unresolved,
                            location: None,
                        })
                    }
                } else {
                    panic!()
                }
            }
        }
    }
}

impl RuntimeValueUtils for Reference {
    fn type_name(&self) -> &str {
        "reference"
    }

    fn wrap(&self) -> RuntimeValue {
        RuntimeValue::Reference(self.clone())
    }

    fn to_string(&self, is_display: bool, color: bool) -> Result<String, ZephyrError> {
        Ok(format!(
            "&{}@{}",
            self.inner()?.to_string(is_display, color, false)?,
            self.location.as_basic().unwrap()
        ))
    }

    fn iter(&self) -> Result<Vec<RuntimeValue>, ZephyrError> {
        (*self.inner()?).iter()
    }
}
