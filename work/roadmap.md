# Roadmap

Slices are vertical: each one ends in something you can watch change in a running program. A new layer, a refactor, or scaffolding that does nothing yet is not a slice.<br>
One slice is one plan at `work/plans/<slug>.md`. Write that plan when the slice starts, and keep it small enough for one pass to implement and verify.<br>
Slice IDs run `VS1`, `VS2`, and so on down this page, and a slice keeps the number it was given. A new slice takes the next free number, and its plan states the ID in Context.<br>
A slice is done when every step in its plan's Verification section passes, including the smoke run against the live target.<br>
The first column carries a checkbox: `[x]` marks a slice whose plan passed every step, `[ ]` marks the rest.<br>
Each requirement ID appears once, on the slice that completes it. An earlier slice may implement part of that requirement.<br>

## M0: Windows Terminal target

| Done | VS | Slice | Deliverable | Closes |
| --- | --- | --- | --- | --- |
| [x] | VS1 | `wt-scheme-fragment` | Defines the theme model, validates it, ships one bundled theme, derives the scheme from base16 roles, answers `list`, and writes the fragment when `use` runs | R-1, R-2, R-3, R-5, R-6, R-9, R-10, R-11, R-32, R-41, R-42, N-1 to N-6 |
| [x] | VS2 | `wt-settings-splice` | Edits the three owned keys in `settings.json` through the JSONC CST, restores the file from its backup and fails the apply when the result no longer parses, and adds `--dry-run` with the key-level diff plus `--targets` | R-12, R-13, R-14, R-15, R-23, R-31 |
| [x] | VS3 | `wt-profile-pins` | Reports profiles that pin a scheme, repoints them under `--profile-color-scheme all`, and keeps foreign `themes` entries | R-16, R-17 |
| [ ] | VS4 | `wt-slot-state` | Persists the slot assignment, accepts `--slot`, and answers `current` | R-8, R-25 |

## M1: omp target

| Done | VS | Slice | Deliverable | Closes |
| --- | --- | --- | --- | --- |
| [ ] | VS5 | `omp-theme-files` | Resolves the agent directory, maps every token, writes the two stable theme files, pins `theme.dark` and `theme.light`, and reads the reserved-name list from the harness | R-7, R-21, N-7 |
| [ ] | VS6 | `omp-live-reload` | Rewrites a theme file only when its bytes change, and proves a running session repaints without a restart | R-22, R-24 |

## M2: Herdr target

| Done | VS | Slice | Deliverable | Closes |
| --- | --- | --- | --- | --- |
| [ ] | VS7 | `herdr-theme` | Writes `[theme]` and its appearance layers with `toml_edit`, gates the result on `herdr config check`, and reloads through `herdr server reload-config` when a server socket exists | R-18, R-19, R-20 |

## M3: Inspection and appearance

| Done | VS | Slice | Deliverable | Closes |
| --- | --- | --- | --- | --- |
| [ ] | VS8 | `doctor` | Reports resolved paths, detected binaries and sockets, profile pins, and state age, and writes nothing | R-30 |
| [ ] | VS9 | `verify-drift` | Re-derives the artifacts and compares them with the files on disk, reports drift per target with the offending keys, treats reformatting as no drift, and exits 1 from `--check` on any drift or coherence failure | R-26, R-27, R-29 |
| [ ] | VS10 | `validate-coherence` | Adds the `validate` command, checks ΔE00 spacing between surfaces, and compares roles across targets | R-4, R-28 |
| [ ] | VS11 | `appearance-watch` | Watches the registry for appearance changes, applies the matching slot, repairs drift under `--repair`, and proves both pairs per target | R-37, R-38, R-39 |

## M4: Import and packaging

| Done | VS | Slice | Deliverable | Closes |
| --- | --- | --- | --- | --- |
| [ ] | VS12 | `import-base16` | Turns base16 sources into canonical themes without losing a palette value | R-35 |
| [ ] | VS13 | `import-existing` | Imports omp themes and Windows Terminal schemes, re-renders them exactly, marks derived themes, and completes the command set | R-33, R-34, R-36, R-40 |
| [ ] | VS14 | `packaging` | Ships the binary with Windows Terminal keybinding and fragment documentation, plus a README | none |
