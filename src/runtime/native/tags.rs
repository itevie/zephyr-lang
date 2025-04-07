use crate::runtime::{
    native::add_native,
    values::{self, RuntimeValue, RuntimeValueUtils},
    R,
};

use std::sync::Arc;

use super::{make_no_args_error, NativeExecutionContext};

pub fn all() -> Vec<(String, RuntimeValue)> {
    vec![
        add_native!("add_tag", add_tag),
        add_native!("delete_tag", delete_tag),
        add_native!("set_tag", set_tag),
    ]
}

pub fn add_tag(ctx: NativeExecutionContext) -> R {
    match &ctx.args[..] {
        [target, RuntimeValue::ZString(key), RuntimeValue::ZString(value)] => {
            target
                .options()
                .tags
                .borrow_mut()
                .insert(key.value.clone(), value.value.clone());
            Ok(values::Null::new().wrap())
        }
        _ => Err(make_no_args_error(ctx.location)),
    }
}

pub fn delete_tag(ctx: NativeExecutionContext) -> R {
    match &ctx.args[..] {
        [target, RuntimeValue::ZString(key)] => {
            target
                .options()
                .tags
                .borrow_mut()
                .remove(&key.value.clone());
            Ok(values::Null::new().wrap())
        }
        _ => Err(make_no_args_error(ctx.location)),
    }
}

pub fn set_tag(ctx: NativeExecutionContext) -> R {
    match &ctx.args[..] {
        [target, RuntimeValue::ZString(key), RuntimeValue::ZString(value)] => {
            let mut lock = target.options().tags.borrow_mut();

            lock.remove(&key.value.clone());
            lock.insert(key.value.clone(), value.value.clone());
            Ok(values::Null::new().wrap())
        }
        _ => Err(make_no_args_error(ctx.location)),
    }
}
