# VS2 — `wt-settings-splice`

## Context

VS2 is the second slice of `onecoat`: VS1 landed the theme model, validation, the bundled `nord` theme, the `list` command, and `use <id>` writing onecoat's own Windows Terminal fragment. This slice makes `use` finish the job on Windows Terminal: it splices the three keys a fragment cannot carry into `settings.json` through the JSONC CST, re-reads every file immediately before writing so a concurrent writer is never clobbered, re-parses each written file and restores it from the backup when the result no longer parses, and adds `--dry-run` with the key-level diff plus `--targets` to limit an apply. Slot state (`--slot`, `current`, `state.json`) stays in VS4, profile pins in VS3, and `verify` in VS9. Closes R-12, R-13, R-14, R-15, R-23, R-31.

## Facts established on this machine (do not re-derive)

- Windows Terminal settings live at `%LOCALAPPDATA%\Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe\LocalState\settings.json`. The live file (checked 2026-09-27, 6760 bytes) is UTF-8, LF line endings, four-space indent, **no trailing newline**, and carries `"profiles" > "defaults" > "colorScheme": "onecoat-dark"` (a plain string, set through WT's own UI), `"themes": []` as the last root-ish key, a profile pinned to `"Dainty Nord C1 L0"`, and **no root `theme` key at all**.
- The vendored schema is the authority for key spellings (`tests/fixtures/wt/profiles.schema.json`, WT 1.24.11911.0): `$defs.SchemePair` is `{ "light"?: string, "dark"?: string }`, both defaulting to `"Campbell"` (line 301); `$defs.ThemePair` is `{ "light"?: string, "dark"?: string }` defaulting to the built-in `"light"`/`"dark"` themes (line 2070); `$defs.Theme` is `{ name, tab { background, unfocusedBackground, … }, tabRow { background, unfocusedBackground }, window { applicationTheme, frame, unfocusedFrame, … } }` with `additionalProperties: false` (lines ~2015–2062); a profile's `colorScheme` is `oneOf [SchemePair, string]` (line 2869); root `theme` is `anyOf [string, enum("dark","light","system"), ThemePair]` (line 2599); root `themes` is an array of `Theme` (line 2618). There is **no** `lightColorScheme` key in this schema.
- Fragments accept only `profiles`, `schemes`, and `defaultProfile`, so the window themes and both root keys must go into `settings.json`. They also forbid the theme names `light`, `dark`, and `system`.
- `jsonc-parser` 0.33.2 is the current release and its `cst` feature is the CST the roadmap names: `CstRootNode::parse(text: &str, parse_options: &ParseOptions) -> Result<Self, ParseError>`, `CstRootNode::object_value_or_set() -> CstObject`, `CstObject::{get, object_value_or_set, array_value_or_set, append, insert}`, `CstArray::{elements, append, insert}`, `CstNode::{remove, replace_with}`, `CstObject::ensure_multiline`, and `Display for CstRootNode` via `to_string()`. Values are `CstInputValue::{Null, Bool, Number(String), String(String), Array(Vec<Self>), Object(Vec<(String, Self)>)}` with `From` for strings, numbers, bools, and `Vec`s. The docs state the CST "keeps every comment and every piece of whitespace, so a document can be edited and written back out with everything the author wrote still in place".
- VS1's code surface: `PlannedWrite { target: Target, path: PathBuf, bytes: Vec<u8> }` is byte-fixed at plan time; `Plan::fragment(path: &Path, fragment: &WtFragment) -> Result<Self, Error>`; `exec::execute(plan: &Plan) -> Result<Vec<WriteReport>, Error>` with the private `write_one` doing read → compare → mkdir → temp+sync → backup copy → rename, and **no re-read, re-plan, re-parse, or restore**; `Paths { wt_fragment, user_themes }` with no settings path; `Target { Wt, Herdr, Omp }` with no clap derive; `ThemeId`/`Slot`/`Appearance` and `impl From<Appearance> for Slot` exist; `rolemap::derived_ansi` is the only role derivation today.
- CI's purity gate greps `src/model`, `src/render`, `src/rolemap.rs`, `src/validate.rs` for `std::fs|std::process|std::env`; `src/jsonc.rs` does not exist yet and must be added to that list because it is pure too.
- `Get-FileHash` is absent from this machine's PowerShell, so the live smoke compares hashes through Python (as in VS1).

## Approach

Steps are sequential: each ends with a tree that builds and with the tests written so far passing.

### 1. Dependency, path, and CLI vocabulary

- `Cargo.toml`: add `jsonc-parser = { version = "0.33", features = ["cst"] }`. Justified against N-1 and N-2: pure Rust, no network, no runtime, no unsafe in the crate we call; the `cst` feature is default-off, so name it.
- `src/targets.rs`: add `pub wt_settings: PathBuf` to `Paths` and resolve it as `%LOCALAPPDATA%\Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe\LocalState\settings.json` (joined component by component like `wt_fragment`, so `LOCALAPPDATA` is still the only environment read). Missing file is not an error here; reading it is where that fails.
- `src/model/ids.rs`: derive `clap::ValueEnum` on `Target` (`#[derive(Clone, Copy, PartialEq, Eq, Debug, clap::ValueEnum)]`) so `--targets wt,herdr` parses into the existing enum; `wt`, `herdr`, `omp` are already the kebab-case names the CLI wants.
- `src/error.rs`: add four variants, each with an R-42 message naming file, key, and action:
  - `KeyNotLocatable { path: PathBuf, key: Key }` → "`{path}`: `{key}` is not an object, so onecoat cannot write inside it; fix the value or remove the key".
  - `FileUnparseable { target: Target, path: PathBuf, line: Option<u32> }` → "`{path}` is not valid JSONC ({line}): {message}; fix the syntax and run the command again" — `line` is `None` when the parser reports no position.
  - `ApplyNotVerified { path: PathBuf, source: Box<Error> }` → "`{path}` did not parse after writing, so onecoat restored it from its backup; {source}".
  - `BackupMissing { path: PathBuf }` → "`{path}` failed its post-write check and has no backup to restore from; restore it by hand or delete it and run the command again".
- New `pub struct Key(String)` in `src/error.rs` with `pub fn of(path: &[&str]) -> Self` joining with `.` and `impl fmt::Display`; `Key::of(&["profiles", "defaults", "colorScheme"])` renders `profiles.defaults.colorScheme`.

### 2. `src/jsonc.rs` — the CST splice (pure)

The module parses JSONC, applies a list of edits to the CST, and renders the document back; it never touches the filesystem, so it stays inside the purity gate added in step 10.

```rust
pub enum Edit {
    SetValue { path: &'static [&'static str], value: CstInputValue },
    SetPairSide { path: &'static [&'static str], side: Side, value: CstInputValue, fill_from_string: bool },
    UpsertNamed { path: &'static [&'static str], name: &'static str, value: CstInputValue },
}

pub enum Side { Light, Dark }

pub fn splice(source: &str, edits: &[Edit]) -> Result<String, Error>;
```

- `SetValue` navigates intermediate objects with `object_value_or_set`, but a mismatch on an existing intermediate (`profiles` present and not an object) is `Error::KeyNotLocatable`, never a clobber.
- `SetPairSide` is the pair rule: when the key is absent, create the object with our side only; when it already holds an object, set our side and leave the other side byte-identical; when it holds a **string** and `fill_from_string` is true, write both sides from that string, then overwrite our side.
- `UpsertNamed` walks the array from `array_value_or_set`, finds the element whose `to_serde_value()["name"]` equals `name`, replaces it with `CstNode::replace_with` (falling back to `remove` + `insert` at the same index), and appends when there is no such element. Foreign elements and their whitespace are untouched.
- Parsing uses `CstRootNode::parse(source, &ParseOptions::default())`, so comments, trailing commas, and missing commas keep parsing; a failure returns `Error::FileUnparseable` with the parser's line. The `path` for that error is filled in by the caller, which knows the file.
- The result is `root.to_string()`; the caller compares it with the input, so an edit that changes nothing yields `Unchanged` and no write. `ensure_multiline` is called on containers this module creates, never on containers the file already had.
- Avoid: `serde_json::to_string_pretty` for anything except freshly generated documents (it destroys comments, which is exactly what R-12 forbids); regenerating `settings.json` wholesale; and any byte-offset arithmetic of our own — the CST owns positions.

### 3. WT renderer: the window theme, the scheme pair, and the fragment upsert

`src/render/wt.rs` keeps `WtScheme` (the 21-field scheme struct, private, `#[allow(non_snake_case)]`) and gains the window theme plus the edit lists. Everything here is pure: it returns `Vec<Edit>` and never reads a file.

- `impl WtScheme { pub fn for_theme(theme: &Theme<Validated>) -> Self }` replaces `WtFragment::for_theme`, keeping VS1's role derivation and setting `name` to `onecoat-<slot>` (`Slot::from(theme.appearance).name()`).
- The window theme is a private `#[derive(Serialize)] struct WtWindowTheme { name: String, window: WtWindow, tabRow: WtTabRow, tab: WtTab }` with the architecture's role table exactly: `window.applicationTheme` = the slot name, `window.frame` = base00, `tabRow.background` = base01, `tabRow.unfocusedBackground` = base00, `tab.background` = base01, `tab.unfocusedBackground` = base00. All colors go through the existing `Serialize for HexColor`, so the spelling stays `#rrggbb` lowercase.
- `pub fn wt_edits(theme: &Theme<Validated>) -> Vec<WtEdit>` where `WtEdit` is the slice's own data (`FragmentScheme { slot, name, scheme }`, `ThemeRootPair { slot, name }`, `SchemePair { slot, name }`, `WindowTheme { slot, theme }`), so `src/plan.rs` can turn each into a `jsonc::Edit` without re-deriving anything:
  - fragment: `UpsertNamed { path: &["schemes"], name: "onecoat-<slot>", value: scheme }`,
  - settings: `SetPairSide { path: &["theme"], side, value: "onecoat-<slot>", fill_from_string: false }`,
  - settings: `SetPairSide { path: &["profiles", "defaults", "colorScheme"], side, value: "onecoat-<slot>", fill_from_string: true }`,
  - settings: `UpsertNamed { path: &["themes"], name: "onecoat-<slot>", value: window theme }`.
- `fill_from_string: false` for root `theme` is deliberate: the alternative side then uses WT's built-in `light`/`dark` theme, which is what R-39's OS-following pair means. `true` for `profiles.defaults.colorScheme` is equally deliberate: its missing side would otherwise default to `Campbell`, a foreign scheme, so the previous string carries over instead.
- Only the slot being assigned is ever written: a side whose scheme does not exist in the fragment is left alone, because WT warns about scheme names it cannot resolve.

### 4. `src/plan.rs` — write kinds

- `pub enum WriteKind { Generated(Vec<u8>), Splice(Vec<jsonc::Edit>) }` and `pub struct PlannedWrite { pub target: Target, pub path: PathBuf, pub syntax: Syntax, pub kind: WriteKind }` with `pub enum Syntax { Jsonc }` (herdr adds `Toml` in VS7).
- `Plan::fragment` becomes `pub fn wt_fragment(path: &Path, scheme: &WtScheme) -> Self`, producing a `Splice` write for `schemes[]`; the *generate* path stays for the case where the fragment file does not exist, where a `Generated` document of `serde_json::to_string_pretty` bytes plus a trailing newline reproduces VS1's golden file exactly.
- `pub fn wt_settings(path: &Path, theme: &Theme<Validated>) -> Self` produces the three settings edits in a fixed order: `theme`, `profiles.defaults.colorScheme`, `themes[]`.
- Order matters at the plan level: both writers put the **fragment write first**, so the schemes a pair references exist on disk before the keys that name them.
- `--dry-run` needs no new plan type: the same plan is resolved and printed instead of executed (step 5).

### 5. `src/exec.rs` — re-read, re-plan, verify, restore

- `PlannedWrite.bytes` disappears from the executor's view: a private `fn resolve(planned: &PlannedWrite) -> Result<Resolved, Error>` reads the file fresh, then produces the final bytes — `Generated` bytes are used as they are, `Splice` edits run through `jsonc::splice` against the **bytes just read** (R-13). For a `Splice` write whose file is missing, the caller's target file is missing too: that is `Error::FileUnreadable`, never a created file.
- `struct Resolved { planned: &PlannedWrite, existing: Option<Vec<u8>>, bytes: Vec<u8>, changes: Vec<Change> }` with `pub struct Change { pub key: Key, pub before: Option<serde_json::Value>, pub after: serde_json::Value }`. The `changes` list is what `--dry-run` prints (R-31) and is computed by comparing the parsed before/after documents at the edited paths, so a reformatting is not a change.
- `pub fn execute(plan: &Plan) -> Result<Vec<WriteReport>, Error>` keeps VS1's per-file discipline — unchanged bytes skip the write entirely, otherwise temp + sync, backup copy when the file existed, rename — and then, for every file it actually changed, re-reads and parses it (R-14). A parse failure restores the backup over the target (temp + rename, the same code path) and returns `Error::ApplyNotVerified`; when there is no backup it returns `Error::BackupMissing`. Execution stops at the first failure, as the architecture says.
- `pub fn verify_written(path: &Path, backup: Option<&Path>, syntax: Syntax) -> Result<(), Error>` is the restore path factored out, public so a test can drive it directly: production reaches it only through `execute`, and a test can call it against a deliberately broken file, because the CST cannot produce invalid JSONC from a valid one.
- `pub fn preview(plan: &Plan) -> Result<Vec<PreviewReport>, Error>` resolves every write with the same `resolve` and returns, per file, the path, the outcome (`WriteOutcome::Written`/`Unchanged` as the apply would report it), and the `changes`. It never creates a backup, a temp file, or a directory.
- Output contract, one line per file and one per changed key (stdout, data; diagnostics stay on stderr per R-41):
  - apply: `wrote <path>` or `unchanged <path>` (VS1's lines, now one per file),
  - dry-run: `would write <path>` / `unchanged <path>`, then two spaces and `<key>: <before> -> <after>` per changed key, with `(absent)` for a missing side and compact JSON for values, e.g. `  profiles.defaults.colorScheme: "onecoat-dark" -> {"dark":"onecoat-dark","light":"onecoat-dark"}` and `  themes[onecoat-dark]: (absent) -> {"name":"onecoat-dark",…}`.

### 6. `--dry-run`

- `use <id> --dry-run` resolves and prints the plan; no file, backup, or directory is touched, and the exit code is the one the apply would produce (0, or 3 when the file cannot be read or parsed, so a dry run is a real check).
- The flag is added to `use` only in this slice; `watch --dry-run` is VS11's, when `watch` exists.

### 7. `--targets`

- `use <id> --targets wt,omp` accepts the three names, applies only to those targets that have a writer in this build (only `wt` today), and leaves everything else untouched (R-23).
- A named target with no writer prints `<target>: no writer yet; skipped` on stderr and does not fail the run; an empty intersection exits 0 having written nothing, because "apply to nothing" is a legal answer, not an error.
- An unnamed target is never touched: the default set is still every target that has a writer.

### 8. CLI wiring

- `src/main.rs`: `UseArgs` gains `dry_run: bool` (`--dry-run`) and `targets: Vec<Target>` (`--targets`, comma-separated, `clap::ValueEnum` from step 1, defaulting to all implemented targets).
- `apply()` builds the plan from the theme and the resolved paths, then calls `exec::preview` or `exec::execute` and prints the lines from step 5. Diagnostics keep going to stderr and the exit codes stay 0/2/3 (1 stays reserved for `verify --check`).

### 9. Fixtures and tests

Fixtures (real files, mirroring the live shapes):

- `tests/fixtures/wt/settings.json` — four-space indent, LF, **no trailing newline**, a `//` comment, `profiles.defaults.colorScheme` as the string `"onecoat-dark"`, a profile pinned to another scheme, `themes` holding a foreign entry, and no root `theme` key. Not a copy of the live file (no personal paths or GUIDs).
- `tests/fixtures/wt/settings-spliced.json` — the golden result of applying `nord` to it, so the writer test asserts whole-file bytes as well as the number of changed hunks.
- `tests/fixtures/wt/settings-crlf.json` — CRLF line endings, a trailing newline, comments above and below the owned keys, and an existing `theme` object with only a `light` side, to prove both the EOL preservation and the "leave the other side alone" rule.
- `tests/fixtures/wt/settings-broken.json` — a JSONC syntax error with a known line, for `FileUnparseable`.

Tests in `tests/wt_settings.rs` (pure, over fixture bytes):

- `the_three_owned_keys_change_and_nothing_else_does` — splice the settings fixture for `nord`, assert equality with the golden and that every line outside the edited spans is byte-identical.
- `an_existing_string_scheme_fills_the_light_side` — the pair rule with `fill_from_string`.
- `the_root_theme_key_is_inserted_with_the_files_own_indentation` — absent key case.
- `a_foreign_window_theme_entry_survives_the_upsert` — `themes[]` keeps unknown entries, ours is appended once.
- `crlf_and_a_trailing_newline_survive` — the CRLF fixture.
- `splicing_twice_is_byte_identical` — idempotency at the pure level.
- `a_broken_document_reports_its_line` — `FileUnparseable` with `line`.
- `a_profiles_value_that_is_not_an_object_is_not_clobbered` — `KeyNotLocatable`, and the source text is unchanged.
- `a_replaced_scheme_keeps_its_position_in_the_array` — upsert in place, not append.

Tests in `tests/cli.rs` (extending the existing `Sandbox`):

- `use_writes_the_fragment_and_the_settings_file` — both `wrote` lines, the golden settings bytes, the fragment golden, no `.tmp` left.
- `a_second_use_writes_neither_file` — both `unchanged`, mtimes and sizes unchanged.
- `dry_run_prints_the_key_diff_and_writes_nothing` — the diff lines for the three keys, and no file, backup, or directory created.
- `dry_run_reports_an_unchanged_file` — second run prints `unchanged` and no diff lines.
- `targets_limit_the_apply_to_the_named_target` — `--targets wt` writes both wt files, `--targets omp` writes nothing and says so.
- `an_attack_on_the_result_is_rolled_back` — drives `exec::verify_written` with a corrupted file and a backup, asserting the restore is byte-identical and `ApplyNotVerified` names the path (R-14; the CST cannot emit invalid JSONC from valid input, so this is the only honest way to reach the restore path).
- `the_settings_path_is_the_packaged_local_state_file` — the write lands under `<root>/Packages/Microsoft.WindowsTerminal_8wekyb3d8bbwe/LocalState/settings.json`.

Test in `tests/wt_schema.rs`:

- `the_spliced_settings_file_satisfies_the_windows_terminal_schema` — validate the golden against the vendored schema (not the synthetic fragment document).

### 10. Docs and the CI purity list

- `.github/workflows/ci.yml`: add `src/jsonc.rs` to the N-5 grep paths, so the new pure module is held to the same rule as the renderers.
- `docs/requirements.md` R-15: append the pair rule the code implements — the pairs carry only the slots that have a theme, and a previous string value carries into the unnamed side of `profiles.defaults.colorScheme`.
- `docs/architecture.md` **Writers**: replace "wt fragment: fully generated JSON, so a plain serializer is safe" with the merge rule (the fragment is a two-slot store, upserted by scheme name), and state that the JSONC CST performs the byte-range splice for both `settings.json` and an existing fragment.
- No changelog file exists yet; the user-visible change is carried by R-12/R-13/R-14/R-15/R-23/R-31 and the architecture text.

## Critical files & anchors

- `src/jsonc.rs` (new) — the CST splice; every R-12 guarantee lives here, so the edit kinds and the "never clobber an intermediate" rule are the load-bearing part.
- `src/exec.rs` — `resolve`/`execute`/`preview`/`verify_written`; R-13, R-14, and R-31 all hinge on the resolve-then-apply split, and it is the only module that may touch the filesystem.
- `src/plan.rs` — `WriteKind::Generated` vs `Splice`; the fragment-first order and the identity of the settings edits are decided here.
- `src/render/wt.rs` — the window theme's exact keys and the two `fill_from_string` choices; a wrong key here is a schema failure, a wrong flag is a behaviour change in the other appearance.
- `tests/cli.rs` — the `Sandbox` helper the end-to-end tests extend; the write-location test also pins the packaged settings path.

## Verification

Run from the repository root (`C:\projects\private\onecoat`); all commands are PowerShell unless noted.

1. Gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, `$env:RUSTDOCFLAGS="-D warnings"; cargo doc --no-deps` — all clean on stable 1.98.1.
2. New behaviour inside the suite: `cargo test --test wt_settings`, `cargo test --test cli`, `cargo test --test wt_schema` — the splice, the CLI surfaces, and the schema check pass.
3. Purity (N-5) with the new module: `Get-ChildItem -Recurse -File src\jsonc.rs, src\model, src\render, src\rolemap.rs, src\validate.rs | Select-String -Pattern 'std::fs|std::process|std::env'` returns nothing.
4. Hand run on a temp root, seeing the observable output:
   ```powershell
   $t = New-Item -ItemType Directory "$env:TEMP\onecoat-smoke-2"
   New-Item -ItemType Directory -Force "$t\Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe\LocalState" | Out-Null
   Copy-Item tests\fixtures\wt\settings.json "$t\Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe\LocalState\settings.json"
   $env:LOCALAPPDATA = "$t"; $env:APPDATA = "$t"
   cargo build --release
   .\target\release\onecoat.exe use nord --dry-run      # prints `would write …schemes.json`, `would write …settings.json`, then the three key diffs
   (Get-Item "$t\Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe\LocalState\settings.json").LastWriteTime   # unchanged by the dry run
   .\target\release\onecoat.exe use nord               # prints `wrote …schemes.json`, `wrote …settings.json`
   (Get-Content "$t\Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe\LocalState\settings.json" -Raw) -eq (Get-Content tests\fixtures\wt\settings-spliced.json -Raw)   # True
   .\target\release\onecoat.exe use nord               # prints `unchanged …` twice
   .\target\release\onecoat.exe use nord --targets omp # prints `omp: no writer yet; skipped`, writes nothing
   ```
5. Live smoke against the real Windows Terminal, with the real environment (no `$env:` overrides). Save a byte copy of the file before touching it, because the live file has no backup yet:
   ```powershell
   $s = "$env:LOCALAPPDATA\Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe\LocalState\settings.json"
   python -c "import hashlib,sys;print(hashlib.sha256(open(sys.argv[1],'rb').read()).hexdigest())" "$s"   # before
   Copy-Item $s "$env:TEMP\settings.before.json"
   .\target\release\onecoat.exe use nord --dry-run    # the diff names exactly the three keys
   .\target\release\onecoat.exe use nord
   python -c "import hashlib,sys;print(hashlib.sha256(open(sys.argv[1],'rb').read()).hexdigest())" "$s.onecoat.bak"   # equals the `before` hash
   ```
   Then observe the change in the running program: open Windows Terminal (`wt.exe`), press `Ctrl+,`; on the **Appearance** page the Theme dropdown offers `onecoat-dark`, and the window's tab row and frame carry the theme's `#0b1018`/`#151b26` instead of the default chrome, which proves WT parsed both the fragment and the spliced `settings.json`.
6. Idempotency and drift evidence on the live file: a second `onecoat use nord` prints `unchanged` for both files, the settings file's `LastWriteTime` does not move, and `Get-ChildItem` in the fragment directory shows `schemes.json`, `schemes.json.onecoat.bak`, and no `.tmp`.
7. Restore the live file: `Copy-Item "$env:TEMP\settings.before.json" $s -Force`, then re-run the before-hash command and get the same hash. The fragment stays in place (that is the feature working); delete `%LOCALAPPDATA%\Microsoft\Windows Terminal\Fragments\onecoat\` to undo it.

## Assumptions & contingencies

- **Pair semantics**: only the assigned slot's side is written; `profiles.defaults.colorScheme` also carries a previous string value into the other side (`fill_from_string: true`), root `theme` does not (`false`). If the project wants the built-in theme preserved for root `theme` instead, only the flag in step 3 and its test change.
- **The fragment becomes a two-slot store**: this amends VS1's "regenerated, not merged" note. If the existing fragment does not parse, it is regenerated from the assigned slot alone and the stale file is reported on stderr; no error, because the file is wholly onecoat-owned.
- **Missing `settings.json` fails the apply** with `FileUnreadable` rather than creating a file: a settings document requires `profiles`, `schemes`, and `defaultProfile`, and inventing them is not this slice's job.
- **`--targets` naming a target with no writer** is reported and skipped, not an error; that keeps the flag honest while herdr and omp are still to come.
- **UTF-8 without BOM** is assumed for both files (the live file is); a BOM would be treated as document text by the CST. If a fixture ever shows otherwise, the read in step 5 grows a strip-and-restore around the splice.
- **Non-UTF-8 bytes** map to `Error::FileUnreadable` with an `InvalidData` source whose message says so, so no new variant is needed and R-42 still names the file and the action.
- **If the CST turns out not to preserve a span it did not touch** (the docs claim it does, and the golden tests prove it for our fixtures): fall back to running the CST twice — once to compute the changed byte ranges, once to splice those ranges into the original bytes — which keeps every guarantee of R-12 without hand-written JSON parsing.

## As implemented

Everything above landed as written; these are the points where the plan was adjusted while implementing it.
