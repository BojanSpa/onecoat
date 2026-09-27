# VS6 — `omp-live-reload`

## Context

VS5 writes the applied slot's theme file and pins its name.<br>
This slice proves what a running session does with that file.<br>
The harness watches the themes directory and repaints when the active theme file changes.<br>
A theme switch therefore costs one generated file and no restart.<br>
The slice adds the missing regression test for that contract and the live proof.<br>
Closes R-22 and R-24.<br>

## Facts established on this machine (do not re-derive)

- The harness bundles its source with `packages/tui/src/theme/…` markers inside `omp.exe`.<br>Its code can therefore be read out of the binary.<br>
- Its theme watcher watches the themes directory and not the file: `fs.watch(<agent dir>/themes, …)`.<br>
- A reported filename is kept only when it equals `<active theme>.json`, so events for `.onecoat.tmp` are dropped.<br>
- The watcher debounces 100 ms, re-reads the file, and repaints.<br>It is not installed for the built-in `dark` and `light`.<br>
- A second watcher covers `config.yml`, `settings.json` and the other settings files and re-applies them on change.<br>
- Custom themes are read from disk on each load; only built-ins come from the in-memory map.<br>
- VS5 already proved the no-op half: `a_second_use_neither_rewrites_nor_rebacks_up` stamps `omp_theme` and `omp_config` (`tests/cli.rs:358`).<br>

## Approach

1. Add `a_theme_rewrite_leaves_the_pin_untouched` to `tests/cli.rs`.<br>Apply `nord`, then install the shadowing theme and apply `nord` again.<br>The theme file's bytes and stamp change, while `config.yml`'s bytes and stamp do not.<br>A third apply changes nothing at all.<br>
2. Correct the omp reload column in `docs/architecture.md`.<br>The active theme file is watched through its directory, and `config.yml` has a settings watcher of its own.<br>Name the temp-file filter as the reason a rewrite is the only event onecoat raises.<br>
3. Prove it live in a pseudo-terminal.<br>Pin the current slot, start `omp`, rewrite the active theme file, and read the repaint out of the session's output.<br>

## Critical files & anchors

- `tests/cli.rs` — the CLI contract tests; the omp block is where the new case belongs.<br>
- `docs/architecture.md` — the omp reload column and the constraints that shape the writers.<br>
- `tests/fixtures/themes/shadow-nord.toml` — a same-id theme with different colors, which is the rewrite trigger.<br>

## Verification

1. Gates, all clean:<br>`cargo fmt --all --check`<br>`cargo clippy --workspace --all-targets -- -D warnings`<br>`cargo test --workspace --all-targets`<br>`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`<br>`cargo run -p tidy`<br>
2. `cargo test --workspace --all-targets a_theme_rewrite` → the new test passes.<br>It fails if a same-name apply rewrites the pin.<br>
3. Live smoke: copy the live `config.yml` aside, apply `nord` to the current slot, and start `omp` in a pseudo-terminal.<br>Create `%APPDATA%\onecoat\themes\probe.toml` from `nord` with `base05 = "#FF00FF"` and apply it to the same slot.<br>Expected: the session's next frames carry the probe color, and `config.yml` is byte-identical.<br>
4. Second run: apply `probe` again → the theme file's stamp is unchanged, and the session shows no new frame.<br>
5. Restore: stop the session, restore the `config.yml` copy byte-identically, and delete `themes/onecoat-<slot>.json` and the probe theme.<br>`onecoat current` then reads as it did before the smoke.<br>

## Assumptions & contingencies

- **The rename wakes the watcher**: the replace renames onto `<active>.json`, and the directory watcher reports that name.<br>If the session does not repaint, write the theme file in place instead.<br>That deviation costs R-10's atomic path for this one generated file.<br>
- **Truecolor escapes**: the capture should carry `38;2;255;0;255` for the probe's `text`.<br>If the TUI emits 256-color codes, assert that the frames changed instead of the sequence.<br>
- **The pin may also reload live**: the settings watcher may pick up a pin change, which R-22 does not require.<br>Record what the smoke shows and leave the requirement text alone.<br>

## As implemented

Everything above landed as written.<br>These are the points where the plan was adjusted while implementing it.<br>
