# Stage 01 — design

Turn the frozen change-spec and the existing codebase into an ECB design for the change.

## Inputs

| Source | File / Location | Scope | Why |
|--------|-----------------|-------|-----|
| spec | the run's `spec.md` | full file | what to build |
| code | the target project worktree | the modules the change touches | design against reality, not a blank page |
| reference | `../../references/architecture.md` | full file | the hexagonal / ECB / functional-core constraints |

## Process

1. Read the spec and locate every requirement **and** every acceptance criterion.
2. Read the parts of the worktree the change touches; note the existing architecture.
3. Number every requirement `R1..Rn` into a **requirement ledger**; assign each to an
   entity, control, or boundary, and mark its **verification mode**: `host-test` (a
   `cargo test`/gate can prove it) or `operator-witness` (hardware, a live endpoint, an
   acoustic/PDM check, or a detached cross-compiled workspace the host gate cannot see).
4. Name the ports (the traits the domain owns) and the adapters that implement them.
5. State the module/crate layout and the dependency direction (inward).
6. Run the Audit. Revise until it passes, then write the output.

## Outputs

| Artifact | Location | Format |
|----------|----------|--------|
| the design | `output/design.md` | markdown: the `R1..Rn` requirement ledger (id -> ECB role -> verification mode), ports, layout |

The ledger is the completeness backbone: stage 02 tests against it, stage 03 must fulfil
every row, and the operator signs off against it. A dropped requirement here is invisible
downstream — that is exactly the false green the trial guards against.

## Audit

| Check | Pass condition |
|-------|----------------|
| coverage | every spec requirement AND acceptance criterion appears in the ledger with an ECB role; none dropped |
| verification | every ledger row is marked `host-test` or `operator-witness`, honestly (do not label a testable requirement witness to dodge a test) |
| direction | all dependencies point inward; the domain imports no framework or I/O |
| fit | the change fits the existing architecture, or a real violation is called out (not copied) |
| purity | no effect (clock/disk/network/global state) sits in the domain core |
