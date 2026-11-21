//! Rgt App: the Rust program. The hot paths run in Goose, through the safe
//! API in `rgt-app-core`.

use std::process::ExitCode;

use rgt_app_core::{Goose, reference};

fn main() -> ExitCode {
    let goose = match Goose::start() {
        Ok(goose) => goose,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    println!("{}", goose.version());

    let x: Vec<f64> = (0..1_000_000)
        .map(|i| f64::from(i % 1000) * 0.001)
        .collect();
    let mut y = vec![1.0; x.len()];
    goose.saxpy(&mut y, &x, 0.5);
    let summary = goose.summary(&y).expect("y is not empty");
    let (count, min, max, mean) = (summary.count, summary.min, summary.max, summary.mean);
    println!("saxpy over {count} values: min {min}, max {max}, mean {mean:.6}");
    println!("primes up to 1,000,000: {}", goose.count_primes(1_000_000));

    // The same work in plain Rust, as a check that both sides agree.
    let mut check = vec![1.0; x.len()];
    reference::saxpy(&mut check, &x, 0.5);
    if y != check {
        eprintln!("error: Goose and Rust disagree");
        return ExitCode::FAILURE;
    }
    println!("Goose and Rust agree.");
    ExitCode::SUCCESS
}
