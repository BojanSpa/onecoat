# VS2 — `wt-settings-splice`

## Context

`use <id>` must leave Windows Terminal fully themed.<br>
A fragment cannot carry window themes or the root keys.<br>
This slice splices `theme`, `themes[]`, and `profiles.defaults.colorScheme` into `settings.json`.<br>
The splice goes through the JSONC CST, and `--dry-run` shows it without writing.<br>
Slot state stays in VS4, profile pins in VS3, and `verify` in VS9.<br>
Closes R-12, R-13, R-14, R-15, R-23, R-31.<br>

## Facts established on this machine (do not re-derive)

- The live `%LOCALAPPDATA%\Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe\LocalState\settings.json` is LF and four-space indented.<br>It has **no trailing newline** and **no root `theme` key**.<br>It keeps `profiles.defaults.colorScheme` as a bare string and `themes: []`.<br>So this slice must insert keys as well as replace them.<br>
- Key names and shapes come from the vendored schema in `tests/fixtures/wt/`.<br>A pair is `{light?, dark?}`.<br>A profile's `colorScheme` takes a pair or a string.<br>A root `theme` takes a string, a built-in name, or a pair.<br>WT 1.24 has no `lightColorScheme`.<br>
- `jsonc-parser` 0.33 with the `cst` feature keeps every untouched byte.<br>`Get-FileHash` is absent here, so the live smoke hashes with Python.<br>

## Approach

1. `Paths` gains `wt_settings`, and `Target` derives `clap::ValueEnum`.<br>`jsonc-parser` joins `Cargo.toml`.<br>`Key` holds dotted names like `profiles.defaults.colorScheme`.<br>Four errors land in `src/error.rs`.<br>They cover a non-object intermediate, a bad document, a failed re-parse, and a missing backup.<br>
2. A new pure `src/jsonc.rs` exposes `splice(source: &str, edits: &[Edit]) -> Result<String, Error>`.<br>There are three edit kinds.<br>They set a value, set one side of a pair, or upsert an array element.<br>The pair edit takes a fill-from-string flag.<br>Missing intermediates are created.<br>An existing key of the wrong type is an error, never a clobber.<br>The purity check over `src/` covers it with no list to update.<br>
3. `src/render/wt.rs` adds the window theme keys.<br>The role table in `docs/architecture.md` is the whole spec.<br>It returns edit data instead of a fragment document.<br>Scheme names stay `onecoat-<slot>`.<br>
4. `src/plan.rs` splits writes into `Generated` bytes and `Splice(Vec<Edit>)`.<br>The fragment comes first.<br>The pair keys name schemes that the fragment must already carry.<br>
5. `src/exec.rs` grows `resolve`, which reads fresh, computes the bytes, and records the changed keys.<br>`execute` writes, then re-parses every changed file.<br>It restores the backup when the re-parse fails.<br>`preview` prints the same resolution and touches nothing.<br>
6. `use` learns `--dry-run` and `--targets`.<br>It prints one line per file: `wrote`, `unchanged`, or `would write`.<br>It prints one line per changed key.<br>
7. Fixtures: a commented settings file and its spliced golden.<br>Also a CRLF variant with `theme.light` already set, and a broken file.<br>Tests are named for the behaviour they prove.<br>The three-key splice changes nothing else.<br>A foreign `themes` entry survives.<br>A second splice is byte-identical.<br>Dry-run writes nothing.<br>`--targets` limits the apply.<br>A corrupted result is restored.<br>The spliced golden validates against the vendored schema.<br>

## Critical files & anchors

- `src/jsonc.rs` holds the CST splice.<br>R-12 lives or dies here.<br>
- `src/exec.rs` holds `resolve`, `execute`, `preview`, and `verify`.<br>R-13, R-14, and R-31 are the split between them.<br>
- `src/render/wt.rs` holds the window-theme keys and the pair flags.<br>A wrong key fails the schema.<br>A wrong flag changes the other appearance.<br>
- `tests/cli.rs` holds the `Sandbox` helper.<br>The end-to-end tests extend it.<br>

