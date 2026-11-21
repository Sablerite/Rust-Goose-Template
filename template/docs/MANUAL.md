# Rgt App manual

Rgt App is a Rust program that hands its speed- and memory-critical work to
Goose. Rust is the main program; Goose code is compiled to C on every
build, linked in, and called through a safe Rust API. This manual covers
the daily commands, how the two languages meet, the rules that keep that
meeting safe and fast, and a reference for both toolchains.

Contents:

1. [Daily commands](#1-daily-commands)
2. [Project layout](#2-project-layout)
3. [How Rust and Goose meet](#3-how-rust-and-goose-meet)
4. [Adding a Goose function, step by step](#4-adding-a-goose-function-step-by-step)
5. [Type mapping: Goose, C and Rust](#5-type-mapping-goose-c-and-rust)
6. [Safety rules](#6-safety-rules)
7. [Speed and memory guide](#7-speed-and-memory-guide)
8. [What the tests prove](#8-what-the-tests-prove)
9. [Rust and Cargo reference](#9-rust-and-cargo-reference)
10. [Goose reference](#10-goose-reference)
11. [Troubleshooting](#11-troubleshooting)
12. [Settings](#12-settings)

---

## 1. Daily commands

Every command goes through `just`. Run `just` on its own to list them.

| Command | What it does |
|---|---|
| `just` | List every command |
| `just doctor` | Check every tool is installed and matches the pinned versions |
| `just build` | Debug build: Goose's runtime checks on, nothing optimized |
| `just release` | Optimized build: Goose and Rust at full speed |
| `just run` / `just run-release` | Run the program (arguments go after: `just run --help`) |
| `just check` | Typecheck the Goose code, then the Rust code (fast, builds nothing) |
| `just fmt` | Format the Goose and Rust code |
| `just lint` | Format check for both languages, then clippy with warnings as errors |
| `just test` | Goose tests, then Rust tests, then documentation tests |
| `just test-goose` / `just test-rust` | One side only |
| `just test-release` | The Rust tests again, against the optimized code that ships |
| `just bench` | Goose vs Rust micro-benchmarks |
| `just contest` | Memory and speed contest: Goose alone, Goose inside Rust, and Rust |
| `just bce` | Per-line report of the bounds checks Goose could not remove |
| `just stacks` | How many Goose data stacks the code uses |
| `just goose-c` | Write the generated C and header to `target/goose-c/` to read |
| `just fetch` | Fetch the Goose packages `lib.goose` declares |
| `just audit` | Check dependencies for licence and security problems |
| `just docs` | Build and open the Rust API documentation |
| `just ci` | The full gate: lint, tests in debug and release, audit |
| `just clean` | Remove build output |

A good loop while working: `just check` after small edits, `just test`
before moving on, `just ci` before committing.

### Starting another project

From any PowerShell window:

```powershell
rgnew my-app                  # creates .\my-app
rgnew my-app -Path D:\code    # creates D:\code\my-app
rgnew my-app -NoGit           # no git init, no first commit
```

`rgnew` comes from the template's `install.ps1`. Names are lowercase
letters, digits and single hyphens, starting with a letter.

---

## 2. Project layout

```
rgt-app/
  Cargo.toml              workspace: members, shared lints, release profile
  rust-toolchain.toml     the exact Rust version this project uses
  .goose-version          the Goose version this project was made with
  justfile                every command
  deny.toml               dependency licence and security rules
  .cargo/config.toml      optional build settings (section 12)
  assets/                 files the program loads while running: textures, audio, fonts, shaders, models
  config/                 default settings that ship with the program
  data/                   sample and test inputs, never output
  platform/               per-OS packaging: app icons, manifests, installer files
  scripts/doctor.ps1      the tool check behind `just doctor`
  docs/MANUAL.md          this file
  docs/CONVENTIONS.md     where files go, how to name them, and how the parts connect
  docs/design/            dated design notes
  crates/
    rgt-app/              the Rust program
      src/main.rs
    rgt-app-core/         the bridge: all Goose code lives here
      build.rs            Goose -> C + header -> Rust declarations -> compiled C
      goose/
        kernels/          the Goose code itself (a Goose package named `kernels`)
          kernels.goose   math kernels
          workloads.goose memory-heavy workloads for the contest
        bridge/lib.goose  the boundary: only `export fn`s
        tests/*.goose     Goose-side tests
        bench/contest.goose  the contest workloads, run by Goose alone
      src/lib.rs          the safe Rust API: the `Goose` handle and its methods
      src/ffi.rs          the generated declarations (raw, unsafe)
      src/reference.rs    plain-Rust twins of every Goose function
      src/memory.rs       process memory readings (Windows)
      src/testing.rs      `with_goose`, for tests
      tests/              bridge, property, safety and memory tests
      benches/kernels.rs  Goose vs Rust micro-benchmarks
      examples/contest.rs the memory and speed contest
```

The program crate (`rgt-app`) never touches Goose directly. It uses the
methods on `rgt_app_core::Goose`, the same way it would use any library.

`assets/`, `config/`, `data/` and `platform/` start with only a README
saying what goes there. How the program loads files from them at run time
is up to the project. [CONVENTIONS.md](CONVENTIONS.md) compares the ways
of doing it and covers file names and formats. Build output always stays
in `target/`.

---

## 3. How Rust and Goose meet

```
 goose/kernels/*.goose   goose/bridge/lib.goose
          \                     /
           \   (export fn ...) /
            v                 v
        goose --standalone --header          one C file + a header
                     |
          +----------+-----------+
          |                      |
     bindgen (libclang)     clang-cl -O2         the same flags goose build uses
          |                      |
   Rust declarations       static library
     (src/ffi.rs)          (rgt_app_goose)
          \                      /
           +--------------------+
                     |
         safe API (src/lib.rs): Goose::start(), goose.saxpy(...), ...
                     |
              crates/rgt-app
```

On every `cargo build`, the bridge's build script:

1. Runs `goose` on `goose/bridge/lib.goose`. Goose compiles the whole
   program, with every module it imports and its runtime, into one C file,
   plus a header that declares the exported functions.
2. Runs bindgen on that header. bindgen uses a real C front end, so the
   Rust declarations always match the C exactly, packed structs included,
   and the build checks every struct's size and field offsets at compile
   time.
3. Removes Goose's C name decorations, so Rust sees `Summary { count, .. }`
   instead of `kernels_Summary_g7 { count_g, .. }`.
4. Compiles the C with clang-cl at `-O2` (in release), exactly the
   optimization `goose build` uses, and links it.

The script reruns whenever a file under `goose/` changes, a setting
changes, or the `goose` executable itself changes. So Rust can never call
an out-of-date Goose function: if a Goose signature changes, the Rust code
that uses it stops compiling.

### Build modes

| Cargo profile | Goose | C |
|---|---|---|
| debug (`just build`, `just test`) | `--debug`: overflow, range and tag checks on | unoptimized, with debug info |
| release (`just release`, `just bench`, `just contest`) | `-O2` | `-O2`, nothing else |

This mirrors Rust: debug builds catch mistakes, release builds are fast.
`GOOSE_PROFILE` overrides it (section 12).

---

## 4. Adding a Goose function, step by step

The example adds a dot product. Replace `dot` with your own function.

**1. Write it in Goose**, in `crates/rgt-app-core/goose/kernels/kernels.goose`
(or a new file there that starts with `namespace kernels;`):

```goose
/// The dot product of two series of the same length.
fn dot(x: const f64[:], y: const f64[:]) -> f64 {
    assert(x.len == y.len);
    var total = 0.0;
    for i in x.len { total += x[i] * y[i]; }
    return total
}
```

The `assert` also lets Goose remove every bounds check in the loop (see
`just bce`).

**2. Export it** in `crates/rgt-app-core/goose/bridge/lib.goose`. The C name
starts with `rgt_app_`, which keeps it from clashing with other C code:

```goose
export "rgt_app_dot" fn dot(x: const f64[:], y: const f64[:]) -> f64 {
    return kernels::dot(x, y)
}
```

**3. Build.** `just build` generates `ffi::rgt_app_dot` on its own. Nothing
is written by hand on the Rust side of the boundary.

**4. Add the safe method** to `impl Goose` in `crates/rgt-app-core/src/lib.rs`.
Check in Rust everything that would make Goose abort, then call it:

```rust
/// The dot product of `x` and `y`.
///
/// # Panics
///
/// If `x` and `y` have different lengths.
pub fn dot(&self, x: &[f64], y: &[f64]) -> f64 {
    assert!(x.len() == y.len(), "dot: x has {} elements but y has {}", x.len(), y.len());
    // SAFETY: the runtime runs on this thread; both slices are valid for
    // the whole call and only read.
    unsafe { ffi::rgt_app_dot(slice_f64(x), slice_f64(y)) }
}
```

**5. Write the Rust twin** in `src/reference.rs`, doing the same steps in
the same order (so floats match bit for bit):

```rust
pub fn dot(x: &[f64], y: &[f64]) -> f64 {
    assert!(x.len() == y.len(), "dot: x has {} elements but y has {}", x.len(), y.len());
    let mut total = 0.0;
    for (a, b) in x.iter().zip(y) {
        total += a * b;
    }
    total
}
```

**6. Test both sides.** A Goose test in `goose/tests/` (any new `.goose`
file there is picked up by `just test-goose`):

```goose
import kernels from "../kernels";

fn main() {
    var a: f64[>..] = [1.0, 2.0, 3.0];
    var b: f64[>..] = [4.0, 5.0, 6.0];
    assert(kernels::dot(a, b) == 32.0);
    print("dot: ok");
}
```

And a Rust test in `tests/bridge.rs` (plus a property test in
`tests/properties.rs` for random inputs):

```rust
#[test]
fn dot_matches_reference() {
    with_goose(|goose| {
        let (x, y) = (ramp(1000), ramp(1000));
        assert_eq!(goose.dot(&x, &y).to_bits(), reference::dot(&x, &y).to_bits());
    });
}
```

**7. Measure it.** Add a pair to `benches/kernels.rs` and run `just bench`.
If Goose is not faster, keep the Rust version: Goose is worth its
boundary only where it wins.

**8. Gate it.** `just ci`.

### Calling Goose from many Rust threads

`Goose` stays on one thread. When several threads need it, give it a
thread of its own and send it work over a channel:

```rust
use std::sync::mpsc;
use std::thread;

use rgt_app_core::Goose;

enum Job {
    Hash(Vec<u8>, mpsc::Sender<u64>),
}

fn spawn_goose_thread() -> mpsc::Sender<Job> {
    let (jobs, inbox) = mpsc::channel::<Job>();
    thread::spawn(move || {
        let goose = Goose::start().expect("the only Goose runtime");
        for job in inbox {
            match job {
                Job::Hash(bytes, reply) => {
                    let _ = reply.send(goose.fnv1a(&bytes));
                }
            }
        }
    });
    jobs
}
```

For parallel work inside Goose itself, Goose's `thread_fn` workers run in
parallel with no shared memory (Goose tutorial, "Threads that share
nothing"). Every worker must have finished before the exported function
returns.

### Calling Rust from Goose

This direction is not set up or tested here. The ingredients: a Rust
function with a C signature (`#[unsafe(no_mangle)] pub extern "C" fn ...`),
declared on the Goose side with `extern fn`, using the same types as in
section 5. Goose's packages documentation ("A program with C of its own")
explains how a Goose program links outside code.

---

## 5. Type mapping: Goose, C and Rust

Only values with a plain C shape cross the boundary.

| Goose | C (in the header) | Rust (generated) | Notes |
|---|---|---|---|
| `i8` ... `i64`, `u8` ... `u64` | `int8_t` ... `uint64_t` | `i8` ... `u64` | |
| `f32`, `f64` | `float`, `double` | `f32`, `f64` | |
| `bool` | `uint8_t` | `u8` | 0 or 1; turn it into `bool` in the safe method |
| flat struct `S` | packed `struct` | `#[repr(C, packed)] struct S` | read fields by value: `let n = s.count;` |
| `T&` (flat `T`) | `T *` | `*mut T` | |
| `T[:]`, `const T[:]` | `sl_T { T *data; int64_t len; }` | `sl_T { data, len }` | build it from `&[T]` / `&mut [T]` |
| fixed array or inline string in a struct (`u8[..8]`) | `l8_u8 { uint8_t len; uint8_t e[8]; }` | the same struct | |
| fixed-mode `enum` | a tag byte plus a union | the same | |
| returned `const u8[:]` | `const uint8_t *` + `int64_t *` out length | `*const u8` + `*mut i64` | borrowed: copy it before the next Goose call |

Not allowed in an export: `varint` fields, references inside structs,
variable-size or growable values passed by value, optionals, generic
functions, default arguments, `return ... from`, and builders
(`u8[>..]&`). Goose rejects these at the `export fn` line, so they never
reach Rust.

**Struct packing.** Goose packs structs to one byte. A `u32` followed by an
`f64` puts the `f64` at offset 4, where a normal C struct would put it at 8.
The generated Rust structs are packed the same way, and the build checks
every offset. The one consequence on the Rust side: a reference to a field
of a packed struct is not allowed (error E0793), so copy the field out
first.

---

## 6. Safety rules

Each Goose rule below is enforced for you by the safe API. The list
explains what is underneath, so new methods keep the same guarantees.

1. **One thread.** Exported functions must run on the thread that started
   the runtime. `Goose` is neither `Send` nor `Sync`, so moving it to
   another thread is a compile error. To use Goose from many threads, see
   "Calling Goose from many Rust threads" above.
2. **One runtime per process.** `Goose::start()` returns
   `Err(StartError::AlreadyRunning)` while one exists anywhere in the
   process. Dropping `Goose` shuts the runtime down, after which it can
   start again.
3. **A failed Goose check ends the whole process.** An `assert`, a bounds
   check, an overflow in a debug build, or running out of a data stack
   prints `goose runtime error: ...` (with the Goose file and line) and
   exits immediately with code 1. Rust does not unwind, destructors do not
   run, and buffered Rust output may be lost. So every safe method checks
   in Rust, first, everything that would make Goose abort, and panics
   there with a clear message. This is the most important rule when
   adding a method.
4. **Borrowed results.** A returned `const u8[:]` points into Goose memory
   and is valid only until the next Goose call. Copy it right away, as
   `version()` does.
5. **Overflow differs between the languages.** Goose's unsigned math always
   wraps; Rust's panics on overflow in debug builds. Use `wrapping_*`
   when Rust code must match Goose. Signed overflow aborts in Goose debug
   builds and wraps in release, like Rust's.
6. **Data stack size.** Goose keeps growing data in data stacks with 2 GiB of
   address space each by default. Cap input sizes in the safe method (as
   `MAX_PRIMES_N` does), or raise `GOOSE_STACK_RESERVE` (section 12).
7. **Goose workers end before the export returns.** Shutting down while a
   `thread_fn` worker still runs is a runtime error.
8. **Every `unsafe` block says why it is sound** in a `// SAFETY:` comment,
   one unsafe operation per block. Clippy enforces both.

Goose's own guarantees still hold inside Goose: no use-after-free, no
dangling references, checked bounds, and no data races, all without
annotations.

---

## 7. Speed and memory guide

- **Measure in release.** Debug builds turn Goose's checks on and its
  optimizer off. `just bench`, `just contest` and `just run-release` use
  release builds.
- **Call once per batch, not once per item.** A call into Goose costs
  about as much as an ordinary function call. It cannot be inlined into
  Rust, though, so pass whole slices and let Goose loop. Calling Goose
  once per element of a million-element array throws its advantage away.
- **The compiler matters.** Goose code is compiled with clang-cl, the
  compiler that gives Goose its best numbers. Without LLVM installed, the
  build falls back to MSVC's `cl`, which was 2-12% slower in testing, and
  says so with a warning.
- **Remove bounds checks.** `just bce` shows, line by line, which bounds
  checks Goose could not prove unnecessary. An `assert` on lengths before a
  loop (as in `saxpy` and `dot`) usually removes them all, and then the
  loop vectorizes.
- **`simd fn`** builds a function for several CPU instruction sets and
  picks the widest one at run time. Use it for byte and float kernels.
  It works because the C compiler is clang.
- **Data layout is where Goose wins memory.** Nested data is stored
  inline, `varint` fields take only the bytes their values need, and links
  can be 4-byte relative offsets instead of 8-byte pointers. Freeing a
  scope's data is a single pointer store.
- **Prove it with `just contest`.** It runs the same workloads in Goose
  alone, Goose inside Rust, idiomatic Rust and tuned Rust, each in a fresh
  process, and reports extra peak memory and time. When this template was
  made, a typical run looked like this (numbers vary by machine):

  | Workload | Goose alone | Goose in Rust | Rust | Tuned Rust |
  |---|---|---|---|---|
  | records (1M) | 24 MB, 62 ms | 25 MB, 59 ms | 110 MB, 160 ms | - |
  | tree (1M keys) | 13 MB, 428 ms | 14 MB, 420 ms | 29 MB, 818 ms | 25 MB, 421 ms |
  | primes (control) | 47 MB, 260 ms | 48 MB, 252 ms | 48 MB, 247 ms | - |

  Goose inside Rust matches Goose alone, so the bridge costs nothing. The
  control (a flat byte array, where neither language has an edge) ties,
  as it should.

---

## 8. What the tests prove

| File | Proves |
|---|---|
| `goose/tests/*.goose` | Each Goose function is right on its own, with Goose's runtime checks on |
| `tests/bridge.rs` | Each Goose function gives exactly the same answer as its Rust twin |
| `tests/properties.rs` | The same, on thousands of random inputs: empty, NaN, infinities, subnormals, extremes. A failure prints the smallest failing input |
| `tests/safety.rs` | Each safety rule in section 6, including that a broken Goose check really ends the process (tested in a child process) |
| `tests/memory.rs` | Repeated calls do not grow memory, and shutting down returns Goose's memory |
| `src/lib.rs` (doc tests) | The API example works, and moving `Goose` to another thread does not compile |
| the build itself | Every generated struct's size and field offsets match the C header |

Tests that use Goose go through `testing::with_goose(|goose| ...)`, which
gives each test a fresh runtime and lets tests run under both
`cargo nextest` (one process per test) and plain `cargo test` (threads).

Floats are compared bit for bit, which is valid because neither compiler
reorders float math. The property tests count any NaN as equal to any NaN,
because IEEE leaves the NaN bit pattern of an operation open.

---

## 9. Rust and Cargo reference

### Building and running

| Command | What it does |
|---|---|
| `cargo build` / `cargo build --release` | Build the workspace (debug / optimized) |
| `cargo run` / `cargo run --release` | Build and run the program; arguments after `--` |
| `cargo check --workspace --all-targets` | Typecheck everything without building |
| `cargo clean` | Delete `target/` |
| `cargo build -vv` | Show every command, including the build script's |
| `cargo tree` | The dependency tree; `cargo tree -i <crate>` shows who pulls a crate in |

### Testing

| Command | What it does |
|---|---|
| `cargo nextest run --workspace` | All tests, one process per test |
| `cargo nextest run <filter>` | Tests whose names contain `<filter>`, e.g. `cargo nextest run start_twice` |
| `cargo nextest run --test safety` | Every test in one file (`tests/safety.rs`) |
| `cargo nextest run --no-capture` | Show test output as it happens |
| `cargo nextest list` | List the tests without running them |
| `cargo test --workspace` | All tests with the standard runner (threads) |
| `cargo test --doc` | Documentation examples only |
| `$env:PROPTEST_CASES = 20000; cargo nextest run --test properties` | Many more random cases, once in a while |

### Quality

| Command | What it does |
|---|---|
| `cargo fmt --all` / `cargo fmt --all --check` | Format / check formatting |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lints, warnings as errors |
| `cargo deny check` | Licences, security advisories, banned or duplicate crates |
| `cargo doc --workspace --no-deps --open` | API documentation |

### Dependencies

| Command | What it does |
|---|---|
| `cargo add <crate> -p rgt-app` | Add a dependency to the program |
| `cargo remove <crate> -p rgt-app` | Remove one |
| `cargo update` | Update `Cargo.lock` within the allowed versions |
| `cargo update -p <crate>` | Update one crate |
| `cargo info <crate>` | Versions and features of a crate |

Shared dependency versions live in the root `Cargo.toml` under
`[workspace.dependencies]`; a crate uses one with `name.workspace = true`.

### Toolchain

| Command | What it does |
|---|---|
| `rustup show` | Which toolchain this folder uses (from `rust-toolchain.toml`) |
| `rustup update` | Update installed toolchains |
| `rustup toolchain install 1.xx.y` | Install a specific version |

To move the project to a newer Rust, change `channel` in
`rust-toolchain.toml` (and `rust-version` in `Cargo.toml`), then run
`just ci`.

---

## 10. Goose reference

`goose` is the compiler, runner, build tool, package fetcher, formatter and
language server in one. `goose help` lists everything, and
`goose help <command>` shows one command. Full documentation is in the
`docs` folder of the Goose installation.

### Commands

| Command | What it does |
|---|---|
| `goose file.goose` or `goose run file.goose` | Compile and run in-process (no C compiler needed) |
| `goose run file.goose -- a b` | Arguments after `--` go to the program |
| `goose build file.goose` | Build an executable beside the file |
| `goose build -o out/name file.goose` | Build it somewhere else |
| `goose check file.goose` | Parse and typecheck only (prints nothing when correct) |
| `goose format dir` / `goose format --check dir` | Format files / list unformatted ones |
| `goose fetch file.goose` | Fetch the packages the file declares |
| `goose fetch --update file.goose` | Fetch packages pinned to a tag or branch again |
| `goose toolchain` | The C compiler, cmake and git Goose found, and `GOOSE_HOME` |
| `goose version` | The compiler and runtime versions |
| `goose lsp` | The language server (editors start it themselves) |
| `goose completion powershell` | Tab-completion script (the template installer sets this up) |

### Options for `run`, `build` and `check`

| Option | What it does |
|---|---|
| `--debug` | Overflow, range and tag checks on; a build also gets debug info |
| `-O0`, `-O1`, `-O2` | How much Goose inlines (default `-O2`) |
| `-D NAME=VALUE` | Define a C macro in the generated program |
| `--stack-reserve 8G` | Address space per data stack (default 2G) |
| `--stacks` | Report data stack use |
| `--bce-lines` | Report, per line, the bounds checks removed and kept |
| `--no-bce` | Keep every bounds check |
| `--stats` | A few lines about what the compile did |
| `--specs` | Every specialization of each function, as source |
| `--diagnostics json` | Errors and warnings as JSON |
| `--offline` | Fetch nothing |
| `--cc clang` | Pick the C compiler for this command |
| `-v` | Show each C command and its output |
| `--dry-run` (build) | Print the build commands and run none |

### Writing C for a host (what the bridge does)

| Command | What it does |
|---|---|
| `goose -o out.c file.goose` | Write the program's C |
| `goose --standalone --header out.h -o out.c file.goose` | C with its runtime and no `main`, plus a header for a host program |
| `goose --emit-runtime runtime.c` | The runtime's C on its own |

`just goose-c` runs the second one into `target/goose-c/`, which is the
quickest way to see the names and layouts the bridge works with.

### Packages

Only the root file declares packages, with their source:

```goose
import mylib from "joe/mylib@v2";              // GitHub, at a tag
import mylib from "github.com/joe/mylib@3f2c1a9";  // a commit (reproducible)
import mylib from "../mylib";                  // a local folder, used in place
```

The bridge's root file is `goose/bridge/lib.goose`. After adding a package
there, run `just fetch` once: the build itself never touches the network.

### Language reminders

- `let` is immutable, `var` mutable; `x .= y` rebinds a reference, `x = y`
  copies.
- Growable arrays: `T[>..]` (grow-only) and `T[>..<]` (grow and shrink).
  Slices: `T[:]`. Fixed arrays: `T[N]`.
- Copies of non-fixed values are explicit: `copy(x)`, or pass a slice
  `x[..]`.
- `guard cond else { ... }` for early exits; `return err from caller` to
  return several frames up.
- `recursive fn` for recursion; it cannot keep growable data across the
  recursive call.
- Unsigned math wraps; signed overflow aborts in `--debug`.
- `max`, `min`, `sort` and friends need `import std;`; time and files need
  `import os;`.

---

## 11. Troubleshooting

| Message | Fix |
|---|---|
| `goose was not found` | Install Goose, or set `GOOSE` to `goose.exe`'s full path |
| `libclang was not found` | `winget install LLVM.LLVM`, or set `LIBCLANG_PATH` to the folder holding `libclang.dll` |
| warning: `clang-cl was not found, so the Goose code is compiled with MSVC's cl` | Works, but slower. Install LLVM for full speed |
| warning: `this project was made with goose vX but vY is installed` | Run `just ci`; if it passes, put the new version in `.goose-version` |
| `goose: file.goose:12: error: ...` at the top of a build failure | A Goose compile error; the full message with the code is under `--- stderr` |
| `a Goose runtime is already running` | Drop the other `Goose` first. In tests, use `with_goose` |
| `*mut ()` cannot be sent between threads safely | A `Goose` was moved to another thread. Keep it on one thread (section 4) |
| The program exits with `goose runtime error: ...` | A Goose check failed at the file and line shown. Add the matching check to the safe method so it becomes a Rust panic |
| `goose runtime error: ... data stack ...` | The data is larger than one data stack: raise `GOOSE_STACK_RESERVE`, or cap the input in Rust |
| error E0793, reference to a packed field | Copy the field first: `let v = s.field;` |
| `cargo deny` reports an advisory | Read it; usually `cargo update -p <crate>` fixes it. Accept it deliberately in `deny.toml` only with a comment saying why |
| `goose build` uses a different C compiler than before | `goose toolchain` shows which one and why; pin one with `cc = "...";` in `%USERPROFILE%\.goose\config` |
| The first build is slow | It compiles bindgen and the dependencies once; later builds reuse them |
| The Goose extension is missing in VS Code | `code --install-extension "<Goose folder>\vscode\goose-language.vsix"` |

When something is odd, `just doctor` is the first thing to run. To see
exactly what the build script ran, use `cargo build -vv`.

---

## 12. Settings

Each can be set as an environment variable or, for this project only, in
`.cargo/config.toml` under `[env]` (an environment variable wins).

| Setting | Default | What it does |
|---|---|---|
| `GOOSE` | `goose` on PATH | The goose executable |
| `GOOSE_C_COMPILER` | `clang-cl` | `clang-cl`, `cl` (MSVC), or a full path |
| `GOOSE_PROFILE` | follows Cargo | `debug` or `release`, regardless of Cargo's profile |
| `GOOSE_STACK_RESERVE` | `2G` | Address space per Goose data stack, `64K` to `256T` |
| `LIBCLANG_PATH` | `C:\Program Files\LLVM\bin` | Where `libclang.dll` is |
| `GOOSE_CONTEST_EXE` | set by `just contest` | Goose's own contest executable, to include "Goose alone" |
| `PROPTEST_CASES` | per test | More random cases for the property tests |

Goose's own settings, used by `goose` itself:

| Setting | What it does |
|---|---|
| `GOOSE_HOME` | Where Goose keeps fetched packages and build files (default `%USERPROFILE%\.goose`) |
| `GOOSE_CC` | The C compiler for `goose build` |
| `%USERPROFILE%\.goose\config` | `cc = "...";` pins `goose build`'s C compiler for every project |
| `NO_COLOR` | Turn off colored errors |
