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

### S4 — declaring what a steer answer changed is a courtesy, not a contract

**This entry was rewritten after I checked my own claim and found it overstated. The
original text is preserved at the end of this section, because a field report that quietly
edits its own findings is worth less than one that shows its corrections.**

The workflow *result* — `{"stage":"implement","allAuditsPassed":true}` — says nothing about
which steer answer was read or what was done about it. That much is accurate, and it is why
I went and hand-diffed the test files.

But the stage's own `change-note.md` did declare it, in full: a section headed **"Test-file
edits made under the answered steer"**, tabulating all four edits with a rationale each,
plus an explicit statement that nothing weakens, skips or removes a scenario and that the
counts are unchanged (41 lib tests, 23 scenarios). It is a better record than my hand-diff
was. My original finding — "obeyed and ignored produce identical tool output" — was wrong at
the artifact level and right only at the tool-result level.

The real gap is narrower and still worth closing: **that declaration happened because my
steer answer explicitly demanded it.** I wrote "Report every test file edit explicitly in the
stage output so the changes are reviewable." Nothing in the flow's stage contract requires
it. An operator who answers a steer without thinking to ask for an audit trail — which is
most operators, most of the time — gets whatever the stage chooses to volunteer.

Recommend: make "declare what you changed under an answered steer, and affirm what you did
not weaken" part of the implement/commit stage contract rather than something the operator
has to think to request. The behaviour already exists and is good; it just is not guaranteed.

Secondary, and unchanged from the original finding: the workflow result JSON could cheaply
carry a `steerConsumed: "03-implement"` field so the driving agent knows an answer was read
without opening an artifact.

> *Original text, superseded:* "After `resolve` and re-dispatch, the stage reported
> `allAuditsPassed: true`. Nothing in that result says which steer answer it read or what it
> did about it… Right now 'the answer was obeyed' and 'the answer was ignored and the stage
> got green another way' produce identical tool output." — The second sentence was true of
> the tool result and false of the stage's artifacts. I filed it before reading
> `change-note.md`.

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

### E5 — the run cost is not disclosed anywhere, and it is the thing that stopped this run

The operator killed the `commit` stage mid-flight at **800K+ tokens** for the run. That is
for *one* domain crate: ~700 lines of `no_std` Rust, 41 unit tests and 23 scenarios, with one
steer pause. Measured per stage from the workflow results:

| stage | subagent tokens | wall clock |
|---|---|---|
| design | 86K | 4m54s |
| design-review | 70K | 3m37s |
| tests | 180K | 13m24s |
| implement (paused on steer) | 125K | 6m18s |
| implement (re-run) | 103K | 6m18s |
| commit | killed | — |

`SKILL.md` says nothing about cost. It documents `all` — design → tests → implement → verify
→ commit — as a casual convenience, with no hint that invoking it on a medium bead spends the
better part of a million tokens. An operator reading the skill has no way to form an estimate
before committing to a run, and the per-stage design means the cost arrives in five
unannounced instalments.

Two things would help, neither of them large:

- **State the order of magnitude in `SKILL.md`**, next to the `all` stage. Even "expect
  100–200K tokens per stage on a medium change" would let an operator choose `design` alone
  over `all` deliberately rather than by luck.
- **Report cumulative run cost in `bin/state status`.** The workflow results already carry
  `subagent_tokens`; nothing aggregates them where the operator looks. A run that has spent
  800K should be able to say so before the operator finds out from the billing side.

Worth noting what the tokens bought, because the answer is not "nothing": the `tests` stage
(the most expensive at 180K) produced the suite that later *caught* the four-way expectation
contradiction, and the `implement` stage spent 125K discovering and arguing that
contradiction rather than papering over it. This is not obviously bad value. But it should be
a choice the operator makes with the number in front of them.

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

## 4. What should be automated, by kind of step

The useful cut is not "which stage" but **how many right answers a step has**. Three kinds,
and they want opposite treatment:

**Deterministic — one right answer, must never be an LLM.** `verify` and `record-commit`
already live here and the doc defends that well. These do not, and I did every one of them
by hand this run:

| Step | Doing it by hand cost | What it should be |
|---|---|---|
| Extract a spec from the tracker | `br show --json \| jq .[0].description`; I got the jq path wrong first try (it returns an array, not an object) | `run setup --spec-from-bead <id>` |
| Thread `flow` into every dispatch | ~1 KB of JSON, five times | E1 — `da-stage.js` reads it from `runDir` |
| **Confirm the tests are RED before `implement`** | I ran `cargo test` myself and read the counts | a gate, see below |
| Compare binary size before/after | two builds, a `diff`, a scratch dir | `run cost --baseline <ref>` |
| Squash-merge the run branch | by hand after the commit stage was killed | `run land --into main` |
| Move the bead's status | still not done | part of `land` |

