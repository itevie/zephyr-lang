use crate::runtime::{
    values::{self, RuntimeValue},
    R,
};

use super::NativeExecutionContext;

pub fn test(ctx: NativeExecutionContext) -> R {
    let event = values::EventEmitter::new(vec!["test".to_string()]);
    let event_2 = event.clone();
    std::thread::spawn(move || {
        let mut channel = ctx.interpreter.mspc.unwrap();
        channel.thread_start();

        event_2.emit_from_thread(
            "test".to_string(),
            vec![values::Number::new(4f64)],
            &mut channel,
        );

        channel.thread_destroy();
    });
    return Ok(RuntimeValue::EventEmitter(event.clone()));
}
