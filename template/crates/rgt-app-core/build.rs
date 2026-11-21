//! Turns the Goose code into something Rust can call, on every build.
//!
//! 1. `goose` compiles `goose/bridge/lib.goose` (and everything it imports)
//!    into one C file plus a header describing the exported functions.
//! 2. bindgen reads that header with a real C front end (libclang) and writes
//!    the Rust declarations, so they always match the Goose code exactly.
//!    Goose's C name decorations are then removed (`kernels_Summary_g7`
//!    becomes `Summary`, the field `count_g` becomes `count`).
//! 3. The C is compiled with the same flags `goose build` uses (`-O2` and
//!    nothing else in release), preferably with clang-cl, so Goose code runs
//!    as fast inside the Rust program as it does on its own.
//!
//! Settings (environment variables): `GOOSE` (path to goose),
//! `GOOSE_C_COMPILER` (`clang-cl`, `cl` or a path), `GOOSE_PROFILE`
//! (`debug` or `release`, overriding Cargo's profile), `GOOSE_STACK_RESERVE`
//! (address space per Goose data stack, such as `8G`; default 2G),
//! `LIBCLANG_PATH`.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

/// The Goose file that declares the C boundary, relative to this crate.
const GOOSE_ROOT: &str = "goose/bridge/lib.goose";
/// Name of the static library holding the compiled Goose code.
const LIB_NAME: &str = "rgt_app_goose";
/// Where the LLVM installer puts clang-cl and libclang on Windows.
const LLVM_BIN_WINDOWS: &str = r"C:\Program Files\LLVM\bin";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Goose's runtime checks on (`--debug`), C unoptimized with debug info.
    Debug,
    /// Goose's default optimization, C at `-O2` and nothing else.
    Release,
}

fn main() {
    let manifest_dir = PathBuf::from(env_var("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(env_var("OUT_DIR"));
    let target = env_var("TARGET");
    let workspace_dir = manifest_dir.join("..").join("..");

    // Rerun when any Goose source, the version pin or a setting changes.
    println!("cargo::rerun-if-changed=goose");
    println!(
        "cargo::rerun-if-changed={}",
        workspace_dir.join(".goose-version").display()
    );
    for var in [
        "GOOSE",
        "GOOSE_C_COMPILER",
        "GOOSE_PROFILE",
        "GOOSE_STACK_RESERVE",
        "LIBCLANG_PATH",
    ] {
        println!("cargo::rerun-if-env-changed={var}");
    }

    let goose = find_goose();
    // A new Goose release must regenerate the C even if no source changed.
    println!("cargo::rerun-if-changed={}", goose.display());
    check_goose_version(&goose, &workspace_dir);

    let mode = build_mode();
    let c_file = out_dir.join("goose_lib.c");
    let header = out_dir.join("goose_lib.h");
    generate_c(
        &goose,
        &manifest_dir.join(GOOSE_ROOT),
        &c_file,
        &header,
        mode,
    );
    generate_bindings(&header, &out_dir.join("bindings.rs"), &target);
    compile_c(&c_file, &target, mode);
}

/// Stops the build with one clear error line.
fn fail(message: &str) -> ! {
    println!("cargo::error={message}");
    process::exit(1);
}

fn env_var(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| fail(&format!("Cargo did not set {name}")))
}

fn exe_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_owned()
    }
}

/// Looks for an executable in each directory on PATH.
fn find_on_path(base: &str) -> Option<PathBuf> {
    let name = exe_name(base);
    env::split_paths(&env::var_os("PATH")?)
        .map(|dir| dir.join(&name))
        .find(|candidate| candidate.is_file())
}

/// `GOOSE`, then PATH, then Goose's default Windows install folder.
fn find_goose() -> PathBuf {
    if let Some(path) = env::var_os("GOOSE") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return path;
        }
        fail(&format!(
            "GOOSE is set to {}, which does not exist. Fix or unset it.",
            path.display()
        ));
    }
    if let Some(path) = find_on_path("goose") {
        return path;
    }
    if let Some(local) = env::var_os("LOCALAPPDATA") {
        let path = PathBuf::from(local)
            .join("Programs")
            .join("goose")
            .join("goose.exe");
        if path.is_file() {
            return path;
        }
    }
    fail(
        "goose was not found. Install Goose, or set GOOSE to the full path of the goose executable.",
    )
}

