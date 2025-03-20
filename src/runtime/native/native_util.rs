macro_rules! handle_thread {
    ($channel: ident, $expr: expr) => {
        $channel.thread_start();
        std::thread::spawn(move || {
            $expr;

            $channel.thread_destroy();
        })
    };
}

pub(crate) use handle_thread;

macro_rules! expect_one_arg {
    ($val:expr) => {
        match &$val[..] {
            [v] => v,
            _ => {
                return Err(ZephyrError {
                    message: "Expected at least one argument".to_string(),
                    code: ErrorCode::InvalidArgumentsError,
                    location: None,
                })
            }
        }
    };
}

pub(crate) use expect_one_arg;
