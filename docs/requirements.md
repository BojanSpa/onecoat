# Requirements

`onecoat` applies one canonical theme to Windows Terminal, Herdr, and the omp harness, so all three look like a single surface when they sit on screen together.<br>

## Vocabulary

- **theme** — a canonical palette definition (a TOML file); the single source of truth for colors.
- **slot** — `dark` or `light`; the appearance a theme occupies.
- **target** — a program onecoat writes to: `wt`, `herdr`, or `omp`.
- **artifact** — the rendered output for one target and slot.
- **apply** — render, write, and reload the selected targets.
- **state** — what onecoat last wrote, per slot and target.
- **drift** — an owned value in a target file no longer matches the theme it came from.
- **coherence** — a palette's own surfaces staying perceptually distinct, so a raised panel reads as the same surface raised.

## Scope

In scope: the three targets on a Windows host; theme definition, application, verification, and import of existing colors; following OS appearance change.<br>

Out of scope: remote/SSH Herdr clients; Windows Terminal window themes delivered by fragments (the format forbids it); OS accent color, PowerShell profile theming, VS Code, or any fourth target; editing omp's built-in themes; theme marketplaces or download; a runtime cross-target coherence check, because the renderers share one palette and the committed writer fixtures pin each mapping.<br>

## Theme definition

- R-1 A theme is TOML with `id`, `name`, `appearance` (`dark` or `light`), `palette` (base16 `base00`–`base0F`), optional `[ansi]`, and optional `[targets.wt]`, `[targets.herdr]`, `[targets.omp]` overrides. Verified by: parse test over every bundled theme.
- R-2 All sixteen palette entries are required and parse as colors; a missing or malformed entry names the offending token. Verified by: error-case unit tests.
- R-3 `appearance` must agree with the luminance of `base00`; a dark theme with a light background is rejected. Luminance is WCAG relative luminance; `light` requires ≥ 0.5 and `dark` requires < 0.5. Verified by: inverted-pair test.
- R-4 `validate` warns when adjacent surfaces (`base00`/`base01`/`base02`) are perceptually indistinguishable, and fails under `--strict`. Verified by: too-close surface pair.
- R-5 User themes in `<config>/themes/*.toml` shadow bundled themes by `id`; `list` reports each theme's origin. Verified by: shadowing fixture.
- R-6 Theme ids must not collide with omp built-in theme names (`dark`, `light`, `titanium`, …), because built-ins win over custom files. Verified by: rejecting such an id.
- R-7 Every generated omp theme file is named `onecoat-<slot>.json`; the name is stable across theme switches. Verified by: CLI tests that apply both slots and read the written paths back.

## Application

