//! The memory and speed contest (`just contest`).
//!
//! Each workload runs once per contender in a fresh child process. After the
//! child exits, Windows reports its peak committed memory; the same
//! contender's peak for an empty run (n = 0) is subtracted, so start-up costs
//! cancel and only the workload's own memory counts. Each figure is the best
//! of three runs. Every contender builds identical data and must produce the
//! same checksum.
//!
//! Contenders:
//! - `goose-native`: Goose on its own, built by `goose build` (only when
//!   `GOOSE_CONTEST_EXE` names that executable, which `just contest` does).
//! - `goose-in-rust`: the same Goose code called through this crate.
//! - `rust`: idiomatic Rust (`String`/`Vec` fields, `Option<Box<Node>>`).
//! - `rust-tuned`: the "best safe Rust" layout where one exists
//!   (`Vec<Node>` with `u32` indices).

use std::env;
use std::io::Read;
use std::process::{Command, ExitCode, Stdio};
use std::time::Instant;

use rgt_app_core::{Goose, memory, reference};

const SEED: u64 = 0x5EED_CAFE;
const RUNS: usize = 3;
const WORKLOADS: [(&str, u32); 3] = [
    ("records", 1_000_000),
    ("tree", 1_000_000),
    ("primes", 50_000_000),
];
const CONTENDERS: [&str; 4] = ["goose-native", "goose-in-rust", "rust", "rust-tuned"];

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--child") {
        return child(&args[1..]);
    }
    parent()
}

/// `--child <contender> <workload> <n> <seed>`: run one workload and print
/// `<elapsed_ns> <checksum>`, the same line Goose's contest program prints.
fn child(args: &[String]) -> ExitCode {
    let [contender, workload, n, seed] = args else {
        eprintln!("usage: contest --child <contender> <workload> <n> <seed>");
        return ExitCode::from(2);
    };
    let (Ok(n), Ok(seed)) = (n.parse::<u32>(), seed.parse::<u64>()) else {
        eprintln!("contest: n and seed must be whole numbers");
        return ExitCode::from(2);
    };
    // Only the Goose contender starts the runtime: the empty-run baseline
    // already accounts for whatever a contender needs just to start.
    let goose = (contender == "goose-in-rust").then(|| Goose::start().expect("fresh process"));
    let start = Instant::now();
    let checksum = match (contender.as_str(), workload.as_str(), &goose) {
        ("goose-in-rust", "records", Some(g)) => g.records_checksum(n, seed),
        ("goose-in-rust", "tree", Some(g)) => g.tree_checksum(n, seed),
        ("goose-in-rust", "primes", Some(g)) => g.count_primes(n),
        ("rust", "records", _) => reference::records_checksum(n, seed),
        ("rust", "tree", _) => reference::tree_checksum_boxed(n, seed),
        ("rust", "primes", _) => reference::count_primes(n),
        ("rust-tuned", "tree", _) => reference::tree_checksum_indexed(n, seed),
        _ => return ExitCode::from(3), // this contender has no entry for this workload
    };
    println!("{} {checksum}", start.elapsed().as_nanos());
    ExitCode::SUCCESS
}

/// What one child run reported.
struct Sample {
    nanos: u128,
    checksum: u64,
    peak: Option<u64>,
}

/// Runs one child; `None` when the contender has no entry for the workload.
fn run(contender: &str, workload: &str, n: u32, native: Option<&str>) -> Option<Sample> {
    let mut cmd = if contender == "goose-native" {
        Command::new(native?)
    } else {
        let mut c = Command::new(env::current_exe().expect("own path"));
        c.args(["--child", contender]);
        c
    };
    cmd.args([workload, &n.to_string(), &SEED.to_string()])
        .stdout(Stdio::piped());
    let mut child = cmd.spawn().expect("child starts");
    let mut out = String::new();
    child
        .stdout
        .take()
        .expect("piped")
        .read_to_string(&mut out)
        .expect("child output");
    let status = child.wait().expect("child ends");
    if status.code() == Some(3) {
        return None;
    }
    assert!(status.success(), "{contender} {workload} failed: {status}");
    // Read before `child` is dropped: it keeps the process handle open.
    let peak = memory::peak_commit_of(&child);
    let mut fields = out.split_whitespace();
    let nanos = fields
        .next()
        .and_then(|t| t.parse().ok())
        .expect("elapsed time");
    let checksum = fields
        .next()
        .and_then(|c| c.parse().ok())
        .expect("checksum");
    Some(Sample {
        nanos,
        checksum,
        peak,
    })
}

/// Best of `RUNS`: lowest extra memory and lowest time, with one checksum.
struct Result {
    extra_mb: Option<f64>,
    millis: f64,
    checksum: u64,
}

#[allow(clippy::cast_precision_loss)] // display only
fn measure(contender: &str, workload: &str, n: u32, native: Option<&str>) -> Option<Result> {
    let baseline = (0..RUNS)
        .filter_map(|_| run(contender, workload, 0, native)?.peak)
        .min();
    let samples: Vec<Sample> = (0..RUNS)
        .map(|_| run(contender, workload, n, native))
        .collect::<Option<_>>()?;
    let checksum = samples[0].checksum;
    assert!(
        samples.iter().all(|s| s.checksum == checksum),
        "{contender} {workload}: runs disagree"
    );
    let peak = samples.iter().filter_map(|s| s.peak).min();
    let extra_mb = peak
        .zip(baseline)
        .map(|(p, b)| p.saturating_sub(b) as f64 / 1_048_576.0);
    let millis = samples.iter().map(|s| s.nanos).min().unwrap_or(0) as f64 / 1e6;
    Some(Result {
        extra_mb,
        millis,
        checksum,
    })
}

fn parent() -> ExitCode {
    let native = env::var("GOOSE_CONTEST_EXE")
        .ok()
        .filter(|p| std::path::Path::new(p).is_file());
    if native.is_none() {
        println!("(Goose alone is skipped: run `just contest` to build and include it.)\n");
    }
    println!(
        "{:<9} {:<14} {:>18} {:>11}  checksum",
        "workload", "contender", "extra peak memory", "time"
    );
    let mut all_agree = true;
    for (workload, n) in WORKLOADS {
        let mut reference_sum = None;
        let mut memory_of = Vec::new();
        for contender in CONTENDERS {
            let Some(r) = measure(contender, workload, n, native.as_deref()) else {
                continue;
            };
            let agrees = *reference_sum.get_or_insert(r.checksum) == r.checksum;
            all_agree &= agrees;
            let mem = r
                .extra_mb
                .map_or_else(|| "n/a".to_owned(), |mb| format!("{mb:.1} MB"));
            let verdict = if agrees { "ok" } else { "MISMATCH" };
            println!(
                "{workload:<9} {contender:<14} {mem:>18} {:>8.1} ms  {verdict}",
                r.millis
            );
            memory_of.push((contender, r.extra_mb));
        }
        let goose = memory_of
            .iter()
            .find(|(c, _)| *c == "goose-in-rust")
            .and_then(|(_, m)| *m);
        for (contender, mb) in &memory_of {
            if let (Some(g), Some(m)) = (goose, mb)
                && contender.starts_with("rust")
                && g > 0.0
            {
                println!(
                    "{:<9} -> {contender} used {:.2}x the memory of Goose",
                    "",
                    m / g
                );
            }
        }
        println!();
    }
    if all_agree {
        ExitCode::SUCCESS
    } else {
        eprintln!("error: a contender's checksum differs from the others");
        ExitCode::FAILURE
    }
}
