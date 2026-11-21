//! Plain-Rust versions of every Goose kernel and workload.
//!
//! They exist to check Goose (the tests compare results bit for bit) and to
//! race it (the benchmarks and the contest). Each one repeats its Goose
//! twin's algorithm exactly: same order of random draws, same order of
//! float operations. Unsigned math uses `wrapping_*` because Goose's unsigned
//! math always wraps, while Rust's panics on overflow in debug builds.

use crate::Summary;

/// `y[i] += a * x[i]` for every `i`.
///
/// # Panics
///
/// If `y` and `x` have different lengths.
pub fn saxpy(y: &mut [f64], x: &[f64], a: f64) {
    assert!(
        y.len() == x.len(),
        "saxpy: y has {} elements but x has {}",
        y.len(),
        x.len()
    );
    for (yi, &xi) in y.iter_mut().zip(x) {
        *yi += a * xi;
    }
}

/// How many primes are at most `n` (a sieve of Eratosthenes).
pub fn count_primes(n: u32) -> u64 {
    if n < 2 {
        return 0;
    }
    let n = n as usize;
    let mut flags = vec![1u8; n + 1];
    flags[0] = 0;
    flags[1] = 0;
    let mut i = 2;
    while i * i <= n {
        if flags[i] == 1 {
            let mut j = i * i;
            while j <= n {
                flags[j] = 0;
                j += i;
            }
        }
        i += 1;
    }
    flags.iter().map(|&f| u64::from(f)).sum()
}

/// 64-bit FNV-1a hash.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 14_695_981_039_346_656_037;
    for &b in bytes {
        h = (h ^ u64::from(b)).wrapping_mul(1_099_511_628_211);
    }
    h
}

/// Count, min, max and mean of `xs`, or `None` when it is empty. `min` and
/// `max` start at the first value and use plain `<`/`>`; the sum runs left to
/// right.
///
/// # Panics
///
/// If `xs` has more than `u32::MAX` elements.
pub fn summary(xs: &[f64]) -> Option<Summary> {
    let first = *xs.first()?;
    let count = u32::try_from(xs.len()).expect("summary: more than u32::MAX values");
    let (mut min, mut max, mut total) = (first, first, 0.0);
    for &x in xs {
        if x < min {
            min = x;
        }
        if x > max {
            max = x;
        }
        total += x;
    }
    #[allow(clippy::cast_precision_loss)] // the same i64 -> f64 conversion Goose does
    let mean = total / xs.len() as f64;
    Some(Summary {
        count,
        min,
        max,
        mean,
    })
}

/// One step of splitmix64, the generator Goose's workloads use.
pub fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// `splitmix64(state) % m` as a narrower integer (always in range).
fn draw(state: &mut u64, m: u64) -> u64 {
    splitmix64(state) % m
}

/// The idiomatic Rust record: three allocations (the struct's slot in the
/// `Vec`, the name's bytes, the scores) where Goose stores one flat run.
struct Record {
    id: u32,
    name: String,
    scores: Vec<u16>,
}

/// The Rust twin of Goose's `records_checksum`.
pub fn records_checksum(n: u32, seed: u64) -> u64 {
    let mut state = seed;
    let mut book = Vec::new();
    for id in 0..n {
        let name_len = 4 + draw(&mut state, 16);
        let name: String = (0..name_len)
            .map(|_| char::from(b'a' + u8::try_from(draw(&mut state, 26)).expect("< 26")))
            .collect();
        let score_count = draw(&mut state, 8);
        let scores = (0..score_count)
            .map(|_| u16::try_from(draw(&mut state, 1000)).expect("< 1000"))
            .collect();
        book.push(Record { id, name, scores });
    }
    let mut h: u64 = 0;
    for r in &book {
        h = h.wrapping_mul(31).wrapping_add(u64::from(r.id));
        for b in r.name.bytes() {
            h = h.wrapping_add(u64::from(b));
        }
        for &s in &r.scores {
            h = h.wrapping_add(u64::from(s));
        }
    }
    h
}

/// The keys both tree versions insert, in order.
fn tree_keys(n: u32, seed: u64) -> impl Iterator<Item = u32> {
    let range = 4 * u64::from(n) + 1;
    let mut state = seed;
    (0..n).map(move |_| u32::try_from(draw(&mut state, range)).expect("< 4n + 1"))
}

