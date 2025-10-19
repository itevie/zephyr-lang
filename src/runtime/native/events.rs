use crate::runtime::{
    native::add_native,
    values::{self, FunctionType, RuntimeValue, RuntimeValueUtils},
    R,
};

use std::sync::Arc;

use super::{make_no_args_error, NativeExecutionContext};

pub fn all() -> Vec<(String, RuntimeValue)> {
    vec![add_native!("add_event_listener", add_listener)]
}

pub fn add_listener(ctx: NativeExecutionContext) -> R {
    match &ctx.args.clone()[..] {
        [RuntimeValue::EventEmitter(event), RuntimeValue::ZString(string), val] => {
            let func = FunctionType::from(val.clone())?;
            event.add_listener(string.value.clone(), func, ctx)?;

            Ok(values::Null::new().wrap())
        }
        _ => Err(make_no_args_error(ctx.location)),
    }
}
