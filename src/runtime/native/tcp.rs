use super::{native_util::handle_thread, NativeExecutionContext};
use crate::errors::{ErrorCode, ZephyrError};
use crate::runtime::native::native_util::expect_one_arg;
use crate::runtime::{
    native::{add_native, make_no_args_error},
    values::{
        self, struct_mapping::from_runtime_object, struct_mapping::FromRuntimeValue, RuntimeValue,
        RuntimeValueUtils,
    },
    R,
};
use std::{
    collections::HashMap,
    io::{ErrorKind, Read, Write},
    net::TcpStream,
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};

pub fn all() -> Vec<(String, RuntimeValue)> {
    vec![add_native!("create_tcp_stream", create_tcp_stream)]
}

from_runtime_object!(TcpStreamOptions {
    url: String,
    block_till_finished: bool,
    presend: Vec<u8>,
});

pub fn create_tcp_stream(ctx: NativeExecutionContext) -> R {
    let options = TcpStreamOptions::from_runtime_value(expect_one_arg!(ctx.args))?;

    if options.block_till_finished {
        let mut stream = TcpStream::connect(&options.url).unwrap();

        stream.write_all(&options.presend).unwrap();

        let mut buffer = vec![0; 1024];
        let mut received_data = Vec::new();

        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break, // Connection closed
                Ok(n) => received_data.extend_from_slice(&buffer[..n]),
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => break,
            }
        }

        return Ok(
            values::ZString::new(String::from_utf8_lossy(&received_data).to_string()).wrap(),
        );
    }

    let (send_rx, send_val) = values::MspcSender::new_handled();

    let event = values::EventEmitter::new(vec!["receive", "close"]);
    let event_2 = event.clone();
    let mut channel = ctx.interpreter.mspc.clone().unwrap();

    handle_thread!(channel, {
        let mut stream = TcpStream::connect(&options.url).unwrap();
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
