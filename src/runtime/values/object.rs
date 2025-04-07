use std::{cell::RefCell, collections::HashMap, rc::Rc};

use super::{RuntimeValue, RuntimeValueDetails, RuntimeValueUtils};

pub type ObjectItemsType = Rc<RefCell<HashMap<String, RuntimeValue>>>;

#[derive(Debug, Clone)]
pub struct Object {
    pub options: RuntimeValueDetails,
    pub items: ObjectItemsType,
}

impl Object {
    pub fn new(items: HashMap<String, RuntimeValue>) -> Self {
        Object {
            items: Rc::from(RefCell::from(items)),
            options: RuntimeValueDetails::default(),
        }
    }

    pub fn new_from_rc(items: ObjectItemsType) -> Self {
        Self {
            items: items.clone(),
            options: RuntimeValueDetails::default(),
        }
    }
}

impl RuntimeValueUtils for Object {
    fn type_name(&self) -> &str {
        "object"
    }

    fn wrap(&self) -> RuntimeValue {
        RuntimeValue::Object(self.clone())
    }

    fn to_string(
        &self,
        is_display: bool,
        color: bool,
    ) -> Result<String, crate::errors::ZephyrError> {
        let parts = self
            .items
            .borrow()
            .iter()
            .map(|(k, v)| {
                let value_str = v.to_string(true, color, false)?;
                Ok(format!("{}: {}", k, value_str))
            })
            .collect::<Result<Vec<String>, crate::errors::ZephyrError>>()?;

        Ok(format!(
            "{}{{{}}}",
            if is_display { "." } else { "" },
            parts.join(", ")
        ))
    }
}
