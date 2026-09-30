# VS9 — `verify-drift`

## Context

VS9 adds `onecoat verify`, the first command that says whether the files still match the themes.<br>
It re-derives the owned values from the assigned themes on every run, so state stays a record of the assignment.<br>
Drift is reported per target and slot with the offending keys, and `verify --check` turns any finding into exit 1.<br>
Cross-target coherence (R-28) and palette spacing (R-4) arrive with VS10 and reuse this exit path.<br>
Re-applying what this command reports is `watch --repair` in VS11; nothing here writes a file.<br>
Closes R-26, R-27, R-29.<br>

## Facts established on this machine (do not re-derive)

- The live `%APPDATA%\onecoat\state.json` assigns `nord` to both slots and records `wt`, `herdr`, `omp`.<br>
- The live omp themes directory holds `onecoat-dark.json` and its backup, but no `onecoat-light.json`.<br>So a live `verify` before an apply reports that light file's owned keys as missing.<br>
- The live fragment, `settings.json`, the herdr config, and the omp config all exist.<br>`herdr` is on `PATH` and its socket is up.<br>
- `HexColor` renders and serializes lowercase `#rrggbb`, so a color compares as an equal string on both sides.<br>
- `jsonc_parser`'s CST exposes `to_serde_value`, which is how the writer already diffs a document's values after an edit.<br>
- The purity check fails any `src/` file that reaches `std::{env,fs,process}` except `doctor`, `exec`, `main`, `targets`, and `themes`.<br>So `src/verify.rs` parses and compares only text it is handed.<br>
- The herdr renderer writes four theme-level keys and five shared tokens from the applied slot's theme.<br>Every apply therefore overwrites those nine keys, whatever slot it runs for.<br>
- The omp config writer already has private helpers that find the `theme:` block.<br>Its line helpers find one child key, so a reader can reuse them.<br>
- `tests/cli.rs`'s sandbox points `LOCALAPPDATA`, `APPDATA`, and `PI_CODING_AGENT_DIR` at a temp root, and installs the herdr config by default.<br>
- `serde_json` is usable from integration tests, and the existing tests already rewrite fixture files through it.<br>
- `tests/fixtures/themes/herdr-overrides.toml` keeps nord's palette and names the herdr theme `nord`.<br>So two slots assigned to it and to `nord` disagree on the shared theme keys.<br>

## Approach

1. Add `src/verify.rs` as a pure module, and register `pub mod verify;` after `pub mod validate;` in `src/lib.rs`.<br>
2. `Finding` carries `target`, `slot`, `key`, `expected`, and `found`.<br>It derives `Serialize` and holds the last two as `Option<String>`.<br>One row of the report is one `Finding`, and the `--json` output is the array of them:<br>

```json
[{"target": "wt", "slot": "dark", "key": "schemes.onecoat-dark.background", "expected": "#0b1018", "found": null}]
```

