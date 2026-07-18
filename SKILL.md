---
name: da-run
description: Run one stage of the directory algorithm (design, tests, implement, verify, commit — or all) against a requirements file, via the algorithm's bundled Claude Code workflows. Use when the user says "/da-run", "run stage X on <requirements>", or wants to advance a directory-algorithm run one stage at a time.
argument-hint: <stage> <requirements-file> [--project P] [--run RUNDIR] [--attempts N]
---

# da-run — one stage of the directory algorithm, on demand

Advance a directory-algorithm run instance by exactly one stage (or the full pipeline). This
skill is **self-contained**: the workflows and the algorithm folder it drives are bundled beside
this file. Two required arguments:

1. **stage** — one of `design`, `design-review`, `tests`, `implement`,
   `implement-parallel-attempt`, `verify`, `commit`, or `all`.
2. **requirements** — path to the frozen change-spec (markdown). This becomes the run's
   `spec.md`.

## Step 0 — locate the bundle

Let `SKILL_DIR` = the absolute path of the directory containing this SKILL.md (you know it from
how this skill was loaded; otherwise find it with
`ls ~/.claude/skills/da-run/SKILL.md .claude/skills/da-run/SKILL.md 2>/dev/null`). Everything
below is addressed from it:

- `$SKILL_DIR/workflows/` — `da-stage.js`, `da-arm-pre.js`, `da-post-gate.js`
- `$SKILL_DIR/algorithm/` — the factory: `CLAUDE.md`, `CONTEXT.md`, `references/`, `stages/`,
  `bin/run`

Dependencies: `bb` (babashka) and `git` on PATH. If `bb` is missing, tell the user
(`https://babashka.org` — single static binary) rather than improvising the setup by hand.

## Step 1 — resolve the run instance

A stage always executes inside a run instance (the folder holding `CLAUDE.md`, `stages/`,
`worktree/`, `run.edn`, `spec.md`). Resolve it in this order:

1. `--run RUNDIR` given → use it. Verify `RUNDIR/run.edn` exists; refuse if not.
2. The cwd contains `run.edn` → the cwd is the run instance.
3. Otherwise **create one**: the target project is `--project P` if given, else the cwd (it must
   be a git repo with a clean working tree — the driver refuses a dirty one; relay its message).

   ```sh
   bb "$SKILL_DIR/algorithm/bin/run" setup --project <P> --spec <requirements> --arm folder --round ad-hoc
   ```

   Parse `run-dir` from its JSON output — that is the run instance. Its worktree is
   `<run-dir>/worktree`. (Run root: `$DA_RUN_ROOT`, else `~/.cache/directory-algorithm/runs`.)

If the run instance already exists but the given requirements file differs from its `spec.md`,
copy the requirements over `spec.md` and say so — the spec on disk is the one every stage reads.

## Step 2 — dispatch the stage

**Agent stages** (`design`, `design-review`, `tests`, `implement`, `implement-parallel-attempt`,
`commit`): invoke the **Workflow** tool with `scriptPath` `$SKILL_DIR/workflows/da-stage.js` and
args (absolute paths only):

```json
{ "runDir": "<absolute run dir>", "stage": "<stage>", "round": "ad-hoc",
  "workflowsDir": "<SKILL_DIR>/workflows", "attempts": <N-if-given> }
```

`workflowsDir` is required here — without it `da-stage.js` falls back to a project-local
`.claude/workflows/` that does not exist outside the trial repo. `da-stage.js` routes to the
right engine (per-stage models: design/implement on opus, tests/commit on sonnet; `commit` runs
the atomized adversarial reviewer first and is blocked by any violated scenario). Relay the
workflow's returned JSON to the user — audits passed, files written, and for `commit` whether the
adversarial gate passed and the commit sha.

**`verify`**: never a workflow (the mechanical gate stays mechanical — no LLM in a deterministic
check). Run it yourself with Bash:

```sh
cd <run-dir>/worktree && bash ../stages/04-verify/gate.sh
```

The verbatim output goes to `<run-dir>/stages/04-verify/output/gate-report.md` (write it there if
the gate script didn't). Report the final `GATE GREEN` / `GATE RED` line honestly — a red gate is
the result, not a problem to talk around. The gate prefers the target project's own
`.da/gate` when present and executable; otherwise it runs the bundled default host chain.

**`all`**: run the sequence `design → tests → implement → verify → commit`, in order, stopping at
the first failure: an agent stage whose workflow throws or reports `allAuditsPassed: false`, or a
red gate. Never continue past a red verify into commit.

## Ordering guards

Refuse (with a one-line explanation) any dispatch that violates the pipeline's handoff order:

- `tests` before `stages/01-design/output/` has a design.
- `implement` before `stages/02-tests/output/` has a test plan.
- `commit` before `stages/04-verify/output/gate-report.md` shows `GATE GREEN`.

`design` and `design-review` may always run; `design-review` warns if there is no design yet to
review. The operator can steer between stages by editing any `output/` file — that is the point
of running one stage at a time.

## After the run (optional)

The committed change lives on the run branch (`da/<run-id>`) in the target project's own git —
merge it as the user prefers. To freeze an immutable run record (manifest, diff, gate report,
traces), set `DA_RECORDS` to a directory the user owns and run:

```sh
DA_RECORDS=<records-dir> bb "$SKILL_DIR/algorithm/bin/run" capture --run <run-dir> --round ad-hoc
```

Skip capture silently for casual runs; offer it when the user cares about provenance.

## Report

End with: the stage run, its verdict (audit passed / gate color / commit sha), the files it
wrote, and the natural next stage.
