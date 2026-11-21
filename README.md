# Rust + Goose project template

One command creates a new project where Rust is the main program and
[Goose](https://github.com/aardappel/goose) runs the parts where speed and
memory matter most. Each project comes ready to build, test, benchmark and
audit, with a manual for both languages.

## Install (once)

```powershell
pwsh -File install.ps1
```

This adds `rgnew` (short for `New-RustGooseProject`) to the PowerShell
profile, plus tab completion for `goose` and `just`. Open a new shell
afterwards. Re-run it if this folder moves. `install.ps1 -Uninstall`
removes everything it added.

## Create a project

```powershell
rgnew my-app                  # creates .\my-app
rgnew my-app -Path D:\code    # creates D:\code\my-app
rgnew my-app -NoGit           # skip git init and the first commit
```

The name is used everywhere it belongs:

- the folder, `my-app`
- the crates, `my-app` and `my-app-core`
- the Goose exports, prefixed `my_app_`
- the title in the docs, "My App"

The command also pins the installed Rust and Goose versions, starts a git
repository with a first commit, and checks that every tool is installed.
Then:

```powershell
cd my-app
just build
just run
```

Every generated project has its own manual in `docs/MANUAL.md`.

## What a project includes

- **A bridge that always matches the Goose code.** On every build, Goose
  compiles to C, and the Rust declarations are generated from Goose's own
  header (bindgen). The C is compiled with clang-cl using the same flags as
  `goose build`, so Goose code runs as fast inside the Rust program as on
  its own.
- **A safe Rust API.** Goose's same-thread rule is a type rule. Only one
  runtime can run per process. Inputs that would make Goose abort are
  checked in Rust first.
- **Tests on both sides:**
  - Goose-side tests;
  - Rust tests that check every Goose function against a plain-Rust twin;
  - property tests on thousands of random inputs;
  - a safety suite that pins each boundary rule;
  - memory tests.
- **A memory and speed contest** (`just contest`): the same workloads in
  Goose alone, Goose inside Rust, idiomatic Rust and tuned Rust, with peak
  memory and time side by side.
- **Standard folders** for assets, settings, sample data and per-OS
  packaging. Each has a README saying what goes there, git rules keep
  asset files intact, and a conventions guide (`docs/CONVENTIONS.md`)
  covers file naming, formats and how the parts connect.
- **Standard tooling:** pinned toolchains, rustfmt and clippy with strict
  unsafe-code lints, cargo-deny, `just` recipes for everything, and VS Code
  settings for rust-analyzer and the Goose extension.

## Requirements

- Rust (rustup)
- Goose
- LLVM (`winget install LLVM.LLVM`), for libclang and clang-cl
- Visual Studio's C++ build tools
- `just`, `cargo-nextest` and `cargo-deny`
- PowerShell 7 for the `just` recipes

## Working on the template

```powershell
just selftest
```

This generates a throwaway project in a temporary folder whose path
contains a space, and runs that project's full `just ci` plus a few extra
checks:

- no placeholder name is left anywhere;
- plain `cargo test` passes;
- the program runs;
- editing a Goose file triggers a rebuild.

Run it after any change to the template.

The project content is in `template/`. It is a real, buildable project
named `rgt-app`, and the generator replaces `rgt-app`, `rgt_app` and
`Rgt App` with the new name. Design notes are in `docs/design/`.
