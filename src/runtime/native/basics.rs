use crate::runtime::{
    native::add_native,
    values::{self, RuntimeValue},
    R,
};

use std::sync::Arc;

use super::{make_no_args_error, NativeExecutionContext};

pub fn all() -> Vec<(String, RuntimeValue)> {
    vec![add_native!("iter", iter)]
}

pub fn iter(ctx: NativeExecutionContext) -> R {
    match &ctx.args[..] {
        [r] => Ok(values::Array::new(r.iter()?)),
        _ => Err(make_no_args_error(ctx.location)),
    }
}
