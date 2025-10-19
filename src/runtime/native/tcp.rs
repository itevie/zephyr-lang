use super::{native_util::handle_thread, NativeExecutionContext};
use crate::errors::{ErrorCode, ZephyrError};
use crate::runtime::values::thread_crossing::{ThreadInnerValue, ThreadRuntimeValue};
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
use crate::runtime::native::native_util::expect_one_arg;

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
    let event_part = event.thread_part.clone();


    handle_thread!(channel, {
        let mut stream = TcpStream::connect(&options.url).unwrap();
        stream.set_nonblocking(true).unwrap();

        if !options.presend.is_empty() {
            stream.write_all(&options.presend).unwrap();
        }

        let mut buffer = vec![0; 1024];
        let mut received_data = Vec::new();

        loop {
            // Check for incoming messages from the client
            match stream.read(&mut buffer) {
                Ok(0) => {
                    event_part.emit_from_thread("close", vec![].into(), &mut channel.clone());
                    break;
                }
                Ok(n) => {
                    received_data.extend_from_slice(&buffer[..n]); // Append received bytes
                    event_part.emit_from_thread(
                        "receive",
                        vec![ThreadRuntimeValue::new(ThreadInnerValue::ZString(String::from_utf8(buffer.clone()).unwrap()))]
                        .into(),
                        &mut channel.clone(),
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
                        &match &msg.args.get(0).unwrap().value {
                            ThreadInnerValue::ZString(s) => s.clone(),
                            _ => panic!(),
                        }
                        .as_bytes(),
                    ) {
                        println!("Failed to send message: {}", e);
                        break;
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
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
    .wrap())
}
