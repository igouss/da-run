# Open question: flow-specific assumptions still baked into the engine

**Status:** PARTLY RESOLVED in da-run2 (2026-07-18), per recommendation B: items 1-2 fixed as flow data (`design_from`/`tests_from`/`judge_reference` in flow.ron, flow-supplied atomizer.md; positional guesses deleted). Items 3-4 decided as ENGINE LAW: the Inputs/Process/Outputs/Audit contract shape and CLAUDE.md/CONTEXT.md names are the algorithm itself, and workspace-lint enforcing them across flows is working as intended. Item 5 (SKILL.md stage list) stays labeled documentation.
**Area:** `engine/workflows/*.js`, `engine/bin/workspace-lint`, `SKILL.md`.

---

## Context

The 2026-07-18 split moved pipeline content out of the engine: stage names, dirs, order, roles,
guards, artifact names and dispatch metadata are all `flow.ron` data now, and a throwaway flow
with different stage names, a different stage count and a differently-placed gate was verified
to run end to end with zero engine changes.

What follows is the **residue** — places where the engine still assumes something only the
`rust-factory` flow is guaranteed to provide. None of it breaks that flow. All of it would
surface when someone writes a second serious flow, especially a non-Rust or non-TDD one.

## The specific couplings

### 1. `da-arm-pre.js` hardcodes a reference filename

```js
`${runDir}/references/rust-standards.md`   // :134
```

The parallel-attempt judge is pointed at a file named for one language's house standards. A
Clojure or TypeScript flow would ship different references, and the judge would read a path
that does not exist.

*Fix shape:* a per-dispatch `judge_reference` in `flow.ron`, or a `standards:` list on the flow.

### 2. `da-post-gate.js` assumes Gherkin

```js
'read the test plan, list Gherkin scenarios (mechanical, one agent)'   // :6
`List every Gherkin scenario (or property, treated as one scenario) that ...`   // :87
`You are an INDEPENDENT ADVERSARIAL reviewer. You get exactly ONE Gherkin scenario ...`   // :97
```

This is deeper than a filename. The whole atomize-then-verify structure assumes the test plan
decomposes into Gherkin scenarios. A flow whose verification is, say, property-based only, or a
proof obligation list, needs a different atomizer — not a reworded prompt.

*Fix shape:* let the flow supply the atomizer prompt (a file in the flow dir) rather than trying
to parameterize the string. Attempting to make one prompt cover every decomposition style is
likely worse than letting each flow own it.

### 3. Contract filenames are conventions, not data

`CLAUDE.md` (root contract) and `CONTEXT.md` (per-stage contract) are assumed by the workflow
prompts and by the linter. A flow cannot rename them.

*Fix shape:* optional `contracts: (root: "...", stage: "...")` in `flow.ron`, defaulting to
today's names. Low value unless a flow actually wants different names — arguably these should
stay engine-wide convention.

### 4. `workspace-lint` encodes one flow's contract shape

```clojure
(def context-max 80)
(def reference-max 200)
(def stage-sections ["Inputs" "Process" "Outputs" "Audit"])   // :17-19
```

Line budgets and the required section names are `rust-factory`'s house style, applied to every
flow the linter walks. Since the linter now lints *all* flows by default, a second flow with a
different contract shape would be reported as violating rules it never signed up for.

*Fix shape:* an optional `lint:` block in `flow.ron`. **But see the open question below** —
this may be intentional.

### 5. `SKILL.md` duplicates the stage list

The `argument-hint` and the stage enumeration in §1 list `design | tests | implement | verify |
commit` in prose, which is `flow.ron` data restated by hand. It is now labelled as
"rust-factory's stage names, another flow declares its own", but it is still a second copy that
can drift.

*Fix shape:* generate it, or accept the duplication as documentation and keep the label.

## The question underneath

Items 3 and 4 are not obviously bugs. **Is the Inputs/Process/Outputs/Audit contract shape —
and the line budget that goes with it — engine-wide law, or per-flow convention?**

- If it is **law**, the linter is correct as written and should say so explicitly, and item 4
  is closed as "working as intended". That is a defensible reading: the uniform contract shape
  is arguably the algorithm itself, not a Rust-specific preference.
- If it is **convention**, a flow must be able to declare its own, and item 4 is a real gap.

This needs an answer before item 4 is worth implementing, because the two readings lead to
opposite code.

## Effect on you

Nothing today — `rust-factory` supplies everything the engine assumes. The cost lands entirely
on the *next* flow, and it lands as confusing symptoms rather than clear errors: a judge
silently reading a missing file, an adversarial reviewer asked to enumerate Gherkin scenarios in
a test plan that has none, a lint failure about a section the flow never intended to have.

That is the argument for recording it now rather than discovering it under the pressure of
writing a second flow.

## Options

**A. Leave it until a second flow exists.** Avoids designing for a hypothetical; the second
flow's actual needs would drive the right abstraction rather than a guessed one.
*Cost:* the second flow's author hits all of it at once.

**B. Fix 1 and 2 now** (the two that would actively misbehave), defer 3-5.
*Cost:* moderate; item 2 in particular needs a design decision about flow-supplied prompts.

**C. Fix everything now.** Full parameterization.
*Cost:* speculative generality against one real flow — precisely the kind of abstraction that
tends to be wrong until a second case exists.

## Recommendation on record

**B, and answer the contract-shape question above before touching item 4.** Items 1 and 2 will
mislead rather than fail loudly, which is the worst failure mode; the rest can wait for a real
second flow to define what they should look like.

## Anchors

- `engine/workflows/da-arm-pre.js:134`
- `engine/workflows/da-post-gate.js:6,:87,:97`
- `engine/bin/workspace-lint:17-19`
- `SKILL.md` — `argument-hint` and §1
