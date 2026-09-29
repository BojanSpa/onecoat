# VS8 — `doctor`

## Context

VS8 adds `onecoat doctor`, the first command that only reads.<br>
It reports what onecoat sees on this machine right now.<br>
It covers the resolved paths, the `herdr` program, the herdr socket, the profile pins, and the state age.<br>
It writes nothing, so it is safe to run at any time.<br>
`verify`, `watch`, `import`, and `validate` stay for later slices.<br>
Closes R-30.<br>

## Facts established on this machine (do not re-derive)

- `where.exe herdr` finds `herdr.exe` under `C:\Users\bs\.herdr\packages\standalone\releases\0.9.2-preview.2026-09-29-8e78f929d8f0-x86_64-pc-windows-msvc\`.<br>
- The live `%APPDATA%\herdr\herdr.sock` exists; the live `%APPDATA%\onecoat\state.json` was written 2026-09-28 00:38 CEST.<br>
- The live `settings.json` pins only `PowerShell`; `jsonc::pinned` renders its pair as `onecoat-dark/Dainty Nord C1 L0`.<br>
- All seven live paths exist today, and the smoke expects seven `present` rows.<br>
- The omp agent directory defaults to `%USERPROFILE%\.omp\agent`; `PI_CODING_AGENT_DIR` and `HERDR_CONFIG_PATH` are unset.<br>
- `src/exec.rs` spawns only `herdr`, so `herdr` is the only program doctor needs to find.<br>
- `jsonc::pinned` returns no rows when `profiles.list` is absent, so a settings file without profiles is no error.<br>
- `tests/cli.rs` points `PATH` at a missing directory, so no test finds `herdr` today.<br>
- The purity check fails unless every file named in `IMPURE` exists in every tree under `tools/tidy/fixtures/purity/`.<br>
- `python` 3.12 is installed, so the live smoke can parse the JSON output.<br>

## Approach

1. Add `src/doctor.rs` with the report types, `pub fn probe(paths: &Paths) -> Result<Report, Error>`, `Report::table()`, and `Report::json()`.<br>
2. Register it as `pub mod doctor;` in `src/lib.rs`, above `pub mod error;`.<br>
3. The `paths` section holds seven rows.<br>The order is `wt fragment`, `wt settings`, `herdr config`, `omp themes`, `omp config`, `user themes`, `state`.<br>
4. Each row maps to the same-named `Paths` field and expects a file.<br>`omp themes` and `user themes` expect a directory instead.<br>
5. A row is `present` only when the entry exists with the expected kind; every other case is `missing`.<br>
6. The `binaries` section holds one row for `herdr`.<br>
7. Discovery walks `PATH` in order, trying the bare name first and then each `PATHEXT` entry.<br>
8. `PATHEXT` falls back to `.COM;.EXE;.BAT;.CMD`, and the first existing file fills the path column.<br>No hit means `missing` and a `-` path.<br>
9. The `sockets` section holds one row for `paths.herdr_socket`: `present` or `missing`.<br>
10. The `pins` section comes from `jsonc::pinned` over `settings.json`, using `wt::PROFILES_KEY` and `wt::COLOR_SCHEME_FIELD`.<br>
11. Pins keep the file order; a missing settings file or a missing `profiles.list` yields no rows.<br>A pin row shows the profile in `name`, the settings path, and the scheme in `status`.<br>
12. The `state` section holds only the age of `paths.state`, from its modification time; doctor never parses it.<br>
13. A missing file is a finding; an unreadable or unparseable file is the existing error, so exit 3.<br>
14. The table renders four columns, `kind`, `name`, `path`, and `status`, with the width style of `print_current`.<br>The first line is the header `kind name path status`.<br>
15. Kind literals are `path`, `binary`, `socket`, `pin`, `state`; status literals are `present`, `missing`, `found`.<br>
16. `-` fills a column with no value; the state row is `state`, `age`, `-`, and the formatted age.<br>
17. Ages read `just now` under a minute, `{m}m` under an hour, and `{h}h` or `{h}h {m}m` under a day.<br>
18. Older ages read `{d}d` or `{d}d {h}h`, and a modification time in the future counts as zero.<br>
19. Neither renderer ends with a newline; `main` prints both with `println!`, and the JSON is pretty-printed.<br>
20. The JSON field names are exactly these:<br>

```json
{
  "paths": [{"name": "wt fragment", "path": "C:\\...", "present": false}],
  "binaries": [{"name": "herdr", "found": false, "path": null}],
  "sockets": [{"name": "herdr", "path": "C:\\...", "present": false}],
  "pins": [{"profile": "PowerShell", "scheme": "One Half Dark"}],
  "state": {"age_seconds": null}
}
```

21. Wire `Command::Doctor(ReadArgs)` in `src/main.rs` after `Current`; `fn doctor` prints either form to stdout.<br>
22. The subcommand `about` value is exactly the line below.<br>

```
Report the resolved paths, the herdr program, the socket, the profile pins, and the state age
```

23. Add `pub const HERDR_PROGRAM: &str = "herdr";` to `src/targets.rs`, and use it from `exec.rs` and `doctor.rs`.<br>
24. Add `src/doctor.rs` to `IMPURE` in `tools/tidy/src/checks/purity.rs`, and stub it in the passes and fails trees.<br>
25. Both stubs hold pure code, so the failing positions stay `src/jsonc.rs:2`, `src/plan.rs:2`, `src/render/wt.rs:1`.<br>
26. In `tests/cli.rs`, point the sandbox `PATH` at a new `programs` directory instead of `no-programs`.<br>
27. Add `tests/unit/doctor.rs`: `59` seconds reads `just now`, `60` reads `1m`, `3600` reads `1h`, `3660` reads `1h 1m`.<br>
28. It also covers `90000` as `1d 1h` and a modification time in the future as `just now`.<br>
29. Add five CLI tests: `doctor_reports_the_paths_and_the_program`, `doctor_json_names_each_section`, `doctor_reports_a_pin_and_the_state_age`, `doctor_finds_a_program_on_path`, `doctor_fails_on_a_broken_settings_file`.<br>
30. The first asserts the exact table, exit 0, an untouched file tree, and unchanged stamps.<br>
31. The second parses the JSON: seven paths, one binary, one socket, no pins, and a null age.<br>
32. The third applies `nord` first, then expects the `PowerShell` pin and a `just now` age.<br>
33. The fourth and fifth cover a `programs/herdr.exe` on `PATH` and a broken settings file at exit 3.<br>
34. Update `docs/architecture.md` with the layout line, the edge list, and a short Inspection section.<br>
35. Update `docs/requirements.md` R-30 with a `Verified by` clause, and the README command lists.<br>
36. Flip VS8 in `work/roadmap.md` and commit this plan as `work/plans/vs8-doctor.md` with the slice.<br>
37. Branch `feat/vs8-doctor` from `origin/main`, push, and ask for review; the PR names R-30.<br>

## Critical files & anchors

- `src/doctor.rs` — new; the probe, the rows, and both renderers.<br>
- `src/main.rs` — the subcommand, the print path, and the exit code.<br>
- `src/targets.rs` — the shared `HERDR_PROGRAM` constant and the socket path.<br>
- `tools/tidy/src/checks/purity.rs` — the `IMPURE` list and its fixture obligation.<br>
- `tests/cli.rs` — the sandbox `PATH` change and the new command tests.<br>

## Verification

1. Gates, all clean:<br>

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo run -p tidy
```

