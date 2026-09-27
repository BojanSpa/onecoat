# VS<N> — `<slice-slug>`

<!-- Slice plan. Copy to `work/plans/vs<N>-<slug>.md` when the slice starts and commit it with the slice.
     One plan, one slice; the plan states its ID and the requirement IDs it closes.
     Required sections, in order: Context, Facts established on this machine, Approach,
     Critical files & anchors, Verification, Assumptions & contingencies, As implemented.
     Banned sections: Non-Goals, Out of Scope, Alternatives Considered, Risks/Mitigations, Future Work,
     and cleanup or changelog tails; a material scope boundary gets one inline line, never a section.
     No human-time estimates anywhere.
     Bar to clear: an implementer who never saw the planning conversation executes every step top to bottom
     with zero design decisions and no questions, and can tell at each step whether it succeeded.
     Delete every comment once the section is filled in. -->

## Context

<!-- 2 to 4 sentences: the literal ask, the need it serves, the intended end state.
     Name the slice ID and say which work is deliberately left to later slices.
     End with the requirement IDs from docs/requirements.md this slice closes,
     as in "Closes R-1, R-2, R-3, N-1 to N-6.". -->

## Facts established on this machine (do not re-derive)

<!-- Facts an implementer would otherwise re-derive: toolchain and tool versions, install and config paths,
     external schema or file-format versions, platform behaviours the slice leans on, and what an earlier
     slice already proved. Record how each was established (command output, file, tag) so a reader can
     re-check it when it looks stale. -->

- FACT — how it was established.

## Approach

<!-- Ordered, load-bearing change steps grouped by behavior, never by file.
     Keep them sequential: each step ends with a tree that builds and every test written so far passing.
     Each step carries: the concrete edit (verb, exact target, new behavior, never an area to "handle");
     existing code to reuse; the exact signature or literal for every new or changed symbol, error string,
     config key, and wire or JSON field; every callsite plus deletions for a rename, signature change, or
     removal; the rival pattern to avoid; and the empty, missing, conflict, and error handling for each
     new path. Derive new symbols from the owning R-N in docs/requirements.md, and update the affected
     contract in docs/architecture.md in the same step. -->

### 1. BEHAVIOR

- Reuse: PATH or SYMBOL
- Exact signature or literal: SIGNATURE
- Avoid: RIVAL PATTERN
- Empty, missing, conflict, error: HANDLING

### 2. BEHAVIOR

## Critical files & anchors

<!-- At most five files that disambiguate non-obvious work, and why each one matters.
     Omit files the approach already makes obvious. Anchors are hints, not line numbers to trust;
     the implementer re-reads before editing. -->

- `PATH` — SYMBOL or REGION decides REASON.

## Verification

<!-- End-to-end proof, run from the repository root; state the shell and the prerequisites (env vars,
     fixtures, build profile, restore steps for anything the checks mutate).
     Work down from the gates to the behavior: the gates in AGENTS.md, then the checks that pin this
     slice's requirement IDs, then a hand run of the new behavior outside the test suite, then the smoke
     run against the live target, then the second-run evidence that nothing else shifted.
     Every step is an exact command or action with its expected observable result — a green suite is not
     proof. The slice is done when every step here passes. -->

1. Gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` → all clean.
2. COMMAND → EXPECTED OBSERVABLE RESULT.
3. Live smoke: COMMAND → EXPECTED OBSERVABLE RESULT, including the change observed in the running program.
4. Second run: COMMAND → unchanged output, target files byte-identical to the first run.

## Assumptions & contingencies

<!-- Only decisions the user could override; implementer decisions belong in Approach.
     For each load-bearing assumption that may fail mid-execution, pre-decide the fallback so execution
     never stalls: "if reality is X, do Y instead", and name where the fallback lands. -->

- **ASSUMPTION**: default taken. If REALITY, do FALLBACK instead.

## As implemented

<!-- Record the points where reality adjusted the plan — renamed symbols, extra tests, substituted
     commands, moved contracts — with the reason, plus the additions the snippets implied but did not
     spell out. Short and factual; it is the plan's own drift report, and reviewers read it. -->

Everything above landed as written; these are the points where the plan was adjusted while implementing it.

- ADJUSTMENT — why.
