# VS5 — `omp-theme-files`

## Context

The omp harness is the third target, and `use --targets omp` prints `no writer for omp` today.<br>
This slice resolves the agent directory and writes the assigned slot's stable theme file.<br>
It also pins that file's name in `config.yml` once.<br>
One `use` then paints Windows Terminal and omp together.<br>
`current` reports both targets.<br>
Live reload and no-op quietness belong to VS6, not here.<br>
Closes R-7, R-21, N-7.<br>

## Facts established on this machine (do not re-derive)

- omp here is 18.3.5, while the architecture still pins 18.3.2 (`omp --help`).<br>The vendored 101-name reserved list still matches it.<br>
- `PI_CODING_AGENT_DIR` is unset here, so the agent dir is `C:\Users\bs\.omp\agent`.<br>The variable overrides it, as omp's own help text says.<br>
- The live theme files are `themes/nord.json` and `themes/dainty-nord-1-0.json`.<br>Both hold `{"name": …, "colors": {…}}` and carry no `$schema` or `vars`.<br>
- omp embeds 99 theme modules as JS text.<br>The union of their `colors` keys is 69 tokens.<br>Each theme carries 66 to 68 of them.<br>
- Only `link`, `thinkingMax` and `toolText` are missing from some themes.<br>Every other token is in all of them.<br>
- Recipe: scan `omp.exe` for `// packages/tui/src/theme/` markers.<br>Brace-match each `colors: {` object and take the union of its keys.<br>
- `config.yml` has 169 lines with no comments and no blank lines.<br>`theme:` is a top-level mapping holding only `dark: dainty-nord-1-0`.<br>Indents are 2, 4 and 6 spaces.<br>
- `omp config get theme.light` prints the built-in `light`.<br>An absent pin therefore reads as a harness default theme.<br>
- `omp config get theme.dark` runs against an isolated `PI_CODING_AGENT_DIR`.<br>It exits 0 and leaves `config.yml` untouched.<br>
- The live `config.yml` carries `providers` and `secrets` sections.<br>No copy of it may be committed, so the fixture is hand-authored in its shape.<br>
- Theme failures surface as `Theme not found`, `Unknown theme` or `Invalid theme` (binary scan).<br>

## Approach

1. Vendor the token list as a fixture and mirror it as an `OmpToken` enum in `src/render/omp.rs`.<br>A test asserts the enum matches the fixture.<br>A harness update then shows up as a fixture diff.<br>
2. Render the assigned slot's file as `{"name": "onecoat-<slot>", "colors": {…}}`.<br>Every token takes a literal hex value derived by `rolemap`.<br>`[targets.omp]` overrides replace single tokens.<br>The file carries no `$schema` and no `vars`.<br>
3. Splice `config.yml` line by line and keep every other byte.<br>Set only the applied slot's key.<br>Insert a missing key under `theme:` with its sibling's indentation.<br>Append a `theme:` block when the file has none.<br>Refuse a `theme:` that is not a mapping, and refuse a missing `config.yml`.<br>
4. Add the omp writes to the plan after the wt writes, theme file before its pin.<br>`--targets` still filters, and `herdr` stays the one target with no writer.<br>
5. Resolve `omp_themes` and `omp_config` from `PI_CODING_AGENT_DIR`.<br>Fall back to `USERPROFILE\.omp\agent`, reporting a missing variable like the other paths do.<br>
6. Give every token a row in the architecture's omp mapping table.<br>Refresh the pinned omp version and the token count.<br>Add `Verified by:` clauses for R-7, R-21 and N-7.<br>

## Critical files & anchors

- `src/render/omp.rs` — the token list, the generated file, and the `config.yml` splice rules.<br>
- `src/rolemap.rs` — the single place a token's role is derived.<br>
- `src/plan.rs` — where the omp writes join the ordered plan.<br>
- `src/exec.rs` — resolve, back up, replace, and re-parse discipline for both new writes.<br>
- `tests/fixtures/omp/` — the vendored token list and the hand-authored `config.yml`.<br>

## Verification

1. Gates, all clean:<br>`cargo fmt --all --check`<br>`cargo clippy --workspace --all-targets -- -D warnings`<br>`cargo test --workspace --all-targets`<br>`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`<br>`cargo run -p tidy`<br>
2. Sandbox run: `onecoat use nord` with `PI_CODING_AGENT_DIR` pointing at a fixture agent dir → writes `themes/onecoat-dark.json` and `config.yml`.<br>`current` then lists `wt,omp`.<br>
3. Same dir: `omp config get theme.dark` → prints `onecoat-dark` and exits 0.<br>
4. Second `onecoat use nord` → `unchanged` for both paths and every file byte-identical.<br>The pin is not rewritten.<br>
5. Live smoke: copy the live `config.yml` aside, then `onecoat use nord --slot light`.<br>`omp config get theme.light` → `onecoat-light`.<br>Restore the copy byte-identically and delete the generated file.<br>

## Assumptions & contingencies

- **The applied slot only**: one apply writes one theme file and one pin.<br>The other slot may hold a theme onecoat never wrote.<br>If R-21 must mean two files always, render the other slot from the state's assignment.<br>Skip its pin when there is none.<br>
- **Hex only**: an `Index` override renders as its xterm-256 RGB value.<br>Every working theme file holds hex strings.<br>If the harness accepts bare numbers, emit the index instead.<br>
- **No `$schema`**: the only known schema URL is remote and N-1 forbids fetching it.<br>The vendored token list is the validator.<br>If the harness insists on the reference, vendor the URL as a constant.<br>Never fetch it.<br>
- **No omp spawn during `use`**: a 239 MB binary would break N-3.<br>The token list carries the check.<br>A real harness check belongs to `verify`, not `use`.<br>
- **A missing `config.yml` fails the apply**, matching `settings.json`.<br>onecoat never invents the harness's config.<br>If that proves too strict, write the file only when the agent dir exists.<br>
- **The token list is a vendored fact**: a harness update means re-running the recipe and diffing the fixture.<br>

## As implemented

Everything above landed as written.<br>These are the points where the plan was adjusted while implementing it.<br>

- The omp token-to-role table lives in `src/render/omp.rs`, beside the file it renders.<br>`rolemap` still derives the roles themselves.<br>
- The vendor fixture is read at test time, so the fixture is the contract.<br>The token count is stated once (`src/render/omp.rs:19`) with the 66-to-68 reminder.<br>
- The fixture generator's `$schema` key was a false positive and is not in the fixture.<br>
- The pin and the theme file carry one value.<br>A pinned key changes only when the name differs, and the other slot's pin is kept (`src/render/omp.rs:375-401`).<br>
- A file that pins neither slot gains a `theme:` block with both keys, so the sibling keeps its value.<br>
- `PI_CODING_AGENT_DIR` counts as unset when it is empty, matching the other path variables (`src/targets.rs`).<br>
- The omp CLI tests sit in `tests/cli.rs` beside the Windows Terminal ones, not in a new module.<br>
- A no-op apply prints `unchanged` for the theme file and the pin already, because the executor compares bytes.<br>VS6 still owns the reload proof.<br>
- The live `config.yml` carries no comments, so byte preservation is proven against the hand-authored fixture.<br>
