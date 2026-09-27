# Code review

Scope: all Rust in the repository, so `src/`, `tests/`, and `tools/tidy/`.<br>
Revision: `b73486d` on `feat/vs3-wt-profile-pins`.<br>
The tree is about 5,800 lines of Rust, tests included.<br>

## Short answer

The Rust code is in good shape, and I found no bug that damages a user file.<br>
The pipeline is real: pure rendering, a plan of edits, and one executor at the edge.<br>
Every writer has a byte-exact golden, an idempotency twin, and a failure-path test.<br>
I found three issues worth fixing before VS4, and seven smaller notes.<br>
The most important one is that "this file parses" is looser than the file format.<br>

## What I ran

```sh
cargo fmt --all --check                             # clean
cargo clippy --workspace --all-targets -- -D warnings   # clean
cargo test --workspace --all-targets                # 132 passed, 0 failed
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps  # clean
cargo run -p tidy                                   # 3 checks clean
```

I also ran the release binary against sandbox roots, with `LOCALAPPDATA` and `APPDATA` pointed at a temp directory.<br>
Release startup measured about 8 ms per run of `--version` (20 runs), so N-3 has room.<br>

## What is good

- **The plan/execute split is honest.** `exec::resolve` reads every file and computes every byte before `execute` writes anything (`src/exec.rs:107`).<br>A missing or broken `settings.json` therefore fails the apply with the fragment untouched, and a test proves it.<br>
- **One write primitive, carefully ordered.** `replace` writes a temp file, syncs it, copies the backup, then renames (`src/exec.rs:169`).<br>A failed backup removes the temp file and leaves the target alone.<br>
- **Preservation is proven, not asserted.** The splice keeps comments, key order, indentation, CRLF, and a missing trailing newline, and the goldens in `tests/fixtures/wt/` are byte-exact.<br>
- **Idempotency is tested at every layer.** `a_second_splice_is_byte_identical`, `a_second_repoint_is_byte_identical`, and `a_second_use_neither_rewrites_nor_rebacks_up` pin both the bytes and the mtimes.<br>
- **Type discipline is consistent.** `Theme<Parsed>` and `Theme<Validated>` make rendering unreachable from an unvalidated theme, `Key` and `HexColor` parse at the boundary, and there is no `unwrap`, no `unsafe`, and no dead `#[allow]` outside `non_snake_case` for Windows Terminal's own key spellings.<br>
- **Errors carry the file, the key, and the next action**, which is what R-42 asks for.<br>`JsoncUnparseable` even names the line and column.<br>
- **The repo's own checks have their own fixtures.** `tools/tidy` has passing and failing trees per check, so a check that stops firing fails its own test.<br>
- **The negative controls are real.** `tests/wt_schema.rs` proves the vendored schema rejects a renamed magenta role and an alpha colour, so the passing schema test means something.<br>

## Findings

Ordered by what I would fix first.<br>

### 1. JSONC parsing accepts text that is not JSON, so R-14 checks too little (medium)

`src/jsonc.rs:129` parses with `ParseOptions::default()`.<br>
That value turns on single-quoted strings, unquoted keys, missing commas, hex numbers, and unary plus.<br>
onecoat therefore splices and reports `wrote` on a document that is not JSON, and `verify_written` calls it valid.<br>

How to see it: a `settings.json` written with single quotes and one missing comma still applies.<br>

```
$ onecoat use nord --targets wt
wrote ...\settings.json
```

The file keeps `'profiles'` in single quotes and stays unreadable for a strict JSON reader.<br>
I did not verify how Windows Terminal treats that file, but its own settings files are JSON with comments.<br>
The contract problem is certain: "no longer parses" now means "no longer parses under jsonc-parser's lenient mode".<br>

Fix: build `ParseOptions` with comments and trailing commas on, and the other five flags off.<br>
Add a test that a single-quoted document and a document with a missing comma are rejected with file, line, and column.<br>

### 2. A byte-order mark blocks every apply (medium)

A UTF-8 BOM in front of the first `{` makes the document unparseable.<br>
The user sees `not valid JSONC: Unexpected token at line 1, column 1`, which does not mention a BOM.<br>
Windows users produce such files easily: Windows PowerShell 5.1 writes a BOM by default, and editors offer it as an encoding.<br>

How to see it: write `\ufeff{\n    "profiles": { "list": [] }\n}\n` as `settings.json`.<br>
`onecoat use nord` exits 3 and writes nothing, which is safe but not actionable.<br>

Fix: strip one leading BOM when reading a file, or detect it and say so in the message.<br>
If you strip it, keep it when writing, so onecoat does not change an unrelated byte.<br>

### 3. `[targets.omp]` and `[targets.herdr]` accept any key name (medium)

