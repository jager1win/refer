[🇷🇺 Русская версия](README.ru.md)

### Refer - User Reference Constructor

- - -

The main purpose is to provide a fast local tool for anyone who needs not just to store data, but to perform calculations on the fly "in the field" without an internet connection.

- - -

### Key Feature — Operations
Added optionally. Turn your reference book into an engineering calculation tool.

Create a formula with variables that can be:
- `number` type fields from the reference itself,
- manually entered values,
- or both at the same time.

Each Operation is applied to all items in the reference.

Implemented using the [exmex](https://docs.rs/exmex/latest/exmex/index.html) crate — see its documentation for the syntax of variables, operators, and constants.

- - -

### Versions
Android, Linux, Windows — all builds on Github.

### Languages
13 languages. EN and RU — manually, the rest generated with LLM: ES, FR, DE, PT, ZH, JA, KO, IT, NL, TR, AR.

- - -

### Built-in Examples
6 example references with pre-defined operations. Feel free to break and edit them. They can be recreated from the "Create" menu with a single click.

- **Shrinkflation** — compare prices per unit weight/volume
- **Dilution** — calculate solution mixing ratios
- **Ballistics** — ballistic trajectory calculator
- **Deposit** — calculate compound interest growth
- **Geometry** — circle and sphere dimensions, enter the radius
- **Oscillator** — wave value at time t — use the time hint as a reference

- - -

### Storage
Directory `~/Documents/refer`, extension `.refer`. These are actually SQLite databases. Create on one device, transfer to another.

Subfolders are supported — built-in examples are created in `refer/example`.

The application only works with this folder and does not use the internet at all.

- - -

### Import
Create a reference from scratch or import from:

- **CSV, TSV** — UTF-8 encoding is preferred
- **XLS, XLSX, ODT** — the first sheet is imported. Not recommended for very large files: slow import, large resulting size
- **SQLite** — the first table is imported

- - -

### Principles

**Simplicity.** All known alternatives are overloaded with features. Refer works with a specific item; the full list is never displayed — the search returns a maximum of 10 items.

**Minimum restrictions.** You will likely encounter errors — view logs in the "Settings" menu or at the path displayed on the main screen.

**Security.** Encryption is intentionally not added — use your OS tools.

- - -

### ⚠ Warning for Android
The "All files access" permission is required to work with the `Documents/refer` folder.

Enable it manually (depending on your phone):
- Settings → Apps → Refer → Permissions → All permissions
- Special access → All files access → Enable

- - -

[Rust](https://github.com/rust-lang/rust) · [Tauri](https://github.com/tauri-apps/tauri) · [Leptos](https://github.com/leptos-rs/leptos)  
Operations — [Exmex](https://docs.rs/exmex/latest/exmex/index.html)

Open source (GPL-3.0). No ads, no data collection, no cloud services.
