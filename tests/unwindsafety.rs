#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_term;
extern crate slog_atomic;

use slog::{Logger, Drain};

fn main() {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain)
        .chan_size(64 * 1024)
        .build()
        .filter_level(slog::Level::Info);
    // .fuse();
    let drain = slog_atomic::AtomicSwitch::new(drain);
    let _log_ctrl = drain.ctrl();

    let _log = Logger::root(drain.fuse(), o!());
}