`is_token_name` (`src/validate.rs:270`) only checks that a key looks like an identifier.<br>
So `[targets.omp] mdHeadng = "#81A1C1"` validates, and `[targets.herdr] panel_bgg = ...` validates.<br>
Every other section rejects unknown keys, and `[palette]`, `[ansi]`, and `[targets.wt]` list the accepted names.<br>
When VS5 and VS7 land, those misspellings will be silently ignored and one token will keep its default colour.<br>

Fix: in VS5 and VS7, validate the token sections against the renderer-owned token lists, which those slices introduce.<br>
Keep the error shape of `UnknownKey`, so `accepted keys are ...` stays the pattern.<br>

### 4. A strange profile `colorScheme` stops the whole apply, even in report mode (medium-low)

`jsonc::pinned_scheme` (`src/jsonc.rs:358`) errors when a profile's `colorScheme` is neither a string nor a pair with at least one side.<br>
`exec::report_pins` (`src/exec.rs:153`) runs in both modes, so the default `report` mode fails too, although it writes nothing to that key.<br>
The message then blames `profiles.list`, which is the array, not the element.<br>
An empty pair `{}` is valid per the vendored `SchemePair` definition, which has no required properties.<br>

How to see it: a `profiles.list` element with `"colorScheme": {}` makes `onecoat use nord --targets wt` exit 3 with `profiles.list is not a JSON object or a string`.<br>

Fix: treat an unreadable pin as a pin that cannot be named, and report it in report mode.<br>
Only fail under `--profile-color-scheme all`, where onecoat actually writes that key.<br>
Name the element, for example `profiles.list[3].colorScheme`, not the array.<br>

### 5. A write can be reported with no changed key (low)

`splice` (`src/jsonc.rs:112`) decides `changed` by comparing serde values.<br>
`execute` decides whether to write by comparing bytes.<br>
When only key order or spacing inside the owned array element differs, onecoat rewrites the file and lists no key.<br>
A hand-edited fragment hits this: `use nord --dry-run` prints `would write ...schemes.json` with no key under it.<br>

Fix: push the key when the rendered text changed, not when the value changed.<br>
That keeps the dry-run report and the write decision on the same question.<br>

### 6. `list` requires `LOCALAPPDATA` although it does not read it (low)

`Paths::resolve()` (`src/targets.rs:13`) needs both variables and runs before command dispatch (`src/main.rs:77`).<br>
`list` only needs the user theme directory under `APPDATA`.<br>
The message also states no action: `onecoat needs it to locate the target paths`.<br>

Fix: resolve only the paths a command uses, or name the concrete action in the message.<br>

### 7. The purity gate misses `std::time` (low)

`REACHES` in `tools/tidy/src/checks/purity.rs:10` lists `env`, `fs`, and `process`.<br>
A pure module could call `SystemTime::now()` and pass the check, so N-5 is not fully enforced.<br>
Adding `time` is safe today: no pure module uses it.<br>

### 8. Small copies and unused flexibility (low)

- `to_map` (`src/model/theme.rs:216`) deep-copies every parsed TOML value into a fresh `BTreeMap`.<br>`toml::Table` is already an ordered map of `Value`, so `RawTheme` could hold it and skip the copy.<br>
- `Edit::Repoint` carries a `Fill`, but production only builds it with `Fill::FromString` (`src/render/wt.rs:165`).<br>
- `OverrideValue::Index` and `OverrideValue::Text` are parsed and tested but not yet read by any renderer.<br>That is expected before VS5 and VS7, and it is the one place where the "no dead code" rule is ahead of the slices.<br>

### 9. `docs/architecture.md` lists modules that do not exist yet (low)

The Layout section names `src/render/herdr.rs`, `src/render/omp.rs`, `src/state.rs`, `src/coherence.rs`, and `src/appearance.rs` (`docs/architecture.md:56`).<br>
The document is the target contract, so this is defensible, but a reader cannot tell the built parts from the planned ones.<br>
The error list already does this well: `ExternalCheckFailed`, `Drift`, and `Incoherent` are marked as arriving later.<br>

Fix: mark the unbuilt entries with their slice, in the style of the error list.<br>

### 10. Test gaps (low)

- No test applies a **light** theme end to end, because the bundled set holds only `nord`.<br>The light half of R-15 is proven only at the splice level.<br>VS4 arrives with slot state, so a light fixture is worth adding now.<br>
- `tests/unit/model/reserved.rs` pins `OMP_BUILTIN_THEMES.len() == 101`.<br>The name promises coverage of the harness registry, and a count cannot prove it.<br>VS5's plan reads the real list, so rename the test until then.<br>

## Should the theme parse use serde?

