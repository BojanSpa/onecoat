# General

- No human-time estimates; agents implement, test, and verify everything.

# Docs

- `docs/requirements.md` — behavior contract and source of truth: `R-N` functional and `N-N` non-functional requirements with verification gates.
- `docs/architecture.md` — how onecoat works: components, target contracts, owned keys, and the pure render→plan→execute pipeline.
- `work/roadmap.md` — build order: vertical slices VS1–VS14 across milestones M0–M4; each plan closes its requirement IDs.

# Workflow

- The unit of work is one roadmap slice; the next one is the lowest-numbered unstarted `VS`.
- Never start a slice on your own: ask for approval of the next `VS`, and wait for a yes before writing its plan or code.
- A slice is done only when every step in its plan's Verification section passes.
- Implement against the owning `R-N`; update it and affected architecture contracts in the same PR.
- The PR that completes a slice is titled `feat(vs<N>): <what the slice does>`, and its body names the requirement IDs it closes.
- Only a slice PR carries a scope, as in `feat(vs2): ...`; every other PR title uses a plain prefix.
- All changes land through a PR; `main` is protected and rejects direct pushes.
- Never open a pull request on your own; push the branch and ask for approval, or await it.
- Commit subjects use conventional prefixes (`feat:`, `fix:`, `docs:`, `chore:`, `refactor:`, `test:`, `ci:`, `build:`).

# Plans

- Copy `work/plans/_template.md` to `work/plans/vs<N>-<slug>.md` when the slice starts, and commit the plan with the slice.
- Read the `plain-lang` skill (`skill://plain-lang`) before writing a plan.
- Keep every plan sentence under 20 words, and end each sentence with an explicit `<br>`, except a table row.
- A plan is the veto list, not the design document: pin the decisions a reviewer could veto and the facts that cost time to re-derive, not the signatures, schema paths, and API shapes the implementer will look up.
- Keep a plan under a page; a longer plan means the slice is too big or the plan is repeating lookups.
- The style rules cover plans being written or edited; merged plans are not rewritten for style alone.

# Code style

- Don't write comments, write self-evident code instead; contracts that code cannot carry live in `docs/architecture.md`.
- Separate items with one blank line; a closing brace is never the line before an item.
- Separate statements with one blank line when either one spans lines; a single-line `let` stays glued to the block below it.
- Newtype domain values; parse at the boundary so validation happens exactly once.
- Encode state as enums, not booleans: illegal states should be unrepresentable.
- Model phases with typestate: `Parsed`, `Validated`, `Written` are distinct types, not flags.
- Traits describe capabilities for generic bounds, never class hierarchies or inheritance.
- Match exhaustively on your own enums; never write catch-all `_ =>` arms.
- Errors are enums via `thiserror`; propagate with `?`, never `unwrap` outside tests.
- Take `&str`, `&[T]`, `impl Read`; return owned values or concrete iterator types.
- Prefer iterators and combinators over mutable loops; clone only deliberately, not reflexively.
- Private by default; narrow public APIs; no getters, setters, or blanket `Default` impls.
- Keep the core pure: theme→artifacts is just a function; IO stays at the edges.

# Dependencies

- Justify every new crate against N-1 (no network) and N-2 (single static binary).
- No async runtime, no network-capable crate, no telemetry; prefer std and the existing tree.

# Testing

- Unit tests live in the owning crate's `tests/unit/`, mirroring `src/`; the module declares them with `#[cfg(test)] #[path = "..."] mod tests;`. Integration tests stay in `tests/`.
- Assert observable behavior, boundaries, and error cases; never implementation details or incidental wording.
- Use real fixtures, not mocks: actual commented `settings.json` and `config.toml` in `tests/fixtures/`.
- Never touch the live target files; tests run against temp roots and committed fixtures only.
- Assert byte-exact writer output: only intended spans change, nothing else shifts.
- Pair every writer test with an idempotency check: the second run is a no-op.
- Keep tests deterministic and isolated: no clock, no network, no shared temp paths.
- Name each test for the behavior it proves; one behavior per test.
- Delete tautologies, `assert!(is_ok())` shapes, and tests pinning incidental values.
- Land a failing-before regression test with every bug fix.

# Definition of Done

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

- Repo conventions no compiler checks live in `tools/tidy`; `cargo run -p tidy` reports them compactly, and `cargo test --workspace` fails on any finding.
- Behavior proven by running the real binary, not tests alone.
- No new `unsafe` without a safety justification.
- No dead code, stubs, `#[allow(dead_code)]`, or leftover TODO markers.
- Docs and changelog updated for every user-visible change.
