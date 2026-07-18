---
id: example-feature-spec
title: "Worked example: a full feature spec (backup retention pruner)"
kind: guide
voice: derived
date: 2026-07-18
derived-from-sources:
  - 2026-06-22-wiegers-software-requirements
  - 2026-06-27-goldstein-dissertation-defense
provenance: >
  Authored 2026-07-18 as the illustration companion to
  guides/writing-specifications.md: one complete feature spec written
  end-to-end by that guide's rules, with teaching callouts explaining why
  each part is shaped the way it is. The domain (GFS backup retention) is
  invented for the example — chosen because it is "a really good abstraction
  with a complicated implementation" (Goldstein's selection rule), so a
  property genuinely earns its place. Not a spec for any real system.
---

# Worked example: a full feature spec

A complete feature specification written by the rules of
[writing-specifications](writing-specifications.md), for education. Teaching
notes appear as `> ✎` blockquotes — everything else is the spec as an agent
(e.g. a [da-run](https://github.com/igouss) pipeline) would receive it.

The feature: a **backup retention pruner** — given a catalog of snapshots and
a grandfather-father-son policy, decide which snapshots to keep and which are
prunable.

> ✎ **Why this domain.** It passes Goldstein's selection rule for climbing to
> the Property register: *"a really good abstraction with a complicated
> implementation."* The abstraction is one sentence (keep D daily, W weekly,
> M monthly, newest wins); the implementation is a thicket of calendar
> boundaries, ties, and off-by-ones. Three test cases cannot convince you —
> so the spec's real content is the ∀-claims, and everything above them is
> their readable shadow.

---

# Feature: retention pruning `[FS-7]`

## Vision / why

Bounded backup storage without losing the ability to restore from any recent
day, any recent week, any recent month — the operator sets a policy once and
never hand-picks snapshots again.

> ✎ One sentence, PRD-altitude: the *outcome*, not the mechanism
> ([prd-vs-spec](prd-vs-spec.md)). This is the line the human intake review
> checks the rest of the document against — fidelity-to-intent is invisible
> to every register below, so it must be stated where a human can see it.

## Scope

In scope:

- Selecting the retained / prunable partition of an existing snapshot catalog.
- Emitting the decision as a **plan** (a value), listing every snapshot with
  its verdict and the rule that justified it.

Out of scope:

- Deleting anything. Deletion is a separate, explicitly confirmed command
  that consumes a plan. This feature never touches storage.
- Creating snapshots, scheduling, transport, encryption.
- Policies other than daily/weekly/monthly counts (no yearly tier, no
  size-based rules).

> ✎ The out-list is load-bearing: each bullet is a requirement someone would
> otherwise assume in. "Never deletes" also pre-shapes the hexagon — it
> forces the core to be a pure function `catalog × policy → plan`, which is
> exactly what makes the Properties section cheap to run.

## Actors & context

- **Operator** — invokes `prune plan` (CLI boundary), reads the plan, and
  separately confirms deletion.
- **Snapshot catalog** — the input: a set of snapshots, each with a unique id
  and a UTC creation timestamp. Read via a catalog port; this spec treats it
  as a value.
- **Policy** — `daily D, weekly W, monthly M`, each a non-negative count,
  configured per backup target.

## Requirements

Each clause: one behavior, active voice, `shall`, stable ID. Priority is a
separate field, not smuggled into the verb.

> ✎ Stable IDs (`FR-71`), never hierarchical numbers (`3.1.4.2`) — insert /
> delete / reorder must not renumber the traceability spine.

**FR-71** (priority: must)
WHEN a plan is requested with a catalog and a policy, the pruner SHALL
partition the catalog into a retained set and a prunable set such that the
retained set contains the newest snapshot of each of the D most recent
calendar days, the W most recent ISO weeks, and the M most recent calendar
months that contain at least one snapshot (UTC).

**FR-72** (priority: must)
WHEN a plan is requested against a non-empty catalog, the pruner SHALL retain
the most recent snapshot, regardless of policy — including the policy
`0 daily, 0 weekly, 0 monthly`.

**FR-73** (priority: must)
WHEN two or more snapshots fall in the same retention period, the pruner
SHALL select the newest snapshot in that period as the period's
representative.

**FR-74** (priority: must)
WHEN a plan is requested against an empty catalog, the pruner SHALL return an
empty plan that reports zero snapshots examined.

**FR-75** (priority: must)
WHEN a plan is produced, the pruner SHALL record, for every retained
snapshot, the rule(s) that retained it, and for every prunable snapshot, the
verdict `no rule retains it`.

**FR-76** (priority: should)
WHEN two snapshots in the same period carry identical timestamps, the pruner
SHALL select the one with the lexicographically greatest id, so that the plan
is deterministic across runs.

> ✎ Wiegers' lint applied: no `and/or` joining behaviors (FR-71's three
> tiers are one selection rule, not three behaviors), no weak words
> (*robust, efficient, seamless* — banned), each clause discretely testable
> with two or three tests. FR-74 exists because **the zero case is the one
> agents skip** — it is written down precisely so it cannot be skipped.
> FR-76 shows a real elicitation find: the tie was discovered while writing
> FR-73's near-miss, which is the near-miss doing its diagnostic job.

