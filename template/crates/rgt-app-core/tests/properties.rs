//! Goose and Rust agree on thousands of random inputs, including empty
//! inputs, NaN, infinities, subnormals and extreme values.
//!
//! Floats are compared bit for bit, which is valid because neither compiler
//! reorders float math. The one exception is NaN: IEEE does not fix which NaN
//! bit pattern an operation returns, and a compiler may legally swap the
//! operands of `a * x`, so any NaN matches any NaN.
//!
//! A failure prints the smallest input that still fails.

use proptest::collection::vec;
use proptest::prelude::*;
use proptest::test_runner::{Config, TestRunner};
use rgt_app_core::{Summary, reference, testing::with_goose};

/// Same value: identical bits, or both NaN.
fn same(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

fn same_summary(a: Summary, b: Summary) -> bool {
    let (ac, amin, amax, amean) = (a.count, a.min, a.max, a.mean);
    let (bc, bmin, bmax, bmean) = (b.count, b.min, b.max, b.mean);
    ac == bc && same(amin, bmin) && same(amax, bmax) && same(amean, bmean)
}

/// `cases` random inputs, or `PROPTEST_CASES` of them when that is set.
fn runner(cases: u32) -> TestRunner {
    let cases = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(cases);
    TestRunner::new(Config {
        cases,
        failure_persistence: None,
        ..Config::default()
    })
}

/// Any f64 at all: normal, subnormal, zero, infinite or NaN.
fn any_f64() -> impl Strategy<Value = f64> {
    proptest::num::f64::ANY
}

#[test]
fn saxpy_agrees() {
    let inputs = (0..512usize).prop_flat_map(|n| (vec(any_f64(), n), vec(any_f64(), n), any_f64()));
    with_goose(|goose| {
        runner(2000)
            .run(&inputs, |(y, x, a)| {
                let (mut from_goose, mut from_rust) = (y.clone(), y);
                goose.saxpy(&mut from_goose, &x, a);
                reference::saxpy(&mut from_rust, &x, a);
                let agree = from_goose.iter().zip(&from_rust).all(|(&g, &r)| same(g, r));
                prop_assert!(agree, "goose {from_goose:?} vs rust {from_rust:?}");
                Ok(())
            })
            .unwrap();
    });
}

#[test]
fn fnv1a_agrees() {
    with_goose(|goose| {
        runner(2000)
            .run(&vec(any::<u8>(), 0..4096), |bytes| {
                prop_assert_eq!(goose.fnv1a(&bytes), reference::fnv1a(&bytes));
                Ok(())
            })
            .unwrap();
    });
}

#[test]
fn summary_agrees() {
    with_goose(|goose| {
        runner(2000)
            .run(&vec(any_f64(), 0..512), |xs| {
                match (goose.summary(&xs), reference::summary(&xs)) {
                    (None, None) => prop_assert!(xs.is_empty()),
                    (Some(g), Some(r)) => {
                        prop_assert!(same_summary(g, r), "goose {g:?} vs rust {r:?}")
                    }
                    (g, r) => prop_assert!(false, "goose {g:?} vs rust {r:?}"),
                }
                Ok(())
            })
            .unwrap();
    });
}

#[test]
fn count_primes_agrees() {
    with_goose(|goose| {
        runner(500)
            .run(&(0u32..100_000), |n| {
                prop_assert_eq!(goose.count_primes(n), reference::count_primes(n));
                Ok(())
            })
            .unwrap();
    });
}

#[test]
fn workloads_agree() {
    with_goose(|goose| {
        runner(300)
            .run(&(0u32..2000, any::<u64>()), |(n, seed)| {
                prop_assert_eq!(
                    goose.records_checksum(n, seed),
                    reference::records_checksum(n, seed)
                );
                let tree = goose.tree_checksum(n, seed);
                prop_assert_eq!(tree, reference::tree_checksum_boxed(n, seed));
                prop_assert_eq!(tree, reference::tree_checksum_indexed(n, seed));
                Ok(())
            })
            .unwrap();
    });
}
