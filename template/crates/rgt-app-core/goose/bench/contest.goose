// The contest workloads run by Goose alone, built with `goose build` (see
// `just contest`). Prints `<elapsed nanoseconds> <checksum>`, the same line
// the Rust contenders print, so the results line up in one table.
//
//   contest <records|tree|primes> <n> <seed>
import std;
import os;
import kernels from "../kernels";
import kernels.workloads;

fn main() {
    let a = args();
    guard a.len == 4 else {
        write_stderr("usage: contest <records|tree|primes> <n> <seed>\n");
        exit(2);
    }
    let n, n_ok = parse_int(a[2]);
    let seed, seed_ok = parse_int(a[3]);
    guard n_ok && seed_ok && n >= 0 && seed >= 0 else {
        write_stderr("contest: n and seed must be whole numbers, 0 or more\n");
        exit(2);
    }
    let t0 = clock_ns();
    var result: u64 = 0;
    if a[1] == "records" {
        result = kernels::records_checksum(n, seed as! u64);
    } else if a[1] == "tree" {
        result = kernels::tree_checksum(n, seed as! u64);
    } else if a[1] == "primes" {
        result = kernels::count_primes(n) as! u64;
    } else {
        write_stderr("contest: unknown workload (records, tree or primes)\n");
        exit(2);
    }
    let elapsed = clock_ns() - t0;
    print(elapsed, " ", result);
}
