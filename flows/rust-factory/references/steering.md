# Steering — how a stage asks the operator (F7)

A stage that CANNOT proceed without operator input does not guess and does not fail: it
**asks**, by writing a steer-request file and stopping. The file is the tool call; the
operator's edit is the response. Every request is a machine-legible **correction-steer**
event on the ADR-0010 meter — captured into the run record, counted, and mined.

## When to raise one

Raise a steer-request only when the ambiguity is **load-bearing**: two readings of the spec
produce materially different designs, a referenced file/endpoint does not exist, or a
constraint contradicts the house standards. Do NOT raise one for choices the references
already settle, or that any reasonable reading settles — an unnecessary ask is itself a
defect (it spends the exact operator attention this system exists to retire).

## The file

Write `stages/<NN-stage>/output/STEER-REQUEST.md`, then STOP the stage (write no other
output; do not proceed past the ambiguity):

```markdown
# STEER-REQUEST — <NN-stage>

## Question

One specific question. Cite the spec line or file that forced it.

## Options

- A: <a concrete resolution, one line>
- B: <another>

## Answer

```

`## Answer` stays empty — the operator fills it. Options are suggestions, not a menu; the
operator may answer anything.

If your stage steers **again** after an earlier answered request, do not overwrite the
record: move the answered file to `STEER-REQUEST-<n>.md` beside it, then write the new
request. (Correctness does not depend on this — the durable park is keyed on the question's
content, so a new ask can never collide with an old answer — but the archive preserves every
meter event.)

The harness stamps meter metadata as top-level lines after the title (`Raised:`,
`Answered:`, `Reason:`) — leave them alone; they are outside the sections and do not change
how the file parses.

## The answer

The operator writes the decision under `## Answer` (directly, via `bin/steer resolve`, or
through the Restate UI when the run is parked durably). When the stage reruns, **an answered
STEER-REQUEST.md in your output/ is operator steering — it binds like the spec** for this
run. Honor it, then do the stage's work; leave the file in place (it is the steer's record).

## The harness contract

- An **unanswered** request pauses the arm: the harness detects it before any completion
  barrier, exits with code **3** (paused, not failed) — or, when `DA_STEER_INGRESS` is set,
  parks durably on the homelab Restate server (`bin/steer park`) until the answer arrives
  from either side (file edit or Restate resolve; the harness bridges both ways).
- An **answered** request unblocks the same stage on the next drive of the same run dir.
- `bin/steer check --run R` reports pending/answered requests (exit 3 = pending).
- The durable park's workflow key is `<run-id>--<stage>--<fingerprint of Question+Options+
  Round>`: a different question is always a fresh workflow, so a completed park can never
  answer a later, different ask; only the byte-identical question replays its memoized answer.
- `bin/steer resolve --reason <spec-gap|spec-wrong|scope-cut|preference|other>` classifies
  the steer for the meter; `capture` freezes each steer's open time and classification into
  `metrics.json`.
- Waiting without blocking an interactive session: run `bin/steer park` in the background,
  or point a Monitor at `bin/steer check` / the Restate workflow output endpoint and get
  woken when the answer lands.
