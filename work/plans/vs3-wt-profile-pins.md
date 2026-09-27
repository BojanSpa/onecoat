# VS3 — `wt-profile-pins`

## Context

`use <id>` already splices three owned keys into `settings.json`.<br>
Windows Terminal also lets one profile pin its own `colorScheme`.<br>
Such a profile stops following the pair that `profiles.defaults.colorScheme` carries.<br>
So every apply reports those profiles, and a flag repoints them.<br>
Slot state stays in VS4 and `verify` in VS9.<br>
Closes R-16, R-17.<br>

## Facts established on this machine (do not re-derive)

- The live `settings.json` holds three profiles and one pin.<br>The PowerShell entry — the default profile — pins `Dainty Nord C1 L0`; the other two pin nothing.<br>`profiles.defaults.colorScheme` is `onecoat-dark`, a bare string.<br>
- The fixture carrying this case is `tests/fixtures/wt/settings.json`.<br>The vendored schema beside it allows a pair for a profile's `colorScheme`.<br>
- VS2 proved with a byte-exact golden that the CST never rewrites an untouched byte.<br>
- `jsonc::Edit` has three kinds today: `Set`, `Pair`, and `Element`.<br>None reaches inside the elements of an array, so a fourth kind is needed.<br>

## Approach

1. A pure query over the parsed document lists every `profiles.list` element carrying `colorScheme`.<br>It reports the name and the value, and a pair as both sides.<br>A profile without a `name` falls back to its `guid`.<br>
2. A new edit kind sets the pair on the named field of array elements that already carry it.<br>It never creates the field, so inheritance from `profiles.defaults` survives.<br>A wrong-typed field is an error, never a clobber.<br>
3. `exec::resolve` reads the pins from the file it just read.<br>So a plan built before a profile changed cannot name a stale one.<br>The pins leave `resolve` beside the changed keys, and `preview` prints them too.<br>
4. `--profile-color-scheme` takes `report` (the default) or `all`.<br>The plan carries the repoint edit only under `all`.<br>
5. `use` prints one line per pinned profile.<br>Under `all` it prints one repointed line per profile instead.<br>
6. Tests: unit tests for the query and the edit.<br>An integration test repoints the fixture against a new golden.<br>Another proves a foreign `themes` entry survives the repoint.<br>
7. `docs/architecture.md` records the per-profile key in the writer contract.<br>

## Critical files & anchors

- `src/jsonc.rs` — decides the new edit kind and what the query returns.<br>
- `src/render/wt.rs` — decides which keys onecoat owns, and when the repoint edit joins the plan.<br>
- `src/exec.rs` — decides where the pins are read and how they leave `resolve`.<br>
- `src/main.rs` — decides the flag, its default, and the printed lines.<br>
- `tests/fixtures/wt/settings.json` — the pinned profile the tests rest on.<br>

## Verification

1. Gates, all clean:<br>`cargo fmt --all --check`<br>`cargo clippy --workspace --all-targets -- -D warnings`<br>`cargo test --workspace --all-targets`<br>`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`<br>
2. `cargo test --test wt_settings --test cli --test wt_schema` → all pass.<br>
3. Sandbox run with the fixture copied into a temp `LOCALAPPDATA`:<br>`onecoat use nord --dry-run` names the pinned profile and writes nothing.<br>`onecoat use nord` writes the three keys and names the pin.<br>`onecoat use nord --profile-color-scheme all` repoints, and `settings.json` is byte-equal to the new golden.<br>The same command again says `unchanged` and still names the pin.<br>
4. Live smoke: copy the live `settings.json` and fragment aside, then hash both.<br>`onecoat use nord` reports the PowerShell pin.<br>`onecoat use nord --profile-color-scheme all` repoints it to the pair.<br>Windows Terminal then shows that profile following onecoat, which needs your eyes.<br>Restore both files and re-hash.<br>
5. Second run of step 3: output unchanged, files byte-identical.<br>

## Assumptions & contingencies

- **`--profile-color-scheme` takes `report` or `all`, default `report`**: R-17 reports pins on every apply, so the flag only opts into writing.<br>If a bare flag is wanted, drop the value and keep `all` as its only behavior.<br>
- **A profile that pins nothing is never touched**: creating the key would stop inheritance, and `defaults` already carries the pair.<br>If every profile should be pinned, that is a later slice.<br>
- **The changed-key line names `profiles.list`**: a dotted key cannot name an array element, and the pin lines carry the names.<br>If per-profile key lines are wanted, the key type has to grow first.<br>
- **The other side of the pair carries the previous string**: the same rule as `profiles.defaults.colorScheme`.<br>A light appearance therefore stays on onecoat.<br>If the built-in fallback is wanted for profiles, flip the fill flag.<br>
- **The report comes from the resolve-time read**: dry-run and a real apply agree.<br>A profile removed since planning is not reported.<br>Reading earlier would break R-13.<br>

## As implemented

Everything above landed as written.<br>These are the points where the plan was adjusted while implementing it.<br>

- The fact above named the Command Prompt entry.<br>The pin belongs to the PowerShell entry, which is also WT's default profile.<br>
- `jsonc::pinned` returns `Pin { name, scheme }`.<br>A pair joins its sides as `dark/light`.<br>So a second `all` run prints the pair it found, not a bare name.<br>
- A profile without a `name` is labelled by its `guid`.<br>The fixture and the live file both carry names.<br>
- `ProfileScheme` lives in `src/render/wt.rs` and derives `clap::ValueEnum`.<br>`plan` keeps depending on `render`, never the reverse.<br>
- `PlannedWrite` gained `pins: Option<PinReport>`.<br>The fragment write carries `None`, because schemes are not profile pins.<br>
- `exec` builds the printed lines inside `resolve`.<br>So `preview` and `execute` print the same lines, and the read stays where R-13 put it.<br>
- The pair fill carries the previous string.<br>A profile pinned to a foreign scheme therefore keeps that name on the light side.<br>The plan's "a light appearance stays on onecoat" holds only when the previous pin was already onecoat.<br>
- `docs/architecture.md` gained a `wt pins` bullet, and its owned-values row now says the repoint needs the flag.<br>
- VS2's exact-stdout CLI tests gained the pin line in five assertions.<br>
- The live smoke's restore step was dropped.<br>The repoint is the wanted end state, so both files stay applied, and the pristine copies sit in `%TEMP%\vs3-live-aside\`.<br>The backup chain was checked instead: `settings.json.onecoat.bak` is byte-identical to a pre-run document re-derived from the aside copy.<br>
