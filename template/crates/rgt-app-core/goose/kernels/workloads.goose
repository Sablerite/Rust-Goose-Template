// Memory-heavy workloads for the contest (`just contest`). Each one builds
// a large structure the Goose way, walks it, and returns a checksum that
// the Rust versions in src/reference.rs must match exactly.
namespace kernels;

import std;
import kernels;

/// A record kept flat: the id and both lengths are varints (as wide as
/// their values need), and the name and scores are stored inline, so a
/// whole record is one short run of bytes with no pointers in it.
struct Record { id: varint, name: u8[varint], scores: u16[varint] }

/// Builds `n` records from `seed`, then walks them for a checksum.
///
/// For record `i`, in this order: the name length is `4 + r % 16`, each
/// letter is `'a' + r % 26`, the score count is `r % 8`, and each score is
/// `r % 1000`, where every `r` is the next splitmix64 value. The checksum,
/// wrapping, is `h = h * 31 + id`, then `+ byte` per name byte and
/// `+ score` per score.
fn records_checksum(n: i64, seed: u64) -> u64 {
    var state = seed;
    var book: Record[>..] = [];
    for i in n {
        var name: u8[>..] = [];
        let name_len = 4 + splitmix64(state) % 16;
        for k in name_len { name.push((97 + splitmix64(state) % 26) as! u8); }
        var scores: u16[>..] = [];
        let score_count = splitmix64(state) % 8;
        for k in score_count { scores.push((splitmix64(state) % 1000) as! u16); }
        // Slices copy the scratch bytes straight into the record's place.
        book.push(Record { id: i, name: name[..], scores: scores[..] });
    }
    var h: u64 = 0;
    for r in book {
        h = h * 31 + (r.id as! u64);
        for b in r.name { h += b; }
        for s in r.scores { h += s; }
    }
    return h
}

/// A binary search tree node: 16 bytes, with 4-byte self-relative links
/// where a pointer would take 8. The whole tree is one grow-only array.
struct Node { key: u32, count: u32 = 1, left: Node&<u32>?, right: Node&<u32>? }

/// Adds `key` to the tree in `pool` (slot 0 is the root). A key already
/// present has its count raised instead.
fn tree_insert(pool: Node[>..]&, key: u32) {
    if pool.len == 0 { pool.push(Node { key: key }); return; }
    var cur .= pool[0];
    loop {
        if key == cur.key { cur.count += 1; return; }
        if key < cur.key {
            let l = cur.left;
            if l { cur .= l; continue; }
            cur.left .= pool.push(Node { key: key });   // a push never moves cur
            return;
        }
        let r = cur.right;
        if r { cur .= r; continue; }
        cur.right .= pool.push(Node { key: key });
        return;
    }
}

/// How many times `key` was inserted (0 when absent).
fn tree_find(pool: Node[>..]&, key: u32) -> u32 {
    guard pool.len > 0 else { return 0; }
    var cur .= pool[0];
    loop {
        if key == cur.key { return cur.count; }
        let next = if key < cur.key { cur.left } else { cur.right };
        guard next else { return 0; }
        cur .= next;
    }
}

/// Levels from `n` down to its deepest leaf, counting `n` itself.
recursive fn tree_height(n: Node&) -> i64 {
    var h = 0;
    let l = n.left;
    if l { h = tree_height(l); }
    let r = n.right;
    if r { h = max(h, tree_height(r)); }
    return h + 1
}

/// Inserts `n` keys (`r % (4n + 1)`, from splitmix64 seeded with `seed`)
/// into an unbalanced tree, then generates the same keys again and looks
/// each one up. The result, wrapping, is
/// `distinct * 1000003 + sum of the counts found + height`.
fn tree_checksum(n: i64, seed: u64) -> u64 {
    guard n > 0 else { return 0; }
    let range = (4 * n + 1) as! u64;
    var pool: Node[>..] = [];
    var state = seed;
    for i in n { tree_insert(pool, (splitmix64(state) % range) as! u32); }
    state = seed;
    var found: u64 = 0;
    for i in n { found += tree_find(pool, (splitmix64(state) % range) as! u32); }
    let distinct = pool.len as! u64;
    return distinct * 1000003 + found + (tree_height(pool[0]) as! u64)
}
