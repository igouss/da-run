# Open questions

Decisions that are **raised but deliberately not taken**, and known gaps that are recorded
rather than fixed. Each note states the situation, what triggers it, the options with their
costs, and a recommendation on record — but nothing here has been implemented.

These are distinct from ADRs: an ADR records a decision that was made. These are the ones still
waiting for a call, kept here so they are visible instead of living in someone's memory.

| Question | Area | Severity | Recommendation on record |
|---|---|---|---|
| [partial-holistic-verdict](partial-holistic-verdict.md) — should a `partial` adversarial verdict block the commit? | `da-post-gate.js` | **High** — the only detector for "test plan missed a requirement" is discarded | Route to a steer-request, and re-tune the reviewer's bias in the same change |
| [commit-record-trust](commit-record-trust.md) — the commit sha is self-reported and never verified | `da-post-gate.js`, `fs_snapshot.rs` | Medium — a run can read `Committed` with no commit | Verify the sha outside the agent |
| [publish-atomicity](publish-atomicity.md) — mirror state and artifacts are two calls | `mirror.rs`, `da-steer` | Medium — mirror can advertise a stage its artifacts don't support | Reverse the order now; single call if mirror state becomes load-bearing |
| [flow-content-in-engine](flow-content-in-engine.md) — flow-specific assumptions left in the engine after the split | `workflows/`, `workspace-lint` | Medium — costs the *next* flow, not this one | Fix the two that misbehave silently; answer the contract-shape question first |
| [run-branch-transport](run-branch-transport.md) — should run branches be pushed, or stay patch-only? | `bin/run` | Low — current transport is verified | Stay patch-only; revisit if the base commit is ever unreachable |
| [artifact-encoding](artifact-encoding.md) — artifact content is lossy for non-UTF-8 bytes | `run_artifacts.rs` | Low today, latent in the durability layer | Error instead of lossy-converting |

## Reading order

If you are picking one up cold, `partial-holistic-verdict` is the one with real consequences
for a change you are about to merge, and the only one whose answer depends on taste rather than
on evidence. The rest have fairly clear right answers and are open mainly on cost.

## Two of these share a root cause

`commit-record-trust` and `partial-holistic-verdict` are both instances of the same pattern:
**an agent's report of success is accepted as evidence of success.** The gate/worktree binding
closed the largest instance of it (a green verdict no longer means anything unless the worktree
matches what was verified); these two are what remain. Worth treating as a family rather than
as unrelated tickets — a fix for one is a template for the other.
