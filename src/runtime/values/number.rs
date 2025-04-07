use super::{RuntimeValue, RuntimeValueDetails, RuntimeValueUtils};

macro_rules! define_runtime_value {
    ($struct_name:ident { $($field:ident : $ty:ty),* $(,)? }, $struct_safe_name:ident) => {
        #[derive(Debug, Clone)]
        pub struct $struct_name {
            pub options: RuntimeValueDetails,
            $(pub $field: $ty),*
        }

        #[derive(Debug, Clone)]
        pub struct $struct_safe_name {
            $($field: $ty),*
        }

        impl From<&$struct_name> for $struct_safe_name {
            fn from(value: &$struct_name) -> $struct_safe_name {
                $struct_safe_name {
                    $($field: value.$field),*
                }
            }
        }
    };
}

define_runtime_value!(Number { value: f64 }, NumberBase);

impl Number {
    pub fn new(value: f64) -> Self {
        Self {
            value,
            options: RuntimeValueDetails::with_proto("object".to_string()),
        }
    }

    pub fn new_wrapped(value: f64) -> RuntimeValue {
        RuntimeValue::Number(Self {
            value,
            options: RuntimeValueDetails::with_proto("object".to_string()),
        })
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
