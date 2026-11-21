# Rust + Goose template: implementation plan

**Goal:** a template folder plus a one-command generator that creates
Rust-host projects calling Goose for hot paths, with tests, a memory
contest, a safety suite and a manual.

**Architecture:** `template/` is a real Cargo workspace named `rgt-app`.
Its bridge crate's build script turns Goose source into C and a header,
bindgen turns the header into Rust declarations, and clang-cl compiles the
C exactly as `goose build` would. A safe Rust layer enforces Goose's
threading rule through types. PowerShell scripts at the template root copy
`template/` with the name replaced.

**Tech stack:** Rust 1.99.1 (edition 2024, MSVC target), Goose 2026.4,
LLVM 23.1.6 (libclang, clang-cl), bindgen, cc, windows-sys, proptest,
divan, just 1.59, cargo-nextest, cargo-deny, PowerShell 7.

**Spec:** `docs/design/2026-11-21-rust-goose-template-design.md`

## Global constraints

- Placeholder forms: `rgt-app` (kebab), `rgt_app` (snake), `Rgt App` (title).
  No other spelling of the project name anywhere in `template/`.
- Exported C symbols are `rgt_app_<name>`. The Goose package is `kernels`
  in every project.
- Goose C flags must equal `goose build`'s: release `-O2` with no debug info
  and no forced frame pointers; debug `-O0` with debug info plus Goose
  `--debug`.
- `unsafe` only in the bridge crate. Every `unsafe` block has a
  `// SAFETY:` comment (clippy `undocumented_unsafe_blocks`).
- Every path in scripts and recipes is quoted: the template lives under
  `Whimsical Oracles` (with a space).
- Recipes use `pwsh`. The scripts also run under Windows PowerShell 5.1
  (no `??` and no ternaries).
- Generated docs never mention tools used to build the template, internal
  labels or people.

## Review focus

1. **A path with a space.** `just selftest` creates its project under a
   temp folder whose name contains a space. The whole gate must pass.
2. **A hyphenated name with digits** (`selftest-app2`). Folder, crate and
   Goose symbol names must all be replaced. The selftest uses this name.
3. **Missing tools.** Without `goose`, or without libclang, the build must
   stop with one clear line naming the tool and the fix. The build script
   is tested by running it with `GOOSE` pointing at a missing file.
4. **Plain `cargo test`** (threads instead of nextest processes). It must
   pass thanks to `with_goose` serializing runtime use. Verified once
   during the full gate.
5. **Editing a Goose file triggers a rebuild.** Touch `kernels.goose`, then
   `cargo build -v` must re-run the build script. Verified once during the
   full gate.

---

### Task 1: Goose package, boundary file, Goose-side tests

**Files:** `template/crates/rgt-app-core/goose/kernels/kernels.goose`,
`goose/kernels/workloads.goose`, `goose/lib.goose`,
`goose/tests/kernels_test.goose`, `goose/tests/workloads_test.goose`,
`goose/bench/contest.goose`

**Interfaces (Goose, namespace `kernels`):**

- `struct Summary { count: u32, min: f64, max: f64, mean: f64 }`
- `fn saxpy(y: f64[:], x: const f64[:], a: f64)`, which asserts equal lengths
- `fn count_primes(n: i64) -> i64`, the primes `<= n` (0 for `n < 2`)
- `fn fnv1a(xs: const u8[:]) -> u64`: 64-bit FNV-1a, offset basis
  14695981039346656037, prime 1099511628211
- `fn summary(xs: const f64[:]) -> Summary`, which asserts `xs.len > 0`.
  `min`/`max` follow `<`/`>` comparisons in order, and `mean = sum / count`
  with the sum taken left to right.
- `fn splitmix64(state: u64&) -> u64`: the standard splitmix64 step
  (`state += 0x9E3779B97F4A7C15`, then mixes with `0xBF58476D1CE4E5B9`,
  `0x94D049BB133111EB` and shifts 30/27/31)
- `fn records_checksum(n: i64, seed: u64) -> u64`, per record `i`:
  `id = i`, `name_len = 4 + r % 16`, then `name[k] = 'a' + r % 26` for each
  letter, `score_count = r % 8`, `scores[k] = r % 1000`, where each `r` is
  the next splitmix64 value in that order. Checksum, wrapping:
  `h = h * 31 + id`, then `+ byte` for each name byte, then `+ score` for
  each score.
- `fn tree_checksum(n: i64, seed: u64) -> u64`: insert `n` keys
  `(r % (4n+1)) as u32` into a BST (duplicates bump a count). Then
  re-generate the same keys from the seed and look each up, adding the
  found node's count. Result = `distinct * 1_000_003 + sum_of_counts` plus
  the tree height, wrapping.
- `lib.goose` exports `rgt_app_saxpy`, `rgt_app_count_primes`,
  `rgt_app_fnv1a`, `rgt_app_summary`, `rgt_app_version` (returns
  `"rgt-app kernels 0.1.0"`), `rgt_app_records_checksum` and
  `rgt_app_tree_checksum`.
- `bench/contest.goose`: `contest <records|tree|primes> <n> <seed>`
  prints `<elapsed_ns> <checksum>`.

