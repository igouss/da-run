---
name: da-run
description: Run one stage of the directory algorithm (design, tests, implement, verify, commit — or all) against a requirements file, via the algorithm's bundled Claude Code workflows. Use when the user says "/da-run", "run stage X on <requirements>", or wants to advance a directory-algorithm run one stage at a time.
argument-hint: <stage> <requirements-file> [--project P] [--run RUNDIR] [--attempts N]
---

# da-run — one stage of the directory algorithm, on demand

Advance a directory-algorithm run instance by exactly one stage (or the full pipeline). This
skill is **self-contained**: the engine that drives a run and the flows it can drive are
bundled beside this file. The stage names below are `rust-factory`'s, the default flow —
another flow declares its own in `flow.ron`. Two required arguments:

1. **stage** — one of `design`, `design-review`, `tests`, `implement`,
   `implement-parallel-attempt`, `verify`, `commit`, or `all`.
2. **requirements** — path to the frozen change-spec (markdown). This becomes the run's
   `spec.md`.

## Step 0 — locate the bundle

Let `SKILL_DIR` = the absolute path of the directory containing this SKILL.md (you know it from
how this skill was loaded; otherwise find it with
`ls ~/.claude/skills/da-run/SKILL.md .claude/skills/da-run/SKILL.md 2>/dev/null`). Everything
below is addressed from it:

- `$SKILL_DIR/engine/` — the reusable machine, identical for every flow: `bin/run`,
  `bin/state`, `bin/steer`, `bin/workspace-lint`, `workflows/` (`da-stage.js`,
  `da-arm-pre.js`, `da-post-gate.js`), and `gate/dispatch.sh`
- `$SKILL_DIR/flows/<name>/` — one pipeline's content: `flow.ron`, `CLAUDE.md`, `CONTEXT.md`,
  `references/`, `stages/`. `rust-factory` is the default; `--flow <name>` selects another.
  A new pipeline is a new directory here — no engine change.
- `$SKILL_DIR/services/da-steer/` + `$SKILL_DIR/infra/systemd/` — optional Restate endpoint:
  the durable steer park (DaSteer) and the run-state mirror (DaRun, fed by `bin/state
  notify`). Not needed for file-only steering; see its README.

Dependencies: `bb` (babashka), `git`, and `cargo` on PATH. If `bb` is missing, tell the user
(`https://babashka.org` — single static binary) rather than improvising the setup by hand;
same for `cargo` (`https://rustup.rs`) — it builds `bin/state`, the run-state authority,
on first use.

## Step 1 — resolve the run instance

A stage always executes inside a run instance (the folder holding `CLAUDE.md`, `stages/`,
`worktree/`, `run.json`, `spec.md`). Resolve it in this order:

1. `--run RUNDIR` given → use it. Verify `RUNDIR/run.json` exists; refuse if not.
2. The cwd contains `run.json` → the cwd is the run instance.
3. Otherwise **create one**: the target project is `--project P` if given, else the cwd (it must
   be a git repo with a clean working tree — the driver refuses a dirty one; relay its message).

   ```sh
   bb "$SKILL_DIR/engine/bin/run" setup --project <P> --spec <requirements> --arm folder --round ad-hoc
   ```

   Add `--flow <name>` to drive a pipeline other than the default `rust-factory` (a name under
   `$SKILL_DIR/flows/`, or an absolute path to a flow dir). The run dir is flattened from the
   flow, so its shape is the same whichever flow built it; `run.json` records which one did.

   Parse `run-dir` from its JSON output — that is the run instance. Its worktree is
   `<run-dir>/worktree`. (Run root: `$DA_RUN_ROOT`, else `~/.cache/directory-algorithm/runs`.)

If the run instance already exists but the given requirements file differs from its `spec.md`,
copy the requirements over `spec.md` and say so — the spec on disk is the one every stage reads.

## Step 2 — dispatch the stage

**Agent stages** (`design`, `design-review`, `tests`, `implement`, `implement-parallel-attempt`,
`commit`): invoke the **Workflow** tool with `scriptPath` `$SKILL_DIR/engine/workflows/da-stage.js` and
args (absolute paths only):

```json
{ "runDir": "<absolute run dir>", "stage": "<stage>", "round": "ad-hoc",
  "workflowsDir": "<SKILL_DIR>/engine/workflows", "attempts": <N-if-given>,
  "stateCheck": <the JSON printed by the ordering-guard check below> }
```