/// Warns (but still builds) when the installed Goose differs from the pin.
fn check_goose_version(goose: &Path, workspace_dir: &Path) {
    let Ok(pinned) = fs::read_to_string(workspace_dir.join(".goose-version")) else {
        return;
    };
    let output = Command::new(goose).arg("version").output();
    let installed = match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).into_owned(),
        _ => fail(&format!("could not run `{} version`", goose.display())),
    };
    let (pinned, installed) = (pinned.trim(), installed.lines().next().unwrap_or("").trim());
    if pinned != installed {
        println!(
            "cargo::warning=this project was made with `{pinned}` but `{installed}` is installed. \
             If everything still passes `just ci`, update .goose-version."
        );
    }
}

fn build_mode() -> Mode {
    let requested = env::var("GOOSE_PROFILE").ok();
    let profile = requested.clone().unwrap_or_else(|| env_var("PROFILE"));
    match profile.as_str() {
        "debug" => Mode::Debug,
        "release" => Mode::Release,
        other if requested.is_some() => fail(&format!(
            "GOOSE_PROFILE must be `debug` or `release`, not `{other}`"
        )),
        // Cargo only ever reports `debug` or `release` here.
        _ => Mode::Release,
    }
}

/// `goose --standalone --header ... -o ...`: the program plus its runtime as
/// one C file, with no `main`, and a header for the host.
fn generate_c(goose: &Path, root: &Path, c_file: &Path, header: &Path, mode: Mode) {
    let mut cmd = Command::new(goose);
    cmd.arg("--standalone")
        .arg("--header")
        .arg(header)
        .arg("-o")
        .arg(c_file)
        // Fetching packages is `just fetch`'s job, never the build's.
        .args(["--offline", "--color", "never"]);
    if mode == Mode::Debug {
        cmd.arg("--debug");
    }
    // More address space per data stack, for data larger than the 2G default.
    if let Ok(reserve) = env::var("GOOSE_STACK_RESERVE") {
        cmd.args(["--stack-reserve", reserve.trim()]);
    }
    cmd.arg(root);
    let output = cmd
        .output()
        .unwrap_or_else(|e| fail(&format!("could not run {}: {e}", goose.display())));
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        // Each Goose error line becomes a Cargo error, so it is shown first;
        // the full text (with the code and a caret) follows under `--- stderr`.
        let errors: Vec<&str> = stderr.lines().filter(|l| l.contains(": error:")).collect();
        // A message about an option or a missing tool has no `file: error:` form.
        let shown = if errors.is_empty() {
            stderr
                .lines()
                .filter(|l| !l.trim().is_empty())
                .take(3)
                .collect()
        } else {
            errors
        };
        for line in shown {
            println!("cargo::error=goose: {line}");
        }
        eprintln!("{stderr}");
        fail("goose could not compile the Goose code; the full messages are under `--- stderr`");
    }
    for line in stderr.lines().filter(|l| l.contains(": warning:")) {
        println!("cargo::warning=goose: {line}");
    }
}

/// Windows only: point bindgen at the LLVM install when LIBCLANG_PATH is unset,
/// and fail early with a useful message when libclang cannot be found at all.
fn locate_libclang() {
    if !cfg!(windows) {
        return;
    }
    if let Some(dir) = env::var_os("LIBCLANG_PATH") {
        if Path::new(&dir).join("libclang.dll").is_file() {
            return;
        }
        fail(&format!(
            "LIBCLANG_PATH is {}, but libclang.dll is not there",
            PathBuf::from(dir).display()
        ));
    }
    let default_dir = Path::new(LLVM_BIN_WINDOWS);
    if default_dir.join("libclang.dll").is_file() {
        // SAFETY: a build script is single-threaded, so nothing else is reading
        // the environment while it changes; bindgen reads it right after.
        unsafe { env::set_var("LIBCLANG_PATH", default_dir) };
        return;
    }
    if find_on_path_file("libclang.dll").is_none() {
        fail(
            "libclang was not found. Install LLVM (winget install LLVM.LLVM) or set LIBCLANG_PATH.",
        );
    }
}

