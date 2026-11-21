# data/

Input files for developing and testing Rgt App: sample files to try the
program on, fixtures that tests read, and small datasets.

**Goes here**

- sample inputs you open by hand while developing
- fixtures: small files that a test reads, named after what they test
- small datasets for benchmarks

**Doesn't go here**

- the program's output. While developing, write it under `target/`, which
  git ignores. Once installed, the program writes to the user's data
  folder (on Windows, under `%LOCALAPPDATA%`).
- files the program needs to run (`assets/`, `config/`)
- large or private datasets. Keep those outside the repository and say in
  a README where to get them.

**Suggested subfolders**

```
data/
  samples/    fixtures/
```

A Rust test in `crates/<crate>/tests/` can reach this folder with
`concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/fixtures/<file>")`.
`docs/CONVENTIONS.md` has the details and naming examples.