- [ ] Write the Goose tests: `fnv1a("") == 14695981039346656037`,
  `fnv1a("a") == 12638187200555641996`, `count_primes(100) == 25`,
  `count_primes(1) == 0`, `summary([1,2,3]).mean == 2.0`, saxpy on 3
  elements, records/tree with `n = 0` are 0, and both workloads are
  deterministic for the same seed.
- [ ] Run `goose run --debug` on each test file. They fail, because the
  kernels are missing.
- [ ] Implement the kernels, workloads, `lib.goose` and `contest.goose`.
- [ ] Run the tests (they pass), plus `goose check` on `lib.goose` and
  `contest.goose`, and `goose format --check goose/`.

### Task 2: Workspace and bridge build script

**Files:** `template/Cargo.toml`, `rust-toolchain.toml`, `.goose-version`,
`crates/rgt-app-core/Cargo.toml`, `crates/rgt-app-core/build.rs`,
`crates/rgt-app-core/src/ffi.rs`

**Interfaces:**

- Consumes: Task 1's `lib.goose` export names.
- Produces: `crate::ffi` holding the raw `unsafe extern` declarations:
  `rgt_app_saxpy(y: sl_f64, x: sl_f64, a: f64)` and the rest, plus
  `goose_init` and `goose_exit`. Also `pub struct Summary { count: u32,
  min: f64, max: f64, mean: f64 }` (`#[repr(C, packed)]`, `Copy`,
  `Debug`), and the slice structs `sl_f64 { data, len }` and
  `sl_u8 { data, len }`.
- Build settings read: `GOOSE` (path to goose), `GOOSE_C_COMPILER`
  (`clang-cl`, `cl` or a path), `GOOSE_PROFILE` (`debug`/`release`),
  `LIBCLANG_PATH`.

- [ ] Write an `ffi.rs` unit test, `summary_is_packed`:
  `size_of::<ffi::Summary>() == 28` and `align_of == 1`.
- [ ] `cargo test -p rgt-app-core summary_is_packed` fails (no build
  script yet).
- [ ] Implement `build.rs` with these steps:
  1. find goose and warn on a version mismatch;
  2. generate the C and header in `OUT_DIR`;
  3. emit rerun lines for every file under `goose/` and for each setting;
  4. run bindgen (allowlist the header's file, Rust 2024 edition, derive
     Debug/Copy/Default, `--target` from `TARGET`);
  5. clean up names with a token pass: `ns_Type_gN` becomes `Type` when
     `N == len(ns)`, `Type_g` becomes `Type`, and a field `name_g` becomes
     `name`, with keyword escaping;
  6. compile with clang-cl (or `cl` with a warning), matching the
     constraints.

  Failure messages: `goose was not found: install Goose or set GOOSE`, and
  `libclang was not found: install LLVM (winget install LLVM.LLVM) or set
  LIBCLANG_PATH`.
- [ ] The test passes. `cargo build -vv` shows `clang-cl` with `-O2`
  only in release.

### Task 3: Safe API, bridge tests, safety suite

**Files:** `crates/rgt-app-core/src/lib.rs`, `src/reference.rs`,
`src/testing.rs`, `src/memory.rs`, `tests/bridge.rs`, `tests/safety.rs`

**Interfaces:**

- Produces: `pub struct Goose` (not `Send`/`Sync`) and
  `pub enum StartError { AlreadyRunning }` (implements `Error`). Methods:
  - `Goose::start() -> Result<Goose, StartError>`
  - `saxpy(&self, y: &mut [f64], x: &[f64], a: f64)` (panics
    `"saxpy: y has {} elements but x has {}"`)
  - `count_primes(&self, n: u32) -> u64`
  - `fnv1a(&self, bytes: &[u8]) -> u64`
  - `summary(&self, xs: &[f64]) -> Option<Summary>`
  - `version(&self) -> String`
  - `records_checksum(&self, n: u32, seed: u64) -> u64`
  - `tree_checksum(&self, n: u32, seed: u64) -> u64`
  - `pub use ffi::Summary`
- `reference::{saxpy, count_primes, fnv1a, summary, records_checksum,
  tree_checksum_boxed, tree_checksum_indexed}` with the same signatures,
  without `&self`.
- `testing::with_goose<R>(f: impl FnOnce(&Goose) -> R) -> R`
- `memory::{current_commit() -> Option<u64>, peak_commit() ->
  Option<u64>, peak_commit_of(child: &std::process::Child) ->
  Option<u64>}`. These return `None` off Windows.

- [ ] Write `tests/bridge.rs` (each method equals its reference on fixed
  inputs) and `tests/safety.rs` (every safety-suite row in design §9 except
  the property and memory tests). Also add the `compile_fail` doc test on
  `Goose`. The death test re-runs its own test binary with
  `GOOSE_DEATH_TEST=1`, calls `ffi::rgt_app_summary` on an empty slice, and
  expects a non-zero exit with `goose` in stderr.
