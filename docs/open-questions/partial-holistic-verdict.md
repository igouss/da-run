# Open question: should a `partial` holistic verdict block the commit?

**Status:** RESOLVED in da-run2 (2026-07-18) as option C: a `partial` verdict raises a STEER-REQUEST in the gate stage and parks the run; the answered steer binds like the spec and travels into the commit context. The assume-incomplete prompt bias was re-tuned in the same change, and commitPrompt now states the actual verdict. See `engine/workflows/da-post-gate.js`.
**Area:** `engine/workflows/da-post-gate.js` (the adversarial review that runs between the
mechanical gate and the commit stage).
**Decision owner:** the operator. Nothing in this note has been implemented.

---

## The mechanism

After the mechanical gate goes green, `da-post-gate.js` runs two kinds of adversarial review
in parallel:

1. **Per-scenario atoms** — one independent reviewer per test-plan scenario, each answering
   "is this scenario actually violated by the diff?"
2. **One holistic pass** — a single reviewer over the whole change against the spec.

The holistic reviewer exists to catch the residue that the atoms structurally cannot. Its
prompt says so (`da-post-gate.js:112-117`):

> look instead for what no single scenario would catch: requirements the test plan itself
> missed, architectural drift, or a diff that satisfies every scenario yet misreads the
> spec's intent.

It answers on a three-valued enum plus a list of concrete gaps (`:66`):

```js
verdict: { type: 'string', enum: ['fully', 'partial', 'no'] },
gaps:    { type: 'array', items: { type: 'string' } },
```

## Current behaviour

```js
const blocked = violations.length > 0 || dropped > 0 || holistic.verdict === 'no'   // :203
```

Only `no` blocks. **`partial` passes through to commit**, with its `gaps` written to
`stages/<gate>/output/adversarial-review.md` (`:155-159`, path from `da-stage.js:112`) — a
file no gate reads. The run proceeds to a normal commit and reaches `Committed` state.

## Why this is a real gap, not a nitpick

- **No other check covers this class.** The mechanical gate runs tests; a requirement the
  *test plan itself missed* has no test, so the gate is green by construction. The atoms
  review scenarios that exist, so they cannot flag a missing one. A `partial` verdict is the
  only signal in the whole pipeline for "the test plan under-specified the spec."
- **It contradicts a rule already written down.** `flows/rust-factory/stages/03-implement/CONTEXT.md:23`:
  *"a gate going green on a subset of the ledger is **not** done."* A `partial` verdict is
  precisely a subset of the ledger, described one stage later and then ignored.
- **It makes the commit prompt state something false.** `commitPrompt` tells the commit agent
  the review *"found no unresolved violation"* (`:124`). With `partial` + a populated `gaps`
  array that is untrue, and the commit message is written on that premise.
- **It is inconsistent with how the same file treats other uncertainty.** A reviewer that
  errored is counted as unresolved rather than pass (`:199-200`, `dropped > 0` blocks). Crashed
  reviewers are treated conservatively; a reviewer that ran fine and said "incomplete" is not.

## The honest argument on the other side

`holisticPrompt` ends with *"Assume incomplete until the evidence says otherwise"* (`:116`).
The reviewer is **deliberately biased toward finding gaps**. Making `partial` blocking without
touching that instruction converts a bias into a stall, and a check that stalls often is a
check people route around. This is likely why it was left advisory.

Note the coupling: **the prompt bias and the blocking rule must be decided together.** Today
the bias is harmless because the verdict is advisory. Any option that makes `partial`
load-bearing should re-tune that sentence at the same time.

## When this actually fires

Concrete situations that produce `partial` rather than `no`:

- The spec has five requirements; the test plan atomised four. The implementation satisfies
  the four, tests pass, gate is green. The holistic reviewer notices the fifth. → `partial`,
  gaps: `["requirement 5 (retry on transient failure) has no implementation"]`.
- The change satisfies every scenario literally but drifts architecturally — an adapter
  reaching into the domain, say. No scenario asserts on layering. → `partial`.
- The spec is ambiguous, and the implementation picked a defensible reading that the reviewer
  thinks misses the intent. → `partial` (this is the noisy case, and the one most likely to
  be a false red).
- The implementation is a deliberate first slice and the operator *knows* it is partial. →
  `partial`, correctly, and blocking would be an annoyance rather than a save.

