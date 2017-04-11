//! Slog runtime switchable drain
//!
//! `AtomicSwitch` allows swapping drain that it wraps atomically, race-free, in
//! runtime. This can be useful eg. for turning on debug logging
//! in production.
//!
//! See [`slog` `signal.rs`
//! example](https://github.com/dpc/slog-rs/blob/master/examples/signal.rs)
#![warn(missing_docs)]

extern crate slog;
extern crate crossbeam;

use slog::*;
use std::sync::Arc;
use crossbeam::sync::ArcCell;

/// Handle to `AtomicSwitch` that controls it.
pub struct AtomicSwitchCtrl<O=(), E=slog::Never>(Arc<ArcCell<Box<SendSyncRefUnwindSafeDrain<Ok=O,Err=E>>>>);

/// Drain wrapping another drain, allowing atomic substitution in runtime
pub struct AtomicSwitch<O=(), E=slog::Never>(Arc<ArcCell<Box<SendSyncRefUnwindSafeDrain<Ok=O,Err=E>>>>);

impl<O, E> AtomicSwitch<O, E> {
    /// Wrap `drain` in `AtomicSwitch` to allow swapping it later
    ///
    /// Use `AtomicSwitch::ctrl()` to get a handle to it
    pub fn new<D: SendSyncRefUnwindSafeDrain<Ok = O, Err = E> + 'static>(drain: D) -> Self {
        AtomicSwitch::new_from_arc(Arc::new(ArcCell::new(Arc::new(Box::new(drain) as Box<SendSyncRefUnwindSafeDrain<Ok=O,Err=E>>))))
    }

    /// Create new `AtomicSwitch` from an existing `Arc<...>`
    ///
    /// See `AtomicSwitch::new()`
    pub fn new_from_arc(d: Arc<ArcCell<Box<SendSyncRefUnwindSafeDrain<Ok = O, Err = E>>>>) -> Self {
        AtomicSwitch(d)
    }

    /// Get a `AtomicSwitchCtrl` handle to control this `AtomicSwitch` drain
    pub fn ctrl(&self) -> AtomicSwitchCtrl<O, E> {
        AtomicSwitchCtrl(self.0.clone())
    }
}

impl<O, E> AtomicSwitchCtrl<O, E> {
    /// Get Arc to the currently wrapped drain
    pub fn get(&self) -> Arc<Box<SendSyncRefUnwindSafeDrain<Ok = O, Err = E>>> {
        self.0.get()
    }

    /// Set the current wrapped drain
    pub fn set<D: SendSyncRefUnwindSafeDrain<Ok = O, Err = E>>(&self, drain: D) {
        let _ = self.0.set(Arc::new(Box::new(drain)));
    }

    /// Swap the existing drain with a new one
    pub fn swap(&self,
                drain: Arc<Box<SendSyncRefUnwindSafeDrain<Ok = O, Err = E>>>)
                -> Arc<Box<SendSyncRefUnwindSafeDrain<Ok = O, Err = E>>> {
        self.0.set(drain)
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
        self.0.get().log(info, kv)
    }
}
