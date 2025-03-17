use crate::runtime::{
    native::add_native,
    scope::PrototypeStore,
    values::{self, ReferenceType, RuntimeValue, RuntimeValueUtils},
    R,
};

use std::sync::Arc;

use super::{make_no_args_error, NativeExecutionContext};

pub fn all() -> Vec<(String, RuntimeValue)> {
    vec![
        add_native!("get_proto_obj", get_proto_obj),
        add_native!("set_proto_ref", set_proto_ref),
        add_native!("get_proto_obj_of", get_proto_obj_of),
    ]
}

pub fn get_proto_obj(ctx: NativeExecutionContext) -> R {
    match &ctx.args[..] {
        [RuntimeValue::ZString(s)] => {
            Ok(values::Reference::new(PrototypeStore::get(s.value.clone())).wrap())
        }
        _ => Err(make_no_args_error(ctx.location)),
    }
}

pub fn get_proto_obj_of(ctx: NativeExecutionContext) -> R {
    match &ctx.args[..] {
        [s] => Ok(values::Reference::new(s.options().proto.lock().unwrap().unwrap()).wrap()),
        _ => Err(make_no_args_error(ctx.location)),
    }
}

pub fn set_proto_ref(ctx: NativeExecutionContext) -> R {
    match &ctx.args[..] {
        [val, RuntimeValue::Reference(r)] => {
            *val.options().proto.lock().unwrap() = Some(match r.location {
                ReferenceType::Basic(r) => r,
                _ => return Err(make_no_args_error(ctx.location)),
            });

            Ok(values::Null::new().wrap())
        }
        _ => Err(make_no_args_error(ctx.location)),
    }
}
