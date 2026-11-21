// The C boundary between Rust and Goose. Only `export fn`s belong here;
// the Goose code itself lives in ../kernels/. The build script compiles this
// file (and everything it imports) into one C file plus a header, and the
// Rust declarations are generated from that header on every build.
//
// Each export's C name starts with the project name, so it cannot clash
// with another C library linked into the same program. What may cross the
// boundary: integers, floats, bool, flat fixed-size structs, slices of
// those, and a returned `const u8[:]` (see docs/MANUAL.md, "Type mapping").
import kernels from "../kernels";
import kernels.workloads;

export "rgt_app_saxpy" fn saxpy(y: f64[:], x: const f64[:], a: f64) {
    kernels::saxpy(y, x, a);
}

export "rgt_app_count_primes" fn count_primes(n: i64) -> i64 {
    return kernels::count_primes(n)
}

export "rgt_app_fnv1a" fn fnv1a(bytes: const u8[:]) -> u64 {
    return kernels::fnv1a(bytes)
}

export "rgt_app_summary" fn summary(xs: const f64[:]) -> kernels::Summary {
    return kernels::summary(xs)
}

export "rgt_app_version" fn version() -> const u8[:] {
    return "rgt-app kernels 0.1.0"
}

export "rgt_app_records_checksum" fn records_checksum(n: i64, seed: u64) -> u64 {
    return kernels::records_checksum(n, seed)
}

export "rgt_app_tree_checksum" fn tree_checksum(n: i64, seed: u64) -> u64 {
    return kernels::tree_checksum(n, seed)
}
