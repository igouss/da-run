# Atomizer — how this flow decomposes its test plan for adversarial review

This flow's test plans are Gherkin + property tests (references/testing.md).
Decompose them into independently-verifiable scenarios:

- List every Gherkin scenario the tests stage wrote for this change. A
  property test counts as one scenario: its Given is the input domain, its
  When is "any generated input", its Then is the invariant.
- Do not invent scenarios that are not in the test plan or the worktree's
  tests — you enumerate, you do not author.
- Do not merge distinct zero/one/many cases into one entry; each is its own
  scenario (two counts as many, and gets its own line).
- Ids are short and stable, `R<n>-<case>` style (e.g. `R3-zero`), so a
  verdict can be traced back to the requirement ledger.
