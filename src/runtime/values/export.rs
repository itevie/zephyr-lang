use crate::{
    errors::{ErrorCode, ZephyrError},
    runtime::scope::ScopeInnerType,
};

use super::{RuntimeValue, RuntimeValueDetails, RuntimeValueUtils};

#[derive(Debug, Clone)]
pub struct Export {
    pub options: RuntimeValueDetails,
    pub scope: ScopeInnerType,
    pub symbol: Option<String>,
}

impl Export {
    pub fn new(scope: ScopeInnerType, symbol: Option<String>) -> Self {
        Self {
            scope,
            symbol,
            options: RuntimeValueDetails::default(),
        }
    }

    pub fn inner(&self) -> Result<RuntimeValue, ZephyrError> {
        let name = self.symbol.as_ref().unwrap_or_else(|| panic!());

        match self.scope.borrow().lookup(name, None) {
            Ok(ok) => Ok(ok),
            Err(err) => Err(ZephyrError {
                message: format!("Exported variable {} has not been resolved. Please move this expression to the init block, or fix the cyclic dependency.", name),
                code: ErrorCode::Unresolved,
                location: None,
            })
        }
    }
}

impl RuntimeValueUtils for Export {
    fn wrap(&self) -> super::RuntimeValue {
        super::RuntimeValue::Export(self.clone())
    }

    fn type_name(&self) -> &str {
        "export"
    }
}
