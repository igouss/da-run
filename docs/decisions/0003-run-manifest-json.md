# ADR-0003 (da-run2): the run manifest is run.json, not run.edn

**Status:** accepted, implemented 2026-07-18. **Area:** `engine/bin/run`, `engine/bin/steer`,
`crates/adapter-fs`, `flows/rust-factory/stages/04-verify/default-gate.sh`.

## Context

The manifest `bin/run setup` writes was EDN — natural for babashka, foreign to everything
else. The Rust side read it with a tolerant `:key "value"` scan that its own doc comment
disclaimed ("Not an EDN parser"), and `default-gate.sh` extracted `base-commit` with a sed
regex over EDN text. Two consumers, two ad-hoc parsers, both silently wrong on any manifest
shape the scan did not anticipate. The operator called it: use a format both sides read and
write natively.

## Decision

`run.json`, same keys (`run-id`, `base-commit`, `phase`, …, kebab-case strings). babashka
reads and writes it with cheshire, da-state with serde (`ManifestFacts`, unknown keys
ignored, malformed JSON a loud `SnapshotError::Malformed`), and the gate script with a
one-line `bb -e` cheshire call. `run.json`'s presence now defines a run dir, and it rides
the mirror as a root artifact in place of `run.edn`.

## Consequences

- One real parser per language, no scanning; an EDN-era manifest fed to the new code is a
  loud malformed error (pinned by test), never a half-read.
- Runs mirrored before this change carry `run.edn` and will not restore under the new code.
  Homelab-acceptable: active runs re-publish on their next seal; there are no frozen records
  that need restoring across the boundary. No compatibility shim by choice.
- `retry-log.edn` (written by an external harness, only ever copied) is out of scope.
