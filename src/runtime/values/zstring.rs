use crate::runtime::scope::PrototypeStore;

use super::{RuntimeValue, RuntimeValueDetails, RuntimeValueUtils};

#[derive(Debug, Clone)]
pub struct ZString {
    pub options: RuntimeValueDetails,
    pub value: String,
}

impl ZString {
    pub fn new(value: String) -> Self {
        ZString {
            value,
            options: RuntimeValueDetails::with_proto(PrototypeStore::get("string".to_string())),
        }
    }
}

impl RuntimeValueUtils for ZString {
    fn type_name(&self) -> &str {
        "string"
    }

    fn wrap(&self) -> RuntimeValue {
        RuntimeValue::ZString(self.clone())
    }

    fn iter(&self) -> Result<Vec<RuntimeValue>, crate::errors::ZephyrError> {
        Ok(self
            .value
            .chars()
            .map(|v| ZString::new(v.to_string()).wrap())
            .collect::<Vec<RuntimeValue>>())
    }

    fn to_string(
        &self,
        is_display: bool,
        color: bool,
    ) -> Result<String, crate::errors::ZephyrError> {
        let mut res = if is_display { "\"" } else { "" }.to_string();

        res.push_str(&self.value);

        if is_display {
            res.push_str("\"");
        }

        Ok(match color {
            true => format!(
                "{}{}{}",
                crate::util::colors::FG_YELLOW,
                res,
                crate::util::colors::COLOR_RESET
            ),
            false => res,
        })
    }
}