3. `drift(slot, write, source, alternatives) -> Result<Vec<Finding>, Error>` compares one planned write.<br>`shared_alternatives(writes)` reads the herdr writes of every assigned slot and collects the shared values.<br>`Alternatives` maps a dotted key to the expected values of every assigned slot, in dark-then-light order.<br>
4. Expected values come from the plan's edits, never from the file.<br>So the comparison states what an apply would change.<br>
5. JSONC expectations: `Set` at its key, `Element` at `<key>.<name>`, and `Pair` at `<key>.<side>`.<br>`Repoint` contributes nothing, because a profile pin is not an owned value in this slice.<br>Every plan is built with `ProfileScheme::Report`, so a pin is never repointed or compared.<br>
6. TOML expectations are the edit paths joined by `.`, and YAML expectations are `theme.<side>`.<br>A generated file is compared as a whole parsed document, down to keys like `colors.text`.<br>So a token added by hand is drift.<br>
7. Actual values are read through the format's own parser.<br>JSONC and generated JSON go through a new `jsonc::value`.<br>TOML goes through new `herdr::parse` and `herdr::value`, and YAML through a new `omp::config_value`.<br>
8. Comparison is by parsed value, so comments, indentation, key order, and line endings never drift.<br>Equal values pass; a differing scalar reports one finding; containers on both sides recurse key by key.<br>
9. A container present on one side alone reports one finding per leaf of that side.<br>The absent side prints `-`, and a container facing a scalar reads as compact JSON.<br>
10. Herdr's shared keys are every herdr edit outside `theme.custom.<slot>`.<br>That is the four theme keys and the five shared tokens.<br>A shared finding is dropped when its found value equals any assigned slot's expected value.<br>Otherwise its `expected` joins the distinct values in dark-then-light order around `" or "`.<br>
11. Findings that differ only in `slot` collapse to the first, so one shared key never prints twice.<br>
12. Add `exec::read_source(path) -> Result<Option<String>, Error>`, the only new IO, beside the existing read helpers.<br>
13. An absent file that `Absent::Create` allows reports every owned key as missing.<br>An absent file that `Absent::Fail` protects returns `Error::MissingFile`, which exits 3.<br>An unparseable file stays the existing parse error.<br>
14. In `src/main.rs`, add `Command::Verify(VerifyArgs)` between `Current` and `Doctor`, with `--json` and `--check`.<br>Its `about` line is exactly: `Compare the target files with the assigned themes and report drift`.<br>
15. `run` returns `ExitCode`: verify returns 1 when it has findings and `--check` was passed, 0 otherwise.<br>`main` returns that code and still prints every error to stderr with exit 3.<br>
16. Verify reads the state and loads the theme set.<br>It builds every assigned slot's plan, limited to the recorded targets, and compares in dark-then-light order.<br>Plans are built before any comparison, so the shared alternatives are known first.<br>
17. An unassigned slot and an empty recorded-target list yield no findings.<br>A missing assigned theme and a malformed state stay errors.<br>
18. `verify::table(&findings)` prints the header `target slot key expected found`, then one row per finding.<br>With no findings it prints the single line `no drift`.<br>`verify::json(&findings)` prints the array, and both pad columns like doctor's table.<br>Rows follow the plan's edit order, and each container's keys in name order.<br>
19. Pin the comparison with `tests/unit/verify.rs`.<br>A reformatted JSONC document is not drift, and an edited color reports its full key.<br>A missing pair side reports `theme.dark`, and an absent creatable file reports every owned key.<br>An absent required file is an error, and a `Repoint` edit yields nothing.<br>A shared herdr token matching the other slot is not drift, and one matching neither lists both values.<br>An extra token in a generated file reports with `expected` `null`.<br>
20. Pin the command with seven tests in `tests/cli.rs`.<br>Clean after an apply: exit 0, tree and stamps untouched, `--json` prints `[]`.<br>An edited fragment color prints the row, and `--check` exits 1.<br>A reformatted omp theme file and a herdr comment line print `no drift`.<br>Two slots with herdr overrides print `no drift`, then `theme.name` with `terminal or nord` after an edit.<br>A removed owned key reports with `found` `-`, and a broken settings file exits 3 without writing.<br>An untouched sandbox prints `no drift`.<br>Two more tests pin the narrowed apply and the shadowed theme.<br>
21. Update the contracts: R-26, R-27, and R-29 gain `Verified by:` clauses naming those CLI tests.<br>`docs/architecture.md` gains the `src/verify.rs` layout line and verify's rules in the drift section.<br>It loses the state paragraph's expectation-recording and re-derivation sentences.<br>
22. Flip VS9 in `work/roadmap.md` and move `verify` in the README to what works today.<br>Commit this plan as `work/plans/vs9-verify-drift.md` with the slice.<br>
23. Branch `feat/vs9-verify-drift` from `origin/main`, push, and ask for review.<br>The PR title starts `feat(vs9):` and names R-26, R-27, R-29.<br>

## Critical files & anchors

- `src/verify.rs` — new; the keys, the comparison, the shared-key rule, and both renderers.<br>
- `src/main.rs` — the subcommand, the exit code, and the per-slot orchestration.<br>
- `src/exec.rs` — `read_source`, the only new IO, beside `read_state`.<br>
- `src/jsonc.rs` — the JSONC reader the fragment, settings, and generated theme file share.<br>
- `tests/cli.rs` — the nine command tests and the sandbox they rest on.<br>

## Verification

