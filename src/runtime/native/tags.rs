use crate::runtime::{
    native::add_native,
    values::{self, RuntimeValue},
    R,
};

use std::sync::Arc;

use super::{make_no_args_error, NativeExecutionContext};

pub fn all() -> Vec<(String, RuntimeValue)> {
    vec![add_native!("add_tag", add_tag)]
}

pub fn add_tag(ctx: NativeExecutionContext) -> R {
    match &ctx.args[..] {
        [target, RuntimeValue::ZString(key), RuntimeValue::ZString(value)] => {
            target
                .options()
                .tags
                .lock()
                .unwrap()
                .insert(key.value.clone(), value.value.clone());
            Ok(values::Null::new())
        }
        _ => Err(make_no_args_error(ctx.location)),
    }
}