## Scenarios

Per requirement: a **witness** (should-pass) and a **near-miss** (should-fail,
one step over the line). Zero / one / many; two counts as many.

> ✎ The near-miss must be *near* — different in one salient dimension. It
> pins the boundary, and it is the red-first test: it must fail against a
> broken implementation before any fix, or it proved nothing. Witness seeds
> the golden set; near-miss seeds the mutant set
> ([testing-the-spec](../designs/software-factory/11-testing-the-spec.md)).

```gherkin
Feature: retention pruning [FS-7]

  # -- FR-74: zero --------------------------------------------------------
  Scenario: empty catalog yields an empty plan            # witness [FR-74]
    Given an empty catalog
    And a policy of 7 daily, 4 weekly, 12 monthly
    When I request a plan
    Then the plan retains nothing, prunes nothing, and reports 0 examined

  # -- FR-72: one ---------------------------------------------------------
  Scenario: sole snapshot survives a zero policy          # witness [FR-72]
    Given a catalog with one snapshot "s1" taken 2026-07-18T02:00Z
    And a policy of 0 daily, 0 weekly, 0 monthly
    When I request a plan
    Then "s1" is retained with rule "most-recent"

  Scenario: second-newest is not protected by most-recent # near-miss [FR-72]
    Given snapshots "s1" at 2026-07-17T02:00Z and "s2" at 2026-07-18T02:00Z
    And a policy of 0 daily, 0 weekly, 0 monthly
    When I request a plan
    Then "s2" is retained and "s1" is prunable
    # one step over the line: the rule protects THE most recent, not "recent ones"

  # -- FR-71 / FR-73: many ------------------------------------------------
  Scenario: newest-in-day represents the day              # witness [FR-73]
    Given snapshots "a" at 2026-07-18T01:00Z and "b" at 2026-07-18T23:00Z
    And a policy of 1 daily, 0 weekly, 0 monthly
    When I request a plan
    Then "b" is retained with rule "daily" and "a" is prunable

  Scenario: day boundary is UTC midnight, not 24 hours    # near-miss [FR-71]
    Given snapshots "a" at 2026-07-17T23:30Z and "b" at 2026-07-18T00:30Z
    And a policy of 1 daily, 0 weekly, 0 monthly
    When I request a plan
    Then "b" is retained and "a" is prunable
    # one hour apart, but two calendar days: the rule counts days, not hours

  Scenario: days with no snapshots do not consume the budget  # near-miss [FR-71]
    Given snapshots on 2026-07-18, 2026-07-15, and 2026-07-10 (one each)
    And a policy of 3 daily, 0 weekly, 0 monthly
    When I request a plan
    Then all three are retained
    # "3 most recent days THAT CONTAIN a snapshot" — gaps don't burn slots

  # -- FR-76: determinism tie-break ---------------------------------------
  Scenario: identical timestamps break ties by id         # witness [FR-76]
    Given snapshots "aaa" and "zzz", both at 2026-07-18T02:00Z
    And a policy of 1 daily, 0 weekly, 0 monthly
    When I request a plan twice
    Then both plans retain "zzz" and prune "aaa"
```

