# Standard project folders: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every generated project starts with `assets/`, `config/`, `data/` and `platform/`, each holding a README that says what belongs there, git handles the files that go in them correctly, and a guide (`docs/CONVENTIONS.md`) explains naming and how the parts connect.

**Architecture:** The folders and READMEs live in `template/`, so the generator copies them and replaces the name like any other file (no generator change). `.gitignore` and `.gitattributes` get rules so asset files are neither ignored nor mangled. The selftest checks the folders and the git rules in the generated project.

**Tech Stack:** Markdown, git attributes and ignore rules, PowerShell 7 (selftest).

**Spec:** `docs/design/2026-11-22-project-folders-design.md`

## Global Constraints

- Folders: exactly `assets/`, `config/`, `data/`, `platform/` at the project root. No subfolders are created; READMEs suggest them.
- Each new folder holds only `README.md`.
- Nothing in a generated project may contain `template`, `rgnew` or `install.ps1` (the selftest's clean-slate check), or a leftover `rgt-app`, `rgt_app` or `Rgt App`.
- The project name in READMEs is written as `Rgt App` / `rgt-app` so the generator replaces it.
- No run-time code: loading files from these folders is left to each project.
- Plain English, written for someone who has never seen the project.
- No commits unless asked. The template folder has no commits yet.

## Review Focus

1. **A Wavefront `.obj` model in `assets/` is silently ignored by git**, because `.gitignore` has `*.obj` (for stray compiler objects). Expected: model files are tracked. Test in Task 1.
2. **A binary asset is corrupted by line-ending conversion.** `.gitattributes` has `* text=auto eol=lf` and marks only `*.png` binary, and git's binary guess can be wrong (for example a file with no zero byte near the start). Expected: common asset types are always binary. Test in Task 1.
3. **A README mentions "template"** and fails the clean-slate check. Covered by the existing selftest check, run in Task 2.
4. **A large asset bloats the repository.** Nothing enforces a limit. The `assets/` README points to Git LFS for large binaries. There's no test; README wording only.
5. **Program output written into `data/`** gets committed. The `data/` README says output goes to `target/` during development (already ignored). There's no test; README wording only.

---

### Task 1: Folders, READMEs and git rules

**Files:**
- Create: `template/assets/README.md`, `template/config/README.md`, `template/data/README.md`, `template/platform/README.md`
- Modify: `template/.gitignore`, `template/.gitattributes`
- Test: `scripts/selftest.ps1` (new check after the clean-slate check, before `Push-Location`)

**Interfaces:**
- Produces: the four folder names, used by Task 2's docs.

- [ ] **Step 1: Add the failing selftest check**

A new block, `== standard folders, each with a README, and git handles asset files`:
- For each of `assets`, `config`, `data`, `platform`: `<project>\<folder>\README.md` exists and its length is greater than 0. Otherwise `Stop-Selftest "missing or empty <folder>\README.md"`.
- `git -C $project check-ignore -q assets/models/ship.obj` must exit non-zero (not ignored). Otherwise `Stop-Selftest 'git ignores .obj models in assets/'`. Reset `$LASTEXITCODE` afterwards, because `Invoke-Step` reads it.
- `git -C $project check-ignore -q crates/stray.obj` must exit 0, so stray compiler objects are still ignored.
- For each of `assets/a.png`, `assets/a.ogg`, `assets/a.ttf`, `assets/a.glb`, `assets/a.ktx2`: `git -C $project check-attr binary -- <path>` output ends in `: set`. Otherwise `Stop-Selftest "<path> is not marked binary"`.
- If git is missing or the project has no `.git`, the git checks fail the selftest. The selftest always runs git init, so a missing `.git` is a real failure.

- [ ] **Step 2: Run it and see it fail**

Run: `just selftest` (template root, about 5–10 minutes)
Expected: `selftest FAILED: missing or empty assets\README.md`

- [ ] **Step 3: Write the four READMEs**

Each one starts with `# <folder>/`, then one sentence on its job, then "Goes here" and "Doesn't go here" lists and suggested subfolders. Keep each under about 30 lines. Content each must carry:
- `assets/`: what Rgt App loads while running, read-only at run time. Subfolders: `textures/`, `audio/`, `fonts/`, `shaders/`, `models/`, `ui/`. Doesn't go here: source art files (`.psd`, `.blend`, `.kra`); keep those in an `assets-src/` or outside the repository. Large binaries: consider Git LFS. How the program finds these files is up to the project (from disk beside the executable, bundled with `include_bytes!`, or from an app package).
- `config/`: default settings files that ship with the program, such as `default.toml`. A user's own settings belong in the OS's per-user settings folder, never here. No secrets or keys.
- `data/`: inputs for development: sample files, test fixtures, small datasets. Subfolders: `samples/`, `fixtures/`. Never the program's output: write it under `target/` while developing (ignored by git) and to the user's data folder once installed. Rust tests can reach this folder with `concat!(env!("CARGO_MANIFEST_DIR"), "/../../data")`.
- `platform/`: per-OS packaging: app icons, manifests, installer scripts. Subfolders by OS: `windows/` now, `android/` and `ios/` once the project builds for them. Only the Windows build is set up today.

- [ ] **Step 4: Fix the git rules**

- `template/.gitignore`: after the `*.obj` line, add `!/assets/**/*.obj` with the comment `# ...but Wavefront .obj models in assets/ are real files`.
- `template/.gitattributes`: keep the existing lines and add `binary` lines under a comment `# Asset files: never convert line endings`. Cover images `png jpg jpeg gif webp bmp tga hdr exr ico icns dds ktx ktx2 basis`, audio `wav ogg mp3 flac opus`, fonts `ttf otf woff woff2`, models `glb fbx blend`, archives `zip 7z`, and `pdf`. Leave `.obj`, `.gltf` and `.svg` as text, since they are text formats.

- [ ] **Step 5: Run the selftest and see it pass**

Run: `just selftest`
Expected: `selftest passed: the template generates a project that passes every check.`

- [ ] **Step 6: No commit** (only on request)

---

### Task 2: Docs

**Files:**
- Modify: `template/docs/MANUAL.md` (section 2, "Project layout"), `template/README.md` ("Where things are"), `README.md` at the template root ("What a project includes")

**Interfaces:**
- Consumes: the four folder names from Task 1.

- [ ] **Step 1: Manual section 2.** In the layout tree, add `assets/`, `config/`, `data/` and `platform/` above `crates/` with a one-line job each, matching the READMEs. Under the tree add a short paragraph: each folder's README says what goes there; loading these files at run time is up to the project; build output stays in `target/`.

- [ ] **Step 2: Project README.** Add four bullets to "Where things are", one per folder.

- [ ] **Step 3: Template root README.** Add a bullet to "What a project includes": standard folders for assets, settings, sample data and per-OS packaging, each with a README, and git rules that keep asset files intact.

- [ ] **Step 4: Point to the guide.** In the manual's section 2, each folder README's last line, and the project README, link to `docs/CONVENTIONS.md` (written in Task 3) for naming and wiring.

- [ ] **Step 5: Check for leftovers.** Search the three files' new text for `template` (only allowed in the root README), stage words and tool names. Then run `just selftest`.
Expected: `selftest passed: ...`

- [ ] **Step 6: No commit** (only on request)

---

### Task 3: The naming and wiring guide

**Files:**
- Create: `template/docs/CONVENTIONS.md`
- Test: `scripts/selftest.ps1` (add `docs\CONVENTIONS.md` to Task 1's existence check)

**Interfaces:**
- Consumes: the four folder names and README contents from Task 1. The guide must agree with them.

- [ ] **Step 1: Add the failing check.** Task 1's folder check also requires a non-empty `docs\CONVENTIONS.md`. Run `just selftest` and expect `missing or empty docs\CONVENTIONS.md`.

- [ ] **Step 2: Write the guide.** Title `# Rgt App conventions`. Sections, in order:
  1. **How to use this guide.** Suggestions, not rules the build checks. Where Rust or Goose has a settled convention, follow it.
  2. **Folder by folder.** A table for the root (`assets/`, `config/`, `data/`, `platform/`, `crates/`, `docs/`, `scripts/`, `target/`, `.cargo/`, `.vscode/`, `.claude/`): what's inside and what isn't. Then inside a crate: `Cargo.toml`, `build.rs`, `src/lib.rs` vs `src/main.rs`, `src/bin/`, modules as `name.rs` plus a `name/` folder (not `mod.rs`), `tests/` (integration tests, one file per topic), `benches/`, `examples/`. Then inside `goose/`: `kernels/` (the package), `bridge/` (exports only, its own folder), `tests/`, `bench/`.
  3. **Naming by purpose.** One subsection each, with good and bad examples:
     - Rust: crates `kebab-case`, the project name first (`rgt-app-render`); modules and files `snake_case`; types `UpperCamelCase`; functions `snake_case`; constants `SCREAMING_SNAKE_CASE`; tests named for the behaviour (`empty_input_returns_none`).
     - Goose: the `namespace` matches the package folder; exports `rgt_app_<verb>` (C name) beside the plain Goose name.
     - Assets: lowercase, `_` between words, no spaces. Order: what, then variant, then frame (`player_run_03.png`), with standard suffixes (`_normal`, `_albedo`, `_roughness`, `@2x`). Group by type in subfolders, then by feature once a folder grows past about 50 files.
     - Settings: `default.toml`, and an override per environment (`dev.toml`). Keys `snake_case`.
     - Sample data: name it for what it tests (`fixtures/empty_chapter.json`, `samples/large_library.csv`).
     - Scripts: `kebab-case.ps1`, one job each, run through a `just` recipe.
     - Docs: guides in capitals (`MANUAL.md`, `CONVENTIONS.md`); design notes dated `YYYY-MM-DD-topic-design.md` in `docs/design/`.
  4. **Which format for which job.** A table: textures (PNG while working, KTX2 for GPU-ready), audio (OGG for music, WAV for short effects), fonts (TTF/OTF), 3D models (glTF/GLB), shaders (WGSL for wgpu), settings (TOML), sample data (JSON or CSV), icons (ICO on Windows, PNG for mobile).
  5. **How the parts connect.** Short code snippets for each:
     - crate to crate: a new workspace member plus a path dependency (`rgt-app-core = { path = "../rgt-app-core" }`), and when to make a new crate rather than a module;
     - Rust to Goose: one paragraph pointing to the manual's sections 3 and 4;
     - code to assets, three ways with trade-offs: from disk beside the executable (`std::env::current_exe()`), built into the executable (`include_bytes!` / `include_str!`), or from an app package on mobile;
     - tests to sample data: `concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/...")`;
     - settings: read `config/default.toml` with the `toml` and `serde` crates (named, not added);
     - Windows icon: `platform/windows/app.ico`, built into the executable by a `build.rs` using the `winresource` crate (named, not added).
  Snippets are illustrations, not compiled. Keep each correct Rust 2024 that would compile with the named crates added.

- [ ] **Step 3: Check accuracy.** Every crate, macro and function the guide names must exist as described (`winresource`, `toml`, `serde`, `include_bytes!`, `std::env::current_exe`). Every path must match the layout. Search for `template`, `rgnew`, stage words and tool names.

- [ ] **Step 4: Run the selftest.**
Run: `just selftest`
Expected: `selftest passed: ...`

- [ ] **Step 5: No commit** (only on request)
