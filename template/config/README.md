# config/

Default settings files that ship with Rgt App, such as `default.toml`.
The program reads them at start-up, and they show every setting it
understands with a sensible value.

**Goes here**

- `default.toml`: every setting with its default value and a comment
- an override file per environment when one is needed, such as `dev.toml`

**Doesn't go here**

- a user's own settings. Those belong in the operating system's per-user
  settings folder (on Windows, under `%APPDATA%`), so updating the program
  never overwrites them.
- secrets, passwords or API keys. Read those from environment variables
  instead, and never commit them.
- files the program loads as content, such as level files or images
  (`assets/`)

`docs/CONVENTIONS.md` shows how to read these files and how to name the
settings inside them.
