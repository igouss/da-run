# ADR-0004 (da-run2): the dispatch journal rides `da-state check`

**Status:** accepted, implemented 2026-07-18. **Area:** `bins/da-state`,
`crates/adapter-fs/src/events.rs`, `engine/bin/run`, SKILL.md.

## Context

events.jsonl is the instrument that makes out-of-band operator edits visible (REQ-037):
capture classifies every fingerprint change of the operator-editable surface by the event
that explains it. The dispatch events came from SKILL.md prose — "after every dispatch, run
`bin/run mark`" — so a skipped mark silently misclassified legitimate stage work as
out-of-band. A prompt-enforced instrument measuring prompt discipline is self-defeating.

## Decision

`da-state check` — the one call `da-stage.js` refuses to dispatch without — journals
`dispatch:<kind>` itself on every **allowed** check (opt-out `--no-journal`; refused checks
journal nothing, since no dispatch follows). Journaling stays an adapter concern
(`events.rs`); the domain crate is untouched. `bin/run mark` remains for ad-hoc triggers
only.

Because check runs BEFORE the stage works (mark ran after), classification changed: a
transition is now explained by its **initiating** event — `dispatch:`/`steer` at the window's
start is stage-work; a `gate:` trigger at the window's end explains its own report write
(engine-write); everything else is out-of-band. A surface change landing between a seal and
the NEXT dispatch's check is therefore out-of-band even though a check closes the window —
exactly the operator-edit case the instrument exists to see.

Both sides compute the fingerprint (sha256 over sorted `<rel> <sha256(content)>` lines of
spec.md + `stages/*/output/**`, steer files excluded, hidden files excluded as bb's `fs/glob`
never matches them): parity is pinned by a shared fixture digest in `events.rs`'s tests,
`bin/run --selftest`, and an fs-walk integration test.

## Consequences

- The instrumentation gap is structural, not disciplinary: no dispatch can happen (through
  the skill) without its journal entry, and SKILL.md no longer asks for a separate call.
- Two journal writers exist (bb for setup/seal/gate/record-commit, Rust for dispatches);
  the parity pins are the contract between them. The first live run exposed exactly such a
  divergence (`.gitkeep` hidden-file handling) — the pins and the integration test now
  cover it.
- A check that is allowed but never followed by a dispatch still journals; harmless, since
  an unchanged fingerprint produces no transition.
