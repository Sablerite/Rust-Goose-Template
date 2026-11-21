# Rgt App

A Rust program that runs its speed- and memory-critical parts in
[Goose](https://github.com/aardappel/goose), a memory-safe systems language
that compiles to C. Rust is the main program; the Goose code is compiled on
every build and called through a safe Rust API that enforces Goose's rules
(one thread, one runtime, inputs checked before Goose sees them).

## Quick start

```powershell
just doctor     # check the tools
just build      # the first build takes a minute or two
just run
just test
just            # every command
```

Before committing, run `just ci`: it lints both languages, runs every test
in debug and release builds, and audits the dependencies.

## Where things are

- `crates/rgt-app/`: the Rust program.
- `crates/rgt-app-core/goose/kernels/`: the Goose code.
- `crates/rgt-app-core/goose/bridge/lib.goose`: the Goose functions Rust can call.
- `crates/rgt-app-core/src/lib.rs`: the safe Rust API over them.
- `assets/`: files the program loads while running (textures, audio,
  fonts, shaders, models).
- `config/`: default settings that ship with the program.
- `data/`: sample and test inputs.
- `platform/`: per-OS packaging such as app icons and installer files.

Each of those folders has a README saying what belongs there.
[docs/CONVENTIONS.md](docs/CONVENTIONS.md) covers file naming, formats,
and how the parts connect.

The [manual](docs/MANUAL.md) covers how the two languages meet, how to add
a Goose function step by step, the safety and speed rules, what the tests
prove, and a command reference for Rust, Cargo and Goose.

## Requirements

Rust (the version in `rust-toolchain.toml`), Goose (see `.goose-version`),
LLVM for libclang and clang-cl (`winget install LLVM.LLVM`), Visual
Studio's C++ build tools, `just`, `cargo-nextest` and `cargo-deny`.
`just doctor` checks all of them.