- [ ] The tests fail to compile (API missing).
- [ ] Implement `lib.rs`, `reference.rs`, `testing.rs` and `memory.rs`.
  `start` uses an `AtomicBool` `compare_exchange`, and the `CString`
  program name is kept in a `OnceLock`. `with_goose` locks a static mutex
  and recovers it if poisoned.
- [ ] `cargo nextest run -p rgt-app-core` and `cargo test --doc` pass.

### Task 4: Property tests and memory-growth test

**Files:** `tests/properties.rs`, `tests/memory.rs`

- [ ] Property tests, 2000 cases each inside one `with_goose`:
  - saxpy: random lengths 0..512, any finite or non-finite f64, then
    compare bit for bit
  - fnv1a: random byte vectors up to 4096
  - summary: non-empty vectors including NaN and ±inf, with `count`, `min`
    and `max` compared via `to_bits`
  - count_primes: n up to 100 000
  - records and tree: n up to 2 000 with a random seed
- [ ] `repeated_calls_do_not_grow_memory` (Windows only): warm up each
  workload once (n = 200 000), read `current_commit`, run 50 more calls,
  and assert growth < 2% + 1 MiB.
- [ ] Both pass in debug and release.

### Task 5: Contest, benchmarks, binary crate

**Files:** `crates/rgt-app-core/examples/contest.rs`,
`crates/rgt-app-core/benches/kernels.rs`, `crates/rgt-app/Cargo.toml`,
`crates/rgt-app/src/main.rs`

- [ ] `contest` with no arguments runs the parent mode described in
  design §9. Its child mode is `--child <contender> <workload> <n> <seed>`
  and prints `<elapsed_ns> <checksum>`. The contenders are `goose-native`
  (from `GOOSE_CONTEST_EXE`, skipped with a note if unset or missing),
  `goose-in-rust`, `rust`, and `rust-tuned` (tree only). Sizes are records
  1 000 000, tree 1 000 000 and primes 50 000 000.
- [ ] Divan benches: Goose vs Rust for saxpy (1M), fnv1a (16 MB),
  count_primes (5M) and summary (1M), each Goose bench starting its own
  runtime with `bench_local`.
- [ ] `main.rs`: start Goose, run a small demo, print the version, summary
  and a reference check, then exit non-zero on a mismatch.
- [ ] `cargo run --release -p rgt-app-core --example contest` prints a full
  table with every checksum matching.

### Task 6: Project tooling and settings

**Files:** `template/justfile`, `scripts/doctor.ps1`, `deny.toml`,
`rustfmt.toml`, `.editorconfig`, `.gitattributes`, `.gitignore`,
`.vscode/extensions.json`, `.vscode/settings.json`,
`.cargo/config.toml`

- [ ] Add the justfile recipes exactly as in design §7, with
  `set shell := ["pwsh", "-NoLogo", "-NoProfile", "-Command"]`. `contest`
  first builds `bench/contest.goose` with `goose build -o
  target/goose-native/contest.exe` and passes it as `GOOSE_CONTEST_EXE`.
  `ci` = `lint`, `test`, `test-release`, `audit`.
- [ ] `doctor.ps1` checks every tool in design §7 and exits 1 if goose,
  cargo or libclang is missing.
- [ ] `deny.toml` starts from `cargo deny init`, with licences allowed as
  the dependency tree needs and private crates ignored.
- [ ] `just ci` passes in `template/`.

### Task 7: Generator, installer, template self-test

**Files:** `new-project.ps1`, `install.ps1`, root `justfile`

**Interfaces:**

- `new-project.ps1 -Name <kebab> [-Path <dir>] [-NoGit] [-SkipDoctor]`
- `install.ps1 [-Uninstall] [-NoCompletions]`, which writes a block between
  `# >>> rust-goose-template >>>` and `# <<< rust-goose-template <<<` in
  `$PROFILE`
- Root recipes: `just new <name>`, `just selftest`, `just install`

- [ ] Name validation as in design §7, with clear errors for `Core`,
  `my_app`, `1app`, `std`, `a--b` and `type`.
- [ ] Copy rules:
  - skip `target/`
  - replace tokens in text files and paths
  - write UTF-8 without BOM and keep LF line endings
  - rewrite `rust-toolchain.toml` and `.goose-version` from the installed
    versions
  - `git init -b main` and a first commit when an identity is configured
  - run `just doctor`
- [ ] `just selftest`: create `selftest-app2` under
  `%TEMP%\rgt selftest <random>`, run `just ci` there, and delete the
  folder on success.

### Task 8: Manual and READMEs

**Files:** `template/docs/MANUAL.md`, `template/README.md`, root
`README.md`

- [ ] Write the manual's sections exactly as listed in design §11, every
  command checked against the real justfile and the Goose 2026.4 docs.
- [ ] Search the docs for leftover internal words (design, plan, task,
  stage, phase, the author's name, and the names of tools used only to
  build the template) and rewrite any hits.

### Task 9: Full gate and machine setup

- [ ] Run `just selftest` (it passes), then plain `cargo test --workspace`
  and the rebuild-on-edit check (review focus 4 and 5) in a generated
  project.
- [ ] Run `install.ps1`, then in a new shell `rgnew demo-app -Path
  <scratch>` works.
- [ ] Run `git init` in the template folder.
