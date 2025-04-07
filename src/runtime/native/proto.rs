use crate::runtime::{
    native::add_native,
    values::{self, RuntimeValue, RuntimeValueUtils},
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
        [RuntimeValue::ZString(s)] => Ok(values::Object::new_from_rc(
            ctx.interpreter.prototype_store.get(s.value.clone()),
        )
        .wrap()),
        _ => Err(make_no_args_error(ctx.location)),
    }
}

pub fn get_proto_obj_of(ctx: NativeExecutionContext) -> R {
    match &ctx.args[..] {
        [s] => Ok(values::Object::new_from_rc(
            ctx.interpreter
                .prototype_store
                .get(s.options().proto.borrow().as_ref().unwrap()),
        )
        .wrap()),
        _ => Err(make_no_args_error(ctx.location)),
    }
}

pub fn set_proto_ref(ctx: NativeExecutionContext) -> R {
    match &ctx.args[..] {
        [val, RuntimeValue::Object(r)] => {
            let key = format!("user::{}", uuid::Uuid::new_v4());
            ctx.interpreter
                .prototype_store
                .set(key.clone(), r.items.clone());
            *val.options().proto.borrow_mut() = Some(key.clone());

            Ok(values::Null::new().wrap())
        }
        _ => Err(make_no_args_error(ctx.location)),
    }
}
