//! Every Goose function gives exactly the same answer as its plain-Rust twin
//! in `reference`, on fixed inputs. (Random inputs are in `properties.rs`.)

use rgt_app_core::{Summary, reference, testing::with_goose};

/// Floats compared bit for bit: neither compiler reorders float math.
fn bits(xs: &[f64]) -> Vec<u64> {
    xs.iter().map(|x| x.to_bits()).collect()
}

/// A packed struct's fields, read by value (references to them are not allowed).
fn summary_bits(s: Summary) -> (u32, u64, u64, u64) {
    let (count, min, max, mean) = (s.count, s.min, s.max, s.mean);
    (count, min.to_bits(), max.to_bits(), mean.to_bits())
}

fn ramp(n: u32) -> Vec<f64> {
    (0..n).map(|i| f64::from(i) * 0.25 - 3.0).collect()
}

#[test]
fn saxpy_matches_reference() {
    with_goose(|goose| {
        let x = ramp(1000);
        let mut from_goose = vec![1.5; 1000];
        let mut from_rust = from_goose.clone();
        goose.saxpy(&mut from_goose, &x, -0.75);
        reference::saxpy(&mut from_rust, &x, -0.75);
        assert_eq!(bits(&from_goose), bits(&from_rust));
    });
}

#[test]
fn count_primes_matches_reference() {
    with_goose(|goose| {
        for n in [0, 1, 2, 3, 100, 7919, 1_000_000] {
            assert_eq!(goose.count_primes(n), reference::count_primes(n), "n = {n}");
        }
        assert_eq!(goose.count_primes(1_000_000), 78_498);
    });
}

#[test]
fn fnv1a_matches_published_vectors_and_reference() {
    with_goose(|goose| {
        assert_eq!(goose.fnv1a(b""), 14_695_981_039_346_656_037);
        assert_eq!(goose.fnv1a(b"a"), 12_638_187_200_555_641_996);
        let bytes: Vec<u8> = (0..=255u8).cycle().take(100_000).collect();
        assert_eq!(goose.fnv1a(&bytes), reference::fnv1a(&bytes));
    });
}

#[test]
fn summary_matches_reference() {
    with_goose(|goose| {
        let s = goose.summary(&[3.0, -1.5, 4.5]).expect("not empty");
        assert_eq!(
            summary_bits(s),
            (3, (-1.5f64).to_bits(), 4.5f64.to_bits(), 2.0f64.to_bits())
        );
        let xs = ramp(10_001);
        let (g, r) = (
            goose.summary(&xs).expect("not empty"),
            reference::summary(&xs).expect("not empty"),
        );
        assert_eq!(summary_bits(g), summary_bits(r));
    });
}

#[test]
fn version_comes_back_as_a_string() {
    with_goose(|goose| assert_eq!(goose.version(), "rgt-app kernels 0.1.0"));
}

#[test]
fn workloads_match_reference() {
    with_goose(|goose| {
        for (n, seed) in [(0, 1), (1, 7), (1000, 42), (50_000, 9)] {
            assert_eq!(
                goose.records_checksum(n, seed),
                reference::records_checksum(n, seed),
                "records n = {n}"
            );
            let tree = goose.tree_checksum(n, seed);
            assert_eq!(
                tree,
                reference::tree_checksum_boxed(n, seed),
                "tree (boxed) n = {n}"
            );
            assert_eq!(
                tree,
                reference::tree_checksum_indexed(n, seed),
                "tree (indexed) n = {n}"
            );
        }
    });
}
