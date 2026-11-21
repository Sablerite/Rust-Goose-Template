//! Raw declarations for the Goose exports, generated on every build from the
//! header Goose writes (see `build.rs`).
//!
//! Everything here is `unsafe` to call and has none of the checks the safe
//! API in the crate root adds. It is public for two reasons: tests that
//! deliberately bypass the safe API, and code that needs an export before
//! a wrapper exists. Prefer the [`Goose`](crate::Goose) methods.
//!
//! Structs are `#[repr(C, packed)]`, matching Goose's one-byte packing.
//! Read their fields by value (`let n = s.count;`): taking a reference to a
//! field of a packed struct is a compile error, because it could be
//! unaligned.
#![allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    missing_docs,
    missing_debug_implementations,
    unsafe_op_in_unsafe_fn,
    clippy::all,
    clippy::undocumented_unsafe_blocks,
    clippy::multiple_unsafe_ops_per_block,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::Summary;
    use std::mem::{align_of, size_of};

    /// Goose packs structs to one byte: `u32` + 3 * `f64` is 28 bytes, not
    /// the 32 a normal C struct would take.
    #[test]
    fn summary_is_packed() {
        assert_eq!(size_of::<Summary>(), 28);
        assert_eq!(align_of::<Summary>(), 1);
    }
}
