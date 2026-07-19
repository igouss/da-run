# Open question: the commit record is self-reported and never verified

**Status:** open — raised 2026-07-18. Severity: medium (false green, but narrow).
**Area:** `engine/workflows/da-post-gate.js`, `crates/adapter-fs/src/fs_snapshot.rs`.

---

## What happens

The commit stage's agent runs `git commit` itself and then reports the outcome in its own
structured output — `sha` is a plain string field it fills in (`da-post-gate.js:77`), and it
writes `commit.md` with the sha and message (`:134`). Nothing outside the agent checks that the
sha exists, or that a commit happened at all.

The run-state side is equally trusting: `commit_recorded` is simply *"the commit stage's
output/ is non-empty"* (`crates/adapter-fs/src/fs_snapshot.rs:48`). Writing the file **is** the
proof of committing.

## The failure

The agent's `git commit` fails — a pre-commit hook rejects it, the author identity is unset in
that worktree, the index is empty after a bad `squash`, disk is full. The agent still writes
`commit.md` (a separate action that succeeds) and reports `committed: true` with a plausible
sha. From then on:

- `commit_recorded` is true, so the run derives as `Committed`.
- `bin/state status` shows a finished run.
- No commit exists on the branch.

This is the same shape as the gate/worktree hole that was closed: a *report of success* is
accepted as *evidence of success*.

## Why it is narrower than it looks

The commit stage only dispatches after the gate is green and the worktree identity matches, so
the code is genuinely present and genuinely verified at that point. What is lost is the final
recording step, not the work — the change is still sitting in the worktree, sealed, with a
valid patch in the mirror. Recovery is re-running the commit stage, not redoing the run.

## Effect on you

You come back to a run that says `Committed`, and reasonably conclude it is done. The
divergence surfaces later — when you look for the commit to merge and it is not there, or worse,
when `capture` freezes a record whose manifest names a sha that does not exist. The longer the
gap, the more confusing it is, because everything in the run dir says success.

## Options

**A. Leave it.** The window is narrow and the work is recoverable.
*Cost:* keeps a known "reports success without evidence" path in the one place where the run
declares itself finished.

**B. Verify the sha outside the agent.** After the commit dispatch, run
`git -C <worktree> cat-file -e <sha>` (and check it is a commit whose parent is the base) before
accepting the record. Refuse the stage if it does not resolve.
*Cost:* small; needs a place to run it — the workflow cannot shell out, so it belongs in
`bin/run` as a post-commit verification step, in the same spirit as `seal`/`gate`.

**C. Derive `commit_recorded` from git rather than from a file.** Make the fact
"a commit exists on the run branch beyond base" rather than "a file was written".
*Cost:* puts git knowledge into the fs adapter, which currently only reads files; a bigger
architectural change than B for a similar benefit.

## Recommendation on record

**B**, implemented as a `bin/run record-commit --run <dir>` (or folded into the existing
post-gate flow) that resolves the reported sha and refuses if it does not exist. It closes the
gap without teaching the snapshot adapter about git, and it matches the pattern already used
for the gate: the orchestrator verifies, the agent reports.

## Anchors

- `engine/workflows/da-post-gate.js:77` — `sha` as agent-supplied string.
- `engine/workflows/da-post-gate.js:134` — the agent writes the commit record.
- `crates/adapter-fs/src/fs_snapshot.rs:48` — `commit_recorded` from a non-empty output dir.
