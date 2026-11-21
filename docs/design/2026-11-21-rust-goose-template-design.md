# Rust + Goose project template: design

Date: 2026-11-21

## 1. Why this exists

Future projects will use Rust as the main program and Goose for the parts
where speed and memory matter most. Setting that up by hand every time (a
build script, the C bridge, the Rust declarations, the threading rules,
tests and benchmarks in two languages) is slow and easy to get wrong. The
template turns it into one command, and a manual keeps the commands for
both languages in one place.

## 2. Goals

- One command, run from any PowerShell window, creates a new project folder
  with the project name replaced everywhere: folder names, crate names,
  code and docs.
- A fresh project builds, tests and benchmarks with no edits.
- Goose code is compiled from source on every build. Nothing generated is
  committed.
- The Rust declarations for Goose functions are generated from Goose's own
  header on every build, so they cannot drift out of step.
- Goose code runs at the same speed inside the Rust program as in a program
  built by `goose build`.
- Standard practice on both sides: pinned toolchains, formatter and linter
  settings, tests in both languages, dependency auditing, editor setup.
- A manual in every project: daily commands, the bridge rules, Rust/Cargo
  and Goose command references, and troubleshooting.
- Speed and safety are the top priorities. Every project ships a memory
  contest (the same workloads in Goose and Rust, peak memory and time side
  by side) and a safety suite that pins down the rules at the language
  boundary (section 9).

## 3. Non-goals (for now)

- Goose calling back into Rust. The manual explains how it would work (a
  Rust static library called through `extern fn`), but the template does
  not set it up.
- Continuous integration on GitHub. Running Goose on a CI machine needs a
  way to install it there, which is not settled.
- Testing on Linux and macOS. The build code avoids Windows-only
  assumptions where that costs nothing, but only Windows is tested.
- Inlining Goose functions into Rust (cross-language link-time
  optimization). The measured call cost is negligible for batch-style
  calls, so it is not worth tying the two toolchains' LLVM versions
  together.
- Publishing crates to crates.io.

## 4. Anti-goals (a failure even if everything technically works)

- Goose code running slower when called from Rust than on its own.
- Rust declarations getting out of step with Goose. That would mean silent
  memory corruption instead of a compile error.
- Breaking the existing `goose build` setup on this machine.

## 5. Facts the design rests on

From Goose 2026.4's documentation (command line §8, reference §24, spec
§7.10–7.11) and from throwaway tests on this machine:

- `goose --standalone --header lib.h -o lib.c lib.goose` writes one C file
  (the program plus the runtime) and a header for a C host. The host calls
  `goose_init(argc, argv)` once, then the exported functions, then
  `goose_exit()`. `goose_init` runs Goose's `main` if there is one. After
  `goose_exit`, `goose_init` can start it again.
- Exported functions and Goose globals are not thread-safe. Every call must
  come from the thread that called `goose_init`.
- Only flat fixed-size shapes cross the boundary: integers, floats, `bool`
  (as one byte), flat structs and fixed arrays (by value or by reference),
  slices of those (`{ data, len }`), and a borrowed `const u8[:]` result
  (pointer plus length). Goose structs are packed to one byte
  (`#pragma pack(1)`), so a `u8` followed by an `i32` takes 5 bytes.
- Goose compiles a whole program at once, so one Rust program holds one
  Goose program. Its symbols (`goose_init`, the exported names) are global
  C symbols.
- A failed Goose check (bounds, `assert`, overflow in debug mode) aborts
  the whole process. It does not unwind.
- `goose build` compiles C with only `-O2` (clang) or `/O2` (MSVC).
  Goose's `simd fn` builds per-instruction-set versions only under clang.
- Rust on the MSVC target linked Goose's generated C through the `cc`
  crate with no extra libraries, and every shape above crossed correctly.
  Speed, best of five interleaved runs, identical results in every build:

  | Test | Goose alone (clang) | Rust → Goose (clang) | Rust → Goose (MSVC) | Pure Rust |
  |---|---|---|---|---|
  | saxpy (vector math) | 7.8 ms | 5.8 ms | 6.5 ms | 7.0 ms |
  | prime sieve | 58.2 ms | 59.5 ms | 61.5 ms | 65.9 ms |
  | byte hash | 57.7 ms | 57.8 ms | 59.0 ms | 57.6 ms |

  Calling Goose from Rust costs nothing measurable (gaps are within the
  ±5% run-to-run noise). Clang gives Goose its best numbers.