No. Keep the hand-written shape check, and keep serde where it already is.<br>
I built a derive-based model of the same file and ran it beside `onecoat list` over the same nine theme files.<br>
The derive covers the seven root keys and nothing else.<br>
Every section that carries real rules is keyed by a domain vocabulary: sixteen base16 names, sixteen Windows Terminal ansi names, three target sections, and per-section key rules.<br>
The derive accepts `[targets.vscode]`, a `base0a` beside `base0A`, and any token name, so `validate` keeps all of those checks either way.<br>
What it would replace is the `string_field` calls and the `match table.get` arms in `Theme::parse`, which are seventy lines of the clearest code in the file.<br>

| case | serde derive | onecoat today |
| --- | --- | --- |
| missing `name` | `missing field \`name\``, line 1 | `` `name` is missing; add `name` to the theme file `` |
| `name = 42` | `invalid type: integer \`42\`, expected a string` | `` `name` must be a string; found an integer `` |
| unknown root key | `unknown field \`foo\`, expected one of ...` | `` unknown key \`foo\` in the theme file; accepted keys are ... `` |
| `[targets.vscode]` | accepted | rejected, with the accepted sections named |
| `base0a` beside `base0A` | accepted | rejected, with the accepted spellings named |
| `appearance = "dim"` | `unknown variant \`dim\`` | `appearance = "dim" is not an appearance; use "dark" or "light"` |
| bad id character | accepted unless a newtype re-implements the rule | rejected by the one rule that already exists |

Serde's messages gain the line and the column, which the current shape errors lack.<br>
They lose the file, the remedy, and the section name, and R-42 asks for all three.<br>
A fully typed derive would close the two "accepted" rows, at the cost of a second list of the sixteen palette names beside `Base16Entry::ALL`, and of moving value checks into the parse phase, which the architecture deliberately keeps out.<br>
It would also make `Deserialize` part of the domain types, which no type in the repository implements today.<br>

If the missing line and column matter, add them as their own change.<br>
`toml::Spanned` exists in `toml` 1.1, and using it means a serde-shaped document layer, so it is a separate decision with its own tests.<br>

Serde stays in the three places it already serves: `ListRow`, the Windows Terminal output structs, and the `serde_json::Value` payloads inside `jsonc::Edit`.<br>
It cannot replace the CST splice, because `serde_json` drops the comments, the key order, and the line endings that R-12 protects.<br>

## Deliberate trade-offs, not defects

- **All files resolve before any file is written.**<br>This is what makes "fail with every file still as it was" true, and it widens the window between the read and each rename.<br>A concurrent editor inside that window loses the race.<br>The plans record the choice.<br>
- **The fragment upsert replaces the whole element.**<br>A comment inside onecoat's own scheme object is dropped.<br>The document around it, including comments and other schemes, survives byte for byte.<br>
- **The root `theme` pair keeps the built-in on the other side, while `colorScheme` carries the previous string.**<br>The asymmetry is on purpose and argued in the VS2 plan.<br>
- **`settings.json` must exist.**<br>onecoat never invents a document that Windows Terminal has to own.<br>

## Suggested order

1. Findings 1 and 2, because both decide whether onecoat can read the file at all.<br>
2. Finding 3, while the token lists are being written in VS5.<br>
3. Finding 4, and a light-theme end-to-end test with it.<br>
4. Findings 5 to 10 as cleanup, in one pass, with the doc fix.<br>

## Where the findings stand now

Every finding above was written against `b73486d`.<br>This is the state at VS5 on `feat/vs5-omp-theme-files`.<br>

- Finding 1, the lenient JSONC parse: still open.<br>`src/jsonc.rs:134` still parses with `ParseOptions::default()`.<br>
- Finding 2, the byte-order mark: still open.<br>Nothing reads or strips a BOM.<br>
- Finding 3, unknown token names: fixed for omp in VS5.<br>`src/render/omp.rs:371` refuses a `[targets.omp]` key that is not a token.<br>The herdr half belongs to VS7.<br>
- Finding 4, a strange profile `colorScheme`: still open.<br>`src/jsonc.rs:350` still fails the whole apply in both modes.<br>
- Finding 5, a write with no changed key: still open.<br>`splice` still compares serde values (`src/jsonc.rs:115`).<br>
- Finding 6, `list` needing `LOCALAPPDATA`: still open.<br>`run` resolves every path before dispatch (`src/main.rs:86`).<br>
- Finding 7, the purity gate and `std::time`: still open.<br>
- Finding 8, small copies: `OverrideValue::Index` and `OverrideValue::Text` are read now, by the omp renderer (`src/render/omp.rs:387`).<br>`to_map` still copies each value (`src/model/theme.rs:223`), and `Repoint` still carries a `Fill` that is always `FromString`.<br>
- Finding 9, the layout section: still open.<br>`herdr.rs`, `coherence.rs` and `appearance.rs` are still unmarked.<br>
- Finding 10, test gaps: still open.<br>VS5 verified the vendored reserved list against the harness, but the test still pins only `len() == 101`.<br>