## Properties

The ∀-claims over the pure core — **this section is the specification**; the
EARS clauses and scenarios above are its readable shadows. The core is a pure
function; no I/O, no clock (`now` is a parameter, never read inside).

> ✎ Written as near-code over named core types so a test-writing stage
> transcribes rather than invents. Each property carries the IDs it
> discharges — that link is what the Acceptance section walks.

```
plan : Catalog × Policy → Plan          -- pure; Plan partitions the catalog

P-1  partition            [FR-71]
     ∀ c, p:  retained(plan(c,p)) ∪ prunable(plan(c,p)) = c
              ∧ retained ∩ prunable = ∅
     -- the plan never invents, drops, or double-books a snapshot

P-2  most-recent safety   [FR-72]
     ∀ c ≠ ∅, ∀ p:  max_by(timestamp, id)(c) ∈ retained(plan(c,p))

P-3  soundness of verdicts  [FR-71, FR-73, FR-75]
     ∀ c, p, ∀ s ∈ retained(plan(c,p)):
         justification(s) ≠ ∅  ∧  every cited rule actually holds of s
     ∀ s ∈ prunable(plan(c,p)):  no retention rule holds of s
     -- kept ⇔ some rule keeps it; the two directions are the
     -- over-constraint / under-constraint guard pair

P-4  idempotence          [FR-71]
     ∀ c, p:  plan(retained(plan(c,p)), p) retains exactly retained(plan(c,p))
     -- pruning what pruning kept prunes nothing more

P-5  order independence / determinism   [FR-76]
     ∀ c, p, ∀ permutations c' of c:  plan(c',p) = plan(c,p)

P-6  budget bound         [FR-71, FR-72]
     ∀ c, p:  |retained(plan(c,p))| ≤ p.daily + p.weekly + p.monthly + 1

P-7  monotone safety in the policy      [FR-71]
     ∀ c, ∀ p ⊑ p' (pointwise ≤):  retained(plan(c,p)) ⊆ retained(plan(c,p'))
     -- loosening the policy never prunes something it used to keep
```

> ✎ P-4 and P-7 are the kind of claim no example table ever states — they
> quantify over *all* catalogs and *pairs* of policies. This is the payoff
> of climbing the register: a counterexample here means the code is wrong
> **or the property was** — both are legitimate finds; the spec is a peer
> under test, not an oracle floating above the code.

**Where the ladder would climb next (not climbed — escalation is lazy):**
today the pruner runs single-writer by assumption A-1 below. The day plans
are computed while an ingest can add snapshots concurrently, P-2 starts to
quantify over interleavings — that is a TLA+ invariant, not a property test,
and only that anomaly justifies paying for it.

## Quality attributes

Only the ones that bite; each with Scale / Meter / Must — the Meter is the
check, the Must is the CI threshold.

**NFR-71 planning latency** (priority: should)
- Scale: wall-clock time of `plan(c, p)` for `|c| = 10 000` snapshots.
- Meter: `cargo bench retention_plan_10k` (criterion, pinned machine class).
- Must: p95 < 50 ms.

**NFR-72 plan explainability** (priority: must)
- Scale: fraction of snapshots in the plan whose verdict names its rule.
- Meter: asserted by property P-3 on every CI run.
- Must: 1.0 — no unexplained verdicts.

> ✎ "Fast" and "transparent" were the words the intent arrived in; both are
> banned. NFR-71 is the honest translation of "fast", NFR-72 of
> "transparent" — note it needed no new machinery, an existing property is
> its Meter.

## Constraints & assumptions

