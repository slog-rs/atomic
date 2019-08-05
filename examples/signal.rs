#[macro_use]
extern crate slog;
extern crate slog_term;
extern crate slog_atomic;
extern crate slog_json;
extern crate nix;

#[macro_use]
extern crate lazy_static;

use nix::sys::signal;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use std::{thread, io};
use slog::*;
use slog_atomic::*;
use std::sync::Mutex;

lazy_static! {
    // global atomic switch drain control
    static ref ATOMIC_DRAIN_SWITCH : AtomicSwitchCtrl<(), io::Error> = AtomicSwitch::new(
        Discard.map_err(|_| io::Error::new(io::ErrorKind::Other, "should not happen"))
    ).ctrl();

    // track current state of the atomic switch drain
    static ref ATOMIC_DRAIN_SWITCH_STATE : AtomicBool = AtomicBool::new(false);

    // A flag set by a signal handler to please switch the logger
    // (It can't be switched inside the signal handler, as that would use non async-signal-safe
    // functions).
    static ref SWITCH_SCHEDULED: AtomicBool = AtomicBool::new(false);
}

fn atomic_drain_switch() {
    // Negate in place and get the new value.
    let new = !ATOMIC_DRAIN_SWITCH_STATE.fetch_nand(true, Ordering::Relaxed);

    if new {
        ATOMIC_DRAIN_SWITCH.set(Mutex::new(slog_json::Json::new(std::io::stderr()).build())
                                    .map_err(|_| {
                                                 io::Error::new(io::ErrorKind::Other, "mutex error")
                                             }))
    } else {
        ATOMIC_DRAIN_SWITCH.set(Mutex::new(slog_term::term_full())
                                .map_err(|_| io::Error::new(io::ErrorKind::Other, "mutex error"))
                                )

    }
}

extern "C" fn handle_sigusr1(_: i32) {
    SWITCH_SCHEDULED.store(true, Ordering::Relaxed);
}

fn main() {
    unsafe {
        let sig_action = signal::SigAction::new(signal::SigHandler::Handler(handle_sigusr1),
                                                signal::SaFlags::empty(),
                                                signal::SigSet::empty());
        signal::sigaction(signal::SIGUSR1, &sig_action).unwrap();
    }

    let drain = slog::Duplicate(slog_term::term_full(), ATOMIC_DRAIN_SWITCH.drain()).fuse();

    let drain =
        Mutex::new(drain).map_err(|_| io::Error::new(io::ErrorKind::Other, "mutex error")).fuse();
    atomic_drain_switch();

    let log = Logger::root(drain, o!());

    info!(log, "logging a message every 3s");
    info!(log, "send SIGUSR1 signal to switch output with");
    let pid = nix::unistd::getpid();
    info!(log, "kill -SIGUSR1 {}", pid);
    loop {
        if SWITCH_SCHEDULED.swap(false, Ordering::Relaxed) {
            debug!(log, "Switching the logger");
            atomic_drain_switch();
        }
        info!(log, "tick");
        thread::sleep(Duration::from_millis(3000));
    }
}