`workflowsDir` is required — `da-stage.js` refuses to run without it (the engines it routes
to live beside it in the bundle). `da-stage.js` routes to the
right engine (per-stage models: design/implement on opus, tests/commit on sonnet; `commit` runs
the atomized adversarial reviewer first and is blocked by any violated scenario; a `partial`
holistic verdict raises a steer-request and parks — enter the steer flow below). Relay the
workflow's returned JSON to the user — audits passed, files written, and for `commit` whether the
adversarial gate passed and the commit sha.

Journaling is structural (ADR-0004): the `state check` you already ran journals
`dispatch:<stage>` to events.jsonl itself, so operator edits between stages stay
distinguishable from stage work with no extra call. `bin/run mark` exists only for ad-hoc
triggers (e.g. `--trigger steer:applied` after hand-editing an output as steering).

**After a `commit` dispatch reports `committed: true`**, verify the record against git before
trusting it — the agent's sha is a claim, not evidence:

```sh
bb "$SKILL_DIR/engine/bin/run" record-commit --run <run-dir>
```

It refuses when the branch tip never moved (re-run the commit stage and say so honestly) and
writes the `commit-verified` marker the run state derives `Committed` from. A run without this
marker is NOT committed, whatever the agent said.

**`verify`**: never a workflow (the mechanical gate stays mechanical — no LLM in a deterministic
check). Run it yourself with Bash:

```sh
bash "$SKILL_DIR/engine/bin/run" gate --run <run-dir>
```

The verbatim output goes to `<run-dir>/stages/04-verify/output/gate-report.md` (write it there if
the gate script didn't). Report the final `GATE GREEN` / `GATE RED` line honestly — a red gate is
the result, not a problem to talk around. The gate prefers the target project's own
`.da/gate` when present and executable; otherwise it runs the bundled default host chain.

**`all`**: run the sequence `design → tests → implement → verify → commit`, in order, stopping at
the first failure: an agent stage whose workflow throws or reports `allAuditsPassed: false`, or a
red gate. Never continue past a red verify into commit, and always finish a successful commit
with `record-commit` (above).

## Steer-requests (a stage asking the operator)

Any agent stage may pause by writing `stages/<NN>/output/STEER-REQUEST.md` (protocol:
`$SKILL_DIR/flows/rust-factory/references/steering.md`) instead of completing. After every stage dispatch,
run `bb "$SKILL_DIR/engine/bin/steer" check --run <run-dir>` — exit 3 means pending. Then:

1. Relay the `## Question` and `## Options` to the user verbatim and ask for their decision.
2. Write it under `## Answer` (or `bb "$SKILL_DIR/engine/bin/steer" resolve --run <run-dir>
   --stage <NN> --answer "..." --reason <code>`), then re-dispatch the same stage — the
   answered steer binds like the spec. `--reason` classifies the steer for the ADR-0010 meter
   (`spec-gap | spec-wrong | scope-cut | preference | other`) — ask the user which fits when
   it is not obvious from their answer.
3. If the run is parked durably (`DA_STEER_INGRESS` set, `bin/steer park` running), don't
   block the session waiting: use a Monitor on `bin/steer check` (exit 0 = answered) or on
   the Restate output endpoint, and continue when it fires.

A steer pause is the operator's turn, not a failure — never retry the stage over an
unanswered request, and never answer it yourself.

## Ordering guards

The run-state machine `bin/state` is the authority on dispatch order — do not re-derive its
rules from prose. Before **any** Step-2 dispatch, run:

```sh
bash "$SKILL_DIR/engine/bin/state" check --run <run-dir> <stage>
```

- exit 0 — allowed: dispatch, passing the printed JSON as `stateCheck` in the workflow args
  (Step 2); relay any advisory `warnings`.
- exit 4 — refused: relay the JSON `reason.detail` verbatim and stop.
- exit 3 — a steer-request is pending: enter the steer flow above; never dispatch over it.
- exit 2 — broken run dir: relay the error.

The operator can steer between stages by editing any `output/` file — that is the point of
running one stage at a time; a re-dispatch of a complete stage is a warning, never a refusal.
`bash "$SKILL_DIR/engine/bin/state" status --run <run-dir> --pretty` renders the whole
pipeline (state, gate verdict, parked steers) whenever you need the picture.

## After the run (optional)

The committed change lives on the run branch (`da/<run-id>`) in the target project's own git —
merge it as the user prefers. To freeze an immutable run record (manifest, diff, gate report,
traces), set `DA_RECORDS` to a directory the user owns and run:

```sh
DA_RECORDS=<records-dir> bb "$SKILL_DIR/engine/bin/run" capture --run <run-dir> --round ad-hoc
```

Skip capture silently for casual runs; offer it when the user cares about provenance.

## Report

End with: the stage run, its verdict (audit passed / gate color / commit sha), the files it
wrote, and the natural next stage.
