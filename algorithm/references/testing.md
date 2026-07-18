# Testing standards

Verification is not optional. If a behaviour is not tested, it is not done.

## The test pyramid, this project's shape

- **Property tests prove the rules.** Where a behaviour is a general law ("re-encoding is
  identity", "the total never goes negative"), state it as a property over generated inputs.
- **Unit tests are precision tools.** One specific, sharp case each.
- **Integration tests: keep just a few** — enough to prove the plumbing made it through
  (the wiring across a port to a real adapter).

## Gherkin all the way down

Behaviours are specified as Gherkin scenarios (Given/When/Then). The scenario is the
executable statement of intent; the code exists to make it pass.

## Test discipline

- **Cyclomatic complexity of a test is 1.** No loops, no branches in a test body. If you
  need many cases, use a property test or a table of one-assertion cases, not a loop.
- **Zero, one, many.** For any collection or repetition, test the empty case, the single
  case, and the many case. **Two counts as many.**
- **Red first.** A new test must fail before the implementation exists, for the right
  reason. A test that passes vacuously proves nothing.

## What a tests-stage output must show

- a Gherkin scenario (or property) for every behaviour the spec names;
- zero/one/many covered for every collection or repetition;
- the suite currently **red** (compile-fail or assert-fail), demonstrably non-vacuous;
- no test reads another stage's output as a template.
