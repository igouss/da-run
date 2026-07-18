---
id: example-spec-prompt
title: "Worked example: the prompt that produces a feature spec"
kind: guide
voice: derived
date: 2026-07-18
derived-from-sources:
  - 2026-06-22-wiegers-software-requirements
  - 2026-06-27-goldstein-dissertation-defense
  - 2026-07-08-adzic-bdd-not-requirements
provenance: >
  Authored 2026-07-18 as the input-side companion to
  guides/example-feature-spec.md: the raw problem statement, the reusable
  instruction block, and the elicitation exchange that together produce a
  spec in that format. The exchange is a reconstruction written for
  teaching — it shows the interaction shape, not a transcript of a real
  session. Teaching callouts as in the output-side example.
---

# Worked example: the prompt that produces a feature spec

The input-side twin of [example-feature-spec](example-feature-spec.md): what
the user actually hands the agent, and what the exchange looks like before
the spec exists. Three parts — the **problem statement** as a user really
writes it (messy, on purpose), the **reusable instruction block**, and the
**elicitation exchange** where the spec's sharp edges actually get made.

> ✎ The division of labor this workflow encodes: the human owns
> **fidelity-to-intent** (is this the right thing?), the agent owns
> **discrimination** (is the thing sharp?). No register can check the
> first, so the prompt is designed to keep dragging decisions back to the
> human instead of letting the agent invent them
> ([prd-vs-spec](prd-vs-spec.md) — the review split that survives the
> agentic collapse).

---

## Part 1 — the problem statement (as actually written)

What the user pastes first. Deliberately imperfect — this is the realistic
input, not a pre-cleaned one:

> My backup tool piles up snapshots forever and the disk fills up. I want
> automatic cleanup: keep enough backups that I can restore from any recent
> day, week, or month, delete the rest. Something like restic's forget
> policy. It should be fast, the results should be understandable, and it
> absolutely must never throw away a backup I'd still want.

> ✎ Count the lint hits: *enough*, *fast*, *understandable*, *recent*,
> *"I'd still want"*. A human colleague would push back on each; an agent
> given no instruction will **silently guess** all five. The instruction
> block's whole job is converting silent guesses into questions or recorded
> assumptions. Note also what's genuinely good here: the vision ("disk
> fills up… restore from any recent point") and a hard safety intuition
> ("never throw away…") — the two things only the user could know.

## Part 2 — the instruction block (reusable)

Pasted together with the problem statement. This is the part you keep and
reuse verbatim for every feature:

```text
Write a feature requirements specification for the problem above, following
kbe/guides/writing-specifications.md. Rules of engagement:

1. INTERVIEW FIRST, SPEC SECOND. Before writing anything, ask me your
   clarifying questions — all of them, in one batch. Derive them by:
   - running the weak-word lint over my problem statement: every vague
     word (fast, enough, recent, robust, understandable, ...) must come
     back to me as either a question or a proposed Scale/Meter/Must;
   - walking zero / one / many for every input (two counts as many);
   - probing the boundaries, not the happy path: for each rule you think
     you heard, show me a concrete example pair — one you believe I want
     accepted, one ONE STEP over the line you believe I want rejected —
     and ask if the line is where you drew it.

2. NEVER INVENT A POLICY DECISION. Timezone semantics, tie-breaking,
   rounding, error behavior: ask, or if minor, record it under
   Assumptions with rationale and flag it for my review. Silent choices
   are defects.

3. FORMAT. Use the section skeleton from writing-specifications.md:
   Vision / Scope (in AND out) / Actors & context / Requirements /
   Scenarios / Properties / Quality attributes / Constraints & assumptions
   / Acceptance. No section deleted — if empty, say so and why.

4. REQUIREMENTS. EARS clauses, one behavior each, stable FR-ids, priority
   as a separate field. Banned-word lint applies to your text too.

5. SCENARIOS. Per requirement: a witness and a near-miss (one salient
   dimension over the line), zero/one/many. If you cannot write the
   near-miss for a requirement, the requirement is not sharp yet — come
   back to me instead of papering over it.

6. PROPERTIES ARE THE SPEC. State the ∀-invariants as near-code over a
   pure core (no I/O, no clock — time is a parameter). Everything above
   them is their readable shadow. Climb the register ladder lazily: three
   convincing cases → stay at Gherkin; quantifies over all inputs →
   property; over interleavings/adversaries → name where the formal
   register WOULD start, but do not climb without an anomaly.

7. ACCEPTANCE. Map every FR/NFR id to the check that verifies it. No row
   may read "by inspection".

8. After I answer your questions, produce the spec. End it with the open
   points you still consider risky — an honest "could not pin down" beats
   a plausible invention.
```

