//! The rules at the Rust/Goose boundary, each pinned by a test.
//!
//! Goose's exported functions must run on the thread that started the
//! runtime, only one runtime may run per process, and a failed Goose check
//! ends the whole process. The safe API turns each of these into something
//! Rust can see: a type rule, an `Err`, or a normal panic before Goose runs.

use std::mem::{align_of, size_of};
use std::panic;
use std::process::Command;
use std::thread;

use rgt_app_core::{Goose, MAX_PRIMES_N, StartError, Summary, ffi, reference, testing::with_goose};

#[test]
fn start_twice_is_rejected() {
    with_goose(|_goose| {
        assert_eq!(Goose::start().err(), Some(StartError::AlreadyRunning));
    });
}

#[test]
fn start_from_another_thread_is_rejected() {
    with_goose(|_goose| {
        let other = thread::spawn(|| Goose::start().map(drop))
            .join()
            .expect("thread ran");
        assert_eq!(other, Err(StartError::AlreadyRunning));
    });
}

#[test]
fn restart_after_drop_works() {
    for _ in 0..3 {
        with_goose(|goose| assert_eq!(goose.count_primes(100), 25));
    }
}

#[test]
fn panic_inside_with_goose_still_shuts_down() {
    let result = panic::catch_unwind(|| with_goose(|_goose| panic!("boom")));
    assert!(result.is_err());
    // The runtime was shut down while unwinding, so a new one starts.
    with_goose(|goose| assert_eq!(goose.count_primes(100), 25));
}

#[test]
#[should_panic(expected = "saxpy: y has 3 elements but x has 2")]
fn mismatched_lengths_panic_in_rust() {
    with_goose(|goose| goose.saxpy(&mut [0.0; 3], &[1.0; 2], 1.0));
}

#[test]
#[should_panic(expected = "count_primes: n must be at most")]
fn oversized_input_panics_in_rust() {
    with_goose(|goose| goose.count_primes(MAX_PRIMES_N + 1));
}

#[test]
fn empty_inputs_are_safe() {
    with_goose(|goose| {
        goose.saxpy(&mut [], &[], 2.0);
        assert_eq!(goose.fnv1a(&[]), 14_695_981_039_346_656_037);
        assert!(goose.summary(&[]).is_none());
        assert_eq!(goose.count_primes(0), 0);
        assert_eq!(goose.records_checksum(0, 1), 0);
        assert_eq!(goose.tree_checksum(0, 1), 0);
    });
}

#[test]
fn packed_layout_matches_goose() {
    assert_eq!(size_of::<Summary>(), 28);
    assert_eq!(align_of::<Summary>(), 1);
    // Values that would come back scrambled if any field were misaligned.
    let xs = [
        f64::MIN_POSITIVE,
        -0.0,
        1e300,
        f64::from_bits(0x0123_4567_89AB_CDEF),
    ];
    with_goose(|goose| {
        let g = goose.summary(&xs).expect("not empty");
        let r = reference::summary(&xs).expect("not empty");
        let (gc, gmin, gmax, gmean) = (g.count, g.min, g.max, g.mean);
        let (rc, rmin, rmax, rmean) = (r.count, r.min, r.max, r.mean);
        assert_eq!(gc, 4);
        assert_eq!(
            (gc, gmin.to_bits(), gmax.to_bits()),
            (rc, rmin.to_bits(), rmax.to_bits())
        );
        assert_eq!(gmean.to_bits(), rmean.to_bits());
    });
}

/// Set in the child process that the test below starts.
const DEATH_TEST_VAR: &str = "GOOSE_DEATH_TEST";

/// Goose's own checks still guard the boundary when the safe API is bypassed:
/// breaking `summary`'s `assert(xs.len > 0)` ends the process with Goose's
/// message and exit code 1. It runs in a child process, so only the child dies.
#[test]
fn goose_check_failure_aborts_process() {
    if std::env::var_os(DEATH_TEST_VAR).is_some() {
        let _goose = Goose::start().expect("the child process starts fresh");
        let empty: [f64; 0] = [];
        let slice = ffi::sl_f64 {
            data: empty.as_ptr().cast_mut(),
            len: 0,
        };
        // SAFETY: the runtime is running on this thread and the slice is valid
        // (empty). Calling the raw export with it is legal C; Goose's own
        // `assert` is what rejects it, which is the behaviour under test.
        unsafe { ffi::rgt_app_summary(slice) };
        unreachable!("Goose should have ended the process");
    }
    let exe = std::env::current_exe().expect("test binary path");
    let out = Command::new(exe)
        .args([
            "--exact",
            "goose_check_failure_aborts_process",
            "--nocapture",
        ])
        .env(DEATH_TEST_VAR, "1")
        .output()
        .expect("child ran");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "stderr: {stderr}");
    assert!(stderr.contains("goose runtime error"), "stderr: {stderr}");
}
