# Open question: the mirror publish is two calls, not one

**Status:** RESOLVED in da-run2 (2026-07-18) as option B (a fresh build owes no migration): one `recordSnapshot` handler takes state + files together; the two-call pair is gone from the port, the adapter, and the service.
**Area:** `crates/app/src/mirror.rs`, `services/da-steer/src/index.ts`.

---

## What happens

Publishing a run to the DaRun mirror is two independent ingress calls: the derived state, then
the artifact set. The service side matches — `recordState` and `recordArtifacts` are separate
handlers (`services/da-steer/src/index.ts:60`, `:70`) with no shared transaction.

If the first succeeds and the second fails, the mirror ends up advertising a run state that its
artifacts do not support.

## The failure

`bin/run seal` publishes after every stage. Suppose stage 04 goes green:

1. `recordState` succeeds — the mirror now says `gated-green`.
2. `recordArtifacts` fails (network, service restart, ingress hiccup).
3. The mirror holds `gated-green` alongside the *previous* stage's artifacts — no
   `gate-report.md`, and a `worktree.patch` from before the gate ran.

A restore from that mirror produces a run dir whose `run.edn`-derived state and whose files
disagree. Because state is re-derived from the filesystem on restore, the mismatch usually
self-corrects into "less complete than the mirror claimed" rather than something dangerous —
but the mirror's own view is wrong in the meantime, and anything reading mirror state directly
(a dashboard, another host deciding whether to pick up the run) is misled.

## Why it is not worse than it looks

The commit law does not trust mirror state — it re-derives from files and now also demands a
worktree identity matching the gate report. So the classic bad outcome (restore into a green
state and commit nothing) is already blocked by the durability work, independently of this bug.
That is what keeps this at medium rather than critical.

## Effect on you

Mostly invisible, and that is the problem: there is no error surfaced to the operator when the
halves diverge, because `seal` treats notify as best-effort and non-fatal by design (so a run
never dies because the mirror is down). You would notice only by restoring and finding the run
less advanced than the mirror said.

## Options

**A. Leave it.** Zero work. The dangerous consequence is already blocked elsewhere; the residue
is a stale-looking mirror that the next successful seal repairs.
*Cost:* the mirror is not trustworthy as a source of truth for "how far did this run get."

**B. One `recordSnapshot` handler taking state + files together.** The obvious fix — a single
virtual-object call, so the pair lands or neither does.
*Cost:* a wire-shape change on the service and the adapter, plus a migration for any mirrored
run written by the old pair. Larger payload per call.

**C. Publish artifacts first, then state.** Ordering alone makes the failure benign: artifacts
ahead of state is "the mirror knows less than it holds", which is safe. No protocol change.
*Cost:* narrows the window rather than closing it; a reader can still catch a half-written
artifact set. Cheap, though, and strictly better than today.

## Recommendation on record

**C now, B if the mirror ever becomes something other hosts make decisions from.** Reversing
the order is a small change that turns an inconsistent state into a merely stale one. B is the
correct fix but only earns its migration cost once mirror state is load-bearing.

## Anchors

- `crates/app/src/mirror.rs` — the two-call publish.
- `services/da-steer/src/index.ts:60,:70` — the two handlers.
