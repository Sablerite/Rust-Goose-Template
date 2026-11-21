# platform/

Packaging files for each operating system: app icons, manifests and
installer scripts. These describe how Rgt App is presented and installed,
not what it does.

**Goes here**

- `windows/`: the app icon (`app.ico`), an application manifest,
  installer scripts
- `android/` and `ios/`: launcher icons, manifests and store files, once
  the project builds for them. Only the Windows build is set up today.

**Doesn't go here**

- images the program shows while running, even its own logo inside a
  window (`assets/`)
- build output or finished installers (`target/`)

`docs/CONVENTIONS.md` shows how to build the Windows icon into the
executable.
