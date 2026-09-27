# VS7 — `herdr-theme`

## Context

VS7 gives Herdr the same palette the other two targets already carry.<br>
It writes only the theme keys in `%APPDATA%\herdr\config.toml` and leaves every other byte as it was.<br>
`toml_edit` does the edit, so comments and key order survive.<br>
A `herdr config check` gate and a `herdr server reload-config` reload close the loop.<br>
The slice stops short of appearance watching, which VS11 owns.<br>
Closes R-18, R-19, R-20.<br>

## Facts established on this machine (do not re-derive)

- Herdr 0.9.1-preview is installed and on PATH; `herdr status server` reports the server running.<br>
- `HERDR_CONFIG_PATH` overrides the config path, so a candidate file can be checked without touching the live one.<br>
- `dark_name` and `light_name` accept only built-in theme names.<br>A name like `onecoat-dark` is reported and costs exit 1.<br>
- The base theme that makes panes inherit the host ANSI palette is `terminal`, and it validates the whole artifact.<br>
- `[theme.custom]` and its `.dark`/`.light` layers accept these tokens: `panel_bg`, `sidebar_bg`, `active_row_bg`, `selection_bg`, `text`, `accent`, `red`, `green`, `blue`, `yellow`.<br>
- An unknown key is reported as `unknown config key … ignoring key` and still exits 1.<br>The check therefore cannot be read as a plain boolean.<br>
- The layers apply per appearance only while `auto_switch = true`.<br>
- The live config is 761 bytes and carries `[theme] name = "nord"`.<br>The smoke therefore needs a backup and a restore.<br>
- The socket at `%APPDATA%\herdr\herdr.sock` is a 25-byte text file holding a pipe endpoint.<br>Its presence is the reload precondition.<br>
- `docs/architecture.md`'s role table pins which palette entry feeds each herdr token; the renderer follows it.<br>

## Approach

1. Add `src/render/herdr.rs`: derive the ten tokens from the palette and return an edit list of table paths and values.<br>The `name` key takes the theme's `[targets.herdr] name` override and defaults to `terminal`.<br>The appearance surfaces go in `[theme.custom.<slot>]`, and the accents in `[theme.custom]`.<br>
2. Add `toml_edit` to `Cargo.toml` and a `Content::Toml` arm that splices the edits into the re-read file.<br>Missing tables are created; a key of the wrong type fails the apply.<br>
3. Gate the write by checking the live file and the candidate before the replace.<br>Only the issues the candidate adds fail the apply.<br>The candidate reaches herdr through `HERDR_CONFIG_PATH`, so nothing else reads a half-written file.<br>
4. Reload after a changed write: run `herdr server reload-config` when the socket exists.<br>Print the interactive-reload line when it does not, and never signal a client.<br>
5. Honour `HERDR_CONFIG_PATH` when resolving the config path, so onecoat writes where herdr reads.<br>

## Critical files & anchors

- `src/render/herdr.rs` — new; the token mapping and the edit list.<br>
- `src/plan.rs` — the new content kind joins the plan and the target list.<br>
- `src/exec.rs` — the splice, the check gate, and the reload spawn.<br>
- `src/targets.rs` — the config path, resolved from `APPDATA` with `HERDR_CONFIG_PATH` honoured.<br>
- `tests/fixtures/herdr/config.toml` — new; a commented config with foreign keys, plus its spliced golden.<br>

## Verification

1. Gates, all clean:<br>`cargo fmt --all --check`<br>`cargo clippy --workspace --all-targets -- -D warnings`<br>`cargo test --workspace --all-targets`<br>`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`<br>`cargo run -p tidy`<br>
2. `cargo test --workspace --all-targets herdr` → the new tests pass, including the idempotency pair and the byte-exact golden.<br>
3. Hand run: `onecoat use nord --slot light --targets herdr --dry-run` → prints the owned keys.<br>The real run writes the file and `herdr config check` exits 0.<br>
4. Live smoke: back up the live config and apply.<br>`herdr config check` says ok, `herdr server reload-config` exits 0, and the server log carries the reload.<br>
5. Restore: copy the backup back byte-identically and reload again.<br>The config hash then matches the pre-smoke one, and `onecoat current` reads as before.<br>

## Assumptions & contingencies

- **Token split**: surfaces in the slot layer, accents in the shared block, matching herdr's own example.<br>If the second appearance reads wrong, move every token into the layers.<br>
- **One layer per apply**: only the applied slot's layer is written, because the plan renders from one theme.<br>If both layers must always exist, render the other from the state assignment later.<br>
- **Issue diff over exit code**: the check must not block an apply because of a warning the user already had.<br>If the message format changes, compare the issue lines instead.<br>
- **Base theme `terminal`**: the theme file can override the name.<br>A bad name then fails the check gate instead of silently falling back.<br>
- **Reload without a server**: onecoat prints the interactive line and still exits 0, because R-20 forbids restarting clients.<br>

## As implemented

Everything above landed as written.<br>These are the points where the plan was adjusted while implementing it.<br>

- A missing `herdr` binary skips the check and the reload note says so; both still exit 0.<br>
- The shared `[theme.custom]` table is written for accents only, so a stale surface key there stays.<br>
- The dry run prints the planned keys but no reload notes, which only a real run can know.<br>
- The live smoke needed the herdr release directory added to `PATH`; `where.exe herdr` found nothing.<br>

