# VS<N> — `<slice-slug>`

<!-- Slice plan. Copy to `work/plans/vs<N>-<slug>.md` when the slice starts and commit it with the slice.
     One plan, one slice; the plan states its ID and the requirement IDs it closes.
     Required sections, in order: Context, Facts established on this machine, Approach,
     Critical files & anchors, Verification, Assumptions & contingencies, As implemented.
     Banned sections: Non-Goals, Out of Scope, Alternatives Considered, Risks/Mitigations, Future Work,
     and cleanup or changelog tails; a material scope boundary gets one inline line, never a section.
     No human-time estimates anywhere.
     Bar to clear: an agent that never saw the planning conversation can infer the intent, and every
     judgement call it could get wrong is pinned. A plan is the veto list, not the design document:
     signatures, schema paths, and API shapes are lookups the implementer does, not plan content.
     If a plan runs past a page, the slice is too big or the plan is repeating lookups.
     Plain words, active voice. No sentence runs past 20 words; split it instead of adding a clause.
     Every sentence ends with `<br>`, as in docs/: a paragraph puts one sentence per line, a list item
     keeps its text on one line with `<br>` between sentences. The rendered plan then breaks where the
     author broke, and the diff stays per sentence.
     Delete every comment once the section is filled in. -->

## Context

<!-- Up to six sentences: the literal ask, the need it serves, the intended end state.
     Name the slice ID and say which work is deliberately left to later slices.
     End with the requirement IDs this slice closes, as in "Closes R-1, R-2, N-1 to N-6.". -->

## Facts established on this machine (do not re-derive)

<!-- Facts that cost real time to re-derive and live nowhere in the repo: versions, install and config
     paths, external formats, platform behaviours, and what an earlier slice already proved. Facts that
     are in the repo — key shapes in a vendored schema, signatures, API behaviour — are lookups, not
     plan content; name the file instead of copying from it. -->

- FACT — how it was established.<br>

## Approach

<!-- Ordered steps grouped by behavior, never by file, each ending with a tree that builds and the tests
     so far passing. A step states the intent and the decisions that could go the other way — naming,
     key spellings, ordering, error behavior — not a transcription of the signatures, fields, and
     messages the implementer writes anyway. Derive new symbols from the owning R-N in
     docs/requirements.md and update the affected contract in docs/architecture.md in the same step. -->

1. STEP — what changes, and the decision in it that could go the other way.<br>
2. STEP<br>

## Critical files & anchors

<!-- At most five files that disambiguate non-obvious work, one line each: what the file decides.
     Omit files the approach already makes obvious; anchors are hints, not line numbers to trust. -->

- `PATH` — decides REASON.<br>

## Verification

<!-- End-to-end proof from the repository root, with the shell and the prerequisites (env vars, fixtures,
     build profile, and how to restore anything the checks mutate).
     Gates first, then the checks that pin this slice's requirement IDs, then a hand run of the new
     behavior, then the smoke against the live target, then the second-run evidence that nothing shifted.
     Every step is an exact command or action with its expected observable result; a green suite is not
     proof, and the slice is done when every step here passes. -->

1. Gates, all clean:<br>`cargo fmt --check`<br>`cargo clippy --all-targets -- -D warnings`<br>`cargo test --all-targets`<br>`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`<br>
2. COMMAND → EXPECTED OBSERVABLE RESULT.<br>
3. Live smoke: COMMAND → EXPECTED RESULT, including the change seen in the running program.<br>
4. Second run: COMMAND → unchanged output, files byte-identical.<br>

## Assumptions & contingencies

<!-- The veto list: decisions a reviewer could want the other way, each with the reason it was taken and
     the fallback if reality disagrees. Implementer decisions belong in Approach. -->

- **DECISION**: default taken, because REASON.<br>If REALITY, do FALLBACK instead.<br>

## As implemented

<!-- Record the points where reality adjusted the plan — renamed symbols, extra tests, substituted
     commands, moved contracts — with the reason, plus the additions the snippets implied but did not
     spell out. Short and factual; it is the plan's own drift report, and reviewers read it. -->

Everything above landed as written.<br>These are the points where the plan was adjusted while implementing it.<br>

- ADJUSTMENT — why.<br>
