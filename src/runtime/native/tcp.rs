use crate::runtime::{
    native::{add_native, make_no_args_error},
    values::{self, RuntimeValue, RuntimeValueUtils},
    R,
};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    sync::Arc,
};

use super::{native_util::handle_thread, NativeExecutionContext};

pub fn all() -> Vec<(String, RuntimeValue)> {
    vec![add_native!("create_tcp_stream", create_tcp_stream)]
}

pub fn create_tcp_stream(ctx: NativeExecutionContext) -> R {
    let url = match &ctx.args[..] {
        [RuntimeValue::ZString(s)] => s.value.clone(),
        _ => return Err(make_no_args_error(ctx.location)),
    };

    let (send_rx, send_val) = values::MspcSender::new_handled();

    let event = values::EventEmitter::new(vec!["receive", "close"]);
    let event_2 = event.clone();
    let mut channel = ctx.interpreter.mspc.unwrap();

    handle_thread!(channel, {
        let mut stream = TcpStream::connect(&url).unwrap();
        let request = "GET / HTTP/1.1\r\nHost: dawn.rest\r\nConnection: close\r\n\r\n";
        stream.write_all(request.as_bytes()).unwrap();
        let reader = BufReader::new(stream.try_clone().unwrap());

        for line in reader.lines() {
            if let Ok(ok) = line {
                event_2.emit_from_thread(
                    "receive",
                    vec![values::ZString::new(ok).wrap()],
                    &mut channel,
                );
            }
        }

        event_2.emit_from_thread("close", vec![], &mut channel);
    });

    Ok(values::Object::new(HashMap::from([("send".to_string(), send_val.wrap())])).wrap())
}
