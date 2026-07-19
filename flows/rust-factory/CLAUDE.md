# Directory algorithm — a staged Rust code factory

You advance a **frozen change-spec** against a **target project** (a git worktree on a per-run
branch) into a **verified, committed change**, one stage at a time. There is no framework: you
advance by reading the right files at the right moment and writing each stage's output to its
`output/` folder.

You may be driving **every** stage (one agent, interactive) or **one** stage (the `da-run`
skill dispatches a fresh agent per stage, each on a per-stage model — ADR-0009). Either way: read the
stage's `CONTEXT.md`, load only what it names, and a stage is done when its `output/` has files.

Nothing advances on "looks right." It advances on a stage's Audit passing and, at the end, the
gate going green and the commit written.

## Folder Map

```
directory-algorithm/        (the factory — you generate a run instance from it)
├── CLAUDE.md               (L0 — you are here)
├── CONTEXT.md              (L1 — task routing; start there)
├── flow.ron                (the pipeline as data: stages, dirs, dispatch kinds, guards —
│                            validated at load time; `bin/state flow` prints it as JSON)
├── references/             (L3 — the house standards; internalise as constraints)
│   ├── architecture.md     hexagonal / ECB, functional core
│   ├── testing.md          Gherkin, property/unit, zero-one-many
│   └── rust-standards.md   no unsafe, explicit types, one responsibility/file
└── stages/
    ├── 01-design/          spec + codebase -> a design
    ├── 02-tests/           design -> failing tests (TDD)
    ├── 03-implement/       tests -> passing code (in the worktree)
    ├── 04-verify/          the gate (deterministic edge)
    └── 05-commit/          diff + spec -> one scoped commit
```

Each stage holds `CONTEXT.md` (its contract), an `output/` handoff folder, and sometimes
`references/`. The **target project** is a sibling git worktree; you read and edit real code
there. Committed code lands in the project's own git (run branch); your `output/` folders hold
the design, the test plan, the gate report, and the commit record.

## Triggers

| Keyword | Action |
|---------|--------|
| `status` | Run `bash <skill>/engine/bin/state status --run . --pretty` (the run-state authority) and render its output. |

### How `status` works

`bin/state` derives the pipeline from the filesystem: a stage with `output/` files (beyond
`.gitkeep`, steer files excluded) is COMPLETE, else PENDING; it adds the gate verdict, parked
steer-requests, and any anomalies. Fallback when `cargo` is unavailable: scan `stages/*/output/`
by the same rule and render
`01-design ---> 02-tests ---> 03-implement ---> 04-verify ---> 05-commit` by hand.

## Routing

| Task | Go to |
|------|-------|
| design the change | `stages/01-design/CONTEXT.md` |
| write the tests | `stages/02-tests/CONTEXT.md` |
| implement | `stages/03-implement/CONTEXT.md` |
| verify | `stages/04-verify/CONTEXT.md` |
| commit | `stages/05-commit/CONTEXT.md` |

## What to Load

| At stage | Load | Do NOT load |
|----------|------|-------------|
| 01-design | the spec, the target codebase, `references/architecture.md` | test/impl references |
| 02-tests | `01-design/output/`, the spec, `references/testing.md` | other stages' outputs as templates |
| 03-implement | design + tests, the worktree, `references/rust-standards.md` | the spec's prose (tests are the spec now) |
| 04-verify | the modified worktree | everything else |
| 05-commit | the diff (`git diff <base>`), the spec, the gate verdict | the stage references |

Load only what the current stage names. Loading more context makes the output worse.

## Engines (automated driving only — irrelevant to a hand-driven session)

This folder ships as the `da-run` Claude Code skill. One engine advances it automatically: the
skill dispatches a named stage through `workflows/da-stage.js` (beside this bundle's SKILL.md),
which routes design/tests/implement to `workflows/da-arm-pre.js` and commit to
`workflows/da-post-gate.js` — the atomized adversarial reviewer (one verdict per Gherkin
scenario plus a holistic pass) that must pass before `05-commit` runs.

The mechanical `04-verify` gate is never a workflow and never an agent: the skill (or a human)
runs `stages/04-verify/gate.sh` directly and reads its GATE GREEN / GATE RED verdict.

## Stage Handoffs

Each stage writes to its own `output/`. The next stage reads from there. If the operator
edits an output file between stages, you pick up the edits — that is how a run is steered.
Do not read a later stage's output to "learn the pattern"; the `references/` are the only
authority for how to build.

## Steering (asking the operator)

When a stage hits a **load-bearing ambiguity** it does not guess: it writes
`output/STEER-REQUEST.md` and stops — the protocol (format, when to raise, how the answer
binds) is `references/steering.md`, and it applies to **every** stage. An answered
STEER-REQUEST.md already in your `output/` is operator steering: honor it like the spec.
