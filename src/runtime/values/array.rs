use std::{cell::RefCell, rc::Rc};

use super::{RuntimeValue, RuntimeValueDetails, RuntimeValueUtils};

#[derive(Debug, Clone)]
pub struct Array {
    pub options: RuntimeValueDetails,
    pub items: Rc<RefCell<Vec<RuntimeValue>>>,
}

impl Array {
    pub fn new(items: Vec<RuntimeValue>) -> Self {
        Array {
            items: Rc::from(RefCell::from(items)),
            options: RuntimeValueDetails::with_proto("array".to_string()),
        }
    }
}

impl RuntimeValueUtils for Array {
    fn type_name(&self) -> &str {
        "array"
    }

    fn wrap(&self) -> RuntimeValue {
        RuntimeValue::Array(self.clone())
    }

    fn iter(&self) -> Result<Vec<RuntimeValue>, crate::errors::ZephyrError> {
        Ok(self.items.borrow().clone())
    }

    fn len(&self) -> Result<usize, crate::errors::ZephyrError> {
        Ok(self.items.borrow().len())
    }

    fn to_string(
        &self,
        is_display: bool,
        color: bool,
    ) -> Result<String, crate::errors::ZephyrError> {
        let mut result = String::from("[");
        for (i, item) in self.items.borrow().iter().enumerate() {
            if i > 0 {
                result.push_str(", ");
            }
            result.push_str(&item.to_string(true, color, false)?);
        }
        result.push_str("]");
        Ok(result)
    }
}
