[🇷🇺 Русская версия](README.ru.md)

### Refer - User Reference Constructor
- - -
The main purpose is to provide a fast local tool for anyone who needs not just to store data, but to perform calculations on the fly "in the field" without an internet connection.

The key feature of the application is ***Operations*** (added optionally), which can turn your reference book into an engineering calculation tool.  
You create a formula with variables that can be either "number" type fields from the reference itself or manually entered values.  
Each Operation is applied to all items in the reference.  
Implemented using the exmex crate (see link below for its syntax on variables, operators, and constants).

On Github, you can find versions of the application for Android, Linux, and Windows.

Multilingual, 13 languages (except for the first two, translation files were generated with the help of an LLM): EN, RU, ES, FR, DE, PT, ZH, JA, KO, IT, NL, TR, AR.

The application includes 6 built-in example reference books with pre-defined operations:
- **Shrinkflation** - Compare prices per unit weight/volume
- **Dilution** - Calculate solution mixing ratios
- **Ballistics** - Ballistic trajectory calculator for rifle calibers
- **Deposit** - Calculate compound interest growth
- **Geometry** - Circle and sphere dimensions — enter the radius
- **Oscillator** - Wave value at time t — use the time hint as a reference
You can freely experiment with these — they can be recreated from the "Create" menu with a single click.

On all operating systems, reference files are stored in the `~/Documents/refer` directory and have the `*.refer` extension. They are essentially SQLite databases with a specific structure. This means you can create a reference on one device and then transfer the ready-made file to another device. Subfolders are supported — for example, the built-in examples are created in the `refer/example` folder. The application on all operating systems only works with this folder and its subfolders and does not use the internet at all.

You can create a reference completely from scratch or import from common formats:
- CSV, TSV. UTF-8 encoding is preferred.
- Spreadsheets (XLS, XLSX, ODT). The first sheet is imported. Not recommended for large files: the process may be slow, and the resulting reference size may be large.
- Other SQLite databases. The first table is imported.

One of the goals in creating the application was to impose minimum restrictions on the user. Therefore, you will likely encounter errors — you can view them in the "Settings" menu or at the log file path displayed on the main screen.

Security: I intentionally did not add encryption, which some might find useful, but this can be handled using the tools of any operating system.

⚠ Warning for Android:
- - -
To work with the `Documents/refer` folder, this application requires the "All files access" permission.
Enable it manually (depending on your phone):
- Settings → Apps → Refer → Permissions → All permissions
- Special access → All files access → Enable

Created with [Rust](https://github.com/rust-lang/rust), [Tauri](https://github.com/tauri-apps/tauri), [Leptos](https://github.com/leptos-rs/leptos).   Operations — [Exmex](https://github.com/bertiqwerty/exmex)  
Open source (GPL-3.0). No ads, no data collection, no cloud services.