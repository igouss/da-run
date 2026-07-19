# Stage 05 — commit

The change is implemented and gate-green. Write the one commit that records it.

## Inputs

| Source | File / Location | Scope | Why |
|--------|-----------------|-------|-----|
| diff | `git diff <base-commit>` in the worktree | full | what actually changed — the source of truth for the message |
| spec | the run's `spec.md` | full file | why it changed — the requirement the diff serves |
| gate | `../04-verify/output/gate-report.md` | the verdict line | commit only gate-green work |

## Process

1. Confirm 04-verify was GREEN; if not, stop — never commit unverified work.
2. Read the full `git diff <base-commit>` and the spec.
3. Write a scoped commit message: a `<scope>: <imperative, lowercase>` subject, then a body
   saying WHAT changed and WHY (the spec's intent), not how. Scope names the subsystem touched,
   not a type (house doctrine: scopedcommits — never Conventional-Commits `feat`/`fix`).
4. `git add -A` and commit on the run branch. Exactly one commit for the change.
5. Run the Audit, then write the output.

## Outputs

| Artifact | Location | Format |
|----------|----------|--------|
| the commit | the target project's git (run branch) | one committed change |
| the message record | `output/commit.md` | the sha + the full commit message |

## Audit

| Check | Pass condition |
|-------|----------------|
| green-only | the committed tree is the gate-green tree (04-verify passed) |
| scoped | subject is `scope: imperative`, lowercase, no type-first prefix |
| grounded | the message reflects the actual diff and the spec's intent, not a guess |
| single | exactly one commit records the change on the run branch |
