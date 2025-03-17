use crate::runtime::{
    native::add_native,
    values::{self, RuntimeValue, RuntimeValueUtils},
    R,
};

use std::{fs, sync::Arc};

use super::{make_no_args_error, NativeExecutionContext};

pub fn all() -> Vec<(String, RuntimeValue)> {
    vec![add_native!("file_exists", file_exists)]
}

pub fn file_exists(ctx: NativeExecutionContext) -> R {
    match &ctx.args[..] {
        [RuntimeValue::ZString(path)] => {
            Ok(values::Boolean::new(fs::metadata(path.value.clone()).is_ok()).wrap())
        }
        _ => Err(make_no_args_error(ctx.location)),
    }
}
