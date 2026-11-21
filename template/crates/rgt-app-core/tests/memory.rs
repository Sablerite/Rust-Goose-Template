//! Goose's memory is reused between calls and handed back at shutdown.
//!
//! Goose frees a scope's data by moving a pointer back, and keeps the pages
//! for the next call. So repeated calls must not grow the process, and
//! shutting the runtime down must return what it committed. Windows only:
//! that is where `memory` can read commit.
#![cfg(windows)]

use rgt_app_core::{Goose, memory, testing::with_goose};

/// Allowed drift: 2% plus 1 MiB, for the allocator and the test harness.
fn limit(base: u64) -> u64 {
    base + base / 50 + (1 << 20)
}

fn run_all(goose: &Goose) -> (u64, u64, u64) {
    (
        goose.records_checksum(50_000, 1),
        goose.tree_checksum(50_000, 2),
        goose.count_primes(2_000_000),
    )
}

#[test]
fn repeated_calls_do_not_grow_memory() {
    with_goose(|goose| {
        let first = run_all(goose); // warm up: the data stacks commit their pages
        let base = memory::current_commit().expect("Windows reports commit");
        for _ in 0..50 {
            assert_eq!(run_all(goose), first);
        }
        let after = memory::current_commit().expect("Windows reports commit");
        assert!(
            after <= limit(base),
            "commit grew from {base} to {after} bytes over 50 calls"
        );
    });
}

#[test]
fn shutdown_returns_goose_memory() {
    let before = memory::current_commit().expect("Windows reports commit");
    let mut during = 0;
    with_goose(|goose| {
        // Well over the allowed drift, so a leak could not hide in it.
        goose.count_primes(64_000_000);
        during = memory::current_commit().expect("Windows reports commit");
    });
    let after = memory::current_commit().expect("Windows reports commit");
    assert!(
        during > limit(before),
        "the workload should have committed memory: {before} -> {during}"
    );
    assert!(
        after <= limit(before),
        "shutdown kept memory: {before} before, {during} during, {after} after"
    );
}
