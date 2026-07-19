export const meta = {
  name: 'da-post-gate',
  description: 'Atomized adversarial review (ADR-0027 #3) then the scoped commit, for any directory-algorithm arm whose mechanical gate already went GREEN.',
  whenToUse: 'Called by da-stage.js after gate.sh has been run by the caller (never by this workflow) and returned GATE GREEN. Never call directly on a run whose gate has not passed.',
  phases: [
    { title: 'Atomize', detail: 'read the test plan, list its scenarios (mechanical, one agent)' },
    { title: 'Verify', detail: 'one adversarial reviewer per scenario, in parallel, plus one holistic pass' },
    { title: 'Commit', detail: 'sonnet — diff + spec + review verdict -> one scoped commit' },
  ],
}

// args (all required — this workflow never touches the filesystem itself, so every
// path an agent needs must arrive pre-resolved by the caller from the run's flow.ron;
// no stage dir or artifact name lives in this script):
//   runDir           absolute path to the run instance (holds CLAUDE.md, stages/, worktree/, run.json)
//   worktree         absolute path to the target project's worktree (runDir + '/worktree')
//   round            the round id (for labeling only)
//   commitModel      model id for the commit stage (the flow pins it per ADR-0009; a
//                    caller may override it — the CALLER decides, this workflow just uses it)
//   testPlanPath     absolute path to the test plan artifact (the tests handoff's output)
//   gateReportPath   absolute path to the mechanical gate's verdict report
//   reviewPath       absolute path where the adversarial review verdict is published
//   steerPath        absolute path for the holistic reviewer's steer-request — a `partial`
//                    verdict parks the run on the operator instead of silently committing
//                    (da-run's partial-holistic-verdict question, resolved as option C)
//   atomizerPath     optional: a flow-supplied file with atomization instructions — the
//                    decomposition style (Gherkin, properties, proof obligations) is the
//                    flow's to own, not this engine's
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
  const plan = args.testPlanPath || `${runDir}/spec.md`
  if (args.atomizerPath) {
    return (
      `Your atomization instructions are at ${args.atomizerPath} — read and follow them ` +
      `against the test plan at ${plan} and the tests it describes in ${runDir}/worktree, ` +
      `answering in the schema's shape. This is a read-only reconnaissance step: make no edits.`
    )
  }
  return (
    `Read ${plan} and the tests it describes in ` +
    `${runDir}/worktree. List every scenario (or property, treated as one scenario) that ` +
    `the tests stage wrote for this change, in the schema's shape. Do not invent scenarios that ` +
    `are not in the test plan or the worktree's tests; do not merge distinct zero/one/many cases ` +
    `into one entry — each is its own scenario. This is a read-only reconnaissance step: make no ` +
    `edits.`
  )
}

function atomVerifyPrompt(runDir, scenario) {
  return (
    `You are an INDEPENDENT ADVERSARIAL reviewer. You get exactly ONE scenario and the ` +
    `diff; you do not see any other scenario. Construct an input satisfying the scenario's ` +
    `Given/When where the diff at ${runDir}/worktree (run \`git -C ${runDir}/worktree diff\` ` +
    `against the run's base commit in ${runDir}/run.json to see it) VIOLATES the scenario's Then. ` +
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
    `satisfies every scenario yet misreads the spec's intent. Your \`partial\` verdict is ` +
    `load-bearing — it parks the run on the operator — so use it only for a SPECIFIC gap you can ` +
    `name and quote from the spec, each one listed in \`gaps\`; when you cannot name one, the ` +
    `verdict is \`fully\`. Make no edits.`
  )
}

// The steer-request a `partial` holistic verdict raises: the only detector
// for "the test plan itself missed a requirement" used to be discarded into
// a file no gate read; now it parks the run the way every other
// machine-needs-a-human case parks, and the operator's answer decides
// merge / steer / spec-fix at the moment the evidence is fresh.
function partialSteerContent(holistic) {
  const gaps = (holistic.gaps || []).map((g) => `- ${g}`).join('\n')
  return (
    `# STEER-REQUEST — holistic review\n\n` +
    `## Question\n\n` +
    `The holistic adversarial reviewer judged this change PARTIAL against the spec ` +
    `(justification: ${holistic.justification}). Named gaps:\n${gaps}\n\n` +
    `The mechanical gate is green and every atomized scenario passed — these gaps are things ` +
    `no existing scenario covers. Proceed to commit anyway?\n\n` +
    `## Options\n\n` +
    `- A: proceed — the gaps are out of scope for this slice\n` +
    `- B: implement the missing piece(s) first — answer, then re-run implement before commit\n` +
    `- C: the spec is wrong — revise it and re-run from the affected stage\n\n` +
    `## Answer\n\n`
  )
}

const STEER_CHECK_SCHEMA = {
  type: 'object',
  properties: {
    answered: { type: 'boolean', description: 'true iff the steer file already holds non-blank text under ## Answer' },
    answer: { type: 'string', description: 'the operator answer text when answered, else empty' },
    written: { type: 'boolean', description: 'true iff you wrote the new steer-request file' },
  },
  required: ['answered', 'written'],
}

function steerCheckPrompt(content) {
  return (
    `If the file ${args.steerPath} exists AND its \`## Answer\` section holds non-blank text, ` +
    `report answered=true with that text as \`answer\` and make NO edits. Otherwise write ` +
    `${args.steerPath} with EXACTLY this content (verbatim):\n\n---BEGIN---\n${content}---END---\n\n` +
    `and report answered=false, written=true.`
  )
}

function commitPrompt(runDir, reviewSummary, honestVerdict) {
  return (
    `The commit stage. The mechanical gate at ${args.gateReportPath} ` +
    `is GREEN. Adversarial review outcome: ${honestVerdict}. Read the ` +
    `full \`git -C ${runDir}/worktree diff\` against the run's base commit (in ` +
    `${runDir}/run.json) and ${runDir}/spec.md. Write a scoped commit message: a ` +
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

// A `partial` verdict is the machine needing a human judgment, not a
// failure: raise (or read) the steer-request and only proceed once the
// operator has answered it. Never decide for them.
let honestVerdict =
  `every atomized scenario passed and the holistic verdict is "${holistic.verdict}"`
if (holistic.verdict === 'partial') {
  const steer = await agent(steerCheckPrompt(partialSteerContent(holistic)), {
    label: 'partial-steer',
    model: 'haiku',
    schema: STEER_CHECK_SCHEMA,
  })
  if (!steer || !steer.answered) {
    log('holistic verdict is PARTIAL — steer-request raised, parking for the operator')
    return {
      gate: 'adversarial-verify',
      passed: false,
      steerPaused: 'holistic-partial',
      droppedScenarios: dropped,
      holisticVerdict: holistic.verdict,
      reviewPublished: !!(publish && publish.written),
      committed: false,
    }
  }
  honestVerdict =
    `every atomized scenario passed; the holistic verdict was "partial" and the operator ` +
    `answered the steer-request: "${steer.answer}" — that answer binds like the spec`
}

const commit = await agent(commitPrompt(args.runDir, reviewMd, honestVerdict), {
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
