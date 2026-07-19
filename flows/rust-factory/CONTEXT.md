# Directory algorithm

Spec + a target project's codebase, in one end, a verified Rust change out the other, one
stage at a time. Start here for routing; the identity and folder map are in `CLAUDE.md`.

## Task Routing

| Task Type | Go To | Description |
|-----------|-------|-------------|
| design | `stages/01-design/CONTEXT.md` | derive the ECB design for the change from the spec + existing code |
| tests | `stages/02-tests/CONTEXT.md` | write the failing Gherkin / property / unit tests first |
| implement | `stages/03-implement/CONTEXT.md` | modify the project to pass the tests |
| verify | `stages/04-verify/CONTEXT.md` | run the gate; green or the change does not ship |
| commit | `stages/05-commit/CONTEXT.md` | read the diff + spec; write one scoped commit on the run branch |

## Shared Resources

| Resource | Location | Contains |
|----------|----------|----------|
| architecture | `references/architecture.md` | hexagonal / ECB, functional core, deps point inward |
| testing | `references/testing.md` | Gherkin, property/unit/integration split, zero-one-many |
| rust standards | `references/rust-standards.md` | no unsafe, explicit types, single responsibility |

The `references/` are the canonical home for the house standards. Stages point to them; they
are never copied into a stage's contract.
