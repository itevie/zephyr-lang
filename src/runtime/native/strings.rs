use super::{make_no_args_error, NativeExecutionContext};
use crate::{
    errors::{ErrorCode, ZephyrError},
    runtime::{
        native::add_native,
        values::{self, RuntimeValue, RuntimeValueUtils},
        R,
    },
};
use std::sync::Arc;

pub fn all() -> Vec<(String, RuntimeValue)> {
    vec![add_native!("char_code", char_code)]
}

fn char_code(ctx: NativeExecutionContext) -> R {
    match &ctx.args[..] {
        [RuntimeValue::ZString(value)] => {
            if value.value.len() != 1 {
                return Err(ZephyrError {
                    message: "Expected 1 character".to_string(),
                    location: Some(ctx.location.clone()),
                    code: ErrorCode::InvalidArgumentsError,
                });
            }

            return Ok(values::Number::new_wrapped(
                value.value.chars().next().unwrap() as u8 as f64,
            ));
        }
        _ => Err(make_no_args_error(ctx.location)),
    }
}
