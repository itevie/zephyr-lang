use crate::util;

use super::{RuntimeValue, RuntimeValueDetails, RuntimeValueUtils};

#[derive(Debug, Clone)]
pub struct Boolean {
    pub options: RuntimeValueDetails,
    pub value: bool,
}

impl Boolean {
    pub fn new(value: bool) -> RuntimeValue {
        RuntimeValue::Boolean(Boolean {
            value,
            options: RuntimeValueDetails::default(),
        })
    }
}

impl RuntimeValueUtils for Boolean {
    fn type_name(&self) -> &str {
        "boolean"
    }

    fn to_string(
        &self,
        is_display: bool,
        color: bool,
    ) -> Result<String, crate::errors::ZephyrError> {
        Ok(match color {
            true => match self.value {
                true => format!(
                    "{}{}{}",
                    util::colors::FG_GREEN,
                    true,
                    util::colors::COLOR_RESET
                ),
                false => format!(
                    "{}{}{}",
                    util::colors::FG_RED,
                    false,
                    util::colors::COLOR_RESET
                ),
            },
            false => self.value.to_string(),
        })
    }
}
