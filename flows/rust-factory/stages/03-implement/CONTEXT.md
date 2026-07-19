# Stage 03 — implement

Modify the project so the tests from stage 02 pass, within the quality floor.

## Inputs

| Source | File / Location | Scope | Why |
|--------|-----------------|-------|-----|
| design | `../01-design/output/design.md` | full file | the shape to build to |
| tests | `../02-tests/output/test-plan.md` + the worktree tests | full | the executable spec |
| code | the target project worktree | the touched modules | where the change lands |
| reference | `../../references/rust-standards.md` | full file | no unsafe, explicit types, one responsibility/file |

## Process

1. Implement against the design, editing the project's files in the worktree.
2. Make the stage-02 tests pass — do not weaken or delete a test to make it green.
3. Keep the domain core pure; keep effects at the boundaries.
4. **Walk the design's `R1..Rn` ledger against `git diff <base>`.** Every row must be reflected
   in the change: `host-test` rows proven by their now-green tests; `operator-witness` rows
   present in code (the adapter, the firmware primitive, the composition wiring) even though the
   host gate cannot prove them. Implement any row that is missing — a gate going green on a
   subset of the ledger is **not** done. This is the round-1 failure (built one layer, stopped at
   first green) turned into a hard stop.
5. Seal the stage: `bash "$SKILL_DIR/engine/bin/run" seal --run <runDir> --stage implement`. This
   commits the worktree and refreshes worktree.patch, so the work survives a host move — untracked
   files are invisible to `git diff` and would otherwise be lost. Stage 05 squashes these stage
   commits into the one clean commit once the gate is green.
6. Run the Audit. Revise until it passes, then write the output.

## Outputs

| Artifact | Location | Format |
|----------|----------|--------|
| the change | the worktree (run branch, uncommitted) | source edits |
| a completeness ledger | `output/completeness.md` | markdown: ledger id -> `fulfilled` / `partial` / `missing` + evidence (file:path, or the passing test) |
| a change note | `output/change-note.md` | markdown: what changed, which files, why |

## Audit

| Check | Pass condition |
|-------|----------------|
| complete | every ledger row is `fulfilled`: `host-test` rows have a green test, `operator-witness` rows are implemented in the diff — **zero** `partial` or `missing` |
| green | every stage-02 test passes; none was weakened or removed |
| safety | no `unsafe` anywhere in the change |
| types | explicit annotations on locals and lambda params |
| responsibility | each new/edited file has a single reason to change |
