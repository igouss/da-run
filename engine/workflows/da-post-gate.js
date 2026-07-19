export const meta = {
  name: 'da-post-gate',
  description: 'Atomized adversarial review (ADR-0027 #3) then the scoped commit, for any directory-algorithm arm whose mechanical gate already went GREEN.',
  whenToUse: 'Called by bin/run-arm-wf and bin/dynamic-arm after gate.sh has been run by the babashka harness (never by this workflow) and returned GATE GREEN. Never call directly on a run whose gate has not passed.',
  phases: [
    { title: 'Atomize', detail: 'read the test plan, list Gherkin scenarios (mechanical, one agent)' },
    { title: 'Verify', detail: 'one adversarial reviewer per scenario, in parallel, plus one holistic pass' },
    { title: 'Commit', detail: 'sonnet — diff + spec + review verdict -> one scoped commit' },
  ],
}

// args (all required — this workflow never touches the filesystem itself, so every
// path an agent needs must arrive pre-resolved by the caller from the run's flow.ron;
// no stage dir or artifact name lives in this script):
//   runDir           absolute path to the run instance (holds CLAUDE.md, stages/, worktree/, run.edn)
//   worktree         absolute path to the target project's worktree (runDir + '/worktree')
//   round            the round id (for labeling only)
//   commitModel      model id for the commit stage (the flow pins it per ADR-0009; a
//                    dynamic-arm plan may override it — the CALLER decides, this workflow just uses it)
//   testPlanPath     absolute path to the test plan artifact (the tests handoff's output)
//   gateReportPath   absolute path to the mechanical gate's verdict report
//   reviewPath       absolute path where the adversarial review verdict is published
//   commitRecordPath absolute path where the commit stage records the sha + message

const DEFECT_CLASSES = [
  'spec-completeness', 'spec-misreading', 'compile-toolchain', 'integration-wiring',
  'resource-budget', 'test-vacuity', 'architecture-boundary', 'regression',
]

const SCENARIO_LIST_SCHEMA = {
  type: 'object',
  properties: {
    scenarios: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          id: { type: 'string', description: 'a short stable id, e.g. "R3-zero"' },
          title: { type: 'string' },
          given: { type: 'string' },
          when: { type: 'string' },
          then: { type: 'string' },
        },
        required: ['id', 'title', 'given', 'when', 'then'],
      },
    },
  },
  required: ['scenarios'],
}

const ATOM_VERDICT_SCHEMA = {
  type: 'object',
  properties: {
    scenarioId: { type: 'string' },
    violated: { type: 'boolean', description: 'true iff the diff violates this scenario\'s Then, given its Given/When' },
    constructedInput: { type: 'string', description: 'a concrete input satisfying Given/When that shows the violation, or empty if not violated' },
    defectClass: { type: 'string', enum: DEFECT_CLASSES.concat(['none']) },
    evidence: { type: 'string', description: 'file:line or diff hunk the verdict is grounded in' },
  },
  required: ['scenarioId', 'violated', 'defectClass', 'evidence'],
}

const HOLISTIC_SCHEMA = {
  type: 'object',
  properties: {
    verdict: { type: 'string', enum: ['fully', 'partial', 'no'] },
    gaps: { type: 'array', items: { type: 'string' } },
    justification: { type: 'string' },
  },
  required: ['verdict', 'gaps', 'justification'],
}

const COMMIT_SCHEMA = {
  type: 'object',
  properties: {
    committed: { type: 'boolean' },
    sha: { type: 'string' },
    subject: { type: 'string' },
    outputWritten: { type: 'boolean' },
  },
  required: ['committed', 'outputWritten'],
}

function atomizePrompt(runDir) {
  return (
    `Read ${args.testPlanPath} and the tests it describes in ` +
    `${runDir}/worktree. List every Gherkin scenario (or property, treated as one scenario) that ` +
    `the tests stage wrote for this change, in the schema's shape. Do not invent scenarios that ` +
    `are not in the test plan or the worktree's tests; do not merge distinct zero/one/many cases ` +
    `into one entry — each is its own scenario. This is a read-only reconnaissance step: make no ` +
    `edits.`
  )
}

function atomVerifyPrompt(runDir, scenario) {
  return (
    `You are an INDEPENDENT ADVERSARIAL reviewer. You get exactly ONE Gherkin scenario and the ` +
    `diff; you do not see any other scenario. Construct an input satisfying the scenario's ` +
    `Given/When where the diff at ${runDir}/worktree (run \`git -C ${runDir}/worktree diff\` ` +
    `against the run's base commit in ${runDir}/run.edn to see it) VIOLATES the scenario's Then. ` +
    `Default to violated=false only when you tried and could not construct one — do not default ` +
    `to false out of politeness.\n\n` +
    `Scenario ${scenario.id} — ${scenario.title}\n` +
    `Given: ${scenario.given}\nWhen: ${scenario.when}\nThen: ${scenario.then}\n\n` +
    `If violated, classify the defect using exactly one of: ${DEFECT_CLASSES.join(', ')}. ` +
    `Make no edits — you are a reviewer, not an implementer.`
  )
}