1. Gates, all clean:<br>

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo run -p tidy
```

2. `cargo test --workspace --all-targets --test cli verify` → the nine new tests pass.<br>
3. `cargo test --workspace verify` → the unit tests pass.<br>
4. Hand run in a temp root after `cargo build --release`.<br>Install the fixture settings, the herdr config, and the omp config as the CLI sandbox does:<br>

```sh
root=$(mktemp -d)
mkdir -p "$root/Packages/Microsoft.WindowsTerminal_8wekyb3d8bbwe/LocalState" "$root/herdr" "$root/omp/agent"
cp tests/fixtures/wt/settings.json "$root/Packages/Microsoft.WindowsTerminal_8wekyb3d8bbwe/LocalState/settings.json"
cp tests/fixtures/herdr/config.toml "$root/herdr/config.toml"
cp tests/fixtures/omp/config.yml "$root/omp/agent/config.yml"
export LOCALAPPDATA=$root APPDATA=$root PI_CODING_AGENT_DIR=$root/omp/agent PATH=/nonexistent
./target/release/onecoat use nord
./target/release/onecoat verify          # → no drift, exit 0
./target/release/onecoat verify --check  # → no drift, exit 0
python -c "import json,os,pathlib; p=pathlib.Path(os.environ['LOCALAPPDATA'])/'Microsoft/Windows Terminal/Fragments/onecoat/schemes.json'; d=json.loads(p.read_text()); d['schemes'][0]['background']='#0b1019'; p.write_text(json.dumps(d, indent=2))"
./target/release/onecoat verify          # → the wt dark row, expected #0b1018, found #0b1019
./target/release/onecoat verify --check  # → the same row, exit 1
./target/release/onecoat verify --json   # → one object with the same five fields
```

5. Live smoke.<br>First copy every live file into a temp directory.<br>That is the fragment, `settings.json`, the herdr config, the omp config, both `onecoat-*.json` files, their backups, and `state.json`.<br>Hash every copy, then run `./target/release/onecoat verify` and keep the output as the baseline.<br>
6. `./target/release/onecoat use nord` and `./target/release/onecoat use nord --slot light` → both exit 0, and the light theme file appears.<br>
7. `./target/release/onecoat verify` → `no drift` and exit 0, over all five live target files.<br>A row here is a bug in the comparison, because both slots were applied to every recorded target just now.<br>
8. `./target/release/onecoat verify --check` → exit 0.<br>Then edit the live fragment's `onecoat-dark` background:<br>

```sh
python -c "import json,os,pathlib; p=pathlib.Path(os.environ['LOCALAPPDATA'])/'Microsoft/Windows Terminal/Fragments/onecoat/schemes.json'; d=json.loads(p.read_text()); [s for s in d['schemes'] if s['name']=='onecoat-dark'][0]['background']='#0b1019'; p.write_text(json.dumps(d, indent=2))"
```

9. `./target/release/onecoat verify` → one row, key `schemes.onecoat-dark.background`, expected `#0b1018`, found `#0b1019`, exit 0.<br>
10. `./target/release/onecoat verify --check` → the same row, exit 1.<br>
11. Restore every snapshot and hash the restored files.<br>Compare those hashes with step 5's → identical.<br>Run `verify` once more and compare with the baseline → the machine is as it was.<br>
12. Run `verify` twice in a row → identical output both times, and no file stamp changes.<br>

## Assumptions & contingencies

- **Verify re-derives from the theme on every run**, because R-26 says so.<br>A stale recorded baseline would hide a theme edit.<br>If recorded expectations are wanted later, they land in state.json and this comparison moves behind them.<br>
- **Profile pins are not owned values**, because onecoat repoints them only on request.<br>The state does not record that request.<br>If a hand-restored pin should be caught, record the mode in state and compare the pin sides.<br>
- **A herdr shared key accepts any assigned slot's value**, because the shared layer carries whichever theme applied last.<br>If exactness is wanted, record the applied slot per target in state and compare against it alone.<br>
- **Plain `verify` exits 0 with findings**; only `--check` exits 1, as VS8's plan already promised.<br>If it must be 1 either way, change the one condition in `main`.<br>
- **Missing files split by ownership**: a file onecoat may create (fragment, omp theme file) is drift.<br>`settings.json`, the herdr config, and the omp config are errors instead.<br>If a reviewer wants drift rows for all three, change the `Absent` check in one place.<br>
- **An absent file reports one row per owned key**, so a deleted omp theme file prints every token.<br>If that is too loud, report one row per top-level owned key instead.<br>
- **Verify checks every assigned slot against every target onecoat owns**, because the recorded list is `current`'s report, not a scope.<br>So a `--targets`-limited apply cannot hide drift.<br>The cost: a target narrowed out of every apply is checked too.<br>Its absent required file is then the usual error.<br>If that is unwanted, scope the plan back to the recorded targets and accept the hidden drift.<br>
- **Verify spawns nothing**, so a herdr config only `herdr config check` would reject is left to the next apply.<br>If the check belongs here too, it reuses `exec`'s existing gate.<br>
- **The smoke restores the live backups as well**, because the two applies rewrite them as a side effect.<br>