2. `cargo test --workspace --all-targets --test cli doctor` → the five new tests pass.<br>
3. `cargo test --workspace --all-targets age` → the `age` unit tests pass.<br>
4. Hand run in an empty root, from the repository root, after `cargo build --release`:<br>

```sh
root=$(mktemp -d)
LOCALAPPDATA=$root APPDATA=$root PI_CODING_AGENT_DIR=$root/omp PATH=/nonexistent ./target/release/onecoat doctor
```

5. → all seven rows `missing`, `binary herdr missing`, no pin rows, `state age -`, and exit 0.<br>
6. Live smoke, read-only: snapshot the four live files with `ls -la --time-style=full-iso`.<br>
7. `./target/release/onecoat doctor` → seven `present` rows, `binary herdr found` on the `.herdr` release path, and `socket herdr present`.<br>
8. It also prints the `PowerShell` pin and a `Nd Nh` age.<br>The state file is about a day old today.<br>
9. Write the JSON to a temp file and check the fields with python:<br>

```sh
./target/release/onecoat doctor --json > "$TEMP/doctor.json"
python -c "import json,os; d=json.load(open(os.environ['TEMP']+'/doctor.json')); print(d['binaries'][0]['found'], d['sockets'][0]['present'], d['pins'], d['state']['age_seconds'])"
```

