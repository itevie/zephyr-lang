use crate::runtime::{
    values::{self, FunctionType, RuntimeValue},
    R,
};

use super::{make_no_args_error, NativeExecutionContext};

pub fn add_listener(ctx: NativeExecutionContext) -> R {
    match &ctx.args[..] {
        [RuntimeValue::EventEmitter(event), RuntimeValue::ZString(string), val] => {
            let func = FunctionType::from(val.clone())?;
            event.add_listener(string.value.clone(), func, ctx.location)?;

            Ok(values::Null::new())
        }
        _ => Err(make_no_args_error(ctx.location)),
    }
}
