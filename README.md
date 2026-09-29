# onecoat

onecoat applies one theme to Windows Terminal, Herdr, and the omp harness, so all three look like a single surface when they sit on screen together.

A theme is one TOML file with a base16 palette. base16 is a sixteen-color convention (`base00` to `base0F`) where every color has a job: background, foreground, red, green, and so on. onecoat renders that palette into each program's own configuration format and writes only the keys it owns, so comments, key order, and line endings survive in the rest of the file.

## What onecoat wants to be

- **One source of truth.** A theme is defined once, and all targets derive their colors from the same palette.
- **Safe writes.** Every write goes through a temporary file and a `<file>.onecoat.bak` backup. The command reads the file back and parses it before it reports success.
- **Verification over trust.** `verify` re-derives the expected values from the theme and reports drift (a value that no longer matches the theme) key by key, so a hand-edited color cannot hide.
- **Appearance following.** Once applied, every target switches between dark and light on its own when Windows appearance changes.
- **Import.** An existing base16 file, omp theme, or Windows Terminal scheme can become a onecoat theme.
- **Small and quiet.** One static binary, no network access, no background service, and a cold start fast enough to run from a keybinding.

The behavior contract is in [`docs/requirements.md`](docs/requirements.md). The design is in [`docs/architecture.md`](docs/architecture.md).

## Current state

Early development. All three targets work end to end.

Working today:

- `onecoat list` prints the available themes and where each one comes from. `--json` prints the same data for scripts.
- `onecoat use <id>` writes onecoat's Windows Terminal fragment (a JSON file in Windows Terminal's Fragments folder), then sets the owned keys in `settings.json`: the root `theme` pair, the window entries in `themes`, and the `profiles.defaults.colorScheme` pair. Profiles that pin their own scheme are reported, and `--profile-color-scheme all` repoints them.
- The same command writes the applied slot's omp theme file under the omp agent directory (`PI_CODING_AGENT_DIR`, or `%USERPROFILE%\.omp\agent`) and pins that file's name in the agent directory's `config.yml`. The pin is written once and later applies rewrite only the file, which omp reads when a session starts, so a new session picks up a change.
- The same command writes the `[theme]` block in `%APPDATA%\herdr\config.toml` with a TOML-preserving edit: the base theme, both slot names, the shared accents, and the applied slot's surface layer. The candidate is checked with `herdr config check` before it replaces the file, and a running herdr server is asked to reload.
- `onecoat current` prints the theme assigned to each slot and the targets the last apply wrote.
- `onecoat doctor` reports what onecoat sees right now and writes nothing: the resolved target paths, whether the `herdr` program is on `PATH` and its socket exists, the profiles that pin their own scheme, and how old the state file is. `--json` prints the same report for scripts.
- `--dry-run` prints the planned writes and the keys they change. `--targets` limits an apply to the named targets.
- One theme ships in the binary, `nord`. Your themes go in `%APPDATA%\onecoat\themes\*.toml` and can shadow a bundled theme with the same id.

Not built yet:

- The `verify`, `watch`, `import`, and `validate` commands.

[`work/roadmap.md`](work/roadmap.md) tracks the remaining slices, from VS9 on.

## Try it

```sh
cargo build --release
onecoat list
onecoat use nord --dry-run
onecoat use nord
```

Start with `--dry-run`: it shows the writes without touching the disk. `use` writes to the live Windows Terminal files, and every file it replaces keeps a `.onecoat.bak` copy next to it.

## Development

The definition of done gates:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

CI runs all four on every pull request. [`AGENTS.md`](AGENTS.md) holds the workflow and code rules.

## Limits

- Windows only for now. The targets are Windows programs, and paths come from `LOCALAPPDATA`, `APPDATA`, and the omp agent directory.
- Remote or SSH Herdr clients are out of scope, and so are OS accent color, PowerShell profiles, VS Code, and any fourth target.
- No network access, no telemetry, and no stored secrets.
- Tests never touch the live config files. They run against temp roots and committed fixtures.

## Documentation

- [`docs/requirements.md`](docs/requirements.md): the behavior contract, R-1 to R-42.
- [`docs/architecture.md`](docs/architecture.md): components, target contracts, and the render, plan, execute pipeline.
- [`work/roadmap.md`](work/roadmap.md): the build order, VS1 to VS14.
