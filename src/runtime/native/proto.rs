use crate::runtime::{
    scope::PrototypeStore,
    values::{self, RuntimeValue},
    R,
};

use super::{make_no_args_error, NativeExecutionContext};

pub fn get_proto_obj(ctx: NativeExecutionContext) -> R {
    match &ctx.args[..] {
        [RuntimeValue::ZString(s)] => {
            Ok(values::Reference::new(PrototypeStore::get(s.value.clone())))
        }
        _ => Err(make_no_args_error(ctx.location)),
    }
}
