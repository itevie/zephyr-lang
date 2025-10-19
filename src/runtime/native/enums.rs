use crate::runtime::{
    native::add_native,
    values::{self, RuntimeValue, RuntimeValueUtils},
    R,
};

use std::sync::Arc;

use super::{make_no_args_error, NativeExecutionContext};

pub fn all() -> Vec<(String, RuntimeValue)> {
    vec![add_native!(
        "get_enum_varient_inner",
        get_enum_varient_inner
    )]
}

fn get_enum_varient_inner(ctx: NativeExecutionContext) -> R {
    match &ctx.args[..] {
        [RuntimeValue::EnumVariant(val)] => Ok(*val.inner.clone()),
        _ => Err(make_no_args_error(ctx.location)),
    }
}
