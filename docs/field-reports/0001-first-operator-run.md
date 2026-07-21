# Field report 0001 — the first operator-driven run

**Date:** 2026-07-21
**Operator:** Iouri, driving `/da-run` from a Claude Code session (not from this repo)
**Target:** `stick-c-plus`, bead `stick-c-plus-health-domain-sgu` — a new pure domain crate
(`platform-health`: fault vocabulary, verdict fold, alarm FSM)
**Stages run:** `design`, `design-review`, `tests`, `implement` (×2, one steer pause), `verify`
**Outcome:** `GATE GREEN`. 41/41 lib tests, 23/23 Gherkin scenarios, `hex-lint` and
`effect-audit` clean across both workspaces.

The run succeeded. Everything below is about the friction on the way there, because that is
the part worth writing down. Four defects were found; all four were in `SKILL.md` — the
instructions for driving the machine — and none were in the engine. The engine's ordering
guard, steer park, and run-state machine all did exactly what they claim.

---

## 1. Defects found, in the order they bit

### D1 — `args.flow` was undocumented and is mandatory (severity: blocker)

The very first `design` dispatch died in 19 ms:

```
Error: da-stage needs args.flow — the pipeline definition.
```

`SKILL.md` Step 2 listed `runDir`, `stage`, `round`, `workflowsDir`, `attempts`,
`stateCheck` and `steerState`. It never mentioned `flow`, which `da-stage.js` requires on
**every** agent-stage dispatch. A first-time operator cannot dispatch a single stage from
the written instructions.

The error message itself is excellent — it names the missing arg, gives the exact command to
produce it, and says why (`flow.ron` is the single source of truth). That is the only reason
this cost one minute instead of twenty. **Keep writing errors like that one.**

*Fixed:* `flow` added to the Step 2 arg block, with a note that it is fetched once per run.

### D2 — `bash bin/run gate` cannot work (severity: blocker, latent until `verify`)

`SKILL.md` invoked `bin/run` with `bb` in four places and with `bash` in exactly one — the
verify gate. `bin/run` is `#!/usr/bin/env bb`:

