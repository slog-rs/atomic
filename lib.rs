//! Slog runtime switchable drain
//!
//! `AtomicSwitch` allows swapping drain that it wraps atomically, race-free, in
//! runtime. This can be useful eg. for turning on debug logging
//! in production.
//!
//! See [`signal.rs` example](https://github.com/slog-rs/atomic/blob/master/examples/signal.rs).
#![warn(missing_docs)]

extern crate arc_swap;
extern crate slog;

use std::sync::Arc;

use arc_swap::ArcSwap;
use slog::*;

type Inner<O, E> = Arc<ArcSwap<Box<dyn SendSyncRefUnwindSafeDrain<Ok=O,Err=E>>>>;

/// Handle to `AtomicSwitch` that controls it.
#[derive(Clone)]
pub struct AtomicSwitchCtrl<O=(), E=slog::Never>(Inner<O, E>);

/// Drain wrapping another drain, allowing atomic substitution in runtime.
#[derive(Clone)]
pub struct AtomicSwitch<O=(), E=slog::Never>(Inner<O, E>);

impl Default for AtomicSwitch<(), slog::Never> {
    fn default() -> Self {
        Self::new(Discard)
    }
}

impl<O, E> AtomicSwitch<O, E> {
    /// Wrap `drain` in `AtomicSwitch` to allow swapping it later
    ///
    /// Use `AtomicSwitch::ctrl()` to get a handle to it
    pub fn new<D: SendSyncRefUnwindSafeDrain<Ok = O, Err = E> + 'static>(drain: D) -> Self {
        AtomicSwitch::new_from_arc(Arc::new(ArcSwap::from_pointee(Box::new(drain))))
    }

    // TODO: This one seems a bit fishy
    /// Create new `AtomicSwitch` from an existing `Arc<...>`
    ///
    /// See `AtomicSwitch::new()`
    pub fn new_from_arc(d: Arc<ArcSwap<Box<dyn SendSyncRefUnwindSafeDrain<Ok = O, Err = E>>>>) -> Self {
        AtomicSwitch(d)
    }

    /// Get a `AtomicSwitchCtrl` handle to control this `AtomicSwitch` drain
    pub fn ctrl(&self) -> AtomicSwitchCtrl<O, E> {
        AtomicSwitchCtrl(self.0.clone())
    }
}

impl<O, E> AtomicSwitchCtrl<O, E> {
    /// Get Arc to the currently wrapped drain
    pub fn get(&self) -> Arc<Box<dyn SendSyncRefUnwindSafeDrain<Ok = O, Err = E>>> {
        self.0.load_full()
    }

    /// Set the current wrapped drain
    pub fn set<D: SendSyncRefUnwindSafeDrain<Ok = O, Err = E> + 'static>(&self, drain: D) {
        self.0.store(Arc::new(Box::new(drain)));
    }

    /// Swap the existing drain with a new one
    pub fn swap(&self,
                drain: Arc<Box<dyn SendSyncRefUnwindSafeDrain<Ok = O, Err = E>>>)
                -> Arc<Box<dyn SendSyncRefUnwindSafeDrain<Ok = O, Err = E>>> {
        self.0.swap(drain)
    }

    /// Get a `AtomicSwitch` drain controlled by this `AtomicSwitchCtrl`
    pub fn drain(&self) -> AtomicSwitch<O, E> {
        AtomicSwitch(self.0.clone())
    }
}

impl<O, E> Drain for AtomicSwitch<O, E> {
    type Ok = O;
    type Err = E;
    fn log(&self, info: &Record, kv: &OwnedKVList) -> std::result::Result<O, E> {
        self.0.load().log(info, kv)
    }

    fn is_enabled(&self, level: Level) -> bool {
        self.0.load().is_enabled(level)
    }
}
