use crate::runtime::{
    native::{add_native, native_util::handle_thread},
    values::{self, RuntimeValue, RuntimeValueUtils},
    R,
};

use std::sync::Arc;

use super::NativeExecutionContext;

pub fn all() -> Vec<(String, RuntimeValue)> {
    vec![add_native!("test", test)]
}

pub fn test(ctx: NativeExecutionContext) -> R {
    let event = values::EventEmitter::new(vec!["test"]);
    let event_2 = event.clone();
    let mut channel = ctx.interpreter.mspc.unwrap();

    handle_thread!(channel, {
        event_2.emit_from_thread("test", vec![values::Number::new(4f64).wrap()], &mut channel);
    });

    return Ok(RuntimeValue::EventEmitter(event.clone()));
}
