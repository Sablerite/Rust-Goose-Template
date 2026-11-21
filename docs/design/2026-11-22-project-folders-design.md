# Standard project folders: design

Date: 2026-11-22

## Why

Projects made from the template could become anything: a wgpu game, a
mobile reader app, a small tool. Today a new project only has folders for
code and tooling (`crates/`, `docs/`, `scripts/`), so images, sounds,
settings and sample files have no agreed home and end up scattered. Every
project should start with the same layout, where each kind of file has one
obvious place.

## Goals

- Every generated project has the same top-level folders, each with one
  job.
- Each folder holds a short `README.md` that says what goes in it, what
  does not, and how it is usually split into subfolders. The READMEs are
  what keep files going where they belong.
- A guide in every project, `docs/CONVENTIONS.md`, so naming and wiring
  things up never starts from a web search. It covers every folder (what
  goes inside, down to `src/`, `tests/`, `benches/`, `examples/`,
  `build.rs` and the Goose folders), naming rules per purpose (Rust and
  Goose code, assets, settings, sample data, scripts, docs), which file
  formats suit which job, and how the parts connect: crate to crate, Rust
  to Goose, code to assets, tests to sample data, settings, and the
  Windows icon.
- `just selftest` checks the layout, so it cannot quietly go missing.

## Non-goals

- Building for Android or iOS. The bridge only builds for Windows (MSVC
  with clang-cl); mobile needs cross-compiling Rust and the Goose C with
  the Android NDK, plus app packaging. That is a separate addition. The
  layout only leaves room for it (`platform/android/`).
- Packaging, installers, or building Microsoft's C runtime into the
  executable (`+crt-static`). How a program ships differs per project, so
  each project sets it up itself.
- Code that finds `assets/` and `config/` at run time. Some projects load
  files from disk, others bundle them into the executable or read them
  from an app package, so this is left to each project. (A small helper
  crate was considered and dropped for that reason.)
- Different layouts per kind of project.

## Anti-goals

- A folder whose job overlaps another's, so the same file could go in two
  places.
- Generated or downloaded output inside a source folder (committed by
  accident).
- Anything in the new folders that mentions the template (the
  clean-slate rule).

## Approaches

1. **One fixed layout in every project (chosen).** The same folders
   everywhere, each with a README. One standard, one shape for the
   selftest to check. A small tool carries a few nearly empty folders,
   which costs nothing.
2. A shared core plus a kind chosen at creation (`-Kind game`,
   `-Kind app`). Tidier projects, but three layouts to keep tested, and a
   project that changes kind ends up with the wrong folders.
3. The core layout plus a command that adds folder sets later. The same
   trade-off as 2, plus the most code to build and keep working.

Option 1 was picked: the template is meant to fit every kind of project,
and the READMEs make even an empty folder useful.

## The layout

New folders are marked with `+`; the rest exists today.

```
my-app/
+ assets/        files the program loads while it runs: textures, audio,
                 fonts, shaders, models, UI images. Read-only at run time.
+ config/        default settings files that ship with the program. Users'
                 own settings live in the OS settings folder, not here.
+ data/          inputs for development: sample files, test fixtures, small
                 datasets. Never the program's output.
+ platform/      per-OS packaging: app icons, manifests, installer files.
                 Subfolders by OS (windows/, later android/, ios/).
  crates/        all code, as now
  docs/          the manual, plus design/ for design notes (as now)
  scripts/       helper scripts behind the just commands (as now)
```

Each new folder holds only its `README.md` at first. Suggested subfolders
are listed in the README, not created, so nothing empty is committed
beyond the README itself.

Output the program writes while running goes to the OS's per-user data
folder in real use, and under `target/` during development (already
ignored by git). The `data/` README says so.

## Other changes

- The manual's project-layout section shows the new folders and what each
  is for, and notes that loading files from them at run time is up to the
  project.
- The selftest checks that each new folder and its README exist in the
  generated project. The existing placeholder and clean-slate checks
  already cover the READMEs' contents.
- The generator needs no change: it copies every file in the template and
  replaces the name in `.md` and `.rs` files already.

## Left to the builder

- The exact README wording, in plain English.
- The guide's exact wording and examples. Its examples are suggestions,
  not rules the build enforces; where the Rust or Goose community has a
  settled convention, the guide follows it and says so.