**The red-test check is the important one and it does not exist.** The whole flow is
TDD-shaped, but nothing verifies the premise. If `tests` emitted a suite that passed against
a stub, `implement` would "succeed" instantly and `verify` would go green, and the run would
report exactly what a real run reports. That is the false-green shape this repo has closed in
three other places, sitting in the one place the pipeline never looks. It is also trivially
mechanical: run the suite after `tests`, refuse to advance unless it fails. I checked it
manually this run — 38 failed, 3 passed — and the 3 passers needed a look to confirm they
were legitimately green (derived `Ord`, no `todo!()` behind them) rather than vacuous.

**Mechanical-but-generative — many phrasings, one meaning. Cheap model, schema-constrained.**
Commit-message drafting from `change-note.md`, the ledger↔test cross-reference, the
change-note tables themselves. These are formatting jobs over decisions already made.

**Judgment — genuinely open. Expensive model, and worth it.** Design, adversarial review,
steer adjudication. This is where the run's money should go and largely where it went.

One more automatable thing, learned from the stage you killed: it left probe tests in
`alarm.rs` using `for` loops and `.for_each`, against this project's explicit
complexity-1/no-loops rule. A lint for loops in test bodies would have caught it. More
generally, **scratch code a stage writes to think with should be distinguishable from code it
means to deliver** — right now both just appear in the worktree.

## 5. Orientation — what I had to discover before I could start

Everything below was discovered by hand, in this order, before or during the run. None of it
is in `SKILL.md`, and all of it is derivable from files already in the target project:

1. Whether `bb`/`cargo` existed, and the shebang of each `bin/*` script (found D2 this way).
2. The tracker: that `br` is the tracker, that beads are the real specs, `br ready` for
   unblocked work, and the JSON shape of `br show`.
3. The build/test/lint/gate commands — `just --list`, then reading recipe bodies, then
   grepping `justfile` for the `elf`, `port`, `sg`, `pyshim` variables to learn how the board
   is reached.
4. That there are **two** cargo workspaces (host and `firmware/`), which is what later made
   the zero-cost claim provable — zero changed files under `firmware/` means the device
   binary cannot move.
5. Which app is the canary (`host-monitor`), and that `just run` restores plant-monitor.
6. The conventions: `/home/elendal/CLAUDE.md`, `hex-arch` roles, scoped commits, `kb/`.

Proposal: **`bin/run orient --project <P>`**, emitting JSON, run once at setup and written
into the run dir. Fields: workspace roots, build/test/lint commands, the gate command, the
tracker CLI and its spec-extraction invocation, conventions file paths, hardware recipes, and
the canary binary. Sources are all mechanical — `justfile`, `Cargo.toml` workspace members,
`.da/gate`, `CLAUDE.md`.

Two payoffs beyond saving my time. It would have caught D2 automatically (compare each
`bin/*` shebang against how `SKILL.md` invokes it). And the flow's `CONTEXT.md` is static
today — a generated orientation block would make it true per project instead of generically
worded.

## 6. Model routing — and the one place this run got it backwards

`flow.ron` already carries per-dispatch `model` and `effort`, so all of this is a data change,
not a code change. Current assignment: design/implement opus-high, tests/commit sonnet, verify
no LLM.

**Verify on no LLM, and design/implement on opus, are both right.** Design earned it — the
`AckSet` reasoning and the declined `freshness.rs` carve are the kind of argument a cheap
model does not produce. `design-review` on opus earned it too: it took my steering note,
resolved the multi-fault gap, and added not just the scenario but *its mirror*, which is the
part that makes the test non-vacuous.

**`tests` on sonnet was a false economy on this run, and the numbers say so.** It was the
most expensive stage anyway (180K, 13m24s) and it shipped four mutually contradictory
expectations: a 2–2 split on `silent_for_ms`, an arithmetic error, and a scenario unpassable
by any implementation. Cleaning that up cost an implement pause (125K), an operator decision,
and a full implement re-run (103K) — call it 230K plus your attention, to save the difference
between sonnet and opus on one stage.

Test authorship looks mechanical and isn't. It is where the spec's ambiguities surface, and
that is precisely a judgment task. Two options, not exclusive:

- move `tests` to opus for spec-dense beads, or
- add a deterministic **self-consistency check** on the emitted suite before `implement`
  — same input tuple asserted to two different expected values is mechanically detectable,
  and it is exactly what cost 230K to find the expensive way.

Candidates to move **down**: the commit stage's message-drafting and change-note formatting
are cheap-model work once the adversarial review has rendered its verdict. The review itself
is not.

