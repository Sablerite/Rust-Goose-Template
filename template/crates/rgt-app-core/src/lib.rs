//! The Goose hot paths of Rgt App, behind a safe Rust API.
//!
//! Start the Goose runtime with [`Goose::start`] and call its methods:
//!
//! ```
//! let goose = rgt_app_core::Goose::start()?;
//! let mut y = vec![1.0, 2.0, 3.0];
//! goose.saxpy(&mut y, &[10.0, 20.0, 30.0], 0.5);
//! assert_eq!(y, [6.0, 12.0, 18.0]);
//! # Ok::<(), rgt_app_core::StartError>(())
//! ```
//!
//! # The rules this API enforces
//!
//! Goose's exported functions come with rules that C cannot check. Each one
//! is turned into something Rust can see:
//!
//! | Goose rule | Here |
//! |---|---|
//! | Call exports only from the thread that called `goose_init` | [`Goose`] is neither `Send` nor `Sync`: moving it to another thread does not compile |
//! | One runtime per process | [`Goose::start`] returns [`StartError::AlreadyRunning`] |
//! | Call `goose_exit` when done | dropping [`Goose`] does it, even while a panic unwinds |
//! | A failed Goose check ends the whole process (no unwinding) | each method checks its inputs in Rust first and panics there instead |
//! | Returned `const u8[:]` bytes are only borrowed | they are copied into a `String` before returning |
//! | Structs are packed to one byte | [`Summary`] is `#[repr(C, packed)]`, generated from Goose's own header |
#![warn(missing_docs)]

pub mod ffi;
pub mod memory;
pub mod reference;
pub mod testing;

pub use ffi::Summary;

use std::ffi::c_char;
use std::fmt;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};

/// The largest `n` [`Goose::count_primes`] accepts: one byte per number must
/// fit in one of Goose's data stacks (2 GiB of address space by default).
pub const MAX_PRIMES_N: u32 = 1 << 30;

/// The largest `n` the contest workloads accept (a few hundred MB at most).
pub const MAX_WORKLOAD_N: u32 = 20_000_000;

/// Set while a runtime is running anywhere in this process.
static RUNNING: AtomicBool = AtomicBool::new(false);

/// `argv` for `goose_init`. Goose keeps this pointer while it runs (its
/// `os::args()` reads it), so it must live for the whole program.
struct Argv([*const c_char; 2]);

// SAFETY: the pointers refer to a `'static` C string literal and a null, and
// nothing ever writes through them, so sharing them between threads is sound.
unsafe impl Sync for Argv {}

static ARGV: Argv = Argv([c"rgt-app".as_ptr(), std::ptr::null()]);

/// Why [`Goose::start`] failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StartError {
    /// A Goose runtime is already running in this process, on this thread
    /// or another. Drop that [`Goose`] first.
    AlreadyRunning,
}

impl fmt::Display for StartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyRunning => {
                f.write_str("a Goose runtime is already running in this process")
            }
        }
    }
}

impl std::error::Error for StartError {}

/// A running Goose runtime. All Goose code is called through its methods.
///
/// Only one can exist per process at a time. It stays on the thread that
/// created it, so this does not compile:
///
/// ```compile_fail,E0277
/// let goose = rgt_app_core::Goose::start().unwrap();
/// std::thread::spawn(move || drop(goose));
/// ```
///
/// Dropping it shuts the runtime down; [`Goose::start`] then works again.
pub struct Goose {
    /// A raw pointer is neither `Send` nor `Sync`, and neither is this type.
    _same_thread_only: PhantomData<*mut ()>,
}

impl fmt::Debug for Goose {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Goose { running }")
    }
}

impl Drop for Goose {
    fn drop(&mut self) {
        // SAFETY: `Goose` cannot leave the thread that called `goose_init`, it
        // is dropped once per start, and the kernels start no Goose workers
        // (goose_exit must not run while one is alive).
        unsafe { ffi::goose_exit() };
        RUNNING.store(false, Ordering::Release);
    }
}

/// A Rust slice as the `{ data, len }` pair Goose expects. Goose does not
/// write through a `const` slice, so handing it a `*mut` view is sound.
fn slice_f64(xs: &[f64]) -> ffi::sl_f64 {
    ffi::sl_f64 {
        data: xs.as_ptr().cast_mut(),
        len: len_i64(xs.len()),
    }
}

fn slice_f64_mut(xs: &mut [f64]) -> ffi::sl_f64 {
    ffi::sl_f64 {
        data: xs.as_mut_ptr(),
        len: len_i64(xs.len()),
    }
}

fn slice_u8(xs: &[u8]) -> ffi::sl_u8 {
    ffi::sl_u8 {
        data: xs.as_ptr().cast_mut(),
        len: len_i64(xs.len()),
    }
}

fn len_i64(len: usize) -> i64 {
    // A Rust slice never exceeds isize::MAX bytes, so this cannot fail.
    i64::try_from(len).expect("slice length fits in i64")
}