10. → `True`, `True`, the pair `onecoat-dark/Dainty Nord C1 L0`, and a positive age.<br>
11. Compare the snapshots → every stamp identical, no new file, and no `.onecoat.tmp`.<br>
12. Run `doctor` once more → the same table, except a possible age tick.<br>

## Assumptions & contingencies

- **Binaries are `herdr` only**, because `exec.rs` spawns nothing else.<br>If a later slice spawns another program, add a row for it.<br>
- **Missing rows still exit 0**, because a missing file is information, not a failure.<br>Use `verify --check` (VS9) when a nonzero exit is needed.<br>
- **Doctor does not parse the state file**, because `current` already reports a malformed state.<br>If a reviewer wants it, print the problem as a finding and still exit 0.<br>
- **Discovery walks `PATH` itself**, so doctor spawns nothing and cannot hang.<br>If it misfires on a shim directory, fall back to a bare-name check per directory.<br>
- **The omp rows report the theme directory**, not the per-slot files, because those exist only after an apply.<br>

## As implemented

Everything above landed as written.<br>These are the points where the plan was adjusted while implementing it.<br>

- The sandbox pins `PATHEXT`, and the found-program test writes `programs/herdr.EXE`.<br>
- The reported program path keeps the `PATHEXT` spelling, so it can read `herdr.EXE`.<br>
- A state file that cannot be stat-ed beyond simple absence fails as an unreadable file.<br>
- The state age requires a regular file, so a directory at that path reads as `-`.<br>
- The report rows borrow their paths instead of cloning them.<br>
- A sixth CLI test covers a present socket, and the age tests pin 3599 and 86400.<br>
- The slice lands on `feat/vs8-doctor` as one `chore` commit plus one `feat(vs8)` commit.<br>

## Review findings

Rows 1 to 5 are the fixes; rows 6 to 9 are optional.<br>

| # | Finding | Fix | Done |
| --- | --- | --- | --- |
| 1 | The `{d}d` age form has no test. | Add the `86400` and `3599` asserts. | [x] |
| 2 | `state_age` takes the age of a directory. | Require a file, like the path rows. | [x] |
| 3 | The `Shape` section says only the executor touches the filesystem. | Say `writes to` instead. | [x] |
| 4 | The `targets.rs` layout line claims binary discovery. | Drop `binary` from that line. | [x] |
| 5 | The README says the roadmap covers slices from VS8. | Change it to VS9. | [x] |
| 6 | `probe` clones seven `PathBuf`s. | Optional: borrow them. | [x] |
| 7 | The CLI test helper repeats the padding math. | Optional: assert literal lines. | [ ] |
| 8 | The PATH walk differs from `CreateProcess` search order. | Optional: note it in the Inspection section. | [x] |
| 9 | The socket row has no `present` test. | Optional: add a present socket. | [x] |

Row 7 stays as left: the temp root sets the path column's width, so the helper encodes that width rule.<br>