## 7. Steering — and getting the request onto your phone

The park is durable already: `DaSteer` holds a workflow per steer-request on a Restate
awakeable, `bin/steer park` bridges the file answer into it, and `bin/state notify` mirrors
run state into `DaRun`. **The missing piece is not durability, it is the outbound edge.**
Nothing anywhere tells the operator a steer exists. This run, the stage parked and I printed
the question to a terminal you were not required to be looking at.

Three layers, cheapest first:

**(a) Works today, needs one line in `SKILL.md`.** Every Claude Code session has a
`PushNotification` tool that raises a desktop notification and pushes to the phone when
Remote Control is connected. The driving agent already knows the instant a stage returns
`steerPaused` — it does not even need to poll. `SKILL.md` should say: on a steer pause, push
a notification naming the run and the question before relaying it. I did not do this and
should have; it is the single cheapest fix in this document.

**(b) For headless and cron runs, where no session is attached.** `bin/steer park` should
fire an outbound notification when it parks — webhook, ntfy, or the Restate side calling out.
This is the case that actually matters: a run started by a schedule parks at 02:00 and,
today, waits silently until someone thinks to look.

**(c) Answering from the phone — mostly already possible.** Since `park` bridges the
awakeable to the file, resolving a steer from a phone is a `POST` to the Restate ingress; the
`resolve` path is the same one `bin/steer` uses. This should be **documented with a worked
curl**, because it turns a parked run from "blocked until I am at my desk" into "blocked
until I read my phone." Two caveats worth stating in the doc: the answer is prose that binds
like the spec, so a phone-sized answer is a real constraint — and `--reason` should not be
required from a phone (see S3; let the agent propose it).

Worth pairing with E5: a steer notification should carry **what the run has spent so far**.
The decision "answer this and continue" is also a decision to spend the next 100–200K, and
right now the operator makes it blind. You killed this run at 800K without that number ever
having been shown to you.

## 8. Summary

| # | Item | Severity | Status |
|---|---|---|---|
| D1 | `args.flow` undocumented, mandatory | blocker | `SKILL.md` fixed |
| D2 | `bash bin/run gate` — wrong interpreter | blocker | `SKILL.md` fixed |
| D3 | `steer resolve --stage` needs dir name | high | doc fixed; **tool fix outstanding** |
| D4 | no guidance for a missing argument | low | `SKILL.md` fixed |
| S1 | no `steer show` | medium | open |
| S2 | two answer paths, one unvalidated | medium | open |
| S3 | `--reason` taxonomy asked of the operator | medium | open |
| S4 | declaring steer compliance is a courtesy, not a contract | medium | open (finding corrected) |
| S5 | consumed operator notes vanish silently | medium | open |
| E1 | `flow` threaded by hand every dispatch | **high** | open |
| E3 | no worked session example | medium | open |
| E4 | spec-shape unaddressed | medium | `SKILL.md` fixed |
| E5 | run cost undisclosed — 800K+ tokens, run killed | **high** | open |
| A1 | nothing verifies the tests are RED before `implement` | **high** | open (§4) |
| A2 | spec extraction, size comparison, land+bead-status all manual | medium | open (§4) |
| A3 | stage scratch code is indistinguishable from deliverable | low | open (§4) |
| O1 | no `run orient` — project discovery is all by hand | medium | open (§5) |
| M1 | `tests` on sonnet cost ~230K downstream to repair | **high** | open (§6) |
| N1 | a steer pause notifies nobody — no push, no webhook | **high** | open (§7) |
| N2 | answering a steer from a phone is possible but undocumented | medium | open (§7) |

If only two things get done: **E5** and **E1**.

**E5** ended this run. The operator killed the `commit` stage on cost, which means the
pipeline's own last stage — the adversarial review and the verified commit record — never
ran, and the change had to be landed by hand outside the machine. A pipeline whose final
gate gets skipped for affordability reasons is losing exactly the assurance it exists to
provide. Disclosing the number is the cheap half; making `all` affordable is the real work.

**E1** is pure friction on every single dispatch — ~1 KB of hand-carried JSON that the engine
could read itself from a path it already has.

**S4** was downgraded from `high` after I checked my own claim and found the stage had
declared its test edits properly in `change-note.md`. The residue is real but smaller: the
declaration was volunteered because my steer answer demanded it, not because the contract
requires it.

Of the later findings, three are cheap and pay immediately: **N1(a)** is one line in
`SKILL.md` — push a notification when a stage parks, which every session can already do.
**A1** is "run the suite after `tests` and refuse to advance if it passes," which closes a
false-green hole in the pipeline's own premise. **M1** is a one-word change in `flow.ron`
that this run's numbers argue would have *saved* ~230K rather than cost anything.
