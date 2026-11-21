//! Goose vs Rust micro-benchmarks (`just bench`).
//!
//! Each pair runs the same kernel on the same data: once through the Goose
//! bridge and once as the plain-Rust twin. Every Goose benchmark starts its
//! own runtime and measures on that same thread (`bench_local`), as the
//! runtime requires.

use divan::{Bencher, black_box};
use rgt_app_core::{Goose, reference};

fn main() {
    divan::main();
}

fn goose() -> Goose {
    Goose::start().expect("one benchmark at a time")
}

fn ramp(n: u32) -> Vec<f64> {
    (0..n).map(|i| f64::from(i % 1000) * 0.001).collect()
}

fn bytes(len: usize) -> Vec<u8> {
    (0..=255u8).cycle().take(len).collect()
}

mod saxpy_1m {
    use super::*;

    #[divan::bench]
    fn goose(bencher: Bencher) {
        let g = super::goose();
        let (x, mut y) = (ramp(1_000_000), vec![1.0; 1_000_000]);
        bencher.bench_local(|| g.saxpy(black_box(&mut y), black_box(&x), 0.5));
    }

    #[divan::bench]
    fn rust(bencher: Bencher) {
        let (x, mut y) = (ramp(1_000_000), vec![1.0; 1_000_000]);
        bencher.bench_local(|| reference::saxpy(black_box(&mut y), black_box(&x), 0.5));
    }
}

mod fnv1a_16mb {
    use super::*;

    #[divan::bench]
    fn goose(bencher: Bencher) {
        let (g, data) = (super::goose(), bytes(16 << 20));
        bencher.bench_local(|| g.fnv1a(black_box(&data)));
    }

    #[divan::bench]
    fn rust(bencher: Bencher) {
        let data = bytes(16 << 20);
        bencher.bench_local(|| reference::fnv1a(black_box(&data)));
    }
}

mod count_primes_5m {
    use super::*;

    #[divan::bench]
    fn goose(bencher: Bencher) {
        let g = super::goose();
        bencher.bench_local(|| g.count_primes(black_box(5_000_000)));
    }

    #[divan::bench]
    fn rust(bencher: Bencher) {
        bencher.bench_local(|| reference::count_primes(black_box(5_000_000)));
    }
}

mod summary_1m {
    use super::*;

    #[divan::bench]
    fn goose(bencher: Bencher) {
        let (g, xs) = (super::goose(), ramp(1_000_000));
        bencher.bench_local(|| g.summary(black_box(&xs)));
    }

    #[divan::bench]
    fn rust(bencher: Bencher) {
        let xs = ramp(1_000_000);
        bencher.bench_local(|| reference::summary(black_box(&xs)));
    }
}