## 6. Approaches considered

### Keeping Rust in step with Goose

- **Chosen: bindgen.** The Rust ecosystem's standard tool. On every build it
  reads Goose's generated header with a real C compiler front end
  (libclang) and writes the Rust declarations, packed structs and unions
  included, exactly as the C compiler sees them. Cost: LLVM must be
  installed, and the first build compiles bindgen (cached afterwards).
- Rejected: a small header reader written into the template. It needs no
  install, but it is custom code chasing a header format that includes
  unions, fixed-size strings and pointer typedefs, and a future Goose
  release could break it.
- Rejected: hand-written declarations. Any drift becomes silent memory
  corruption.

### Which C compiler builds the Goose code

- **Chosen: clang-cl** (from the same LLVM install), with `-O2` and nothing
  else, matching `goose build`. clang-cl is clang's MSVC-compatible
  driver, so its output links with MSVC Rust without environment tricks,
  and `simd fn` works.
- Fallback: MSVC `cl` with `/O2` when clang-cl is not found, with a build
  warning. It was 2–12% slower in the tests above.
- Rejected: llvm-mingw's clang in MSVC mode. It works only after Visual
  Studio's environment variables are loaded, which is fragile.

### The new-project command

- **Chosen: a PowerShell script plus a profile function.** Nothing to
  install. The placeholder names are valid names, so the template folder
  is itself a real project that builds and tests.
- Rejected: cargo-generate. It is the usual Rust template tool, but it needs
  an install, and its placeholder syntax makes the template folder
  unbuildable, so the template could not test itself.

## 7. What gets used day to day

### Creating a project

```powershell
rgnew my-app                    # creates .\my-app
rgnew my-app -Path D:\code      # creates D:\code\my-app
rgnew my-app -NoGit             # skip git init and the first commit
```

`rgnew` is a short alias for `New-RustGooseProject`. A one-time
`install.ps1` adds it to the PowerShell profile, with the template folder's
absolute path baked in (re-run it if the folder moves), and offers tab
completion for `goose` and `just`. `install.ps1 -Uninstall` removes
everything it added. The command:

