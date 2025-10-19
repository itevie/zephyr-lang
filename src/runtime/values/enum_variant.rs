use crate::util::colors;

use super::{RuntimeValue, RuntimeValueDetails, RuntimeValueUtils};

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub inner: Box<RuntimeValue>,
    pub enum_id: String,
    pub options: RuntimeValueDetails,
}

impl EnumVariant {
    pub fn new(inner: RuntimeValue, enum_id: String) -> Self {
        Self {
            inner: Box::new(inner),
            enum_id,
            options: RuntimeValueDetails::with_proto("enum".to_string()),
        }
    }
}

impl RuntimeValueUtils for EnumVariant {
    fn type_name(&self) -> &str {
        "EnumVariant"
    }

    fn wrap(&self) -> RuntimeValue {
        RuntimeValue::EnumVariant(self.clone())
    }

    fn to_string(
        &self,
        is_display: bool,
        color: bool,
    ) -> Result<String, crate::errors::ZephyrError> {
        Ok(match color {
            true => format!(
                "{}EnumVariant<{}{}({}{}{}){}>{}",
                colors::FG_CYAN,
                colors::FG_GRAY,
                self.enum_id,
                colors::COLOR_RESET,
                self.inner.to_string(is_display, color, false)?,
                colors::FG_GRAY,
                colors::FG_CYAN,
                colors::COLOR_RESET
            ),
            false => format!(
                "EnumVariant<{}({})>",
                self.enum_id,
                self.inner.to_string(is_display, color, false)?
            ),
        })
    }
}