/// `distinct * 1000003 + found + height`, wrapping, as Goose computes it.
fn tree_result(distinct: u64, found: u64, height: u64) -> u64 {
    distinct
        .wrapping_mul(1_000_003)
        .wrapping_add(found)
        .wrapping_add(height)
}

/// The idiomatic Rust tree: one heap allocation per node.
struct BoxNode {
    key: u32,
    count: u32,
    left: Option<Box<BoxNode>>,
    right: Option<Box<BoxNode>>,
}

fn box_insert(mut slot: &mut Option<Box<BoxNode>>, key: u32) -> bool {
    while let Some(node) = slot {
        if key == node.key {
            node.count += 1;
            return false;
        }
        slot = if key < node.key {
            &mut node.left
        } else {
            &mut node.right
        };
    }
    *slot = Some(Box::new(BoxNode {
        key,
        count: 1,
        left: None,
        right: None,
    }));
    true
}

fn box_find(mut node: &Option<Box<BoxNode>>, key: u32) -> u32 {
    while let Some(n) = node {
        if key == n.key {
            return n.count;
        }
        node = if key < n.key { &n.left } else { &n.right };
    }
    0
}

fn box_height(node: &Option<Box<BoxNode>>) -> u64 {
    node.as_ref()
        .map_or(0, |n| 1 + box_height(&n.left).max(box_height(&n.right)))
}

/// The Rust twin of Goose's `tree_checksum`, written the idiomatic way:
/// `Option<Box<Node>>` links.
pub fn tree_checksum_boxed(n: u32, seed: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut root = None;
    let mut distinct = 0u64;
    for key in tree_keys(n, seed) {
        if box_insert(&mut root, key) {
            distinct += 1;
        }
    }
    let found = tree_keys(n, seed)
        .map(|key| u64::from(box_find(&root, key)))
        .sum();
    tree_result(distinct, found, box_height(&root))
}

/// The tuned "best safe Rust" node: 16 bytes, children as `u32` indices into
/// one `Vec` (0 means none, since slot 0 is the root and never a child).
struct IdxNode {
    key: u32,
    count: u32,
    left: u32,
    right: u32,
}

fn idx_height(pool: &[IdxNode], i: u32) -> u64 {
    if i == 0 && pool.is_empty() {
        return 0;
    }
    let node = &pool[i as usize];
    let left = if node.left == 0 {
        0
    } else {
        idx_height(pool, node.left)
    };
    let right = if node.right == 0 {
        0
    } else {
        idx_height(pool, node.right)
    };
    1 + left.max(right)
}

/// The Rust twin of Goose's `tree_checksum`, tuned: `Vec<Node>` with `u32`
/// indices, the layout safe Rust uses when memory matters.
pub fn tree_checksum_indexed(n: u32, seed: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut pool: Vec<IdxNode> = Vec::new();
    for key in tree_keys(n, seed) {
        if pool.is_empty() {
            pool.push(IdxNode {
                key,
                count: 1,
                left: 0,
                right: 0,
            });
            continue;
        }
        let mut cur = 0usize;
        loop {
            let node = &pool[cur];
            if key == node.key {
                pool[cur].count += 1;
                break;
            }
            let next = if key < node.key {
                node.left
            } else {
                node.right
            };
            if next != 0 {
                cur = next as usize;
                continue;
            }
            let new = u32::try_from(pool.len()).expect("fewer than u32::MAX nodes");
            pool.push(IdxNode {
                key,
                count: 1,
                left: 0,
                right: 0,
            });
            if key < pool[cur].key {
                pool[cur].left = new;
            } else {
                pool[cur].right = new;
            }
            break;
        }
    }
    let find = |key: u32| -> u32 {
        let mut cur = 0usize;
        loop {
            let node = &pool[cur];
            if key == node.key {
                return node.count;
            }
            let next = if key < node.key {
                node.left
            } else {
                node.right
            };
            if next == 0 {
                return 0;
            }
            cur = next as usize;
        }
    };
    let found = tree_keys(n, seed).map(|key| u64::from(find(key))).sum();
    tree_result(pool.len() as u64, found, idx_height(&pool, 0))
}