- R-8 `onecoat use <id>` assigns the theme to its declared slot, then applies the current slot assignment to every enabled target. `--slot` overrides the assignment. Verified by: CLI test reading the state back after a plain and a `--slot` apply.
- R-9 Applying is idempotent: a second identical apply leaves every file byte-identical and does not rewrite unchanged files.
- R-10 Each file is written atomically through a temporary file plus replace; a failure leaves the previous content intact.
- R-11 Before replacing a file, onecoat preserves the previous content as `<file>.onecoat.bak`.
- R-12 Unrelated keys, key order, comments, indentation, and line endings survive every write to `settings.json` and `config.toml`. Only the keys onecoat owns change.
- R-13 Target files are re-read immediately before writing, so a concurrent writer (Windows Terminal's settings UI, `herdr reload config`) is never clobbered by a stale plan.
- R-14 After writing, onecoat re-reads and parses each file it changed; a file that no longer parses is restored from its backup and the apply fails.
- R-15 Windows Terminal artifact: both slot color schemes land in onecoat's own fragment; the root `theme` pair, the `themes` window entries, and `profiles.defaults.colorScheme` pair are spliced into `settings.json`.
- R-16 Windows Terminal ownership: scheme data lives in `<Fragments>/onecoat/`, while window themes and root keys are spliced into `settings.json`, which fragments cannot carry.
- R-17 Profiles that pin their own `colorScheme` are reported on every apply; `--profile-color-scheme all` repoints them to the scheme pair. Verified by: fixture with a pinned profile.
- R-18 Herdr artifact: `[theme] name = "terminal"`, `auto_switch = true`, `dark_name`, `light_name`, plus `[theme.custom]` shared tokens and `[theme.custom.dark]`/`[theme.custom.light]` layers.
- R-19 A generated herdr config must pass `herdr config check`; onecoat runs the check on the candidate before it replaces the original, through `HERDR_CONFIG_PATH`, skips it when the binary is missing, and fails the apply on an issue the original did not have.
- R-20 Herdr reload: run `herdr server reload-config` when a server socket exists; never restart, signal, or kill a client, and say so when only an interactive reload can pick the change up.
- R-21 omp artifact: the applied slot's theme file carries every token the harness requires, and that slot's key under `theme` in `config.yml` is pinned to `onecoat-<slot>`. Verified by: a token-completeness test over the vendored list and a fixture config splice.
- R-22 No client is ever restarted, and no settings write happens while the pinned names are unchanged. omp reads the pinned theme file when a session starts, so a rewrite recolours the next session, not a running one. Verified by: a CLI test that rewrites the theme under an unchanged pin, and a live session smoke that shows a running session keeping its colours.
- R-23 `--targets` limits an apply to the named targets and leaves every other target's files untouched.
- R-24 An apply whose rendering is unchanged rewrites nothing, so every file and its stamp stay as they were. Verified by: a second apply leaves every file stamp untouched.

## Inspection

- R-25 `current` prints the assigned theme per slot and the targets last applied, from onecoat's state. Verified by: CLI test over a missing, a recorded, and a malformed state.
- R-26 `verify` re-derives the expected artifacts from the canonical themes and compares them against the current files of every target onecoat owns; drift is reported per target with the offending keys. Verified by: CLI tests over an edited fragment color, a removed owned key, a shadowed theme, and an untouched sandbox.
- R-27 Drift is semantic: a reformatted file with the same values is not drift; a hand-edited color is. Verified by: CLI tests over a reformatted omp theme file and an added herdr comment line.
- R-29 `verify --check` exits 1 on any drift, 0 otherwise. Verified by: CLI tests over a clean apply, a drifted fragment, an edited herdr theme name, and a target a narrowed apply skipped.
- R-30 `doctor` reports resolved paths, detected binaries and sockets, profile pins, and state age, and writes nothing. Verified by: CLI tests over missing entries, a found program, a pin, a state age, and a broken settings file.
- R-31 `use` and `watch` accept `--dry-run`, which prints planned writes and a key-level diff without touching the disk.
- R-32 Read commands accept `--json` and print machine-readable output on stdout.

## Import

- R-33 `import` accepts a base16 file, an omp theme JSON, or a Windows Terminal scheme by name.
- R-34 Import is lossless for the imported target: applying the imported theme reproduces that target's values exactly. Verified by: round-trip test per source kind.
- R-35 Values base16 cannot express are kept in `[targets.*]` overrides rather than discarded, so re-rendering stays lossless.
- R-36 A theme derived from a Windows Terminal scheme is marked derived, and `validate` warns that its palette is best-effort.

## Appearance

- R-37 `watch` applies the slot matching Windows appearance on change, driven by a registry notification rather than polling.
- R-38 `watch --repair` also reapplies when an owned value has drifted.
- R-39 Once applied, all three targets follow OS appearance without onecoat running: Windows Terminal through its theme/scheme pairs, Herdr through `auto_switch`, omp through its dark and light slots.

## Interface

- R-40 Commands: `list`, `use`, `current`, `verify`, `doctor`, `watch`, `import`, `validate`.
- R-41 Exit codes: 0 success; 1 verification failure; 2 usage error; 3 apply, IO, or parse failure. Diagnostics on stderr, data on stdout.
- R-42 Every failure message names the file, the key, and the action that resolves it.

## Non-functional

- N-1 No network access, ever.
- N-2 A single static binary with no runtime dependency and no background service; it must be safe to invoke from a keybinding.
- N-3 Cold start under 50 ms, release build, reference machine.
- N-4 Writes are confined to the three targets' config files, onecoat's own directory, and `.onecoat.bak` companions.
- N-5 Rendering is pure and platform-independent; only path resolution, appearance detection, and process signalling are Windows-specific.
- N-6 No telemetry, no stored secrets.
- N-7 omp's agent directory is honored from `PI_CODING_AGENT_DIR` with `~/.omp/agent` as the fallback; `--profile` isolated agent directories are out of scope. Verified by: CLI tests that point `PI_CODING_AGENT_DIR` at a sandbox agent directory.

## Acceptance

Per-change gates live in `AGENTS.md`; this document defines what the gates are applied to.<br>
