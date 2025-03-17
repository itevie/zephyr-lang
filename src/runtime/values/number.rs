use crate::runtime::scope::PrototypeStore;

use super::{RuntimeValue, RuntimeValueDetails, RuntimeValueUtils};

#[derive(Debug, Clone)]
pub struct Number {
    pub options: RuntimeValueDetails,
    pub value: f64,
}

impl Number {
    pub fn new(value: f64) -> Self {
        Number {
            value,
            options: RuntimeValueDetails::with_proto(PrototypeStore::get("object".to_string())),
        }
    }
}

impl RuntimeValueUtils for Number {
    fn type_name(&self) -> &str {
        "number"
    }

    fn wrap(&self) -> RuntimeValue {
        RuntimeValue::Number(self.clone())
    }

    fn to_string(
        &self,
        is_display: bool,
        color: bool,
    ) -> Result<String, crate::errors::ZephyrError> {
        Ok(match color {
            true => format!(
                "{}{}{}",
                crate::util::colors::FG_YELLOW,
                self.value,
                crate::util::colors::COLOR_RESET
            ),
            false => self.value.to_string(),
        })
    }
}
