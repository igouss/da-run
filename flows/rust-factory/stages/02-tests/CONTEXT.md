# Stage 02 — tests

Write the tests first, from the design. They must fail before any implementation exists.

## Inputs

| Source | File / Location | Scope | Why |
|--------|-----------------|-------|-----|
| design | `../01-design/output/design.md` | full file | what the change is, in ECB terms |
| spec | the run's `spec.md` | full file | the behaviours to pin |
| code | the target project worktree | the touched modules + their test conventions | match how this project tests |
| reference | `../../references/testing.md` | full file | Gherkin, property/unit, zero-one-many, cyclomatic-1 |

## Process

1. Walk the design's `R1..Rn` ledger. For every `host-test` row, write a Gherkin scenario
   (or a property when it is a law). For every `operator-witness` row, record it in the plan
   as witness-only with the manual check the operator will run — **do not silently drop it**.
2. Cover zero / one / many for every collection or repetition (two counts as many).
3. Add the tests into the worktree, matching the project's test layout and idiom.
4. Run them; confirm they are **red** for the right reason (not vacuous, not compile-noise unrelated to the change).
5. Run the Audit. Revise until it passes, then write the output.

## Outputs

| Artifact | Location | Format |
|----------|----------|--------|
| the test plan | `output/test-plan.md` | markdown: ledger id -> test (with the red result quoted) OR witness-only + the manual check |
| the tests | into the worktree | project-native test files (committed to the run branch) |

## Audit

| Check | Pass condition |
|-------|----------------|
| coverage | every `host-test` ledger row has a red scenario/property; every `operator-witness` row is listed witness-only — the whole ledger is accounted for, nothing dropped |
| zero-one-many | every collection/repetition has empty, single, and many cases |
| red-first | the suite fails now, for the right reason — demonstrably non-vacuous |
| complexity | no test body has a loop or branch (cyclomatic 1) |
