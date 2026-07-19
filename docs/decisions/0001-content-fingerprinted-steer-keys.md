# ADR-0001 (da-run2): steer workflow keys carry a content fingerprint

**Status:** accepted, implemented 2026-07-18. **Area:** `engine/bin/steer`, DaSteer.

## Context

A Restate workflow runs exactly once per key, and da-run keyed the DaSteer park on
`<run-id>--<stage>`. A second steer from the same stage of the same run — answered, stage
re-dispatched, new question — collided with the *completed* first workflow: `run/send` was a
no-op conflict, and `park`'s poll of `/output` got the first question's answer back with
HTTP 200 and wrote it under the new question's `## Answer`. A fabricated operator decision in
the human-contact channel, the one place the system exists to never guess.

## Decision

The key is `<run-id>--<stage>--<8-hex sha256 of Question + Options + Round line>`, computed by
`bin/steer` from the file itself — correctness never depends on agent discipline (archiving,
round numbering). The fingerprint excludes `## Answer`, so park and resolve derive the same key
before and after answering.

## Consequences

- A new question is always a fresh workflow; a completed park can never answer a different ask.
- The byte-identical question replays its memoized answer — treated as a feature: the key is
  (run, stage, question), and the workflow memoizes the operator's decision for exactly that
  question. An agent that re-asks the same words at a new round adds a `Round:` line, which
  moves the key.
- The protocol doc asks agents to archive answered steers as `STEER-REQUEST-<n>.md` before
  writing a new one — for the meter record only, not for correctness.
- Meter stamps (`Raised:`/`Answered:`/`Reason:`) sit above the sections and provably do not
  move the key (selftest-pinned).