```
bin/run: line 2: syntax error near unexpected token `;;'
```

This is the worst kind of placement: it sits behind design, tests, and implement, so an
operator hits it only after the expensive stages have already run. I found it by grepping
shebangs while a stage was running, not by hitting it.

*Fixed:* `bash` → `bb` on that line. Worth a CI check that every `bin/*` invocation in
`SKILL.md` matches the script's own shebang — this class of error is mechanically detectable
and will recur.

### D3 — `steer resolve --stage` wants the directory name, not the number (severity: high)

`SKILL.md` documents `--stage <NN>`. `resolve` builds the path verbatim:

```
error: no steer-request at .../stages/03/output/STEER-REQUEST.md
```

The directory is `03-implement`. This is D3 rather than a footnote because of **when** it
fires: the operator has just composed a long, carefully-worded answer to a subtle question,
and the tool rejects it. In my case the answer was ~40 lines and survived only because it was
in shell history. An operator typing into a file, or one who had used a heredoc, could
plausibly lose it.

Two things would each have prevented it independently:

- accept a bare `03` and resolve it by prefix-glob against `stages/` (there is exactly one
  match, by construction), or
- have the error print the stage names that *do* have pending steers — the tool already
  knows them; `steer check` prints them one call earlier.

*Fixed in `SKILL.md` only:* documented as the full directory name, plus "take it from the
`stage` field of `steer check`'s own JSON rather than retyping it." **The tool-side fix is
the better one and is not done.**

### D4 — no guidance when an argument is missing (severity: low, but it was the first thing that happened)

The invocation was `/da-run <file>` with no stage. `SKILL.md` says "two required arguments"
and then says nothing about the case where one is absent. Defaulting the stage would be
wrong; the instruction to *ask* needed to be written down.

*Fixed:* added, along with a spec-shape check — see §3.

---

## 2. Steering, specifically

The steering machinery is the part that had the least prior exercise, and it is the part I
have the most to say about. **The protocol itself is sound.** What follows is friction, not
a call to redesign it.

### What worked, and is worth protecting

The `implement` stage hit four mutually unsatisfiable test expectations. It did not pick the
reading with fewer failures and proceed. It stopped, and its `STEER-REQUEST.md` contained:

- the question stated as a product decision, not a code detail ("3000 vs 5000 is a materially
  different product — this number goes on an operator banner");
- a table of which tests expected which value, split 2–2;
- **measured** pass/fail counts for *both* readings, run in the worktree;
- the design's own words quoted on both sides, with the admission that the design settles
  neither;
- three options, one of which was "send it back to stage 02" — i.e. it offered to have its
  own work rejected;
- an honest statement of what the worktree currently held and that it was unresolved.

That is a better-argued question than most humans write. Two of the three items it raised
were plain test bugs it could have silently "fixed" and never mentioned. The rule that a
stage does not edit its own tests without asking is doing real work here — it converted a
silent test-rewrite into a reviewable decision. **This is the single most valuable behaviour
in the system and it should be defended in any future change.**

The park/resume cycle also worked exactly as documented: `steer check` exit 3, dispatch
refused, answer written, exit 0, re-dispatch, stage consumed the answer.

### S1 — there is no `steer show`

`SKILL.md` instructs: *"Relay the `## Question` and `## Options` to the user verbatim."*
There is no command that produces them. The operator must read the path out of `steer
check`'s JSON and `cat` the file, which prints the header, the timestamp, and the empty
`## Answer` heading as well.

A `steer show --run <dir>` printing exactly the two sections the skill is told to relay
would make the documented instruction executable in one call instead of two-plus-editing.
Small, and it is on the hot path of the one workflow that has a human waiting.

### S2 — two ways to answer, one of them unvalidated

`SKILL.md` offers "write it under `## Answer` (or `steer resolve ...`)". The file-editing
path has no validation, no `--reason` capture, and no way to tell a half-written answer from
a finished one. The `resolve` path is strictly better. Recommend documenting `resolve` as
*the* way and demoting hand-editing to a recovery note — one blessed path is easier to get
right, and the ADR-0010 meter only gets fed by one of them.

### S3 — `--reason` asks the operator to classify in a taxonomy they have never seen

`SKILL.md` says to ask the user which of `spec-gap | spec-wrong | scope-cut | preference |
other` fits "when it is not obvious." In practice the operator does not know what these mean
or what they are for, and the distinction that mattered here — the design *supported both
readings and settled neither* (`spec-gap`) versus the design *said something wrong*
(`spec-wrong`) — is a distinction about the artifact, not about the operator's intent. The
agent is better placed to classify it and the operator is better placed to correct the
classification.

Recommend inverting it: the agent proposes a `--reason` with a one-line justification, the
operator overrides if wrong. As written, it puts a taxonomy quiz between the operator and
their answer.

### S4 — nothing confirms an answer was actually consumed

After `resolve` and re-dispatch, the stage reported `allAuditsPassed: true`. Nothing in that
result says *which* steer answer it read or what it did about it. I verified compliance by
diffing the test files by hand — checking the two `3000`s had become `5000`s, that the
arithmetic line read `1000`, that `world.last_chirp = None` was present, that the scenario
count was unchanged at 23, and that no `#[ignore]`/`@wip` had appeared.

That check found nothing wrong. It also should not have been mine to do by hand. A stage that
consumed a steer should have to declare, in its output, what the answer was and what it
changed in response — particularly when the answer granted it authority to edit its own
tests. Right now "the answer was obeyed" and "the answer was ignored and the stage got green
another way" produce identical tool output. That is the false-green shape this project
already closed in three other places.

### S5 — operator steering by editing an `output/` file is invisible once consumed

Between `design` and `design-review` I steered by hand: I found a gap the design had missed
(an acknowledgement FSM holding a single key, which drops an ack when a higher-ranked fault
masks it), wrote it into `design.md` as a marked operator note, and ran `run mark --trigger
steer:applied`.

The reviewer handled it correctly — it resolved the gap, chose per-key acknowledgements, added
three ledger rows and a test scenario *plus its mirror*, and removed my note as consumed.
That is the right behaviour.

But "consumed and resolved" and "deleted without being read" leave the same trace: the note
is gone. I could only tell the difference by reading the new design body for the substance.
The `steer:applied` journal entry records that I steered, not that anything answered it.
Consider having a reviewer echo the operator notes it consumed, or leave them struck through
with a pointer to the section that now answers them.

---

## 3. Ergonomics of the instructions

### E1 — threading `flow` by hand is the biggest ergonomic problem (bigger than any single bug)

Every agent-stage dispatch requires the full `flow` JSON pasted into the args — ~1 KB of
nested objects describing all five stages, their dirs, artifacts, models, strategies and
efforts. I re-sent that blob five times. It is identical every time. It is derived
mechanically from `run.json`, and `da-stage.js` already receives `runDir`, from which
`bin/state flow` derives it.

The argument for passing it explicitly is presumably that the engine should not shell out and
`flow.ron` should stay the single source of truth. Both hold if `da-stage.js` reads it
itself; neither requires the *operator* to be the transport. As it stands, the most
error-prone part of every dispatch is a copy-paste that carries no operator intent
whatsoever, and a single stale field in a hand-carried blob would be very hard to spot.

If explicit passing must stay, `bin/state flow` could at least emit a ready-to-paste args
skeleton with `stage` left as a hole.

### E2 — what is good and should not be lost

- **Step 0 / Step 1 / Step 2 structure.** Locate the bundle, resolve the instance, dispatch.
  I never wondered what to do next.
- **The ordering guard as *the* authority**, with the explicit instruction not to re-derive
  its rules from prose. Exit 4 vs 3 vs 2 is a clean contract, and `allowed` with an advisory
  `stage-already-complete` warning on a re-dispatch is exactly right — a warning, never a
  refusal, which is what makes between-stage steering possible at all.
- **"Never a workflow"** on verify. Keeping the mechanical gate free of an LLM is the correct
  call and the doc states it as a principle rather than a preference.
- **`record-commit` refusing when the branch tip never moved.** The doc's framing — "the
  agent's sha is a claim, not evidence" — is the right instinct, stated in the right place.
- **"A steer pause is the operator's turn, not a failure — never answer it yourself."**
  Unambiguous, and it is the instruction that most needed to be unambiguous.

### E3 — no worked example of a session

`SKILL.md` documents each call in isolation. A single ten-line transcript of a real
design→verify run — the actual command sequence with the guard checks in place — would carry
more than the prose does, and would have surfaced D1 and D2 the first time anyone read it.

### E4 — the spec-shape question is unaddressed (now partly fixed)

I was handed `docs/plans/health-supervision-handoff.md`, which says of itself *"This document
is the brief; the beads are the spec."* It covers an eight-task epic. `SKILL.md` says
"requirements — path to the frozen change-spec" and says nothing about what to do when the
file plainly is not one.

Pointing the pipeline at it would have produced one shallow design spanning eight tasks. What
worked was extracting a single bead into its own spec file and running that — and the bead,
written to this project's "specify what, not how" convention, turned out to be an *excellent*
change-spec: self-contained, with acceptance criteria and named properties.

*Fixed:* `SKILL.md` now says to check the spec describes one change and to settle scope before
Step 1. Worth considering whether the flow should refuse a spec above some size, or at least
say what it is assuming.

---

## 4. Summary

| # | Item | Severity | Status |
|---|---|---|---|
| D1 | `args.flow` undocumented, mandatory | blocker | `SKILL.md` fixed |
| D2 | `bash bin/run gate` — wrong interpreter | blocker | `SKILL.md` fixed |
| D3 | `steer resolve --stage` needs dir name | high | doc fixed; **tool fix outstanding** |
| D4 | no guidance for a missing argument | low | `SKILL.md` fixed |
| S1 | no `steer show` | medium | open |
| S2 | two answer paths, one unvalidated | medium | open |
| S3 | `--reason` taxonomy asked of the operator | medium | open |
| S4 | consumption of a steer answer is unverifiable | **high** | open |
| S5 | consumed operator notes vanish silently | medium | open |
| E1 | `flow` threaded by hand every dispatch | **high** | open |
| E3 | no worked session example | medium | open |
| E4 | spec-shape unaddressed | medium | `SKILL.md` fixed |

If only two things get done: **E1** (stop making the operator carry `flow`) and **S4** (make a
stage declare what steer answer it consumed and what it changed). E1 is pure friction on
every single dispatch. S4 is the one open item that is a correctness risk rather than an
annoyance — it is the same "an agent's report of success accepted as evidence of success"
family this repo's open-questions index already declares closed in three other places, and
steering is where the fourth instance lives.
