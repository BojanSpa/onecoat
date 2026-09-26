# General

- No human-time estimates; agents implement, test, and verify everything.

# Code style

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

# Testing

- Assert observable behavior, boundaries, and error cases; never implementation details or incidental wording.
- Use real fixtures, not mocks: actual commented `settings.json` and `config.toml`.
- Assert byte-exact writer output: only intended spans change, nothing else shifts.
- Pair every writer test with an idempotency check: the second run is a no-op.
- Keep tests deterministic and isolated: no clock, no network, no shared temp paths.
- Name each test for the behavior it proves; one behavior per test.
- Delete tautologies, `assert!(is_ok())` shapes, and tests pinning incidental values.
- Land a failing-before regression test with every bug fix.

# Definition of Done

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```

- Public items documented.
- Behavior proven by running the real binary, not tests alone.
- No new `unsafe` without a safety justification.
- No dead code, stubs, `#[allow(dead_code)]`, or leftover TODO markers.
- Docs and changelog updated for every user-visible change.
