# Open questions

da-run (the first trial implementation) recorded six decisions that were raised but
deliberately not taken. The da-run2 fresh build took five of them — each file below keeps its
full original analysis with the resolution stamped in its Status line, because the analysis is
why the resolution is what it is.

| Question | Resolution in da-run2 |
|---|---|
| [partial-holistic-verdict](partial-holistic-verdict.md) | **C** — `partial` raises a steer-request and parks; bias re-tuned in the same change |
| [commit-record-trust](commit-record-trust.md) | **B** — `bin/run record-commit` verifies against git; `Committed` derives from the marker |
| [publish-atomicity](publish-atomicity.md) | **B** — one `recordSnapshot` call; the fresh build owed no migration |
| [flow-content-in-engine](flow-content-in-engine.md) | **B** — items 1-2 are flow data now; contract shape declared engine law |
| [artifact-encoding](artifact-encoding.md) | **B** — non-UTF-8 is a loud typed error in both read paths |
| [run-branch-transport](run-branch-transport.md) | **Still open (A on record)** — patch-only transport stands; revisit on the first unreachable-base restore failure |

The root-cause family the old index named — *an agent's report of success accepted as evidence
of success* — is closed: the gate/worktree binding (da-run), the verified commit record, and
the content-keyed steer park (both da-run2) were its three instances.

New decisions taken during the fresh build are recorded in [`../decisions/`](../decisions/).