## As implemented

Everything above landed as written, except for these points.<br>

- `herdr::parse` was extracted from `splice`, so reader and writer share one parse error.<br>
- `herdr::value` takes that document and the edit path, and converts TOML items to JSON.<br>
- `jsonc::value` resolves a segment after an array by the element's `name`.<br>
- That is what makes `schemes.onecoat-dark` addressable.<br>
- `Key::segments` and `Key::join` widened to `pub(crate)` for the readers.<br>
- The generated file's expectation is parsed with `serde_json`.<br>
- The file on disk goes through `jsonc::value` at the root key.<br>
- Unparseable output from onecoat's own renderer surfaces as the existing `JsonEncodeFailed`.<br>
- Its message already says that this is a onecoat bug.<br>
- The shared-key rule lives in `herdr::is_shared`, derived from the edit path.<br>
- So `verify.rs` does not repeat the `theme.custom.<slot>` layout.<br>
- A container facing a scalar reports one finding, with the container as compact JSON.<br>
- Only an absent side expands the container leaf by leaf.<br>
- `verify::collapse` is public because `main` collects both slots' findings before dropping repeats.<br>
- The unit tests landed as eleven cases in `tests/unit/verify.rs`.<br>
- The report's table and `--json` shapes are pinned by the CLI tests instead.<br>

## Review findings

A code review reran the gates, the isolated end to end, and a read-only live `verify`; all matched this plan.<br>
These gaps remain, in the order I would fix them.<br>

| # | Finding | Fix | Done |
| --- | --- | --- | --- |
| 1 | Design: `verify` scopes to the last apply's recorded targets, and `record` replaces that list. One `--targets` apply therefore hides drift in the other files, and `--check` exits 0 over a hand-edited herdr config. | Plan against every target onecoat owns, or keep the recorded list as a union of applies. | [x] |
| 2 | Docs: the drift section says a container facing a scalar reports once per leaf; the code reports one row with compact JSON, and the live run agrees with the code. | Reword that sentence to match the As implemented note. | [x] |
| 3 | Contract: R-29 promises exit 1 on a coherence failure, and only VS10 can raise one, yet the roadmap closes R-29 here. | Trim the coherence clause until VS10 lands, or move R-29 to VS10. | [x] |
| 4 | Tests: nothing pins the R-26 half that expectations re-derive from the theme; every CLI test edits a target file, not the theme. | Add a CLI test that shadows the assigned theme and expects the drifted rows. | [x] |
| 5 | Tests: an absent create-allowed JSONC file is untested; only the generated file covers the absent path. | Add a unit test with a `Jsonc` write and no source, or delete the fragment after an apply. | [x] |

Each finding is closed on this branch:<br>

- **1** `main` plans every assigned slot over every target, so the recorded list only feeds `current`.<br>`verify_checks_a_target_the_last_apply_narrowed_out` fails without that change.<br>
- **2** The drift section now says a container facing a scalar is one finding.<br>
- **3** R-29 covers drift only; VS10 adds coherence to it.<br>
- **4** `verify_re_derives_expectations_from_the_assigned_theme` shadows `nord` and lists the shadow palette as expected.<br>
- **5** `an_absent_creatable_jsonc_file_reports_every_owned_key` covers the `Jsonc` absent path.<br>
- The CLI tests are nine and the unit tests ten, after 4 and 5.<br>
