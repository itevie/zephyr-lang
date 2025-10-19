use crate::errors::{ErrorCode, ZephyrError};

use super::RuntimeValue;

pub trait FromRuntimeValue: Sized {
    fn from_runtime_value(value: &RuntimeValue) -> Result<Self, ZephyrError>;
}

macro_rules! impl_for {
    ($type:ty, $name:expr, $( $pattern:pat => $result:expr ),+) => {
        impl FromRuntimeValue for $type {
            fn from_runtime_value(value: &RuntimeValue) -> Result<Self, ZephyrError> {
                match value {
                    $(
                        $pattern => Ok($result),
                    )*
                    _ => Err(ZephyrError {
                        message: format!("Expected {} type, but got {}", $name, value.type_name()),
                        location: None,
                        code: ErrorCode::StructMappingError,
                    }),
                }
            }
        }
    };
}

macro_rules! impl_for_vec {
    ($type:ty, $name:expr, $( $pattern:pat => $result:expr ),+) => {
        impl FromRuntimeValue for Vec<$type> {
            fn from_runtime_value(value: &RuntimeValue) -> Result<Self, ZephyrError> {
                match value {
                    RuntimeValue::Array(ref a) => Ok(a
                        .items
                        .borrow()
                        .iter()
                        .map(|v| <$type>::from_runtime_value(v))
                        .collect::<Result<Vec<$type>, ZephyrError>>()?),
                    $(
                        $pattern => Ok($result),
                    )*
                    _ => Err(ZephyrError {
                        message: format!("Expected {} type, but got {}", $name, value.type_name()),
                        location: None,
                        code: ErrorCode::StructMappingError,
                    }),
                }
            }
        }
    };
}

macro_rules! impl_all_for {
    ($type:ty, $name:expr, $($pattern:pat => $result:expr)+) => {
        impl_for!($type, $name, $($pattern => $result),*);
        impl_for!(Option<$type>, format!("{} or null", $name), $($pattern => Some($result)),*, RuntimeValue::Null(_) => None);
    };
}

impl_all_for!(String, "string", RuntimeValue::ZString(ref s) => s.value.clone());
impl_all_for!(bool, "boolean", RuntimeValue::Boolean(ref s) => s.value);
impl_all_for!(u8, "u8", RuntimeValue::Number(ref s) => s.value.round() as u8);
impl_for_vec!(u8, "array of u8", RuntimeValue::ZString(ref s) => s.value.as_bytes().to_vec());

macro_rules! from_runtime_object {
    ($struct_name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Debug)]
        pub struct $struct_name {
            $($field: $ty),*
        }

        impl FromRuntimeValue for $struct_name {
            fn from_runtime_value(value: &RuntimeValue) -> Result<Self, ZephyrError> {
                if let RuntimeValue::Object(obj) = value {
                    Ok($struct_name {
                        $(
                            $field: match obj.items.borrow().get(stringify!($field)) {
                                Some(val) => match <$ty as FromRuntimeValue>::from_runtime_value(val) {
                                    Ok(v) => v,
                                    Err(e) => return Err(e)
                                },
                                None => Default::default(),
                            }
                        ),*
                    })

                } else {
                    Err(ZephyrError {
                        message: format!("Expected object for struct mapping, but got {}", value.type_name()),
                        code: ErrorCode::StructMappingError,
                        location: None,
                    })
                }
            }
        }
    };
}

pub(crate) use from_runtime_object;

