//! Helpers for tests and benchmarks.

use std::sync::{Mutex, PoisonError};

use crate::Goose;

/// Serializes runtime use, so tests also work under plain `cargo test`
/// (one process, many threads), not only `cargo nextest` (one process each).
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

/// Runs `f` with a fresh Goose runtime, one caller at a time.
///
/// The runtime is shut down when `f` returns, and also when it panics, so a
/// failing test never leaves a runtime behind for the next one.
///
/// # Panics
///
/// If a runtime was started some other way and is still running.
pub fn with_goose<R>(f: impl FnOnce(&Goose) -> R) -> R {
    // A test that panicked while holding the lock leaves it poisoned; the
    // runtime it held was still shut down, so carrying on is safe.
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
    let goose = Goose::start().expect("a Goose runtime is already running outside with_goose");
    f(&goose)
}
