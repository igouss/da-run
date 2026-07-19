# ADR-0002 (da-run2): the mirror protocol is one recordSnapshot call

**Status:** accepted, implemented 2026-07-18. **Area:** `crates/ports`, `crates/adapter-restate`,
`crates/wire`, `services/da-steer`.

## Context

da-run published a run to the DaRun mirror as two independent ingress calls — `recordState`
then `recordArtifacts` — so a half failure left the mirror advertising a state its artifacts
did not support. The recorded recommendation was to merely reorder (option C) because option B
(one call) carried a wire migration; a fresh build owes no migration.

## Decision

One handler, `recordSnapshot { state, files }` (`RunSnapshotWire` on the Rust side, pinned by
insta). `RunMirror::publish_snapshot(run_id, derived, files)` is the only publish method.
Restate commits a handler invocation's state changes atomically, so state and artifacts land
together or not at all. `getState` / `getSnapshot` are unchanged.

## Consequences

- The mirror can never advertise a state ahead of (or behind) its artifacts.
- Deploying the da-run2 service replaces the old two-handler protocol: re-register the
  endpoint with Restate, and note that an old `da-state notify` will 404 against the new
  service (and vice versa) — notify is best-effort by design, so runs survive the mismatch,
  but mirrors stop updating until both sides are da-run2.
- Payloads are larger per call (full artifact set + state every seal). Acceptable at homelab
  scale; the full-replace semantics were already paying that cost on the artifacts half.
