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
    vec![
        add_native!("char_code", char_code),
        add_native!("str_split", str_split),
    ]
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

fn str_split(ctx: NativeExecutionContext) -> R {
    match &ctx.args[..] {
        [RuntimeValue::ZString(what), RuntimeValue::ZString(seperator)] => {
            let parts = what.value.split(&seperator.value);
            Ok(values::Array::new(
                parts
                    .collect::<Vec<&str>>()
                    .iter()
                    .map(|x| values::ZString::new(x.to_string()).wrap())
                    .collect(),
            )
            .wrap())
        }
        _ => Err(make_no_args_error(ctx.location)),
    }
}
