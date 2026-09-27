# VS6 — `omp-theme-rewrite`

## Context

VS5 writes the applied slot's theme file and pins its name.<br>
This slice proves where omp reads that file and what a rewrite costs.<br>
A pinned session does not watch the file: it renders the palette it read at launch.<br>
A rewrite therefore reaches the next session, and the pin stays byte-identical.<br>
The slice adds the missing regression test, the live measurement, and the corrected contract.<br>
Closes R-22 and R-24.<br>

## Facts established on this machine (do not re-derive)

- The harness bundles its source with `packages/tui/src/theme/…` markers inside `omp.exe`.<br>Its code can therefore be read out of the binary.<br>
- The theme watcher watches the themes directory, keeps only the active theme file, and debounces 100 ms.<br>It then re-reads the file and repaints.<br>
- Only a session that sets a theme itself installs that watcher.<br>A session that resolves its pin at launch never does.<br>
- Measured on omp 18.3.5: an atomic replace, an in-place write, and a corrupt file left a pinned session unchanged.<br>
- A `config.yml` change during a session fires a settings watcher and is reported.<br>A later theme rewrite still repaints nothing.<br>
- A second watcher covers `config.yml`, `settings.json` and the other settings files and re-applies them on change.<br>
- Custom themes are read from disk on each load; only built-ins come from the in-memory map.<br>
- VS5 already proved the no-op half: `a_second_use_neither_rewrites_nor_rebacks_up` stamps `omp_theme` and `omp_config` (`tests/cli.rs:358`).<br>

## Approach

1. Add `a_theme_rewrite_leaves_the_pin_untouched` to `tests/cli.rs`.<br>Apply `nord`, then install the shadowing theme and apply `nord` again.<br>The theme file's bytes and stamp change, while `config.yml`'s bytes and stamp do not.<br>A third apply changes nothing at all.<br>
2. Correct the omp reload column in `docs/architecture.md`.<br>The applied theme file is read at launch and is not watched.<br>Record the measured attempts and the settings watcher beside it.<br>
3. Correct R-22 to the measured contract and R-24's rationale.<br>Name the next session as omp's reload signal in the README too.<br>

## Critical files & anchors

- `tests/cli.rs` — the CLI contract tests; the omp block is where the new case belongs.<br>
- `docs/architecture.md` — the omp reload column and the constraints that shape the writers.<br>
- `tests/fixtures/themes/shadow-nord.toml` — a same-id theme with different colors, which is the rewrite trigger.<br>

## Verification

1. Gates, all clean:<br>`cargo fmt --all --check`<br>`cargo clippy --workspace --all-targets -- -D warnings`<br>`cargo test --workspace --all-targets`<br>`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`<br>`cargo run -p tidy`<br>
2. `cargo test --workspace --all-targets a_theme_rewrite` → the new test passes.<br>It fails if a same-name apply rewrites the pin.<br>
3. Live smoke, done and recorded: a pinned session kept its rendering after both write paths.<br>The same rewrite is rendered by the next session, which reads the file at launch.<br>
4. Second run: applying `probe` twice left the theme file's stamp unchanged and the session silent.<br>
5. Restore: the session was stopped and `config.yml` was restored byte-identically.<br>The generated files and the probe theme were deleted.<br>`onecoat current` then read as it did before the smoke.<br>

## Assumptions & contingencies

- **The pin may also reload live**: the settings watcher fires on `config.yml`, but a theme rewrite still repainted nothing.<br>
- **Truecolor escapes**: the capture carried no probe color at all, so the repaint was absent rather than differently encoded.<br>
- **The watcher hypothesis is dead**: an in-place write was the fallback, and it repainted nothing either.<br>The write path therefore stays atomic, as VS5 left it.<br>
- The evidence lives in this plan and the architecture constraints, not in a test.<br>No test can watch a session.<br>

## As implemented

Step 3's expectation was wrong: a running session never repaints from a theme rewrite.<br>
The atomic replace and the in-place fallback both left the frame unchanged.<br>
A settings reload did not change that.<br>
The slice therefore lands the test, the measured facts, and a narrowed R-22 instead of a repaint proof.<br>
The roadmap's VS6 row and the README now state the same contract.<br>
