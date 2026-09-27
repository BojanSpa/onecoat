# VS4 — `wt-slot-state`

## Context

`use <id>` applies the theme it names and then remembers nothing.<br>
So no command can say which theme owns `dark` or `light`.<br>
This slice records the assignment in onecoat's own state and adds `current`.<br>
`--slot` lets a theme take the other slot, which R-8 allows.<br>
The expected values that `verify` compares against arrive with VS9.<br>
Herdr and omp writers arrive in VS5 to VS7, so `wt` is the only target applied.<br>
Closes R-8, R-25.<br>

## Facts established on this machine (do not re-derive)

- `%APPDATA%\onecoat` does not exist yet, so the state write must create it.<br>`src/exec.rs` already creates missing parent directories before it writes.<br>
- The fragment directory holds `schemes.json` and one `.onecoat.bak` from the VS1 to VS3 smoke runs.<br>
- `Paths::resolve` reads `LOCALAPPDATA` and `APPDATA` once per invocation.<br>`user_themes` points at `%APPDATA%\onecoat\themes`, which is the state file's directory.<br>
- The tidy purity check allows IO only in `src/exec.rs`, `src/main.rs`, `src/targets.rs` and `src/themes.rs`.<br>So `src/state.rs` parses and renders text, and never touches a file.<br>
- The CLI sandbox in `tests/cli.rs` points both environment variables at a temp directory.<br>State written by a test therefore never reaches the live file.<br>

## Approach

1. Add `src/state.rs`: the slot assignment, the targets last applied, a parse and a render.<br>The document is JSON with the keys `slots.dark`, `slots.light` and `targets`.<br>A malformed document is an error that names the file and calls the state disposable.<br>
2. `Paths` gains the state path beside `user_themes`, so the environment read stays in the edge.<br>
3. `use` gains `--slot`, default the theme's declared appearance.<br>The chosen slot decides the pair side and the `onecoat-<slot>` names.<br>The colors still come from the theme.<br>
4. `exec` writes the state after every planned write succeeded, and only when the text changed.<br>A failed state write fails the apply with exit 3, leaving the target files as written.<br>
5. Add `current`: one row per slot from the state, plus the targets last applied.<br>`--json` prints the same fields.<br>A missing state prints `none` and exits 0, while a malformed one exits 3.<br>
6. Update the state paragraph in `docs/architecture.md`.<br>Give R-8 and R-25 their `Verified by:` clauses in `docs/requirements.md`.<br>
7. Tests: a unit round trip plus a malformed document.<br>CLI tests cover `--slot`, `current`, `--json`, a missing state, and `--dry-run` leaving no state behind.<br>

## Critical files & anchors

- `src/state.rs` — new pure module: the assignment record, its parse and its render.<br>
- `src/targets.rs` — decides the state path and keeps the environment read in the edge.<br>
- `src/exec.rs` — decides when the state is written and what a failed write does.<br>
- `src/main.rs` — decides `--slot`, `current`, and the printed lines.<br>
- `tests/cli.rs` — the sandbox the new CLI tests rest on.<br>

## Verification

1. Gates, all clean:<br>`cargo fmt --all --check`<br>`cargo clippy --workspace --all-targets -- -D warnings`<br>`cargo test --workspace --all-targets`<br>`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`<br>
2. `cargo test --test cli --test wt_settings --test wt_schema` → all pass, with the new state tests running.<br>
3. Sandbox run with both environment variables pointing at a temp directory, and `tests/fixtures/wt/settings.json` installed:<br>`onecoat use nord --slot light` writes the fragment and records `light`.<br>`onecoat current` prints `light  nord`, with no targets applied yet.<br>`onecoat use nord` records `dark` and leaves both slots recorded.<br>`onecoat current --json` prints the assignment and the applied targets.<br>`onecoat use nord --dry-run` leaves no `state.json` behind.<br>A second `onecoat use nord` says `unchanged` and leaves the state byte-identical.<br>
4. Live smoke: copy the live fragment and `settings.json` aside, then hash both.<br>`onecoat use nord --slot light` fills the light slot with nord's colors.<br>`onecoat current` prints `light  nord` and the target `wt`.<br>Windows Terminal then shows `onecoat-light` carrying nord's colors, which needs your eyes.<br>`onecoat use nord` puts the dark slot back.<br>Restore both files from the aside copies and re-hash.<br>
5. `cargo run -p tidy` → `tidy: 5 checks clean`.<br>

## Assumptions & contingencies

- **`--slot` reassigns the theme to the named slot**, the default being its declared appearance.<br>If R-8 means "apply only this slot", the change stays in argument handling.<br>
- **The state path is `%APPDATA%\onecoat\state.json`**, which `docs/architecture.md` already fixes.<br>If it belongs under `%LOCALAPPDATA%`, one line in `Paths` moves it.<br>
- **The state holds the assignment and the last applied targets**, and VS9 adds the expected values.<br>If `current` should also report drift, VS9 grows it.<br>
- **A missing state file is not an error**: `current` prints `none` and exits 0.<br>A malformed file exits 3 and names the file plus the fix, which is to delete it.<br>
- **"Every enabled target" means every target this build can write**, which is `wt` today.<br>If it means a configured set, that set arrives with the later targets.<br>
- **State is not a target**: `--targets` never filters it and `Plan` stays target-only.<br>If the state should follow `--targets`, it becomes a third planned write.<br>
- **The slot decides the names and the pair side**, never the colors.<br>A dark theme may therefore hold the light slot.<br>If R-3 should bind the assignment too, `use --slot` rejects the mismatch.<br>
- **`--dry-run` writes no state** and prints the assignment it would record.<br>If it should print only file writes, drop that line.<br>

## As implemented

The plan landed as written except for the points below.<br>
`current` reports the targets the state holds, so a run before any apply prints `none` and `-`.<br>
The plan's step 3 read `light  nord` and called the targets unapplied.<br>The `--slot light` apply had written `wt`.<br>
`--dry-run` prints `would assign <slot> = <id>` above the file lines, as the assumptions asked.<br>
That line comes from the preview result, so a plan that cannot resolve prints nothing at all.<br>
`use` reads and validates the state before it writes a target file, so a malformed state fails first.<br>
The state write stays last, and it is skipped when the text is unchanged.<br>
The state has no backup; it is disposable, and the writer re-reads and parses it after the write.<br>
`Target` gained a `from_name` beside `ALL`, and `Slot` is a `ValueEnum` for `--slot`.<br>
