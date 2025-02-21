use crate::runtime::{
    native::add_native,
    scope::PrototypeStore,
    values::{self, RuntimeValue},
    R,
};

use std::sync::Arc;

use super::{make_no_args_error, NativeExecutionContext};

pub fn all() -> Vec<(String, RuntimeValue)> {
    vec![add_native!("get_proto_obj", get_proto_obj)]
}

pub fn get_proto_obj(ctx: NativeExecutionContext) -> R {
    match &ctx.args[..] {
        [RuntimeValue::ZString(s)] => {
            Ok(values::Reference::new(PrototypeStore::get(s.value.clone())))
        }
        _ => Err(make_no_args_error(ctx.location)),
    }
}
