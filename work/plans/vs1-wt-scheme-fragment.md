# VS1 — `wt-scheme-fragment`

## Context

VS1 is the first slice of `onecoat` and the repository has no code yet: this plan creates the crate, the theme model, validation, the bundled theme, the Windows Terminal scheme derivation, and the `list` / `use` commands. `use <id>` renders the used theme's Windows Terminal colour scheme into onecoat's own fragment file; nothing else is written in this slice (`settings.json` splicing is VS2, slot state is VS4). Closes R-1, R-2, R-3, R-5, R-6, R-9, R-10, R-11, R-32, R-41, R-42, N-1…N-6.

## Facts established on this machine (do not re-derive)

- Toolchain: `cargo`/`rustc` 1.98.1 stable, `clippy`, `rustfmt`, `rustdoc` installed; network reachable for crate downloads.
- Windows Terminal 1.24.11911.0 is installed; fragments are read from `%LOCALAPPDATA%\Microsoft\Windows Terminal\Fragments\<subdir>\*.json` (`Fragments\` currently holds only `Microsoft.WSL`). WT's settings file is `%LOCALAPPDATA%\Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe\LocalState\settings.json` (untouched by this slice).
- WT 1.24's JSON schema (`doc/cascadia/profiles.schema.json`, tag `v1.24.11911.0`) is draft 2020-12 and defines a colour scheme with exactly these keys: `name`, `background`, `foreground`, `cursorColor`, `selectionBackground`, `black`, `red`, `green`, `yellow`, `blue`, `purple`, `cyan`, `white`, `brightBlack`, `brightRed`, `brightGreen`, `brightYellow`, `brightBlue`, `brightPurple`, `brightCyan`, `brightWhite`; `additionalProperties: false`. **WT spells the magenta role `purple`/`brightPurple`** — the architecture table's "magenta" is the base16 role name, not a WT key. Scheme colours must match `^#[A-Fa-f0-9]{3}(?:[A-Fa-f0-9]{3})?$` (no alpha).
- The schema's root requires `profiles`, `schemes` and `defaultProfile`, so a bare fragment cannot be validated against the whole document — validation uses a synthetic document (see step 10).
- `std::fs::rename` on Windows replaces an existing destination atomically (`MoveFileExW`, falling back to `SetFileInformationByHandle`), so R-10 needs no Win32 crate.
- omp 18.3.2 is installed; its built-in theme names (custom files lose to them) are the 101 names in step 2, extracted from the bundled `packages/tui/src/theme/{dark,light}.json` and `packages/tui/src/theme/defaults/*.json` registry in `omp.exe` (`glyph-bundle.json` is not a theme and is excluded).
- Repo: git on `main`, clean tree, no `.gitignore`, no `Cargo.toml`, `work/plans/` exists and is empty.

## Approach

Steps are sequential: each ends with a tree that builds and with the tests written so far passing.

### 1. Package skeleton and the error contract

Create `Cargo.toml`:

```toml
[package]
name = "onecoat"
version = "0.1.0"
edition = "2024"
description = "Apply one canonical theme to Windows Terminal, Herdr, and the omp harness"

[dependencies]
clap = { version = "4.6", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
toml = "1.1"

[dev-dependencies]
jsonschema = "0.58"
tempfile = "3"
```

No other dependency is added in this slice: `std` covers the file work, `toml` 1.1 pulls `toml_parser`/`toml_writer` (which later slices reuse through `toml_edit`). Commit `Cargo.lock`.

Create `.cargo/config.toml` so the release binary is statically linked against the CRT (N-2):

```toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

Create `.gitignore` containing `/target`.

Create `src/lib.rs`: crate docs (`//!` paragraph describing the pure render→plan→execute shape), `#![deny(missing_docs)]`, and (as the steps below land) the module declarations `pub mod error; pub mod exec; pub mod model; pub mod plan; pub mod render; pub mod rolemap; pub mod targets; pub mod themes; pub mod validate;` plus the single re-export `pub use error::Error;`.

Create `src/error.rs` with this exact enum; the `#[error(...)]` strings are the R-42 contract, every message names the file, the offending key where there is one, and the action:

```rust
use std::{io, path::PathBuf};
use crate::model::{Appearance, Base16Entry, HexColor, ThemeId};

/// The "accepted keys are ..." tail of [`Error::UnknownKey`].
fn names(list: &[&str]) -> String { list.join(", ") }

/// The "is missing base0A, base0D" tail of [`Error::PaletteIncomplete`].
fn missing_names(missing: &[Base16Entry]) -> String {
    missing.iter().map(Base16Entry::name).collect::<Vec<_>>().join(", ")
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("theme `{id}` not found; run `onecoat list` to see the available themes")]
    ThemeNotFound { id: ThemeId },

    #[error("environment variable `{var}` is not set; onecoat needs it to locate the target paths")]
    EnvMissing { var: &'static str },

    #[error("cannot read `{path}`: {source}; check that the path is readable and not locked by another program")]
    FileUnreadable { path: PathBuf, source: io::Error },

    #[error("cannot write `{path}`: {source}; check that the path is writable and not open in another program")]
    FileWriteFailed { path: PathBuf, source: io::Error },

    #[error("cannot create directory `{path}`: {source}; create it by hand or fix its permissions")]
    DirCreateFailed { path: PathBuf, source: io::Error },

    #[error("`{path}` is not valid theme TOML: {source}; fix the syntax at the reported line and column")]
    ThemeUnparseable { path: PathBuf, source: toml::de::Error },

    #[error("`{path}`: `{key}` is missing; add `{key}` to the theme file")]
    MissingField { path: PathBuf, key: &'static str },

    #[error("`{path}`: `{key}` must be {expected}; found {found}. Fix the value in the theme file")]
    FieldType { path: PathBuf, key: &'static str, expected: &'static str, found: &'static str },

    #[error("`{path}`: appearance = \"{value}\" is not an appearance; use \"dark\" or \"light\"")]
    UnknownAppearance { path: PathBuf, value: String },

    #[error("`{path}`: the theme id is empty; use lowercase ASCII letters, digits and hyphens, starting with a letter or digit")]
    ThemeIdEmpty { path: PathBuf },

    #[error("`{path}`: theme id `{id}` contains `{ch}`; use lowercase ASCII letters, digits and hyphens, starting with a letter or digit")]
    ThemeIdInvalidChar { path: PathBuf, id: String, ch: char },

    #[error("`{path}`: theme id `{id}` collides with the omp built-in theme `{id}`; rename the id")]
    ReservedId { path: PathBuf, id: ThemeId },

    #[error("`{path}`: `{key}` = {value} is not a colour; write it as \"#rrggbb\"")]
    MalformedColor { path: PathBuf, key: String, value: String },

    #[error("`{path}`: `{key}` = {value} is not a colour or a token value; use \"#rrggbb\", an integer from 0 to 255, or a string")]
    OverrideValueInvalid { path: PathBuf, key: String, value: String },

    #[error("[palette] in `{path}` is missing {}; add every base16 entry from base00 to base0F", missing_names(.missing))]
    PaletteIncomplete { path: PathBuf, missing: Vec<Base16Entry> },

    #[error("`{path}`: appearance = \"{declared}\" disagrees with base00 = \"{background}\" (relative luminance {luminance:.3}); set appearance = \"{expected}\"")]
    AppearanceMismatch { path: PathBuf, background: HexColor, declared: Appearance, luminance: f64, expected: Appearance },

    #[error("`{path}`: unknown key `{key}` in {section}; accepted keys are {accepted}")]
    UnknownKey { path: PathBuf, section: &'static str, key: String, accepted: String },

    #[error("`{path}`: theme id `{id}` is already declared by `{other}`; give one of the two files a different id")]
    DuplicateThemeId { path: PathBuf, other: PathBuf, id: ThemeId },

    #[error("cannot encode JSON output: {source}; this is a bug in onecoat, please report it")]
    JsonEncodeFailed { source: serde_json::Error },
}
```

`section` is the literal `"the theme file"` (root), `"[palette]"`, `"[ansi]"`, `"[targets]"`, `"[targets.wt]"`, `"[targets.herdr]"` or `"[targets.omp]"`; `accepted` is built with `names(&[...])` from the key list that section accepts (step 4 lists them), so the message always shows the legal keys.

### 2. Domain model

`src/model.rs` declares `pub mod color; pub mod ids; pub mod palette; pub mod reserved; pub mod theme;` and documents the boundary rule: colours and ids are parsed here, so no renderer ever sees an unparsed value.

`src/model/ids.rs`:

```rust
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize)]
#[serde(transparent)]
pub struct ThemeId(String);                      // private field; Serialize emits the bare string

pub enum IdProblem { Empty, BadChar(char) }      // a private helper for ThemeId::parse
impl ThemeId { pub fn parse(raw: &str) -> Result<Self, IdProblem>; }
impl fmt::Display for ThemeId                     // prints the id

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)] #[serde(rename_all = "lowercase")]
pub enum Slot { Dark, Light }
impl Slot { pub fn name(self) -> &'static str; }  // "dark" / "light"

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)] #[serde(rename_all = "lowercase")]
pub enum Appearance { Dark, Light }
impl From<Appearance> for Slot                    // exhaustive match, no catch-all

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target { Wt, Herdr, Omp }

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)] #[serde(rename_all = "lowercase")]
pub enum Origin { Bundled, User }
```

`ThemeId::parse` accepts `[a-z0-9]` followed by zero or more `[a-z0-9-]`; anything else reports `IdProblem::BadChar(ch)` for the first offending character (uppercase letters included), empty input reports `IdProblem::Empty`.

`src/model/color.rs`:

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct HexColor { r: u8, g: u8, b: u8 }

impl HexColor {
    /// `#` followed by exactly six hex digits, either case; `#rgb` and `#rrggbbaa` are rejected.
    pub fn parse(raw: &str) -> Option<Self>;
    /// WCAG 2.x relative luminance of the linearised sRGB channels, 0.0..=1.0.
    pub fn luminance(self) -> f64;
}
impl fmt::Display for HexColor                   // lowercase "#rrggbb"
impl Serialize for HexColor                      // serialises as the Display string
```

`src/model/palette.rs`:

```rust
pub enum Base16Entry { B00, B01, B02, B03, B04, B05, B06, B07, B08, B09, B0A, B0B, B0C, B0D, B0E, B0F }
impl Base16Entry {
    pub const ALL: [Self; 16];
    pub fn name(self) -> &'static str;                  // "base00" … "base0F"
    pub fn from_name(key: &str) -> Option<Self>;        // case-insensitive ("base0a" == "base0A")
}

pub struct Palette([HexColor; 16]);
impl From<[HexColor; 16]> for Palette;
impl Index<Base16Entry> for Palette { type Output = HexColor; }

pub enum AnsiSlot { Black, Red, Green, Yellow, Blue, Purple, Cyan, White,
                    BrightBlack, BrightRed, BrightGreen, BrightYellow, BrightBlue,
                    BrightPurple, BrightCyan, BrightWhite }
impl AnsiSlot {
    pub const ALL: [Self; 16];
    pub fn name(self) -> &'static str;                  // the WT key: "purple", "brightPurple", …
    pub fn from_name(key: &str) -> Option<Self>;        // exact, case-sensitive
}

pub struct AnsiSet([HexColor; 16]);
impl From<[HexColor; 16]> for AnsiSet;
impl Index<AnsiSlot> for AnsiSet { type Output = HexColor; }
impl IndexMut<AnsiSlot> for AnsiSet;                    // used once, to apply [ansi] overrides
```

Unit tests here: `HexColor::parse` accepts `#0B1018` and `#0b1018` and rejects `#abc`, `#0b1018ff`, `0b1018`, `#gggggg`, `""`; `Display` lowercases; `luminance()` of `#000000` is 0.0, of `#FFFFFF` is 1.0, of `#0B1018` is < 0.01 and of `#ECEFF4` is > 0.8; `Base16Entry::from_name("base0a") == Some(B0A)`, `("base10")`/`("base")` are `None`; `AnsiSlot::from_name("purple") == Some(Purple)`, `("magenta")` and `("Purple")` are `None`; `ThemeId::parse` rejects `"NORD"`, `"nord-å"`, `""`.

`src/model/reserved.rs` holds the omp built-in names, with a doc comment recording the provenance (`omp 18.3.2`, `packages/tui/src/theme/{dark,light}.json` plus `defaults/*.json` in the installed binary) and why they are reserved (omp resolves built-ins before custom files):

```rust
pub const OMP_BUILTIN_THEMES: [&str; 101] = [ /* … see the list below … */ ];
pub fn is_omp_builtin(name: &str) -> bool;      // exact match
```

The 101 names, in the order written into the array:

```
alabaster, amethyst, anthracite, basalt, birch, dark, dark-abyss, dark-arctic, dark-aurora,
dark-catppuccin, dark-cavern, dark-celestial, dark-copper, dark-cosmos, dark-cyberpunk,
dark-dracula, dark-eclipse, dark-ember, dark-equinox, dark-forest, dark-github, dark-gruvbox,
dark-lavender, dark-lunar, dark-midnight, dark-monochrome, dark-monokai, dark-nebula, dark-nord,
dark-ocean, dark-one, dark-poimandres, dark-rainforest, dark-reef, dark-retro, dark-rose-pine,
dark-sakura, dark-slate, dark-solarized, dark-solstice, dark-starfall, dark-sunset, dark-swamp,
dark-synthwave, dark-taiga, dark-terminal, dark-tokyo-night, dark-tundra, dark-twilight,
dark-volcanic, graphite, light, light-arctic, light-aurora-day, light-canyon, light-catppuccin,
light-cirrus, light-coral, light-cyberpunk, light-dawn, light-dunes, light-eucalyptus,
light-forest, light-frost, light-github, light-glacier, light-gruvbox, light-haze,
light-honeycomb, light-lagoon, light-lavender, light-meadow, light-mint, light-monochrome,
light-ocean, light-one, light-opal, light-orchard, light-paper, light-poimandres, light-prism,
light-retro, light-sand, light-savanna, light-solarized, light-soleil, light-sunset,
light-synthwave, light-tokyo-night, light-wetland, light-zenith, limestone, mahogany, marble,
obsidian, onyx, pearl, porcelain, quartz, sandstone, titanium
```

Unit test: `is_omp_builtin("titanium")`, `("dark")`, `("dark-nord")`, `("light-zenith")` are true; `is_omp_builtin("nord")` and `("onecoat-dark")` are false; `OMP_BUILTIN_THEMES.len() == 101`.

### 3. Theme parsing (`Parsed` phase)

`src/model/theme.rs` defines the two phases as distinct types, per the architecture's contract:

```rust
pub trait Phase { type Data; }
pub struct Parsed;    impl Phase for Parsed    { type Data = RawTheme; }
pub struct Validated; impl Phase for Validated { type Data = ThemeData; }

pub struct Theme<P: Phase> {
    pub id: ThemeId,
    pub name: String,
    pub appearance: Appearance,
    pub derived: bool,
    pub origin: Origin,
    pub path: PathBuf,
    pub data: P::Data,
}

/// Exactly what the TOML said, keyed and typed but not yet interpreted.
pub struct RawTheme {
    pub palette: BTreeMap<String, toml::Value>,
    pub ansi: BTreeMap<String, toml::Value>,
    pub targets: BTreeMap<String, BTreeMap<String, toml::Value>>,
}

/// Interpreted values; only renderers read this.
pub struct ThemeData {
    pub palette: Palette,
    pub ansi: AnsiSet,
    pub wt: WtOverrides,
    pub omp: BTreeMap<String, OverrideValue>,
    pub herdr: BTreeMap<String, OverrideValue>,
}

pub struct WtOverrides {
    pub background: Option<HexColor>,
    pub foreground: Option<HexColor>,
    pub cursor_color: Option<HexColor>,
    pub selection_background: Option<HexColor>,
}

/// A `[targets.*]` value: `base16` cannot express these. There is no parser method:
/// `validate` (step 4) builds these three cases directly from the raw `toml::Value`,
/// because the failure message differs per case.
pub enum OverrideValue { Color(HexColor), Index(u8), Text(String) }

impl Theme<Parsed> {
    pub fn parse(path: PathBuf, source: &str, origin: Origin) -> Result<Self, Error>;
}
impl<P: Phase> fmt::Display for Theme<P>   // prints the id, used by diagnostics
```

`parse` is IO-free and reads `source` with `toml::Value::from_str`, matching `toml::Value::Table` (anything else is `ThemeUnparseable` with the parser's own error, but a bare `Table::from_str` failure is also `ThemeUnparseable`, so wrap the parse call once). It then:

- rejects top-level keys other than `id`, `name`, `appearance`, `derived`, `palette`, `ansi`, `targets` with `UnknownKey { section: "the theme file", accepted: names(&["id", "name", "appearance", "derived", "palette", "ansi", "targets"]) }`;
- requires `id` and `name` as strings (`MissingField` / `FieldType` with `expected: "a string"`), builds `ThemeId::parse` and maps `IdProblem` to `ThemeIdEmpty` / `ThemeIdInvalidChar`;
- requires `appearance` as a string and maps the two accepted values to `Appearance`, anything else to `UnknownAppearance { value }`;
- reads `derived` as an optional boolean, defaulting to `false`;
- requires `palette` as a table (`MissingField { key: "palette" }` when absent, `FieldType` with `expected: "a table"` otherwise) and copies it into `RawTheme::palette`;
- reads `ansi` (optional table) and `targets` (optional table of tables) the same way, keyed by the section name so that an unknown target section (`[targets.vscode]`) is reported by `validate` (step 4) with the accepted list.

This split is deliberate: `parse` checks the shape of the document, `validate` checks every value and every key inside the sections.

### 4. Validation (`Parsed` → `Validated`) and role derivation

`src/validate.rs` implements `impl Theme<Parsed> { pub fn validate(self) -> Result<Theme<Validated>, Error> }` (consuming `self`, so the raw maps are not cloned), in this order:

1. **Palette completeness** — collect every key of `RawTheme::palette` through `Base16Entry::from_name`; unknown keys are `UnknownKey { section: "[palette]", accepted: names(&Base16Entry::ALL.map(Base16Entry::name)) }`; missing entries are `PaletteIncomplete { missing }` sorted in `Base16Entry::ALL` order. Fill sixteen `Option<HexColor>` slots while iterating the raw map: when a key maps to a slot that is already filled — possible only when the file writes both `base0a` and `base0A`, which TOML accepts as two distinct keys — report `UnknownKey { section: "[palette]", key }` for that second key, whose message lists the canonical spellings. `BTreeMap` iterates in sorted order, so `base0A` always wins and the reported spelling is always the lowercase one, never a coin flip.
2. **Palette colours** — every entry must be `toml::Value::String` and `HexColor::parse`; otherwise `MalformedColor { key, value }` where `value` is the string's contents for `String` and `value.to_string()` for any other TOML type.
3. **Appearance versus background (R-3)** — `palette[B00].luminance()`; `>= 0.5` is light, `< 0.5` is dark; a mismatch is `AppearanceMismatch { background, declared, luminance, expected }` with `expected` being the appearance the luminance implies.
4. **Reserved id (R-6)** — `is_omp_builtin(self.id)` → `ReservedId { path, id }`.
5. **`[ansi]`** — keys through `AnsiSlot::from_name`, unknown keys are `UnknownKey { section: "[ansi]", accepted: names(&AnsiSlot::ALL.map(AnsiSlot::name)) }`; values must be `String` + `HexColor::parse`, else `MalformedColor` with the key `ansi.<name>`. `src/rolemap.rs` derives the base set from the palette, then the overrides are written into it through `IndexMut`:

   | AnsiSlot | Palette entry | | AnsiSlot | Palette entry |
   | --- | --- | --- | --- | --- |
   | `black` | `base00` | | `brightBlack` | `base03` |
   | `red` | `base08` | | `brightRed` | `base08` |
   | `green` | `base0B` | | `brightGreen` | `base0B` |
   | `yellow` | `base0A` | | `brightYellow` | `base0A` |
   | `blue` | `base0D` | | `brightBlue` | `base0D` |
   | `purple` | `base0E` | | `brightPurple` | `base0E` |
   | `cyan` | `base0C` | | `brightCyan` | `base0C` |
   | `white` | `base06` | | `brightWhite` | `base07` |

   `src/rolemap.rs` holds exactly one function, `pub fn derived_ansi(palette: &Palette) -> AnsiSet`, plus a unit test asserting the table above (e.g. `derived_ansi(&nord)[AnsiSlot::White] == palette[B06]` and `[AnsiSlot::BrightWhite] == palette[B07]`).
6. **`[targets.wt]`** — accepted keys are `background`, `foreground`, `cursorColor`, `selectionBackground`; anything else is `UnknownKey { section: "[targets.wt]", accepted: names(&["background", "foreground", "cursorColor", "selectionBackground"]) }`. Values are `String` + `HexColor::parse` into the four `Option<HexColor>` fields of `WtOverrides`, otherwise `MalformedColor` with the key `targets.wt.<name>`. A key that is valid for `[ansi]` but not for `[targets.wt]` (for example `red`) is therefore rejected with a message that shows the accepted list.
7. **`[targets.omp]`, `[targets.herdr]`** — section names other than these three are `UnknownKey { section: "[targets]", accepted: names(&["wt", "herdr", "omp"]) }`. Keys must match `[A-Za-z_][A-Za-z0-9_]*`; anything else is `UnknownKey { section: "[targets.omp]" }`, whose `accepted` is the literal string ``"a token name such as `mdHeading` or `panel_bg`"`` (the exact token sets arrive with VS5/VS7, so this slice validates shape only). Values are interpreted inline, in this order: a `String` whose first character is `#` must parse as `HexColor` (else `MalformedColor`); any other `String` becomes `Text`; an `Integer` in `0..=255` becomes `Index`; everything else — float, bool, array, table, out-of-range integer — is `OverrideValueInvalid`. The same rule applies to `[targets.herdr]`.

Then it returns `Theme<Validated>` with `ThemeData { palette, ansi, wt, omp, herdr }`.

Fixtures for these rules live in `tests/fixtures/themes/` (step 10) and are exercised by unit tests in `src/validate.rs` that call `Theme::parse(path, include_str!(...), Origin::User)` then `validate()`:

- `incomplete-palette.toml` (all entries but `base0D`) → `PaletteIncomplete` whose message contains the path, `base0D` and `[palette]`;
- `malformed-color.toml` (`base07 = "#xyz"`) → `MalformedColor` whose message contains `base07` and `#xyz`;
- `light-declared-dark.toml` (nord's palette with `appearance = "light"`) and `dark-declared-light.toml` (the same palette with `base00 = "#ECEFF4"`/`base07 = "#0B1018"` and `appearance = "dark"`) → `AppearanceMismatch` both ways; the inverted pair with the correct declarations validates;
- `reserved-id.toml` (`id = "titanium"` otherwise valid) → `ReservedId` whose message contains `titanium` and `rename`;
- `unknown-ansi-key.toml` (`[ansi] magenta = "#ff0000"`) → `UnknownKey` whose message contains `magenta` and `purple`;
- `unknown-root-key.toml` (`foo = 1`) → `UnknownKey` for the theme file;
- `overrides.toml` (valid: `[ansi] red = "#ff0000"`, `[targets.wt] cursorColor = "#D8DEE9"`, `[targets.omp] mdHeading = "#81A1C1"`, `[targets.omp] thinkingOff = 240`, `[targets.herdr] name = "terminal"`) → validates, and the unit test asserts the resolved `ansi[Red]`, `wt.cursor_color`, `omp["mdHeading"] == Text/Color`, `omp["thinkingOff"] == Index(240)`, `herdr["name"] == Text("terminal")`.

### 5. Bundled theme, registry, and shadowing

Create `themes/nord.toml` (the architecture's theme-schema example, verbatim values):

```toml
id = "nord"
name = "Nord"
appearance = "dark"
derived = false

[palette]
base00 = "#0B1018"
base01 = "#151B26"
base02 = "#1D232D"
base03 = "#4C566A"
base04 = "#A0A2A8"
base05 = "#DADBDD"
base06 = "#E5E9F0"
base07 = "#ECEFF4"
base08 = "#BF616A"
base09 = "#D08770"
base0A = "#EBCB8B"
base0B = "#A3BE8C"
base0C = "#8FBCBB"
base0D = "#81A1C1"
base0E = "#B48EAD"
base0F = "#BF616A"
```

`src/themes.rs`:

```rust
/// Bundled themes, compiled in so the binary has no data files (N-2).
pub const BUNDLED: &[(&str, &str)] = &[("nord", include_str!("../themes/nord.toml"))];

pub struct ThemeSet { themes: BTreeMap<ThemeId, Theme<Validated>> }

impl ThemeSet {
    /// Bundled themes first, then `<user_themes_dir>/*.toml` sorted by file name.
    pub fn load(user_themes_dir: &Path) -> Result<Self, Error>;
    pub fn get(&self, id: &str) -> Option<&Theme<Validated>>;
    pub fn iter(&self) -> impl Iterator<Item = &Theme<Validated>>;   // id order
}
```

`load` parses and validates every bundled source with `origin: Origin::Bundled` and `path: PathBuf::from("themes/<name>.toml")`, then reads the user directory: a missing directory is an empty set; only regular files whose extension is `toml` are read (`std::fs::read_dir`, entries collected and sorted by file name for determinism); non-`toml` files and subdirectories are skipped; a file that cannot be read is `FileUnreadable { path, source }`. Keep a second map, `BTreeMap<ThemeId, PathBuf>`, of the user themes seen so far: an id already in *that* map is `DuplicateThemeId { path, other, id }`, while an id that only collides with a *bundled* theme replaces that bundled entry — R-5's shadowing — and the map value then carries `origin: Origin::User`, which `list` reports.

Unit tests: every entry of `BUNDLED` parses and validates, and the declared `id` equals the registry string (R-1's parse test over every bundled theme); `ThemeSet::load` on a directory that does not exist yields exactly the bundled themes; a test builds a temp directory with two copies of the shadow fixture to assert `DuplicateThemeId`.

### 6. Windows Terminal scheme rendering and the plan

`src/render.rs` declares `pub mod wt;` and documents that renderers are pure: they take `&Theme<Validated>` and return data, never paths, files, or processes (N-5).

`src/render/wt.rs` — the scheme, with fields in this exact emission order:

```rust
/// One colour scheme. The field order is the key order in the fragment file.
#[derive(Serialize)]
pub struct WtScheme {
    pub name: String,
    pub background: HexColor,
    pub foreground: HexColor,
    pub cursorColor: HexColor,
    pub selectionBackground: HexColor,
    pub black: HexColor,
    pub red: HexColor,
    pub green: HexColor,
    pub yellow: HexColor,
    pub blue: HexColor,
    pub purple: HexColor,
    pub cyan: HexColor,
    pub white: HexColor,
    pub brightBlack: HexColor,
    pub brightRed: HexColor,
    pub brightGreen: HexColor,
    pub brightYellow: HexColor,
    pub brightBlue: HexColor,
    pub brightPurple: HexColor,
    pub brightCyan: HexColor,
    pub brightWhite: HexColor,
}

#[derive(Serialize)]
pub struct WtFragment { pub schemes: Vec<WtScheme> }

impl WtFragment {
    /// One scheme for the theme's declared slot, named `onecoat-<slot>`.
    pub fn for_theme(theme: &Theme<Validated>) -> Self;
}
```

`for_theme` uses the architecture's role table, all through the typed model: `name` is `format!("onecoat-{}", Slot::from(theme.appearance).name())`; `background = wt.background.unwrap_or(palette[B00])`; `foreground = wt.foreground.unwrap_or(palette[B05])`; `cursorColor = wt.cursor_color.unwrap_or(palette[B0D])`; `selectionBackground = wt.selection_background.unwrap_or(palette[B02])`; the sixteen ANSI fields are assigned one by one from `ansi[AnsiSlot::…]` in the order listed above (write all sixteen assignments out; no loop over a key table, because the struct's field names are the only place the WT spelling is written down). The scheme name is stable across theme switches because it depends on the slot, not the theme.

`src/plan.rs`:

```rust
pub struct PlannedWrite { pub target: Target, pub path: PathBuf, pub bytes: Vec<u8> }
pub struct Plan { pub writes: Vec<PlannedWrite> }

impl Plan {
    /// Serialises the fragment as pretty JSON with a trailing newline.
    pub fn fragment(path: &Path, fragment: &WtFragment) -> Result<Self, Error>;
}
```

`Plan::fragment` uses `serde_json::to_string_pretty` (two-space indent, `\n` line endings) plus one trailing `\n`, mapping the encoder error to `JsonEncodeFailed`. Unit test in `src/render/wt.rs`: `WtFragment::for_theme` on the bundled nord theme produces the expected twenty-one values (name `onecoat-dark`, `background == palette[B00]`, `white == palette[B06]`, `brightWhite == palette[B07]`, `purple == palette[B0E]`); a second test with the `overrides.toml` fixture asserts the `[ansi]` and `[targets.wt]` overrides reach the scheme and that the ANSI fields `[ansi]` did not mention keep their derived values.

### 7. Executor: byte comparison, backup, atomic replace

`src/exec.rs`:

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WriteOutcome { Written, Unchanged }

pub struct WriteReport { pub target: Target, pub path: PathBuf, pub outcome: WriteOutcome }

/// Executes every write of the plan in order; the first failure stops it.
pub fn execute(plan: &Plan) -> Result<Vec<WriteReport>, Error>;
```

`execute` is the only code in this slice that touches the filesystem or spawns nothing at all. Per write, in order:

1. `std::fs::read(path)`: `Ok(existing)`; `Err(e)` with `e.kind() == NotFound` means the file is absent; any other error is `FileUnreadable { path, source }` (this is also what a directory at the path produces on Windows).
2. Absent, or `existing != bytes`, means a write is needed; otherwise return `Unchanged` **without touching the file** (R-9: no rewrite, no mtime change, no new backup).
3. `std::fs::create_dir_all(path.parent())` → `DirCreateFailed { path: parent, source }`.
4. Write `path` with `.onecoat.tmp` appended to the whole file name, as `<file>.onecoat.tmp`: `File::create`, `write_all`, `sync_all`, then drop; any error becomes `FileWriteFailed { path: temp, source }` after a best-effort `remove_file(temp)`.
5. If the file existed, copy it to `<file>.onecoat.bak` with `std::fs::copy` (the copy truncates an existing backup, so the backup always holds the immediately previous content, as R-11 requires); on error remove the temp and return `FileWriteFailed { path: bak, source }`.
6. `std::fs::rename(temp, path)` (atomic replacement on Windows); on error remove the temp and return `FileWriteFailed { path, source }`.
7. Report `Written`.

Sequencing matters: the backup is written before the replace, so a failure at steps 4–6 always leaves the target file byte-identical to what it was.

Unit test in `src/exec.rs` for the ordering property: with a temp directory, a plan whose destination already holds old bytes and whose `.onecoat.bak` path is an existing *directory* fails with `FileWriteFailed` naming the backup path, the destination still holds the old bytes, and no `.onecoat.tmp` is left behind.

### 8. Path resolution

`src/targets.rs` is the platform edge (N-5):

```rust
pub struct Paths {
    pub wt_fragment: PathBuf,   // %LOCALAPPDATA%\Microsoft\Windows Terminal\Fragments\onecoat\schemes.json
    pub user_themes: PathBuf,   // %APPDATA%\onecoat\themes
}
impl Paths { pub fn resolve() -> Result<Self, Error>; }
```

`resolve` reads `LOCALAPPDATA` and `APPDATA` with `std::env::var_os`, returning `EnvMissing { var }` when either is absent, and joins the components with `Path::join`: `("Microsoft", "Windows Terminal", "Fragments", "onecoat", "schemes.json")` and `("onecoat", "themes")`. Both variables are resolved for every command (including `list`), deliberately keeping one path-resolution path. The fields are public so tests can build `Paths` directly from a temporary root.

### 9. CLI: `list`, `use`, exit codes

`src/main.rs` uses the library and clap's derive surface:

```rust
#[derive(Parser)]
#[command(name = "onecoat", version, about = "Apply one canonical theme to Windows Terminal, Herdr, and the omp harness", arg_required_else_help = true)]
struct Cli { #[command(subcommand)] command: Command }

#[derive(Subcommand)]
enum Command {
    /// List the available themes and where each one comes from
    List(ReadArgs),
    /// Apply a theme to the current slot
    Use(UseArgs),
}

#[derive(clap::Args)]
struct ReadArgs { /// Print machine-readable JSON on stdout
                  #[arg(long)] json: bool }

#[derive(clap::Args)]
struct UseArgs { /// Theme id, as printed by `onecoat list`
                 id: String }
```

`fn main() -> ExitCode` calls `Cli::parse()` (clap exits 2 on a usage error and 0 for `--help`/`--version`, matching R-41), then `run(cli)`: `Ok(())` → `ExitCode::SUCCESS`; `Err(error)` → `eprintln!("error: {error}")` and `ExitCode::from(3)`. All command output goes to stdout, all diagnostics to stderr. Exit code 1 is not produced by this slice (it belongs to `verify`).

`list` builds `ThemeSet::load(&paths.user_themes)` and prints, in id order:

- human form (default): a header row and one row per theme, columns `id`, `appearance`, `origin`, `name`, each cell padded with spaces to the widest value in that column (header included), joined with a single space, no trailing spaces; `name` is last and not padded.
  With only the bundled theme the output is exactly:
  ```
  id   appearance origin  name
  nord dark       bundled Nord
  ```
- `--json`: one `serde_json::to_string_pretty` document on stdout (keys `id`, `name`, `appearance`, `origin`, `derived`, from a private `#[derive(Serialize)] struct ListRow`), plus a trailing newline; with only the bundled theme:
  ```json
  [
    {
      "id": "nord",
      "name": "Nord",
      "appearance": "dark",
      "origin": "bundled",
      "derived": false
    }
  ]
  ```

`use <id>` resolves the same theme set, looks the id up with `ThemeSet::get` (`None` → `Error::ThemeNotFound`), builds `WtFragment::for_theme`, `Plan::fragment(&paths.wt_fragment, &fragment)`, executes it, and prints one line per report to stdout: `wrote <path>` or `unchanged <path>` (single space, path as `Path::display()`).

### 10. Tests and fixtures

`tests/fixtures/`:

- `themes/*.toml` — the fixtures listed in step 4, plus `shadow-nord.toml`: a valid theme with `id = "nord"`, `name = "Shadow Nord"`, `appearance = "dark"` and `base00 = "#101820"` while the other fifteen entries copy nord's values.
- `wt/profiles.schema.json` — the vendored WT 1.24.11911.0 schema, downloaded from `https://raw.githubusercontent.com/microsoft/terminal/v1.24.11911.0/doc/cascadia/profiles.schema.json` (109 KB, unmodified); its provenance is recorded in the doc comment of the test that uses it.
- `wt/nord-dark-fragment.json` — the golden fragment for the bundled theme, with `\n` line endings, exactly:
  ```json
  {
    "schemes": [
      {
        "name": "onecoat-dark",
        "background": "#0b1018",
        "foreground": "#dadbdd",
        "cursorColor": "#81a1c1",
        "selectionBackground": "#1d232d",
        "black": "#0b1018",
        "red": "#bf616a",
        "green": "#a3be8c",
        "yellow": "#ebcb8b",
        "blue": "#81a1c1",
        "purple": "#b48ead",
        "cyan": "#8fbcbb",
        "white": "#e5e9f0",
        "brightBlack": "#4c566a",
        "brightRed": "#bf616a",
        "brightGreen": "#a3be8c",
        "brightYellow": "#ebcb8b",
        "brightBlue": "#81a1c1",
        "brightPurple": "#b48ead",
        "brightCyan": "#8fbcbb",
        "brightWhite": "#eceff4"
      }
    ]
  }
  ```
  (plus the trailing newline; `background` and `black` are both `#0b1018` because both roles come from `base00`).
- `wt/previous-fragment.json` — a copy of the golden file with `#0b1018` replaced by `#101010` and `#dadbdd` replaced by `#dddddd`, used as pre-existing content for the backup test.

`tests/cli.rs` (integration, one `#[test]` per behaviour, each spawning `env!("CARGO_BIN_EXE_onecoat")` with `LOCALAPPDATA` and `APPDATA` set to fresh `tempfile::TempDir` roots, so the live files are never touched):

1. `list` prints the exact two lines above, exit 0, empty stderr.
2. `list --json` parses as JSON equal to the document above, exit 0.
3. Shadowing (R-5): with `shadow-nord.toml` copied into `<APPDATA>\onecoat\themes`, `list` prints one row for `nord` with origin `user` (the bundled entry is replaced, not duplicated), and `use nord` in the same root writes a fragment whose `background` is `#101820` — the shadow fixture's palette, not the bundled one.
4. `use nord` creates `<LOCALAPPDATA>\Microsoft\Windows Terminal\Fragments\onecoat\schemes.json` with bytes equal to the golden fixture, prints `wrote <path>`, exit 0; no `.onecoat.bak` exists because the file did not exist before.
5. Idempotency (R-9): in a root where `previous-fragment.json` is already at the fragment path, run `use nord` twice; after the second run stdout is `unchanged <path>`, the fragment's bytes and `modified()` timestamp, and the backup's bytes and timestamp, are all identical to their values after the first run — the second run neither rewrites the file nor refreshes the backup.
6. Backup (R-11): with `previous-fragment.json` copied to the fragment path, `use nord` leaves `<file>.onecoat.bak` byte-identical to the previous content and the fragment byte-identical to the golden fixture, and creates no `.onecoat.tmp`.
7. Failure leaves the target intact (R-10): pre-create the fragment file with `previous-fragment.json`, and `<file>.onecoat.bak` as a *directory*; `use nord` exits 3, stderr names the backup path, the fragment is still byte-identical to the previous content, no `.onecoat.tmp` remains, and stdout is empty.
8. `use nope` exits 3, stdout is empty, and stderr contains `nope` and `onecoat list`.
9. Usage errors: `onecoat` with no arguments exits 2; `onecoat use` exits 2; `onecoat bogus` exits 2; `onecoat --version` exits 0.
10. Invalid user theme: copying `reserved-id.toml` into the themes directory makes both `list` and `use nord` exit 3 with a message naming the fixture path and `titanium`.
11. Writes are confined (N-4): after `use nord` in a temp root, walking the whole root yields exactly the fragment directory, `schemes.json` (plus `<file>.onecoat.bak` when a fragment existed beforehand), and nothing else.

`tests/wt_schema.rs`:

12. The golden fixture validates against the vendored WT schema: parse the schema with `serde_json`, build an instance `{"profiles": [], "schemes": <the fragment's schemes>, "defaultProfile": "onecoat"}` (the schema's root requires those three keys, and `ProfileList` accepts an empty array), and assert `jsonschema::options().offline().build(&schema)?.is_valid(&instance)` — `offline()` keeps the check network-free (N-1).
13. Negative control so the check cannot pass vacuously: the same instance with the scheme's `purple` renamed to `magenta` must be invalid, and one with `background: "#0b1018ff"` must be invalid.

### 11. Documentation touch-ups

- `docs/architecture.md` **Layout**: add the paths this slice introduces that the list does not mention — `src/error.rs` (the error enum and the R-42 message contract), `src/themes.rs` (bundled registry, load and shadowing), `themes/nord.toml` (the bundled theme source), `tests/fixtures/` (vendored WT schema and theme fixtures).
- `docs/architecture.md` **Theme schema**: state next to `[ansi]` that its keys are Windows Terminal's scheme key names, so the magenta role is spelled `purple`/`brightPurple`, and that `[targets.wt]` keys are `background`, `foreground`, `cursorColor`, `selectionBackground` in this slice.
- `docs/requirements.md` **R-3**: append the concrete rule so the check is testable: "Luminance is WCAG relative luminance; `light` requires ≥ 0.5 and `dark` requires < 0.5."
- `work/plans/wt-scheme-fragment.md`: the implementation plan file for this slice, committed with it (repo workflow rule). Land the work through a PR whose body states `VS1` and the requirement IDs it closes (R-1, R-2, R-3, R-5, R-6, R-9, R-10, R-11, R-32, R-41, R-42, N-1…N-6), using conventional commit subjects (`feat:` for the crate, `ci:`/`build:` for `.cargo/config.toml` and `.gitignore`, `docs:` for the doc touch-ups).

## Critical files & anchors

- `src/model/theme.rs` — the `Phase`/`Theme<P>`/`RawTheme`/`ThemeData` split; everything else is typed by it, so get the two phases and `parse`'s shape-vs-value split right.
- `src/validate.rs` — every R-2/R-3/R-6 diagnostics string is produced here; the ordering of the seven checks decides which message a broken file produces.
- `src/render/wt.rs` — the only place WT key spellings and the scheme's field order exist (`purple`, `brightPurple`; the 21-field struct is the byte order of the golden file).
- `src/exec.rs` — comparison → backup → temp → rename ordering; R-9/R-10/R-11 all hinge on it.
- `tests/wt_schema.rs` — the synthetic-document trick (the vendored schema's root requires `profiles`/`schemes`/`defaultProfile`) and the negative control that keep the schema check meaningful.

## Verification

Run from the repository root (`C:\projects\private\onecoat`); all commands are PowerShell unless noted.

1. Gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, `$env:RUSTDOCFLAGS="-D warnings"; cargo doc --no-deps` — all clean on stable 1.98.1.
2. Static build (N-2): `cmd /c "findstr /M /C:VCRUNTIME140 target\release\onecoat.exe"` prints nothing and leaves `$LASTEXITCODE` at 1 — the release binary does not import the dynamic CRT. Supporting check that the flag is what produces it: `rustc --print cfg --target x86_64-pc-windows-msvc -C target-feature=+crt-static | Select-String crt-static` prints `target_feature="crt-static"`.
3. No network capability and no telemetry (N-1, N-6): `cargo tree --edges normal` lists only `clap`, `serde`, `serde_json`, `thiserror`, `toml` and their own dependencies — no HTTP, socket, or analytics crate; `Get-ChildItem -Recurse -File -Filter *.rs src | Select-String -Pattern 'std::net|TcpStream|reqwest|ureq|telemetry'` returns nothing.
4. Rendering purity (N-5): `Get-ChildItem -Recurse -File src\render,src\model | Select-String -Pattern 'std::fs|std::process|std::env'` and `Select-String -Path src\rolemap.rs,src\validate.rs -Pattern 'std::fs|std::process|std::env'` both return nothing — those modules are pure; only `src\exec.rs` and `src\targets.rs` touch IO and the environment.
5. Cold start (N-3), release build: `$sw=[Diagnostics.Stopwatch]::StartNew(); 1..20 | % { .\target\release\onecoat.exe list --json | Out-Null }; $sw.Stop(); $sw.Elapsed.TotalMilliseconds/20` — reports well under 50 ms.
6. Behaviour on a temp root (the integration suite already covers this, but run it by hand once to see the observable output). Run this in a throwaway shell or restore `$env:LOCALAPPDATA`/`$env:APPDATA` afterwards, so step 7 uses the real environment:
   ```powershell
   $t = New-Item -ItemType Directory "$env:TEMP\onecoat-smoke-1"
   $env:LOCALAPPDATA = "$t"; $env:APPDATA = "$t"
   .\target\release\onecoat.exe list                 # prints the two lines from step 9
   .\target\release\onecoat.exe list --json          # prints the JSON document from step 9
   .\target\release\onecoat.exe use nord             # prints: wrote <$t>\Microsoft\Windows Terminal\Fragments\onecoat\schemes.json
   .\target\release\onecoat.exe use nord             # prints: unchanged <same path>
   .\target\release\onecoat.exe use nope; $LASTEXITCODE   # 3, message names nope and points at `onecoat list`
   .\target\release\onecoat.exe bogus; $LASTEXITCODE      # 2
   ```
7. Live smoke run against the real Windows Terminal, with the real environment (no `$env:` overrides), after `cargo build --release`:
   ```powershell
   Get-FileHash "$env:LOCALAPPDATA\Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe\LocalState\settings.json" | Select -Expand Hash   # before
   .\target\release\onecoat.exe use nord
   .\target\release\onecoat.exe use nord          # must print `unchanged …`, so the second run does not touch the file
   Get-FileHash "$env:LOCALAPPDATA\Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe\LocalState\settings.json" | Select -Expand Hash   # after: identical
   ```
   `Get-FileHash` of the fragment, as an on-disk byte check against the committed golden file: `(Get-Content "$env:LOCALAPPDATA\Microsoft\Windows Terminal\Fragments\onecoat\schemes.json" -Raw) -eq (Get-Content tests\fixtures\wt\nord-dark-fragment.json -Raw)` is `True`.
   Then observe the change in the running program: open Windows Terminal (`wt.exe`), press `Ctrl+,` → the profile's **Appearance** page → open the **Color scheme** dropdown and confirm **`onecoat-dark`** is offered; select it and confirm the preview background is `#0b1018` with `#dadbdd` text, i.e. Windows Terminal parsed onecoat's fragment. Leave the fragment in place afterwards (that is the feature working); delete `%LOCALAPPDATA%\Microsoft\Windows Terminal\Fragments\onecoat\` to undo it.
8. Drift/second-run evidence on the live file: `Get-Item "<fragment>" | Select LastWriteTime` is unchanged across the second `use nord`, and `Get-ChildItem` in the fragment directory shows only `schemes.json` (no `.onecoat.tmp`, and no `.onecoat.bak` because the file did not exist before the first run).

## Assumptions & contingencies

- **Luminance rule** (R-3 leaves the function open): WCAG 2.x relative luminance with the threshold at 0.5 — at or above 0.5 the theme must declare `light`, below it `dark`. If the project prefers a different rule, only `HexColor::luminance` and the two `AppearanceMismatch` tests change.
- **Bundle set**: exactly one bundled theme, `nord`, with the palette values from the architecture example; the user's own `~/.omp/agent/themes/nord.json` is unrelated (different namespace) and untouched.
- **Fragment content in this slice**: the fragment carries the scheme for the theme named on the command line only, named `onecoat-<slot>`. The dark/light pair needs the slot assignment that R-15/R-8 close in VS2/VS4; the fragment stays wholly onecoat-owned and is regenerated, not merged.
- **Strict theme loading**: any invalid file in `<APPDATA>\onecoat\themes` fails `list` and `use` with exit 3 instead of being skipped, so a broken theme cannot silently disappear; validation happens at exactly one boundary. If that proves too strict, the change is local to `ThemeSet::load`.
- **If `jsonschema` turns out to be unusable on this toolchain** (step 10, tests 12–13): keep the vendored schema, and replace the two tests with a hand-written comparison of the emitted scheme's key set and colour format against the schema file read as JSON (`$defs.SchemeList.items.properties`), so the check still derives its expectations from WT's file rather than from our renderer.
- **If `+crt-static` breaks the build** for the dev-dependency build scripts: move the flag from `.cargo/config.toml` into an explicit `RUSTFLAGS` documented in the release build command instead of dropping it — N-2 requires the static binary, and the verification step 2 is what proves it.

## As implemented

Everything above landed as written; these are the points where the plan was adjusted while implementing it.

- `.gitattributes` pins `text=auto eol=lf`. This machine has `core.autocrlf=true`, and the byte-exact fragment tests need LF in the working tree as well as in the repository.
- The plan is committed as `work/plans/vs1-wt-scheme-fragment.md` — the ID-prefixed name the roadmap's slice numbering asks for, rather than the bare `wt-scheme-fragment.md` this file's step 11 mentions.
- `use <id>` parses the id into a `ThemeId` at the clap boundary, so `use NORD` is a usage error (exit 2), and `Error::ThemeNotFound` can always carry a valid id. `use nope` still exits 3 as step 10 requires.
- Items whose name is their documentation carry an item-level `#[allow(missing_docs)]` instead of doc comments that restate them: the `Error` variants and fields, `WtScheme`'s fields, and the base16/ANSI variant lists. The crate-level `#![deny(missing_docs)]` and the comments that carry a contract, an invariant, or a rationale stay.
- `WtScheme` is `#[allow(non_snake_case)]`, because its field names are Windows Terminal's own key spellings.
- Additions the snippets implied or needed: `Appearance: Display`, `IdProblem: Display + Error`, `ThemeId: FromStr + Borrow<str> + as_str`, the `HexColor`/`Appearance`/`ThemeId`/`Base16Entry` re-exports in `src/model.rs`, and `Debug` on `ThemeSet`.
- The palette array is assembled with a documented `SLOT_UNSET` placeholder: the plan's `[Option<HexColor>; 16]` slots cannot become `[HexColor; 16]` without an `unwrap`, which the repo bans outside tests. The placeholder is only reachable if the completeness and colour checks are removed, and both run first.
- Extra unit tests beyond the listed fixtures: a duplicate palette spelling, an unknown target section, a Windows Terminal key inside `[ansi]`, an out-of-range index, a malformed token colour, the plan's trailing newline, and the registry's skip/duplicate paths.
- `Get-FileHash` is missing from this machine's PowerShell, so the live smoke compared `settings.json` with a SHA-256 taken through Python; every byte comparison in step 7 is unchanged.
