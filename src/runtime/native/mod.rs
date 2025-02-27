use crate::{
    errors::{ErrorCode, ZephyrError},
    lexer::tokens::Location,
};

use super::{values::RuntimeValue, Interpreter};

pub mod basics;
pub mod events;
pub mod proto;
pub mod test;

pub fn all() -> Vec<(String, RuntimeValue)> {
    vec![]
        .iter()
        .cloned()
        .chain(proto::all().iter().cloned())
        .chain(events::all().iter().cloned())
        .chain(test::all().iter().cloned())
        .chain(basics::all().iter().cloned())
        .collect()
}

pub struct NativeExecutionContext {
    pub interpreter: Interpreter,
    pub args: Vec<RuntimeValue>,
    pub location: Location,
}

pub fn make_no_args_error(location: Location) -> ZephyrError {
    ZephyrError {
        message: "Invalid args".to_string(),
        code: ErrorCode::TypeError,
        location: Some(location),
    }
}

macro_rules! add_native {
    ($name:expr, $nv_path:expr) => {
        (
            $name.to_string(),
            values::NativeFunction::new(Arc::from($nv_path)),
        )
    };
}

pub(crate) use add_native;
