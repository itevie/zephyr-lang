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
