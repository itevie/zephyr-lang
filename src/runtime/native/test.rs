use crate::runtime::values::RuntimeValue;

pub fn all() -> Vec<(String, RuntimeValue)> {
    vec![]
}

/*pub fn test(ctx: NativeExecutionContext) -> R {
    let event = values::EventEmitter::new(vec!["test"]);
    let event_2 = event.clone();
    let mut channel = ctx.interpreter.mspc.unwrap();
    let event_thread = event.thread_part.clone();

    handle_thread!(channel, {
        event_thread.emit_from_thread(
            "test",
            vec![ThreadRuntimeValue::Number(2f64)].into(),
            &mut channel,
        );
    });

    return Ok(RuntimeValue::EventEmitter(event.clone()));
}
*/
