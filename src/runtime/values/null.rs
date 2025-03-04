use crate::util::colors;

use super::{RuntimeValue, RuntimeValueDetails, RuntimeValueUtils};

#[derive(Debug, Clone)]
pub struct Null {
    pub options: RuntimeValueDetails,
}

impl Null {
    pub fn new() -> RuntimeValue {
        RuntimeValue::Null(Null {
            options: RuntimeValueDetails::default(),
        })
    }
}

impl RuntimeValueUtils for Null {
    fn type_name(&self) -> &str {
        "null"
    }

    fn to_string(
        &self,
        is_display: bool,
        color: bool,
    ) -> Result<String, crate::errors::ZephyrError> {
        Ok(match color {
            true => format!("{}{}{}", colors::FG_GRAY, "null", colors::COLOR_RESET),
            false => "null".to_string(),
        })
    }
}