The last two are why this is a genuine trade-off and not an obvious fix.

## What this means for you when reviewing a finished run

This is the part that decides the question, because the whole point of the run is that you
end up looking at a result and choosing: **merge / fix the spec / run a steer round.**

### Under today's behaviour (advisory)

The run reaches `Committed` and presents as a success. Nothing in `bin/state status` mentions
the holistic verdict — it is not part of the run state. To discover a `partial` you have to
open `stages/<gate>/output/adversarial-review.md` and read it, knowing to look. If you don't,
the failure mode is: **you merge a change that a reviewer already told you was incomplete, and
the warning was sitting in a file you never opened.** The commit message will not mention it
either, since the commit agent was told there were no unresolved violations.

Concretely, the signal you most need in order to choose "spec fix" over "merge" is the exact
signal being discarded — `gaps` is a list of *what the spec asked for and did not get*, which
is the raw material for that decision.

### If `partial` blocked (hard)

The run stops before the commit stage with the gaps listed. You would decide from the block
message rather than from archaeology. Cost: runs that are legitimately partial-by-design stop
too, and you have to override them. There is currently **no override path** — a hard block on
a bias-toward-incomplete reviewer means some runs cannot be finished without editing the
workflow, which is the worst version of this.

### If `partial` raised a steer-request

The run parks the way it already parks for every other "the machine needs a human judgment"
case, and the existing `bin/steer` protocol applies. You read the gaps, then answer:

- *"these are out of scope for this slice"* → the run continues and commits.
- *"gap 3 is real, implement it"* → steer round, no spec change.
- *"gap 3 means the spec was wrong"* → you fix the spec and re-run.

That maps exactly onto the three-way decision you make anyway, and it puts the decision at the
moment the evidence is fresh rather than after a commit exists.

## Options

### A. Leave it advisory (status quo)

- **+** No new stalls; the reviewer's built-in pessimism stays harmless.
- **+** Zero work, zero risk of a blocked pipeline with no escape hatch.
- **−** The only detector for "test plan missed a requirement" is discarded.
- **−** The commit prompt keeps asserting something false when the verdict is `partial`.
- **−** The pipeline documents the subset rule in the stage contract and does not enforce it.
- **−** You can merge an acknowledged-incomplete change without ever seeing the warning.

### B. Make `partial` block like `no`

- **+** One-line change (`:203`); enforces the subset rule the contracts already state.
- **+** Treats "reviewer says incomplete" the same as "reviewer crashed" — conservative and
  consistent.
- **−** No override path exists today, so a noisy `partial` can wedge a run.
- **−** Guaranteed to fire on legitimate first-slice work.
- **−** Requires re-tuning `holisticPrompt:116` in the same change, or the stall rate will be
  high enough that the check gets disabled.

### C. Make `partial` raise a steer-request

- **+** Reuses machinery that already exists and that the whole system is built around
  (steer parks everything; the operator answers; the run resumes).
- **+** Surfaces the gaps at decision time, in the shape of the merge/spec-fix/steer choice.
- **+** Keeps a pessimistic reviewer useful rather than obstructive — a wrong `partial` costs
  one answered steer, not a wedged run.
- **−** Most work of the three: the workflow must write a `STEER-REQUEST.md` and the commit
  stage must re-check it after the answer.
- **−** Adds a human round-trip to runs that would otherwise complete unattended, which
  partly defeats "run the whole pipeline and come back later."

## Recommendation on record

**C**, with `holisticPrompt:116` re-tuned in the same change so `partial` means "I found a
specific gap I can name" rather than "I was told to assume the worst." B is a defensible
cheaper version *only* if an override path is added at the same time. A is the only option
that leaves a known false-green path open, and it is the current state.

## Minimum change if the answer is B or C

- `da-post-gate.js:203` — the blocking expression.
- `da-post-gate.js:124` — `commitPrompt` must stop claiming "no unresolved violation"
  unconditionally; it should state the actual holistic verdict.
- `da-post-gate.js:116` — the "assume incomplete" bias, re-tuned.
- For C only: write the steer-request, and re-check it before the commit stage dispatches.

## Not affected by this decision

The mechanical gate, the per-scenario atoms, the `dropped > 0` rule, and the commit law in
`da-domain` (green gate + matching non-empty worktree) are all independent. This is purely
about how the holistic reviewer's middle value is treated.