> ✎ Why each rule is there: (1) is Adzic's *"easy to complain, hard to
> explain"* turned into procedure — manufacture concrete examples and
> fight over boundaries, because the happy path everyone agrees on tests
> nothing. (2) exists because an agent, unlike a colleague, will not push
> back unless told that guessing is a defect. (5) is the near-miss as
> diagnostic, running at *elicitation* time — the cheapest moment to
> discover a requirement isn't one yet. (6) is Goldstein taken literally.
> (8) is the false-green discipline applied to spec-writing itself.

## Part 3 — the elicitation exchange (reconstructed)

The shape of the interaction that rule 1 produces. Every sharp edge in
[example-feature-spec](example-feature-spec.md) traces to one of these
answers — none of them was invented by the agent:

**Agent:** *"Recent day/week/month" — I propose Policy = counts (D daily, W
weekly, M monthly), like restic. Confirm? And are the counts per backup
target or global?*
**User:** Counts, per target. → becomes the Policy definition in **Actors &
context**.

**Agent:** *Boundary pair for "daily": snapshots at 2026-07-17T23:30Z and
2026-07-18T00:30Z are one hour apart. With `1 daily`, I believe you want
them treated as TWO days (calendar days, UTC), so the older is prunable.
One step over the line: if you meant "last 24 hours", both survive. Which?*
**User:** Calendar days, UTC. → **FR-71** and the *day boundary* near-miss.

**Agent:** *Zero case: catalog has snapshots on the 18th, 15th, and 10th —
gaps between them. Does `3 daily` keep all three (days *containing*
snapshots) or only the 18th (gap days burn budget)?*
**User:** All three. → the *"that contain at least one snapshot"* clause
in FR-71 and the *gaps don't burn slots* near-miss.

**Agent:** *"Never throw away a backup I'd still want" — I cannot test
"want". Strongest checkable version I can offer: the most recent snapshot
is retained under EVERY policy, including all-zeros. Acceptable
translation, or is there more to "want"?*
**User:** Yes — and deletion itself must be a separate confirmed step.
→ **FR-72**, property P-2, and constraint **C-2** (plan-only, never
deletes).

**Agent:** *While writing the tie near-miss I hit a hole: two snapshots
with identical timestamps in one period — which represents the period? If
I don't pin this, plans are nondeterministic across runs.*
**User:** Greatest id wins. → **FR-76** and property P-5.

**Agent:** *"Fast" — proposed Meter: p95 of planning 10k snapshots < 50 ms
on the CI bench. "Understandable" — proposed Meter: every verdict names
the rule that produced it, asserted by property. Thresholds OK?*
**User:** Both fine. → **NFR-71**, **NFR-72**.

> ✎ Read the exchange against the finished spec and the provenance is
> total: every FR, every near-miss, every Meter has a question it answers.
> That traceability is the point — when a property later fails and you ask
> *"is the code wrong or was the property wrong?"*, the exchange is where
> you look. Note the two moves worth imitating: the agent **proposes a
> concrete translation and asks**, never asks open-ended ("what does fast
> mean to you?" elicits nothing; "p95 < 50 ms on this Meter?" elicits a
> yes/no); and the FR-76 question shows rule 5 firing — a near-miss that
> couldn't be written surfaced a requirement that didn't exist yet.

## What happens next

The answered exchange plus the instruction block produce
[example-feature-spec](example-feature-spec.md). The human's remaining job
is the one no register covers: read the finished spec **against the
original problem statement** — does the Vision line still say what you
meant? — then hand it to the implementation pipeline (e.g. a da-run
`design → tests → implement → verify → commit` run), where its properties
and scenarios travel as the executable content of every stage.

## See also

- [Worked example: a full feature spec](example-feature-spec.md) — the
  output this prompt produces.
- [Writing specifications: a property is a spec](writing-specifications.md)
  — the method the instruction block operationalizes.
- [Adzic — BDD is not requirements](../sources/2026-07-08-adzic-bdd-not-requirements.md)
  — why the interview manufactures boundary examples instead of asking
  open questions.
- [Capturing use cases](capturing-use-cases.md) — the fuller intake
  structure when the feature is bigger than one prompt.
