# VS2 — `wt-settings-splice`

## Context

`use <id>` must leave Windows Terminal fully themed, but a fragment cannot carry window themes or the root keys.<br>
This slice splices `theme`, `themes[]`, and `profiles.defaults.colorScheme` into `settings.json` through the JSONC CST, adds `--dry-run` with the key-level diff and `--targets`, and makes the executor re-read each file before writing and re-parse it after, restoring from the backup when the result is broken.<br>
Slot state stays in VS4, profile pins in VS3, `verify` in VS9.<br>
Closes R-12, R-13, R-14, R-15, R-23, R-31.<br>

## Facts established on this machine (do not re-derive)

- The live `%LOCALAPPDATA%\Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe\LocalState\settings.json` is LF, four-space indented, has **no trailing newline**, keeps `profiles.defaults.colorScheme` as a bare string, `themes: []`, and **no root `theme` key**, so this slice must insert as well as replace.<br>
- Key names and shapes come from the vendored schema in `tests/fixtures/wt/`: a pair is `{light?, dark?}`, a profile's `colorScheme` takes a pair or a string, a root `theme` takes a string, a built-in name, or a pair, and WT 1.24 has no `lightColorScheme`.<br>
- `jsonc-parser` 0.33 with the `cst` feature keeps every comment and whitespace byte it does not touch, and `Get-FileHash` is absent here, so the live smoke hashes with Python.<br>

## Approach

1. `Paths` gains `wt_settings`; `Target` derives `clap::ValueEnum`; `jsonc-parser` joins `Cargo.toml`; `Key` (dotted names like `profiles.defaults.colorScheme`) and four errors land in `src/error.rs`: a non-object intermediate, a document that will not parse, a failed post-write check, and a missing backup.<br>
2. A new pure `src/jsonc.rs` exposes `splice(source: &str, edits: &[Edit]) -> Result<String, Error>` over three edit kinds — set a value at a path, set one side of a pair (with a fill-from-string flag), upsert an array element by `name`.<br>Missing intermediates are created; one holding the wrong type is an error, never a clobber.<br>The module joins the CI purity grep.<br>
3. `src/render/wt.rs` adds the window theme — the role table in `docs/architecture.md` is the whole spec — and returns edit data instead of a fragment document; scheme names stay `onecoat-<slot>`.<br>
4. `src/plan.rs` splits writes into `Generated` bytes and `Splice(Vec<Edit>)`, and orders the fragment first, because the pair keys name schemes the fragment must already carry.<br>
5. `src/exec.rs` grows `resolve` (read fresh, compute the bytes, record the changed keys).<br>`execute` writes and then re-parses every changed file, restoring the backup on failure; `preview` prints the same resolution and touches nothing.<br>
6. `use` learns `--dry-run` and `--targets`, and prints one line per file (`wrote`, `unchanged`, `would write`) plus one line per changed key.<br>
7. Fixtures: a commented settings file, its spliced golden, a CRLF variant that already has a `theme.light`, and a broken one.<br>Tests are named for the behaviours they prove: the three-key splice changes nothing else, a foreign `themes` entry survives, a second splice is byte-identical, dry-run writes nothing, `--targets` limits the apply, a corrupted result is restored, the spliced golden validates against the vendored schema.<br>

## Critical files & anchors

- `src/jsonc.rs` — the CST splice; R-12 lives or dies here.<br>
- `src/exec.rs` — resolve/execute/preview/verify; R-13, R-14, and R-31 are that split.<br>
- `src/render/wt.rs` — window-theme keys and the pair flags; a wrong key fails the schema, a wrong flag changes the other appearance.<br>
- `tests/cli.rs` — the `Sandbox` helper the end-to-end tests extend.<br>

## Verification

- The gates in `AGENTS.md`, `cargo test --test wt_settings --test cli --test wt_schema`, and the N-5 grep over `src/jsonc.rs`.<br>
- Temp root: copy the fixture into a sandbox `LocalState`, then `use nord --dry-run` (diff printed, mtime unchanged), `use nord` (both files written, settings byte-equal to the golden), `use nord` again (`unchanged` twice), `use nord --targets omp` (skipped, nothing written).<br>
- Live smoke: copy the real `settings.json` aside and hash it, `use nord`, confirm `settings.json.onecoat.bak` hashes to the same value, watch WT's Appearance page offer `onecoat-dark` and colour its tab row `#0b1018`/`#151b26`, then restore the copy and re-hash.<br>

## Assumptions & contingencies

- **Only the assigned side of a pair is written.**<br>A missing root `theme` side falls back to WT's built-in theme, which is what OS-following means; a missing `colorScheme` side would fall back to `Campbell`, so there the previous string carries into the other side.<br>Flip the flag if you want the built-in preserved for `theme` too.<br>
- **The fragment becomes a two-slot store**: `use` upserts `onecoat-<slot>` and leaves the other slot's scheme alone.<br>This amends VS1's "regenerated, not merged" — without it, applying a light theme would drop the dark scheme the pair still names.<br>
- **A missing `settings.json` fails the apply**; onecoat does not invent a document that needs `profiles`, `schemes`, and `defaultProfile`.<br>
- **`--targets` naming a target with no writer** reports it and exits 0; an empty intersection writes nothing and is not an error.<br>
- **R-14 is proven through a public `exec::verify_written`** driven by a test, because the CST cannot turn valid JSONC into invalid JSONC.<br>
- **JSONC only in this slice**: herdr's TOML arrives with VS7.<br>

## As implemented

Everything above landed as written; these are the points where the plan was adjusted while implementing it.<br>

- ADJUSTMENT — why.<br>
