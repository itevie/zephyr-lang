use crate::runtime::{
    native::{add_native, make_no_args_error},
    values::{self, RuntimeValue, RuntimeValueUtils},
    R,
};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, ErrorKind, Read, Write},
    net::TcpStream,
    sync::{mpsc, Arc},
    thread,
    time::Duration,
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
    let mut channel = ctx.interpreter.mspc.clone().unwrap();

    handle_thread!(channel, {
        let mut stream = TcpStream::connect(&url).unwrap();
        stream.set_nonblocking(true).unwrap();

        let mut buffer = vec![0; 1024]; // Temporary buffer for reading
        let mut received_data = Vec::new(); // Store all received bytes
        let sender = ctx.interpreter.mspc.unwrap();

        loop {
            // Check for incoming messages from the client
            match stream.read(&mut buffer) {
                Ok(0) => {
                    event_2.emit_from_thread("close", vec![], &mut sender.clone());
                    break;
                }
                Ok(n) => {
                    received_data.extend_from_slice(&buffer[..n]); // Append received bytes
                    event_2.emit_from_thread(
                        "receive",
                        vec![
                            values::ZString::new(String::from_utf8(buffer.clone()).unwrap()).wrap(),
                        ],
                        &mut sender.clone(),
                    );
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {} // No data available yet
                Err(_) => {
                    println!("Connection error, stopping.");
                    break;
                }
            }

            // Check for outgoing messages from the channel
            match send_rx.try_recv() {
                Ok(msg) => {
                    if let Err(e) = stream.write_all(
                        &match msg.args.get(0).unwrap() {
                            RuntimeValue::ZString(s) => s.value.clone(),
                            _ => panic!(),
                        }
                        .as_bytes(),
                    ) {
                        println!("Failed to send message: {}", e);
                        break;
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {} // No messages to send
                Err(mpsc::TryRecvError::Disconnected) => {
                    println!("Message sender dropped, stopping.");
                    break;
                }
            }

            // Small sleep to prevent busy-waiting
            thread::sleep(Duration::from_millis(100));
        }
    });

    Ok(values::Object::new(HashMap::from([
        ("send".to_string(), send_val.wrap()),
        ("event".to_string(), event.clone().wrap()),
    ]))
    .as_ref_val())
}