## Verification

1. Gates, all clean:<br>`cargo fmt --check`<br>`cargo clippy --all-targets -- -D warnings`<br>`cargo test --all-targets`<br>`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`<br>
2. Focused tests pass: `cargo test --test wt_settings --test cli --test wt_schema`.<br>
3. The purity check in `cargo test --workspace` finds nothing in `src/jsonc.rs`.<br>
4. In a temp root, copy the fixture into a sandbox `LocalState`.<br>`use nord --dry-run` prints the diff and leaves mtimes alone.<br>`use nord` writes both files.<br>Then `settings.json` is byte-equal to the golden.<br>`use nord` again says `unchanged` twice.<br>`use nord --targets omp` skips and writes nothing.<br>
5. Live smoke: copy the real `settings.json` aside and hash it.<br>Run `use nord`.<br>`settings.json.onecoat.bak` hashes the same.<br>WT's Appearance page offers `onecoat-dark`.<br>Its tab row shows `#0b1018` on `#151b26`.<br>Then restore the copy and re-hash.<br>
6. Second run of step 4: output unchanged, files byte-identical.<br>

## Assumptions & contingencies

- **Only the assigned side of a pair is written.**<br>A missing root `theme` side falls back to a built-in theme.<br>That is what OS-following means.<br>A missing `colorScheme` side would fall back to `Campbell`.<br>So there the previous string carries into the other side.<br>Flip the flag if you want the built-in preserved for `theme` too.<br>
- **The fragment becomes a two-slot store.**<br>`use` upserts `onecoat-<slot>` and leaves the other slot's scheme alone.<br>This amends VS1's "regenerated, not merged" rule.<br>Without it, a light theme would drop the dark scheme the pair still names.<br>
- **A missing `settings.json` fails the apply.**<br>Onecoat does not invent a document that needs `profiles`, `schemes`, and `defaultProfile`.<br>
- **`--targets` naming a target with no writer reports it and exits 0.**<br>An empty intersection writes nothing and is not an error.<br>
- **R-14 is proven through a public `exec::verify_written`, driven by a test.**<br>The CST cannot turn valid JSONC into invalid JSONC.<br>
- **JSONC only in this slice**: herdr's TOML arrives with VS7.<br>

## As implemented

Everything above landed as written.<br>These are the points where the plan was adjusted while implementing it.<br>

- `Plan::wt` builds both Windows Terminal writes and tags each with an `Absent` policy.<br>The fragment starts from `{}`; a missing `settings.json` fails the apply.<br>
- `PlannedWrite` carries edits, not bytes.<br>`exec::resolve` reads the file and computes the text, which is what makes R-13 true for a plan built once.<br>
- `--targets` prints `no writer for <target>` only when the flag was given.<br>Without it every target is planned, so nothing is reported.<br>
- The fragment's scheme is now serialized by the CST, which writes keys alphabetically.<br>VS1's serializer wrote `name` first, so `nord-dark-fragment.json` is regenerated.<br>
- Fixtures grew to five: the commented file, its spliced golden, a CRLF pair, and a broken document.<br>The CRLF golden is not in the plan, and it is what proves the line-ending rule.<br>
- The architecture rule about counting changed hunks became an owned-key list beside the byte-exact golden.<br>The CST reports keys, not hunks, and the key list fails the same way.<br>
- The architecture error sketch lists the variants that exist now.<br>The purity check covers `src/jsonc.rs` with no list to maintain.<br>
- `tests/cli.rs` was rewritten around the settings fixture.<br>Its confinement test now covers both files, and the dry run asserts mtimes.<br>
- The live smoke ran against the real files with WT already running.<br>The result was byte-equal to the reviewed temp-root document, and both files were restored by hand.<br>