1. Checks the name: lowercase letters, digits and single hyphens, starting
   with a letter. It must not be a Rust, Goose or C keyword, and must not
   clash with a standard crate or Goose module (`core`, `std`, `alloc`,
   `test`, `math`, `os` and the rest of Goose's standard library).
2. Refuses if the folder already exists.
3. Copies `template/` and replaces the placeholder name in file contents
   and paths: `rgt-app` → `my-app`, `rgt_app` → `my_app`,
   `Rgt App` → `My App`.
4. Pins the toolchains it finds: the installed Rust version goes into
   `rust-toolchain.toml`, and the installed Goose version into
   `.goose-version`.
5. Runs `git init -b main`, then makes a first commit if git has a user
   name and email configured.
6. Runs `just doctor` and prints the next steps.

### A generated project

```
my-app/
  Cargo.toml              workspace: members, shared package info, lints, profiles
  rust-toolchain.toml     exact Rust version, plus rustfmt and clippy
  .goose-version          the Goose version the project was made with
  justfile                every command, for both languages
  deny.toml               dependency licence and security rules (cargo-deny)
  rustfmt.toml  .editorconfig  .gitattributes  .gitignore
  .vscode/                recommended extensions and settings
  README.md
  docs/MANUAL.md
  crates/
    my-app/               the Rust program (binary)
      src/main.rs
    my-app-core/          the bridge; all Goose code lives here
      build.rs            Goose -> C + header -> Rust bindings -> compiled C
      goose/
        bridge/lib.goose  the C boundary: only `export fn`s (its own folder: a Goose
                          program owns every .goose file under its folder)
        kernels/
          kernels.goose   the Goose code itself (a local package)
          workloads.goose memory-heavy workloads used by the contest
        tests/*.goose     Goose-side tests
        bench/contest.goose  the contest workloads, run by Goose alone
      src/lib.rs          the safe Rust API (`Goose` handle and methods)
      src/ffi.rs          the generated bindings, pulled in from the build output
      src/reference.rs    plain-Rust versions of the kernels and workloads
      src/memory.rs       process memory readings (Windows), for tests and the contest
      src/testing.rs      `with_goose` helper for tests
      tests/bridge.rs     Goose results checked against the Rust versions
      tests/properties.rs the same check on thousands of random inputs
      tests/safety.rs     the boundary rules (section 9)
      tests/memory.rs     no memory growth across repeated calls
      benches/kernels.rs  Goose vs Rust micro-benchmarks (divan)
      examples/contest.rs the memory and speed contest
```

### Daily commands

| Command | What it does |
|---|---|
| `just` | List every command |
| `just doctor` | Check that cargo, rustc, goose, clang-cl, libclang, nextest and cargo-deny are present and match the pinned versions; print how to fix anything missing |
| `just build` / `just release` | Debug or optimized build |
| `just run *args` / `just run-release *args` | Run the program |
| `just check` | `goose check` on the Goose code, then `cargo check` |
| `just fmt` | Format Goose and Rust code |
| `just lint` | Format check for both languages, then `cargo clippy` with warnings as errors |
| `just test` | Goose tests, then Rust tests (nextest), then doc tests |
| `just test-release` | The Rust tests again against optimized Goose and Rust code |
| `just test-goose` / `just test-rust` | One side only |
| `just bench` | Goose vs Rust micro-benchmarks |
| `just contest` | Memory and speed contest: Goose alone, Goose inside Rust, and Rust, on the same workloads |
| `just bce` | Per-line report of the bounds checks Goose kept (the first thing to look at when a kernel is slow) |
| `just stacks` | Report Goose's data-stack use |
| `just goose-c` | Write the generated C and header to `target/goose-c/` for inspection |
| `just fetch` | Fetch the Goose packages `lib.goose` declares |
| `just audit` | `cargo deny check` |
| `just ci` | lint + test + audit, the full gate |
| `just clean` | Remove build output |

Recipes run under PowerShell 7 (`pwsh`) on Windows.

## 8. How the bridge works

### Build script steps (`my-app-core/build.rs`)

1. Find `goose` (the `GOOSE` environment variable, then `PATH`) and stop
   with a clear message if it is missing. Compare `goose version` with
   `.goose-version` and print a build warning if they differ.
2. Run `goose --standalone --header OUT/goose_lib.h -o OUT/goose_lib.c
   goose/bridge/lib.goose --offline --color never`, plus `--debug` in debug mode
   (below). Network fetches stay out of the build; `just fetch` does them.
   On a Goose error, show Goose's message and fail the build.
3. Ask cargo to rerun the script when any file under `goose/` changes, or
   any of the settings below.
4. Run bindgen on the header: only the exported functions, `goose_init`,
   `goose_exit` and the types they use; derive `Debug`, `Copy`, `Clone`
   and `Default` where possible; keep bindgen's layout tests (size,
   alignment and offset assertions); emit Rust 2024 `unsafe extern`
   blocks.
5. Compile the C with the `cc` crate: clang-cl (the `GOOSE_C_COMPILER`
   setting, then `PATH`, then `C:\Program Files\LLVM\bin`), falling back to
   `cl` with a warning. Override the `cc` crate's own defaults so the flags
   match `goose build`: optimization level 2, no debug info, no forced frame
   pointers. On Linux, also link `m` and `pthread`.

### Build modes

