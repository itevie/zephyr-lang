use std::collections::HashMap;

use super::{RuntimeValue, RuntimeValueDetails, RuntimeValueUtils};

#[derive(Debug, Clone)]
pub struct Object {
    pub options: RuntimeValueDetails,
    pub items: HashMap<String, RuntimeValue>,
}

impl Object {
    pub fn new(items: HashMap<String, RuntimeValue>) -> Self {
        Object {
            items,
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