Constraints (with rationale):

- **C-1** The core (`plan` and everything under it) is pure: no I/O, no
  clock, no `unsafe`. *Rationale: P-1..P-7 are only cheap and meaningful
  over an effect-free core; the clock is a parameter so tests own time.*
- **C-2** The plan is the only output; no code path in this feature deletes.
  *Rationale: blast-radius — a wrong plan is embarrassing, a wrong deletion
  is unrecoverable; deletion lives behind a separately confirmed boundary.*

Assumptions (with rationale):

- **A-1** Single writer: no snapshots are added or removed while a plan is
  being computed. *Rationale: the catalog port hands the core an immutable
  value; if this ever becomes false, see the ladder-climb note under
  Properties.*
- **A-2** Timestamps are UTC and well-formed by the time they reach the
  core (parse, don't validate — the boundary rejects malformed input).

No open issues were identified at authoring time.

> ✎ The empty section is *stated*, never deleted — the reader must know the
> author weighed it. A-1 is the load-bearing assumption: it is precisely
> the fact whose failure forces the formal register.

## Acceptance

The traceability map: every requirement → the check that verifies it. This
table is what an adversarial pre-commit review walks, one verdict per row.

| ID     | Check |
|--------|-------|
| FR-71  | properties P-1, P-3, P-6, P-7; scenarios `day boundary`, `gaps don't burn slots` |
| FR-72  | property P-2; scenarios `sole snapshot survives`, `second-newest is not protected` |
| FR-73  | property P-3; scenario `newest-in-day represents the day` |
| FR-74  | scenario `empty catalog yields an empty plan` (the zero case) |
| FR-75  | property P-3 (justification non-empty and true); NFR-72 |
| FR-76  | property P-5; scenario `identical timestamps break ties by id` |
| NFR-71 | `cargo bench retention_plan_10k` p95 < 50 ms, CI-gated |
| NFR-72 | property P-3, CI-gated |

> ✎ Every clause has an oracle — no row may read "by inspection". A
> requirement with no possible row is not yet a requirement. The rows are
> deliberately redundant across registers (FR-72 has both a property and
> two scenarios): *"any time two representations of the requirements
> disagree, you have found an error"* — the cross-check is the verification.

---

## What this example demonstrates — the checklist, discharged

1. **Every clause has an oracle** — the Acceptance table has a row per ID.
2. **≥ 2 registers, cross-checked** — each FR appears as EARS prose and as a
   property and/or scenario; disagreement between them is a found bug.
3. **Hexagonal placement** — C-1 forces the pure core the properties need;
   the CLI and catalog port stay at the edges with the Gherkin.
4. **Strongest oracle the criterion admits, lazily** — properties where
   correctness quantifies (P-1..P-7), plain scenarios where three cases
   convince (FR-74), and an explicit note of where TLA+ *would* start.
5. **Witness + near-miss per boundary** — including the zero case (FR-74)
   and the one-step-over cases (UTC midnight, second-newest, gap days).
6. **The spec is a ratchet** — FR-76 is a worked instance: a tie surfaced
   while writing a near-miss became a permanent requirement with its own
   property, encoded red-first.

The honest limit travels with it: every register here checks consistency
with what is *written*; whether the written thing is what the operator
*meant* is checked only by a human reading the Vision line against the rest
— which is why that line exists.

## See also

- [Worked example: the prompt that produces a feature spec](example-spec-prompt.md)
  — the input side: the raw problem statement, the reusable instruction
  block, and the elicitation exchange this spec came out of.
- [Writing specifications: a property is a spec](writing-specifications.md)
  — the method this example instantiates.
- [Capturing use cases](capturing-use-cases.md) — the intake layer that
  would have produced this feature's actor/goal/rules before specification.
- [Software factory — specification method](../designs/software-factory/02-specification-method.md)
  — the register ladder and the anomaly → invariant ratchet.
- [The verification-document mirror](verification-document-mirror.md) — why
  the Acceptance table is an inlined RTM.
