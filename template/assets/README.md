# assets/

Files Rgt App loads while it runs. The program only reads them; it never
writes here.

**Goes here**

- images and textures, sounds and music, fonts, shaders, 3D models, UI
  images and icons used inside the program

**Doesn't go here**

- source art: layered or project files such as `.psd`, `.kra`, `.blend`
  or `.aseprite`. Keep them in a separate `assets-src/` folder, or outside
  the repository, and export the finished file into `assets/`.
- settings (`config/`), sample or test data (`data/`), app icons and
  installer files for packaging (`platform/`)

**Suggested subfolders** (create them when you need them)

```
assets/
  textures/   audio/   fonts/   shaders/   models/   ui/
```

Once a subfolder grows past about 50 files, split it by feature
(`textures/player/`, `textures/world/`).

Large binary files make every clone slower. If assets grow past a few
hundred MB, consider Git LFS for them.

How the program finds these files at run time is up to the project. It
can read them from disk beside the executable, build them into the
executable, or read them from an app package on mobile.
`docs/CONVENTIONS.md` compares the three ways and covers file names and
formats.
