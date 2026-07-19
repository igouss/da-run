# Open question: should run branches be pushed, or stay patch-only?

**Status:** undecided — raised 2026-07-18 after the repo gained a remote.
**Area:** `engine/bin/run` (setup / seal / restore), `crates/adapter-fs/src/run_artifacts.rs`.
**Decision owner:** the operator. Current behaviour is patch-only and is unchanged by this note.

---

## Background: how a run's code moves today

A run creates a branch `da/<run-id>` in the **target project's** git and checks it out as a
worktree under the run dir (`engine/bin/run:57`, `:342`). That branch is local to whichever
machine started the run — `README.md:22` states the rule plainly: the result lands on its own
branch, *"never on `main`, never pushed"*.

Because it is never pushed, branch + base commit alone cannot reconstitute a run elsewhere.
So the code travels a different way: `bin/run seal` freezes `git diff --binary <base>` as
`worktree.patch`, which is a mirrored artifact (`run_artifacts.rs` `ROOT_FILES`). On restore,
`bin/run restore` recreates the worktree at the base commit and applies the patch, provided a
clone holding that base commit is present (`engine/bin/run:456-466`).

Two properties follow, and they are the reason the patch exists:

- The patch is **base-relative text**, so *any* clone with the base commit can rebuild the
  run. It does not need the origin host's filesystem paths, its branch, or its reflog.
- The patch's sha256 is the worktree's **content identity**, which is what binds the gate
  verdict to the code it judged. A commit sha could not do this job: a restore re-applies the
  patch and produces different commit shas, so a sha-based identity would refuse every
  restored run. See `crates/domain/src/worktree.rs`.

## What changed

Until 2026-07-18 this repo had no remote, so "push the run branch" was not an option at all.
It now has one (`github.com/igouss/da-run`). That does **not** by itself make branch-pushing
viable — the run branch lives in the *target project's* git, not in this repo — but it removes
the assumption that no remote exists anywhere, and it is worth writing down why the patch is
still the transport before someone "fixes" it.

## Why this is not simply an improvement

The obvious-looking change ("just push the run branch, then restore is a `git fetch`") moves
the problem rather than solving it:

- **It requires every target project to have a writable remote.** The algorithm runs against
  arbitrary local projects; some are homelab repos with no remote at all. Patch transport has
  no such requirement.
- **It pushes work-in-progress to a shared place.** Stage commits are deliberately intermediate
  — `bin/run squash` collapses them before the real commit. Pushing them publishes noise that
  someone then has to clean up, and on a shared remote that noise is visible to others.
- **It does not replace the content identity.** Even with a pushed branch, the gate binding
  still needs a stable content hash across hosts, so `worktree.patch` (or an equivalent digest)
  has to exist anyway. Pushing branches adds a mechanism without removing one.
- **The patch path is verified; the branch path is not.** End-to-end restore via patch has been
  exercised on a scratch project including untracked files. A push/fetch path has no coverage.

## Where patch-only genuinely hurts

Stated honestly, because these are the cases that would justify revisiting:

- **Large or binary-heavy changes.** A patch is a full diff from base, re-serialised on every
  seal and stored whole in the mirror. A run touching large binary assets makes for a large
  artifact, where git would transfer deltas and reuse objects.
- **Long-lived runs that drift far from base.** The patch grows with the change; git's
  transfer cost would not grow the same way.
- **Losing intermediate history on restore.** The patch carries the *state*, not the stage-by-
  stage commits. A restored run cannot show "what did stage 03 do, separately from stage 02" —
  though `squash` throws that history away before the final commit anyway, so it is lost at the
  end regardless.
- **The base commit must be reachable.** If the target project's base commit only ever existed
  on the dead host, restore cannot rebuild the worktree at all (`:456-458` checks for exactly
  this and falls back to printing manual instructions). A pushed branch would carry its own
  base. **This is the strongest argument for pushing** and the one real hole in patch-only
  transport.

## Options

### A. Patch-only (status quo)

- **+** Works against projects with no remote, which is the common homelab case.
- **+** Nothing work-in-progress is published anywhere.
- **+** Already implemented and verified end-to-end, including untracked files.
- **+** The content identity that the commit law depends on falls out of it for free.
- **−** Mirror artifacts grow with diff size; poor fit for large binary changes.
- **−** Cannot restore if the base commit is unreachable on the restoring host.
- **−** Intermediate stage history does not survive a restore.

### B. Push the run branch as well

- **+** Fixes the unreachable-base-commit hole: the branch carries its own history.
- **+** Efficient transfer for large or long-lived changes.
- **+** Restored runs keep stage-by-stage history.
- **−** Requires a writable remote on every target project; breaks the no-remote case unless
  it stays optional, and an optional path that is rarely exercised is a path that rots.
- **−** Publishes work-in-progress commits that `squash` is designed to discard.
- **−** Does not remove the need for `worktree.patch` — the gate's content identity still
  needs a host-stable digest.
- **−** Two transports to keep correct, and a new failure mode (push succeeded, mirror did
  not, or vice versa) on top of the publish-atomicity gap already recorded.

### C. Push only on demand, patch always

Keep the patch as the mechanism the system depends on; add an explicit `bin/run publish-branch`
for the cases where a human wants the branch somewhere shareable (handing work to a colleague,
a genuinely large change, or a base commit at risk of being lost).

- **+** Keeps one verified transport as the default; the push is an operator action with a
  clear reason, not an implicit dependency.
- **+** Addresses the large-change and unreachable-base cases when they actually arise.
- **−** Another command, and it must not become a thing people are expected to remember —
  if the base-commit hole matters routinely, C is a workaround and B is the honest fix.

## Recommendation on record

**A for now, C if the base-commit hole ever bites in practice.** B trades a verified,
dependency-free transport for one that needs a writable remote on every target project, and it
does not let us delete the patch — so it is a net addition of mechanism. The unreachable-base
case is real but has not happened; the sensible trigger for revisiting is the first time a
restore fails at `engine/bin/run:456` for that reason, or the first run whose patch is large
enough to be a problem in the mirror.

## What would have to change for B or C

- `engine/bin/run` — a push step after `seal!` (B) or a new subcommand (C); either way it must
  be non-fatal when the target project has no remote.
- `README.md:22` — the "never pushed" rule is stated as a property of the system and would
  need rewording.
- `restore` (`:456-466`) — prefer fetching the branch when available, falling back to base +
  patch, without losing the content-identity check.
- Nothing in `crates/` — the commit law keys on the patch digest and is indifferent to how the
  code travelled.

## Not affected by this decision

`worktree.patch` stays either way: it is what gives the worktree a host-stable identity, which
is what makes a green gate mean anything after a restore. This question is only about whether a
*second*, git-native transport should exist alongside it.
