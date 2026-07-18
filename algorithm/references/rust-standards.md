# Rust standards

The quality floor for generated code. The mechanical gate (`stages/04-verify`) enforces the
checkable subset; these are the constraints behind it.

## Safety

- **No `unsafe`.** None. If a task seems to need it, the design is wrong — find the safe way.

## Types

- **Explicit type annotations on variables.** Do not lean on inference for locals; annotate.
- **Explicit type annotations on lambda parameters.**
- **Lean on the type system.** Make illegal states unrepresentable; prefer a type that
  forecloses the bad move over a runtime check that catches it.

## Files and clarity

- **One responsibility per file.** A file has one reason to change.
- Match the surrounding code: its comment density, naming, and idiom.
- **Clean over easy.** If the clean solution costs more, take it; if a workaround needs a
  paragraph-long comment to justify it, the code is wrong — fix the code.

## Commits (scoped commits)

`<scope>: <description>` — scope names the subsystem/module touched (`i2c:`, `render:`), not
a type (`feat`/`fix`). Description is short, imperative, lowercase.

## What an implementation must satisfy

- all tests from `02-tests` green;
- no `unsafe`; `cargo clippy -D warnings` clean;
- explicit types on locals and lambda params;
- one responsibility per file;
- `hex-lint` and `effect-audit --strict` green (architecture + functional-core purity).
