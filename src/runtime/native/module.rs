use crate::runtime::{
    native::add_native,
    values::{self, RuntimeValue, RuntimeValueUtils},
    R,
};

use std::{path::PathBuf, sync::Arc};

use super::{make_no_args_error, NativeExecutionContext};

pub fn all() -> Vec<(String, RuntimeValue)> {
    vec![
        add_native!("filename", filename),
        add_native!("dirname", dirname),
    ]
}

pub fn filename(ctx: NativeExecutionContext) -> R {
    match &ctx.args[..] {
        [] => Ok(values::ZString::new(ctx.file_name.clone()).wrap()),
        _ => Err(make_no_args_error(ctx.location)),
    }
}

pub fn dirname(ctx: NativeExecutionContext) -> R {
    match &ctx.args[..] {
        [] => Ok(values::ZString::new(
            PathBuf::from(ctx.file_name)
                .parent()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
        )
        .wrap()),
        _ => Err(make_no_args_error(ctx.location)),
    }
}
