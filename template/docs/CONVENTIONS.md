# Rgt App conventions

Where each kind of file goes, what to name it, which format to use, and
how the parts connect. The aim is that adding a texture, a settings file,
a test fixture or a new crate never starts with a web search.

Contents:

1. [How to use this guide](#1-how-to-use-this-guide)
2. [Folder by folder](#2-folder-by-folder)
3. [Naming by purpose](#3-naming-by-purpose)
4. [Which format for which job](#4-which-format-for-which-job)
5. [How the parts connect](#5-how-the-parts-connect)

---

## 1. How to use this guide

These are suggestions, not rules the build checks. Where Rust has a
settled community convention (crate, module and type names, the crate
folder layout), this guide follows it and says so. Breaking those makes
the code harder for any Rust programmer to read, and clippy or rustc warn
about some of them. Everything else is a sensible default. Change it if
the project needs something else, and write the change down here so the
whole project follows one rule.

Each top-level folder also has its own `README.md` with a short version of
its section below.

---

## 2. Folder by folder

### The project root

| Folder | What goes inside | What does not |
|---|---|---|
| `assets/` | Files the program loads while running: textures, audio, fonts, shaders, models, UI images | Source art (`.psd`, `.blend`), settings, test data |
| `config/` | Default settings files that ship with the program (`default.toml`) | A user's own settings, secrets, content files |
| `data/` | Inputs for development: samples, test fixtures, small datasets | The program's output, large or private datasets |
| `platform/` | Per-OS packaging: app icons, manifests, installer scripts (`windows/`, later `android/`, `ios/`) | Images shown inside the program, finished installers |
| `crates/` | All Rust code, one folder per crate, with the Goose code inside the core crate | Anything that is not code or its tests |
| `docs/` | The manual, this guide, and dated design notes in `docs/design/` | Generated API docs (`just docs` builds those into `target/`) |
| `scripts/` | Helper scripts behind the `just` recipes | One-off scripts nobody runs again (delete them) |
| `target/` | Everything Cargo and the build produce. Ignored by git, safe to delete (`just clean`) | Anything you want to keep |
| `.cargo/` | `config.toml`: optional build settings (manual, section 12) | Secrets |
| `.vscode/` | Editor settings and recommended extensions shared by the project | Personal editor settings |
| `.claude/` | Session notes (`handoff/HANDOFF.md`) | Code |

The files at the root are the workspace's own: `Cargo.toml` (the crate
list, shared versions, lints, profiles), `Cargo.lock` (exact dependency
versions; commit it), `rust-toolchain.toml`, `.goose-version`, `justfile`,
`deny.toml`, `rustfmt.toml`, `.editorconfig`, `.gitattributes`,
`.gitignore` and `README.md`.

### Inside a crate (`crates/<crate>/`)

This is Cargo's standard layout. Cargo finds these by name, so following
it means no extra settings.

| Path | What it is |
|---|---|
| `Cargo.toml` | The crate's name, dependencies and settings |
| `build.rs` | Runs before the crate compiles: generating code, compiling C, embedding resources. Only add one when a crate needs it |
| `src/lib.rs` | The root of a library crate: other crates use what it makes `pub` |
| `src/main.rs` | The root of a program crate: `fn main` |
| `src/bin/<name>.rs` | Extra programs in the same crate, run with `cargo run --bin <name>` |
| `src/<module>.rs` | A module. When it grows submodules, add a `src/<module>/` folder beside it for them. Rust's current style is `parser.rs` plus `parser/`, not `parser/mod.rs` |
| `tests/<topic>.rs` | Integration tests: each file is compiled as its own crate and uses only the public API. One file per topic (`bridge.rs`, `safety.rs`) |
| `tests/common/mod.rs` | Helpers shared by several test files (the `mod.rs` form here stops Cargo treating it as a test of its own) |
| `benches/<topic>.rs` | Benchmarks, run with `just bench` |
| `examples/<name>.rs` | Small runnable programs that show how to use the crate, run with `cargo run --example <name>` |

Unit tests that need private items live at the bottom of the file they
test, in a `#[cfg(test)] mod tests { ... }` block.

### Inside the Goose folder (`crates/rgt-app-core/goose/`)

| Path | What it is |
|---|---|
| `kernels/` | The Goose code itself, a Goose package named `kernels`. One file per topic (`kernels.goose`, `workloads.goose`) |
| `bridge/lib.goose` | The boundary Rust calls: `export fn`s only. It stays alone in its folder, because a Goose program owns every `.goose` file under its folder |
| `tests/<topic>_test.goose` | Goose-side tests, each a program with its own `main`, run by `just test-goose` |
| `bench/` | Goose programs used by the contest |

---

## 3. Naming by purpose

General rules for every file and folder:

- lowercase, no spaces, plain ASCII letters and digits;
- `_` or `-` between words, following the table below for each kind;
- say what it is, not when or by whom (`enemy_spawner.rs`, not
  `new_stuff.rs` or `my_test.rs`);
- no `final`, `v2`, `old` or `copy` in names: git keeps the history.

### Rust code (the community convention, checked by rustc and clippy)

| What | Style | Example |
|---|---|---|
| Crate (folder and `name` in `Cargo.toml`) | `kebab-case`, the project name first | `rgt-app-render`, `rgt-app-net` |
| Crate in code (`use`) | the same name with `_` | `use rgt_app_render::Renderer;` |
| Module, file | `snake_case` | `texture_cache.rs` |
| Type, trait, enum variant | `UpperCamelCase` | `TextureCache`, `LoadError::NotFound` |
| Function, method, variable | `snake_case` | `load_texture`, `frame_count` |
| Constant, static | `SCREAMING_SNAKE_CASE` | `MAX_TEXTURE_SIZE` |
| Lifetime, generic type | short, lowercase / one capital | `'a`, `T` |
| Test function | the behaviour it checks | `empty_input_returns_none`, `rejects_negative_size` |
| Benchmark | the operation and size | `saxpy_1m`, `sort_10k_strings` |

Acronyms count as one word: `HttpClient` and `GpuBuffer`, not `HTTPClient`.

### Goose code (the style the shipped Goose code uses)

| What | Style | Example |
|---|---|---|
| Package folder and its `namespace` | lowercase, the same word | folder `kernels/`, `namespace kernels;` |
| File | `snake_case.goose`, one topic each | `workloads.goose` |
| Test file | `<topic>_test.goose` | `kernels_test.goose` |
| Type | `UpperCamelCase` | `Summary` |
| Function, variable | `snake_case` | `count_primes` |
| Export | `export "<c_name>" fn <name>`, with the C name prefixed by the project's name | `export "rgt_app_count_primes" fn count_primes(...)` |

The C-name prefix keeps the project's symbols from clashing with any other
C library linked into the program. The Rust method drops the prefix
(`goose.count_primes(n)`).

### Assets

Lowercase, `_` between words, no spaces. Build each name from general to
specific: **what it is**, then **its variant or state**, then **a frame or
size number**.

| Kind | Pattern | Examples |
|---|---|---|
| Sprite or image | `<thing>_<state>_<frame>` | `player_run_03.png`, `coin_spin_12.png` |
| Texture set | `<material>_<map>` with a standard map name | `brick_albedo.png`, `brick_normal.png`, `brick_roughness.png`, `brick_ao.png` |
| High-density image | `@2x`, `@3x` suffix | `button_play@2x.png` |
| Sound effect | `sfx_<thing>_<action>_<n>` | `sfx_door_open_01.ogg` |
| Music | `music_<place_or_mood>` | `music_main_menu.ogg` |
| Font | the family and weight, as published | `inter_regular.ttf`, `inter_bold.ttf` |
| Shader | what it draws | `sprite.wgsl`, `post_bloom.wgsl` |
| Model | the object | `crate_wooden.glb` |
| UI image | `ui_<element>_<state>` | `ui_button_hover.png` |

Number frames with leading zeros (`_01` … `_12`) so they sort in order.
Group by kind in subfolders (`textures/`, `audio/`) and then, once a
folder passes about 50 files, by feature (`textures/player/`).

### Settings

| What | Name | Example |
|---|---|---|
| The defaults | `config/default.toml` | |
| An environment's overrides | `config/<environment>.toml` | `dev.toml`, `test.toml` |
| Keys | `snake_case`, grouped in tables | `[window]` `width = 1280` |
| Environment variables that override a setting | the project name in capitals, then the table and key | `MY_APP_WINDOW_WIDTH` for a project named `my-app` |

Secrets never go in these files. Read them from environment variables.

### Sample data

Name a file for what it contains or tests, not for when it was made.

| Kind | Pattern | Examples |
|---|---|---|
| Test fixture | `fixtures/<case>.<ext>` | `fixtures/empty_chapter.json`, `fixtures/broken_header.cbz` |
| Sample to try by hand | `samples/<description>.<ext>` | `samples/large_library.csv` |
| Expected output for a test | `fixtures/<case>.expected.<ext>` | `fixtures/sort_names.expected.txt` |

### Scripts

`kebab-case.ps1`, one job each, run through a `just` recipe so nobody
needs to remember the path (`scripts/doctor.ps1` behind `just doctor`).
Name it for the job: `make-icons.ps1`, `export-assets.ps1`.

### Docs

| What | Name |
|---|---|
| Guides people read often | capitals: `MANUAL.md`, `CONVENTIONS.md` |
| Design notes | `docs/design/YYYY-MM-DD-<topic>-design.md` |
| Implementation plans | `docs/design/YYYY-MM-DD-<topic>-plan.md` |
| Diagrams and images used by docs | `docs/images/<topic>.png` |

Dates first, so the notes list in order.

---

## 4. Which format for which job

| Job | Use | Why |
|---|---|---|
| Textures while working | PNG | Lossless, every tool reads it |
| Textures shipped to the GPU | KTX2 (with Basis compression) | Stays compressed in GPU memory, loads faster |
| Photos, large backgrounds | JPEG or WebP | Much smaller where exact pixels do not matter |
| Music, long sounds | OGG (Vorbis) or Opus | Small, free of licence fees |
| Short sound effects | WAV | No decode delay |
| Fonts | TTF or OTF | What font libraries read |
| 3D models | glTF 2.0, packed as `.glb` | The open standard; one file with its textures |
| Shaders for wgpu | WGSL | wgpu's own shader language |
| Settings | TOML | Readable, comments allowed, Cargo uses it too |
| Data for tests and samples | JSON, or CSV for tables | Readable, supported everywhere |
| Windows app icon | ICO with 16, 32, 48 and 256 px sizes | What Windows shows in Explorer and the taskbar |
| Mobile app icons | PNG at each size the platform asks for | What Android and iOS expect |

Git treats these binary formats as binary (see `.gitattributes`), so line
endings are never converted inside them.

---

## 5. How the parts connect

### One crate to another

Split code into a new crate when it has its own job and could be tested
alone (a renderer, a network layer, a file-format reader), when it has
heavy dependencies the rest should not wait on, or when a second program
needs it. Otherwise, a module inside an existing crate is simpler.

1. Create `crates/rgt-app-render/` with its own `Cargo.toml` and
   `src/lib.rs` (`cargo new --lib crates/rgt-app-render`).
2. Add it to `members` in the root `Cargo.toml`, and give it a line in
   `[workspace.dependencies]`:

   ```toml
   [workspace]
   members = ["crates/rgt-app", "crates/rgt-app-core", "crates/rgt-app-render"]

   [workspace.dependencies]
   rgt-app-render = { path = "crates/rgt-app-render" }
   ```

3. In the crate that uses it:

   ```toml
   [dependencies]
   rgt-app-render.workspace = true
   ```

Give each new crate the workspace's shared settings (`version.workspace =
true` and the rest, plus `[lints] workspace = true`), as the existing
crates do.

### Rust to Goose

Only `rgt-app-core` touches Goose. Other crates call the methods on
`rgt_app_core::Goose`. The manual's section 3 explains how the bridge
works, and section 4 walks through adding a Goose function.

### Code to assets

There are three ways. Pick one per project and use it everywhere.

**1. Read from disk, beside the executable.** Good for large or many
files, and for files that change without a rebuild (mods, level files).
The installed program needs `assets/` copied next to it.

```rust
use std::path::PathBuf;

fn assets_dir() -> std::io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let dir = exe.parent().expect("an executable lives in a folder");
    Ok(dir.join("assets"))
}
```

During development the executable is in `target/debug/`, not next to
`assets/`. Fall back to the source folder there, for example with an
environment variable (`MY_APP_ASSETS` for a project named `my-app`) or a `cfg!(debug_assertions)`
branch using `env!("CARGO_MANIFEST_DIR")`. Make sure an installed copy
never falls back to a path on the build machine.

**2. Build into the executable.** Good for a few small files the program
cannot run without (a default font, a shader, the default settings). One
file to ship, nothing to lose. The path is checked when compiling, and
Cargo rebuilds when the file changes.

```rust
const SPRITE_SHADER: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/shaders/sprite.wgsl"));
const UI_FONT: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/fonts/inter_regular.ttf"));
```

Everything included this way sits in memory for the whole run and makes
the executable bigger, so keep it to small files.

**3. Read from an app package (mobile).** Android and iOS put assets
inside the installed app, and the program reads them through the
platform's API (Android's asset manager, iOS's app bundle), not as plain
files. Set this up when the project builds for mobile.

### Tests to sample data

An integration test in `crates/<crate>/tests/` finds `data/` from its own
crate's folder:

```rust
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/fixtures")
        .join(name)
}

#[test]
fn empty_chapter_has_no_pages() {
    let text = std::fs::read_to_string(fixture("empty_chapter.json")).unwrap();
    assert!(text.contains("\"pages\": []"));
}
```

### Settings

Read `config/default.toml` into a struct with the `serde` crate (with its
`derive` feature) and the `toml` crate. Neither is added yet; add them
when the project needs settings.

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Settings {
    window: Window,
}

#[derive(Debug, Deserialize)]
struct Window {
    width: u32,
    height: u32,
}

fn load_settings(text: &str) -> Result<Settings, toml::de::Error> {
    toml::from_str(text)
}
```

Build the defaults into the executable (`include_str!`, as above), then
apply the user's file from their settings folder on top. The program then
always starts, even with no settings file.

### The Windows icon

Put the icon in `platform/windows/app.ico`. Build it into the executable
from the program crate's `build.rs` with the `winresource` crate (a build
dependency of `crates/rgt-app`, not added yet):

```rust
// crates/rgt-app/build.rs
fn main() {
    println!("cargo::rerun-if-changed=../../platform/windows/app.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../platform/windows/app.ico");
        res.compile().expect("could not embed the Windows icon");
    }
}
```

Explorer and the taskbar then show the icon with no extra files. It
needs the Windows SDK's resource compiler, which Visual Studio's C++
build tools include.