impl Goose {
    /// Starts the Goose runtime on this thread.
    ///
    /// # Errors
    ///
    /// [`StartError::AlreadyRunning`] if a runtime is already running in this
    /// process.
    pub fn start() -> Result<Self, StartError> {
        if RUNNING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(StartError::AlreadyRunning);
        }
        // SAFETY: no runtime is running (the flag above), `ARGV` is a
        // null-terminated array that lives for the whole program, and Goose
        // only reads it.
        unsafe { ffi::goose_init(1, ARGV.0.as_ptr().cast_mut().cast()) };
        Ok(Self {
            _same_thread_only: PhantomData,
        })
    }

    /// `y[i] += a * x[i]` for every `i`.
    ///
    /// # Panics
    ///
    /// If `y` and `x` have different lengths.
    pub fn saxpy(&self, y: &mut [f64], x: &[f64], a: f64) {
        assert!(
            y.len() == x.len(),
            "saxpy: y has {} elements but x has {}",
            y.len(),
            x.len()
        );
        // SAFETY: the runtime runs on this thread (`self`); both slices are
        // valid for the whole call and cannot overlap (`y` is `&mut`); Goose
        // keeps no pointer to them after returning.
        unsafe { ffi::rgt_app_saxpy(slice_f64_mut(y), slice_f64(x), a) }
    }

    /// How many primes are at most `n`.
    ///
    /// # Panics
    ///
    /// If `n` is larger than [`MAX_PRIMES_N`].
    pub fn count_primes(&self, n: u32) -> u64 {
        assert!(
            n <= MAX_PRIMES_N,
            "count_primes: n must be at most {MAX_PRIMES_N}, got {n}"
        );
        // SAFETY: the runtime runs on this thread, and `n` is small enough for
        // the sieve to fit in a Goose data stack.
        let count = unsafe { ffi::rgt_app_count_primes(i64::from(n)) };
        u64::try_from(count).expect("a count is never negative")
    }

    /// 64-bit FNV-1a hash of `bytes`.
    pub fn fnv1a(&self, bytes: &[u8]) -> u64 {
        // SAFETY: the runtime runs on this thread; the slice is valid for the
        // whole call and Goose only reads it.
        unsafe { ffi::rgt_app_fnv1a(slice_u8(bytes)) }
    }

    /// Count, min, max and mean of `xs`, or `None` when it is empty.
    ///
    /// # Panics
    ///
    /// If `xs` has more than `u32::MAX` elements (the count is a `u32`).
    pub fn summary(&self, xs: &[f64]) -> Option<Summary> {
        if xs.is_empty() {
            return None;
        }
        assert!(
            u32::try_from(xs.len()).is_ok(),
            "summary: more than u32::MAX values"
        );
        // SAFETY: the runtime runs on this thread; the slice is valid, not
        // empty (Goose asserts that) and only read.
        Some(unsafe { ffi::rgt_app_summary(slice_f64(xs)) })
    }

    /// The Goose library's version string.
    pub fn version(&self) -> String {
        let mut len: i64 = 0;
        // SAFETY: the runtime runs on this thread; Goose writes the length
        // through the pointer, which is valid for the call.
        let ptr = unsafe { ffi::rgt_app_version(&raw mut len) };
        let len = usize::try_from(len).expect("Goose returned a negative length");
        if ptr.is_null() || len == 0 {
            return String::new();
        }
        // SAFETY: Goose returned `len` readable bytes at `ptr`, borrowed until
        // the next Goose call; they are copied before anything else runs.
        let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
        String::from_utf8_lossy(bytes).into_owned()
    }

    /// Builds `n` flat records from `seed` and walks them for a checksum.
    ///
    /// # Panics
    ///
    /// If `n` is larger than [`MAX_WORKLOAD_N`].
    pub fn records_checksum(&self, n: u32, seed: u64) -> u64 {
        assert!(
            n <= MAX_WORKLOAD_N,
            "records_checksum: n must be at most {MAX_WORKLOAD_N}, got {n}"
        );
        // SAFETY: the runtime runs on this thread; scalars only.
        unsafe { ffi::rgt_app_records_checksum(i64::from(n), seed) }
    }

    /// Builds a binary search tree of `n` keys from `seed`, looks every key
    /// up, and returns a checksum of the tree.
    ///
    /// # Panics
    ///
    /// If `n` is larger than [`MAX_WORKLOAD_N`].
    pub fn tree_checksum(&self, n: u32, seed: u64) -> u64 {
        assert!(
            n <= MAX_WORKLOAD_N,
            "tree_checksum: n must be at most {MAX_WORKLOAD_N}, got {n}"
        );
        // SAFETY: the runtime runs on this thread; scalars only.
        unsafe { ffi::rgt_app_tree_checksum(i64::from(n), seed) }
    }
}
