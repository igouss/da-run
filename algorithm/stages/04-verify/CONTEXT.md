# Stage 04 — verify

The gate. The deterministic edge: the change ships only if this goes green.

## Inputs

| Source | File / Location | Scope | Why |
|--------|-----------------|-------|-----|
| code | the modified target project worktree | full | verify the whole workspace, not just the diff |
| gate | `./gate.sh` here (the dispatcher) | full | the quality bar is project knowledge (ADR-0007) |

## Process

1. Run `bash "$SKILL_DIR/algorithm/bin/run" gate --run <runDir>`. It seals the worktree
   first, then invokes this stage's `gate.sh` from the worktree, then writes the report
   stamped with the sealed worktree's identity — so the verdict can never outlive the code
   it judged. Never invoke `gate.sh` directly: an unstamped report names no code, and the
   commit law refuses it. `gate.sh` is the **dispatcher**. It prefers the project's own gate
   and keeps the champion generic:
   - `<worktree>/.da/gate` present & executable → the project owns its bar, run it;
   - present but not executable → **FAIL closed** (never a silent fallback);
   - absent → run `default-gate.sh` (the generic host chain).
2. The dispatcher renders the single canonical `GATE GREEN` / `GATE RED` verdict and
   records which gate ran + its sha256, so the report is uniform and tamper-evident.
3. Capture the full output verbatim into the report; do not summarise away a failure.
4. Run the Audit.

## Outputs

| Artifact | Location | Format |
|----------|----------|--------|
| the gate report | `output/gate-report.md` | the gate's verbatim stdout+stderr and its exit code |
| the adversarial review | `output/adversarial-review.md` | one verdict per Gherkin scenario (violated/not, defect class, evidence) plus one holistic pass — ADR-0027 item 3, ADR-0028 |

The adversarial review is produced only by the automated engines (`bin/run-arm-wf`,
`bin/dynamic-arm`, via `.claude/workflows/da-post-gate.js`) — it is not part of a hand-driven,
single-agent run, and it never runs before the mechanical gate is green. It is a **hard, required
check**: a scenario found violated, or a `no` holistic verdict, blocks stage 05-commit exactly as a
red mechanical gate does. No generated stage plan (System A prime, ADR-0005/0028) can skip it.

## Audit

| Check | Pass condition |
|-------|----------------|
| green | the gate exited 0 (`GATE GREEN`) |
| no-skip | no gate step reported SKIP where the tool should have run (a skipped gate is not a pass) |
| honest | the report shows the real command output, not a paraphrase |
| adversarial (automated engines only) | every Gherkin scenario checked independently reports `violated: false`, and the holistic pass reports `fully` |