function holisticPrompt(runDir) {
  return (
    `You are an INDEPENDENT ADVERSARIAL reviewer taking ONE holistic pass over the whole change ` +
    `at ${runDir}/worktree against the spec at ${runDir}/spec.md. Do not re-derive the per-scenario ` +
    `atoms (another set of reviewers already did that) — look instead for what no single scenario ` +
    `would catch: requirements the test plan itself missed, architectural drift, or a diff that ` +
    `satisfies every scenario yet misreads the spec's intent. Assume incomplete until the evidence ` +
    `says otherwise. Make no edits.`
  )
}

function commitPrompt(runDir, reviewSummary) {
  return (
    `The commit stage. The mechanical gate at ${args.gateReportPath} ` +
    `is GREEN and the atomized adversarial review below found no unresolved violation. Read the ` +
    `full \`git -C ${runDir}/worktree diff\` against the run's base commit (in ` +
    `${runDir}/run.edn) and ${runDir}/spec.md. Write a scoped commit message: a ` +
    `\`<scope>: <imperative, lowercase>\` subject, then a body saying WHAT changed and WHY (the ` +
    `spec's intent) — never a type-first Conventional-Commits prefix. The stages committed their ` +
    `own work-in-progress so it would survive a host move; collapse that history FIRST with ` +
    `\`bash "$SKILL_DIR/engine/bin/run" squash --run ${runDir}\`, which soft-resets to the base ` +
    `commit and leaves every change staged without altering a single file. Then \`git -C ` +
    `${runDir}/worktree add -A\` and commit on the current branch — exactly one commit, the run's ` +
    `only deliverable. Then write ` +
    `${args.commitRecordPath} with the sha and the full message.\n\n` +
    `--- Adversarial review verdict (for context; do not re-litigate it) ---\n${reviewSummary}`
  )
}

function violationSummaryMd(atoms, holistic) {
  const violated = atoms.filter((a) => a && a.violated)
  const lines = []
  lines.push(`# Atomized adversarial review\n`)
  lines.push(`${atoms.length} scenario(s) checked independently, ${violated.length} violation(s) found.\n`)
  for (const a of atoms) {
    if (!a) continue
    lines.push(`## ${a.scenarioId}\n`)
    lines.push(`- violated: **${a.violated}**`)
    if (a.violated) {
      lines.push(`- defect class: ${a.defectClass}`)
      lines.push(`- constructed input: ${a.constructedInput || '(none given)'}`)
    }
    lines.push(`- evidence: ${a.evidence}\n`)
  }
  lines.push(`## Holistic pass\n`)
  lines.push(`- verdict: **${holistic.verdict}**`)
  lines.push(`- justification: ${holistic.justification}`)
  if (holistic.gaps && holistic.gaps.length) {
    lines.push(`- gaps:`)
    for (const g of holistic.gaps) lines.push(`  - ${g}`)
  }
  return lines.join('\n')
}

phase('Atomize')
const scenarioList = await agent(atomizePrompt(args.runDir), {
  label: 'atomize-scenarios',
  model: 'haiku',
  schema: SCENARIO_LIST_SCHEMA,
})
const scenarios = (scenarioList && scenarioList.scenarios) || []
log(`${scenarios.length} scenario(s) to verify independently`)

phase('Verify')
const [atoms, holistic] = await parallel([
  () =>
    Promise.all(
      scenarios.map((s) =>
        agent(atomVerifyPrompt(args.runDir, s), {
          label: `atom:${s.id}`,
          phase: 'Verify',
          model: 'opus',
          effort: 'high',
          schema: ATOM_VERDICT_SCHEMA,
        }).catch(() => null)
      )
    ),
  () =>
    agent(holisticPrompt(args.runDir), {
      label: 'holistic',
      phase: 'Verify',
      model: 'opus',
      effort: 'high',
      schema: HOLISTIC_SCHEMA,
    }),
])

const survivingAtoms = atoms.filter(Boolean)
const violations = survivingAtoms.filter((a) => a.violated)
const dropped = scenarios.length - survivingAtoms.length
if (dropped > 0) log(`${dropped} scenario verifier(s) errored and were dropped — treated as unresolved, not as pass`)

const reviewMd = violationSummaryMd(survivingAtoms, holistic)
const blocked = violations.length > 0 || dropped > 0 || holistic.verdict === 'no'

phase('Commit')
const publish = await agent(
  `Write the file ${args.reviewPath} with EXACTLY this ` +
    `content (verbatim, no edits):\n\n---BEGIN---\n${reviewMd}\n---END---`,
  { label: 'publish-review', model: 'haiku', schema: { type: 'object', properties: { written: { type: 'boolean' } }, required: ['written'] } }
)

if (blocked) {
  log(`adversarial gate BLOCKED the commit: ${violations.length} violation(s), ${dropped} dropped, holistic=${holistic.verdict}`)
  return {
    gate: 'adversarial-verify',
    passed: false,
    violations: violations.map((v) => ({ scenarioId: v.scenarioId, defectClass: v.defectClass, evidence: v.evidence })),
    droppedScenarios: dropped,
    holisticVerdict: holistic.verdict,
    reviewPublished: !!(publish && publish.written),
    committed: false,
  }
}

const commit = await agent(commitPrompt(args.runDir, reviewMd), {
  label: 'commit',
  model: args.commitModel || 'sonnet',
  schema: COMMIT_SCHEMA,
})

return {
  gate: 'adversarial-verify',
  passed: true,
  droppedScenarios: dropped,
  holisticVerdict: holistic.verdict,
  reviewPublished: !!(publish && publish.written),
  committed: !!(commit && commit.committed),
  commitSha: commit && commit.sha,
}