fn find_on_path_file(name: &str) -> Option<PathBuf> {
    env::split_paths(&env::var_os("PATH")?)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

/// The header -> Rust declarations, written to `out`.
fn generate_bindings(header: &Path, out: &Path, target: &str) {
    locate_libclang();
    let header_path = header
        .to_str()
        .unwrap_or_else(|| fail("the build folder's path is not valid UTF-8"));
    let bindings = bindgen::Builder::default()
        .header(header_path)
        // Only the exports and their types; nothing from the C library headers.
        .allowlist_function("goose_init|goose_exit|rgt_app_.*")
        .rust_edition(bindgen::RustEdition::Edition2024)
        .derive_debug(true)
        .derive_default(true)
        .derive_copy(true)
        .layout_tests(true)
        .merge_extern_blocks(true)
        .clang_arg(format!("--target={target}"))
        .generate()
        .unwrap_or_else(|e| fail(&format!("bindgen could not read the Goose header: {e}")));
    let text = clean_goose_names(&bindings.to_string());
    fs::write(out, text)
        .unwrap_or_else(|e| fail(&format!("could not write {}: {e}", out.display())));
}

/// Removes Goose's C name decorations, token by token:
/// `ns_Type_gN` (where `N` is the length of `ns`) and `Type_g` become `Type`,
/// and a field `name_g` becomes `name` (as `r#name` if it is a Rust keyword).
/// A clash this creates shows up as a duplicate-definition compile error.
fn clean_goose_names(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.char_indices().peekable();
    let mut prev_is_ident = false;
    while let Some((start, c)) = chars.next() {
        let starts_ident = (c.is_ascii_alphabetic() || c == '_') && !prev_is_ident;
        if !starts_ident {
            prev_is_ident = c.is_ascii_alphanumeric() || c == '_';
            out.push(c);
            continue;
        }
        let mut end = start + c.len_utf8();
        while let Some(&(i, next)) = chars.peek() {
            if next.is_ascii_alphanumeric() || next == '_' {
                end = i + next.len_utf8();
                chars.next();
            } else {
                break;
            }
        }
        out.push_str(&clean_identifier(&text[start..end]));
        prev_is_ident = true;
    }
    out
}

fn clean_identifier(ident: &str) -> String {
    // `ns_Type_gN`: a namespaced Goose type, N = the namespace's length.
    if let Some((prefix, digits)) = ident.rsplit_once("_g")
        && !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_digit())
        && let Ok(ns_len) = digits.parse::<usize>()
        && prefix.len() > ns_len + 1
        && prefix.as_bytes()[ns_len] == b'_'
        && prefix[..ns_len]
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        && prefix[ns_len + 1..].starts_with(|ch: char| ch.is_ascii_uppercase())
        && !prefix[ns_len + 1..].contains("_g")
    {
        return prefix[ns_len + 1..].to_owned();
    }
    // `Type_g` or `field_g`: a Goose name without a namespace.
    if let Some(base) = ident.strip_suffix("_g")
        && !base.is_empty()
        && !base.ends_with('_')
        && !base.starts_with("gs_")
    {
        return escape_keyword(base);
    }
    ident.to_owned()
}

/// Rust keywords that Goose allows as names become raw identifiers.
fn escape_keyword(name: &str) -> String {
    const RAW_OK: &[&str] = &[
        "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "do",
        "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl", "in",
        "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref",
        "return", "static", "struct", "trait", "true", "try", "type", "typeof", "unsafe",
        "unsized", "use", "virtual", "where", "while", "yield",
    ];
    // These cannot be raw identifiers, so they keep Goose's suffix.
    const NEVER: &[&str] = &["self", "Self", "super", "crate"];
    if NEVER.contains(&name) {
        format!("{name}_g")
    } else if RAW_OK.contains(&name) {
        format!("r#{name}")
    } else {
        name.to_owned()
    }
}

/// Picks the C compiler for MSVC targets: `GOOSE_C_COMPILER`, then clang-cl
/// on PATH, then clang-cl in the LLVM install folder, then MSVC's `cl`.
fn msvc_compiler() -> Option<OsString> {
    match env::var_os("GOOSE_C_COMPILER") {
        Some(choice) if choice == "cl" => return None,
        Some(choice) if choice != "clang-cl" => return Some(choice),
        _ => {}
    }
    if let Some(path) = find_on_path("clang-cl") {
        return Some(path.into_os_string());
    }
    let installed = Path::new(LLVM_BIN_WINDOWS).join("clang-cl.exe");
    if installed.is_file() {
        return Some(installed.into_os_string());
    }
    println!(
        "cargo::warning=clang-cl was not found, so the Goose code is compiled with MSVC's cl, \
         which was 2-12% slower in tests. Install LLVM (winget install LLVM.LLVM) for full speed."
    );
    None
}

/// Compiles the generated C into a static library and links it.
fn compile_c(c_file: &Path, target: &str, mode: Mode) {
    let mut build = cc::Build::new();
    build
        .file(c_file)
        .warnings(false)
        .force_frame_pointer(false);
    match mode {
        Mode::Release => build.opt_level(2).debug(false),
        Mode::Debug => build.opt_level(0).debug(true),
    };
    if target.contains("msvc")
        && let Some(compiler) = msvc_compiler()
    {
        build.compiler(compiler);
    }
    build.compile(LIB_NAME);
    if target.contains("linux") {
        println!("cargo::rustc-link-lib=m");
        println!("cargo::rustc-link-lib=pthread");
    }
}
