// The hot-path kernels, written in Goose. Rust reaches them through
// ../lib.goose, which exports each one under a C name. Every kernel has a
// plain-Rust twin in src/reference.rs, and the tests check that both give
// the same answer, bit for bit.
namespace kernels;

/// Count, smallest, largest and mean of a non-empty series.
///
/// Goose packs structs to one byte, so this is 28 bytes (4 + 3 * 8), where
/// a normal C struct would be 32. The Rust side sees the same packed layout.
struct Summary { count: u32, min: f64, max: f64, mean: f64 }

/// `y[i] += a * x[i]` for every `i`. Both slices must have the same length.
fn saxpy(y: f64[:], x: const f64[:], a: f64) {
    // The length check also lets the compiler drop every bounds check below.
    assert(y.len == x.len);
    for i in y.len { y[i] += a * x[i]; }
}

/// How many primes are at most `n` (a sieve of Eratosthenes).
fn count_primes(n: i64) -> i64 {
    guard n >= 2 else { return 0; }
    var flags: u8[>..] = [];
    flags.resize(n + 1, 1);
    flags[0] = 0;
    flags[1] = 0;
    var i: i64 = 2;
    while i * i <= n {
        if flags[i] == 1 {
            var j = i * i;
            while j <= n { flags[j] = 0; j += i; }
        }
        i++;
    }
    var count: i64 = 0;
    for f in flags { count += f; }
    return count
}

/// 64-bit FNV-1a hash. Unsigned math wraps in Goose, as the hash expects.
fn fnv1a(xs: const u8[:]) -> u64 {
    var h: u64 = 14695981039346656037;
    for b in xs { h = (h ^ b) * 1099511628211; }
    return h
}

/// Count, min, max and mean of `xs`, which must not be empty.
///
/// `min` and `max` start at the first value and use plain `<` and `>`, so a
/// NaN never replaces a number (and a leading NaN stays). The sum runs left
/// to right, which the Rust twin repeats exactly.
fn summary(xs: const f64[:]) -> Summary {
    assert(xs.len > 0);
    var s = Summary { count: xs.len as! u32, min: xs[0], max: xs[0], mean: 0.0 };
    var total = 0.0;
    for x in xs {
        if x < s.min { s.min = x; }
        if x > s.max { s.max = x; }
        total += x;
    }
    s.mean = total / xs.len;
    return s
}

/// One step of splitmix64, the small generator both languages share so the
/// contest builds identical data on each side.
fn splitmix64(state: u64&) -> u64 {
    state += 0x9E3779B97F4A7C15;
    var z: u64 = state;
    z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9;
    z = (z ^ (z >> 27)) * 0x94D049BB133111EB;
    return z ^ (z >> 31)
}