| Cargo profile | Goose | C |
|---|---|---|
| dev (`cargo build`, `cargo test`) | `--debug`: overflow, range and tag checks on | no optimization, debug info (what `goose build --debug` does) |
| release, bench | `-O2` (Goose's default) | `-O2`, nothing else |

`GOOSE_PROFILE=debug` or `release` overrides the choice, for example to
benchmark a debug Rust build against release Goose. `GOOSE_STACK_RESERVE`
(such as `8G`) passes `--stack-reserve` to Goose for data larger than the
2 GiB-per-data-stack default.

### Names Rust sees

Goose decorates its C names: `kernels::Stats` becomes `kernels_Stats_g7`,
and the field `count` becomes `count_g`. The build script cleans these up,
so Rust sees `Stats { count, .. }`. Rust keywords are escaped as raw
identifiers (`r#ref`); the few that cannot be raw (`self`, `super`,
`crate`) keep their suffix. If cleaning ever made two names collide, the
build fails with a duplicate-definition error, so a collision is never
silent. bindgen's compile-time layout assertions must still pass after
renaming.

The Goose package is named `kernels` in every project, so these C names do
not change with the project name. Exported C symbols are prefixed with the
project's snake-case name (`my_app_saxpy`) so they cannot clash with other C
libraries in the program. The Rust API exposes them as methods without the
prefix.

### The safe Rust API (`my-app-core`)

- `Goose::start() -> Result<Goose, StartError>` starts the runtime, passing
  the program's name as the only argument. Only one can exist per process
  at a time (an atomic flag). Dropping it calls `goose_exit`, after which
  `start` works again.
- `Goose` is neither `Send` nor `Sync`, so moving it to another thread is a
  compile error. That turns Goose's same-thread rule into a type rule.
- One method per export. Rust slices go in and Rust values come out.
  Preconditions (such as equal slice lengths) are checked in Rust first, so
  misuse is an ordinary Rust panic instead of an abort inside Goose.
  Borrowed Goose strings are copied into a `String` before returning.
- Every `unsafe` block carries a `// SAFETY:` comment, enforced by a clippy
  lint.
- `testing::with_goose(|g| ...)` runs a closure with a fresh runtime, one at
  a time (guarded by a mutex that recovers if a test panicked). Tests then
  work under both `cargo test` (threads) and `cargo nextest` (one process
  per test).

### Example code shipped

Small kernels, each exercising one boundary shape:

- `saxpy`: a slice read, plus a slice changed in place.
- `count_primes`: Goose growing its own arrays.
- `fnv1a`: bytes in, an integer out.
- `summary`: a struct returned by value. Its layout (`u32` then three
  `f64`s) is 28 bytes packed and 32 bytes in normal C, so it proves packing
  is handled.
- `version`: a string returned.
- `records_checksum` and `tree_checksum`: the contest workloads (section 9).

Each kernel has a plain-Rust twin, a Goose-side test, a Rust bridge test
comparing the two, and a benchmark entry. They are meant to be replaced.
The manual walks through adding a new Goose function end to end:

1. Write it in `kernels/`.
2. Export it in `lib.goose`.
3. Add the method in `lib.rs`.
4. Test it.
5. Benchmark it.

## 9. Memory contest and safety suite

### Memory contest (`just contest`)

Two workloads where Goose's memory model should matter, plus one control
where it should not:

- **records**: build a million records, each with an id, a 4–19 letter
  name and 0–7 small scores, then walk them for a checksum. Goose keeps
  each record as one inline run of bytes (`varint` fields, inline string,
  inline array). Rust uses the idiomatic
  `Vec<Record { id, name: String, scores: Vec<u16> }>`: three allocations
  per record.
- **tree**: insert a million pseudo-random keys into an unbalanced binary
  search tree, then look them all up. Goose uses one grow-only array of
  16-byte nodes with 4-byte relative links. Rust runs twice: the idiomatic
  `Option<Box<Node>>` version, and the tuned "best safe Rust" version
  (`Vec<Node>` with `u32` indices).
- **primes** (control): a 50-million-entry sieve. Both use one byte per
  entry, so memory should tie.

Both languages draw their pseudo-random data from the same small
generator (splitmix64), so every contender builds identical data and must
produce the same checksum. A mismatch is shown as a failure.

Contenders: Goose alone (`goose build`, Goose's default compiler), Goose
inside Rust (the bridge, release build), Rust (idiomatic), and Rust (tuned,
where one exists). Each run is a fresh child process. The parent reads the
child's peak committed memory from Windows after it exits, and subtracts
the same contender's peak for an empty run (n = 0), so start-up costs
cancel out. Each measurement is the best of three runs. The output is a
table of extra peak memory (MB), time (ms) and whether the checksum matches.

This is a report, not a pass/fail test, because memory and timing vary by
machine. The pass/fail parts are in the suite below.

### Safety suite

| Test | What it proves |
|---|---|
| `start_twice_is_rejected` | A second `Goose::start()` while one is running returns `Err(AlreadyRunning)` |
| `start_from_another_thread_is_rejected` | Another thread cannot start a second runtime while one runs |
| `restart_after_drop_works` | Dropping the handle shuts Goose down cleanly; a new start works and computes correctly |
| `panic_inside_with_goose_still_shuts_down` | A Rust panic while Goose runs still calls `goose_exit`; the next start works |
| `handle_is_not_send` (compile-fail doc test) | Moving the handle to another thread does not compile |
| `mismatched_lengths_panic_in_rust` | Bad input to a wrapper is a normal Rust panic, raised before Goose is called |
| `empty_inputs_are_safe` | Empty slices work everywhere; `summary(&[])` is `None` |
| `packed_layout_matches_goose` | `Summary` is 28 bytes and round-trips exact values |
| `goose_check_failure_aborts_process` | Bypassing the wrapper and breaking a Goose `assert` kills the process with Goose's error message, inside a child process, so the guard really exists |
| property tests (`proptest`) | Goose and Rust agree bit for bit on thousands of random inputs, including empty, NaN, infinities and extremes. Comparing floats bit for bit is valid because neither compiler reorders float math (no fast-math) |
| `repeated_calls_do_not_grow_memory` | 50 more calls to each workload grow committed memory by less than 2%, so Goose's data stacks really are reset between calls |

The Rust tests run twice in `just ci`: once in debug (Goose's runtime
checks on) and once in release (the optimized code actually shipped).

## 10. Quality settings

- **Rust:** edition 2024, resolver 3, exact toolchain pin with rustfmt and
  clippy. Workspace lints: clippy's default set as warnings,
  `undocumented_unsafe_blocks`, `unsafe_op_in_unsafe_fn` as an error,
  `missing_docs` on the bridge crate. Release profile: `lto = "thin"`,
  `codegen-units = 1`. A `cargo-deny` configuration covers licences and
  security advisories.
- **Goose:** `goose format --check` and `goose check` in `just lint`. The
  `.goose-version` pin is checked by the build script (as a warning) and
  by `just doctor`.
- **Editor:** VS Code recommendations (rust-analyzer, the Goose extension
  from Goose's install folder, just, TOML, EditorConfig), format on save,
  and Goose's language server pointed at `lib.goose`, the tests and the
  benchmark as entry files.
- `.editorconfig`, `.gitattributes` (LF line endings) and `.gitignore`
  (`target/`, and any executables a stray `goose build` leaves beside the
  Goose sources).

## 11. The manual (`docs/MANUAL.md`)

1. Daily commands.
2. Project layout.
3. How the bridge works, with a diagram.
4. Adding a Goose function, step by step.
5. Type mapping: Goose, C and Rust side by side.
6. Rules that keep it safe: one thread, packed structs, aborts, borrowed
   strings, batch-style calls.
7. Speed guide: release builds, the compiler choice, `just bce`, `simd fn`,
   `just contest`, and calling once per batch rather than once per item.
8. Rust and Cargo reference.
9. Goose reference: run, build, check, format, fetch, toolchain and their
   flags.
10. Troubleshooting: common errors and their fixes.
11. Settings (environment variables).

## 12. The template folder

```
rust-goose-template/
  README.md          how to install and use the template
  install.ps1        adds rgnew to the PowerShell profile; -Uninstall removes it
  new-project.ps1    the generator
  justfile           template-only commands: new, selftest, install
  template/          the project content: a real project named rgt-app
  docs/design/       this document
```

`just selftest` generates a throwaway project in a temporary folder and
runs its full `just ci`, so any change to the template is checked end to
end.

## 13. Setup on this machine (needs approval)

- Install LLVM 23.1.6 with winget (`winget install LLVM.LLVM`). It is the
  official installer from the llvm-project GitHub releases, roughly 400 MB,
  and provides libclang for bindgen and clang-cl for the Goose C.
- Afterwards, confirm `goose toolchain` still picks the same compiler. If
  the install changed it, pin Goose's own builds back in `~/.goose/config`.
- Run `install.ps1` once, which edits the PowerShell profile to add `rgnew`.
- Make the template folder a local git repository.

## 14. Left to the builder

- Exact bindgen options, and the mechanism for cleaning up names.
- The exact shape of the benchmarks.
- How the build script finds libclang (`LIBCLANG_PATH`, then the default
  LLVM folder).
- Which file types the generator treats as text.
- The manual's wording.

## 15. Calls made without asking

- The command is called `rgnew`, and it creates projects in the current
  folder unless `-Path` is given.
- Generated projects get a first git commit automatically when git has an
  identity configured.
- Debug builds turn Goose's checks on and its optimizations off, the same
  way Rust's debug builds work.
- The toolchain pins are the exact versions installed when a project is
  created, not "latest stable".
- Example kernels ship in every project, meant to be replaced.
- Windows first: portable code, tested only on Windows.
