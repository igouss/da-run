export const meta = {
  name: 'da-arm-pre',
  description: 'Drive the design/tests/implement subsequence of one directory-algorithm arm from a stage list — fixed (System A) or generated (System A prime) — up to but not including the mechanical gate.',
  whenToUse: 'Called by bin/run-arm-wf with the fixed three-stage list, or by da-dynamic-arm.js with a plan-generated list. Never call with a stage list that includes verify/commit — those are hard invariants driven outside this script (ADR-0028).',
  phases: [
    { title: 'design', detail: 'derive or review the ECB design' },
    { title: 'tests', detail: 'red tests from the design' },
    { title: 'implement', detail: 'one clean attempt, or N judged parallel attempts' },
  ],
}

// args:
//   runDir, worktree      absolute paths (worktree = runDir + '/worktree')
//   stageList             array of {kind, model, attempts?}. kind in:
//                          'design' | 'design-review' | 'tests' |
//                          'implement' | 'implement-parallel-attempt'
//   The list is DATA, never re-derived by this script — System A's caller supplies the fixed
//   three-entry list every round; System A prime's caller supplies whatever da-dynamic-arm's
//   Plan phase returned. This script only ever executes what it is given.

const STAGE_SCHEMA = {
  type: 'object',
  properties: {
    filesWritten: { type: 'array', items: { type: 'string' } },
    auditPassed: { type: 'boolean' },
    steerRequested: {
      type: 'boolean',
      description:
        'true ONLY if you wrote output/STEER-REQUEST.md per the steering protocol (references/steering.md) and stopped without completing the stage',
    },
    summary: { type: 'string' },
  },
  required: ['auditPassed', 'filesWritten'],
}

const ATTEMPT_SCHEMA = {
  type: 'object',
  properties: {
    patch: { type: 'string', description: 'unified diff of the attempt against the base commit; empty string if the attempt could not produce one' },
    testsPass: { type: 'boolean' },
    selfAssessment: { type: 'string' },
  },
  required: ['patch', 'testsPass', 'selfAssessment'],
}

const JUDGE_SCHEMA = {
  type: 'object',
  properties: {
    winnerIndex: { type: 'integer' },
    rationale: { type: 'string' },
  },
  required: ['winnerIndex', 'rationale'],
}

const APPLY_SCHEMA = {
  type: 'object',
  properties: {
    applied: { type: 'boolean' },
    testsPassAfterApply: { type: 'boolean' },
  },
  required: ['applied'],
}

// Stage prompts are POINTERS, never paraphrases (F2 / own-your-prompts): the stage's
// CONTEXT.md is the single home of what to load, how to work, and what to write. Restating
// any of it here would let a contract clause survive its own deletion (the JS copy would
// still carry it), blinding the contract-mutation rot oracle (ADR-0012/0027).
function stagePrompt(runDir, stageDir) {
  return (
    `You are running ONLY stage ${stageDir} of a directory-algorithm run. Read ` +
    `${runDir}/CLAUDE.md (identity + folder map), then ${runDir}/stages/${stageDir}/CONTEXT.md — ` +
    `that is your ENTIRE contract: load only what its Inputs table names, do its Process, run ` +
    `its Audit before writing, and write exactly its Outputs. Prior stages' outputs are under ` +
    `${runDir}/stages/*/output/; the codebase under change is ${runDir}/worktree; run metadata ` +
    `is in ${runDir}/run.edn. Do not redo an earlier stage or start a later one.\n\n` +
    `Constraints (isolated experiment run): operate ONLY within the run dir and its worktree; ` +
    `never \`git push\`; never touch \`main\` or any other checkout; never flash hardware, call ` +
    `a live endpoint, or read/use any bearer token.`
  )
}

function designPrompt(runDir, review) {
  const base = stagePrompt(runDir, '01-design')
  if (!review) return base
  return (
    base +
    `\n\nThis is a REVIEW pass: a design already exists at ` +
    `${runDir}/stages/01-design/output/design.md. Re-read it against your contract; revise it in ` +
    `place where the Audit finds a gap, leave it unchanged where it already passes — then re-run ` +
    `the Audit on the final file.`
  )
}

function testsPrompt(runDir) {
  return stagePrompt(runDir, '02-tests')
}

function implementPrompt(runDir) {
  return stagePrompt(runDir, '03-implement')
}

function attemptPrompt(runDir, index) {
  return (
    `Stage 03 — implement, ATTEMPT ${index}, one of several independent parallel attempts that ` +
    `will be judged. Create your own throwaway git worktree off the SAME base commit as ` +
    `${runDir}/worktree (read the base commit from ${runDir}/run.edn): \n` +
    `  git -C <the target project repo, same repo as ${runDir}/worktree> worktree add ` +
    `/tmp/da-attempt-${index} <base-commit>\n` +
    `Implement the change there per the design at ${runDir}/stages/01-design/output/design.md ` +
    `and the tests at ${runDir}/stages/02-tests (copy the stage-02 test files into your attempt ` +
    `worktree first, from ${runDir}/worktree). Run them; report testsPass honestly. Output the ` +
    `unified diff of your attempt against the base commit as \`patch\` — do NOT touch ` +
    `${runDir}/worktree itself, another attempt or the judge may still be using it. Before ` +
    `finishing, remove your throwaway worktree ` +
    `(\`git worktree remove --force /tmp/da-attempt-${index}\`).`
  )
}

function judgePrompt(runDir, attempts) {
  const body = attempts
    .map((a, i) => `--- ATTEMPT ${i} (testsPass=${a.testsPass}) ---\n${a.selfAssessment}\n\nPATCH:\n${a.patch}\n`)
    .join('\n')
  return (
    `${attempts.length} independent implementation attempts were made for the same design and ` +
    `tests (design at ${runDir}/stages/01-design/output/design.md, tests at ` +
    `${runDir}/stages/02-tests/output/test-plan.md). Judge them against the design's requirement ` +
    `ledger, the tests, and the house standards at ${runDir}/references/rust-standards.md. Reject ` +
    `any attempt whose testsPass is false unless every attempt failed. Pick exactly one winner by ` +
    `index (0-based).\n\n${body}`
  )
}

function applyWinnerPrompt(runDir, winnerPatch) {
  return (
    `Apply exactly this patch to ${runDir}/worktree (the real, shared worktree — nothing else has ` +
    `touched it):\n\n${winnerPatch}\n\nUse \`git -C ${runDir}/worktree apply\`; if it fails to ` +
    `apply cleanly because of path drift, re-create the equivalent change by hand from the patch's ` +
    `intent rather than giving up. Then re-run the stage-02 tests in the worktree and confirm ` +
    `green. Write ${runDir}/stages/03-implement/output/completeness.md and change-note.md exactly ` +
    `as stage 03's contract (${runDir}/stages/03-implement/CONTEXT.md) requires.`
  )
}

async function runStage(spec) {
  if (spec.kind === 'design') {
    return agent(designPrompt(args.runDir, false), { label: 'design', model: spec.model || 'opus', effort: 'high', schema: STAGE_SCHEMA })
  }
  if (spec.kind === 'design-review') {
    return agent(designPrompt(args.runDir, true), { label: 'design-review', model: spec.model || 'opus', schema: STAGE_SCHEMA })
  }
  if (spec.kind === 'tests') {
    return agent(testsPrompt(args.runDir), { label: 'tests', model: spec.model || 'sonnet', schema: STAGE_SCHEMA })
  }
  if (spec.kind === 'implement') {
    return agent(implementPrompt(args.runDir), { label: 'implement', model: spec.model || 'opus', effort: 'high', schema: STAGE_SCHEMA })
  }
  if (spec.kind === 'implement-parallel-attempt') {
    const n = Math.max(2, Math.min(4, spec.attempts || 3))
    phase('implement (parallel attempts)')
    const attempts = (
      await parallel(
        Array.from({ length: n }, (_, i) => () =>
          agent(attemptPrompt(args.runDir, i), { label: `attempt-${i}`, model: spec.model || 'opus', effort: 'high', schema: ATTEMPT_SCHEMA }).catch(() => null)
        )
      )
    ).filter(Boolean)
    if (attempts.length === 0) throw new Error('all parallel implementation attempts errored')
    const judged = await agent(judgePrompt(args.runDir, attempts), { label: 'judge-attempts', model: 'opus', effort: 'high', schema: JUDGE_SCHEMA })
    const winner = attempts[Math.max(0, Math.min(attempts.length - 1, judged.winnerIndex))]
    log(`implement: ${attempts.length} attempt(s) judged, winner=${judged.winnerIndex} (${judged.rationale})`)
    const applied = await agent(applyWinnerPrompt(args.runDir, winner.patch), { label: 'apply-winner', model: spec.model || 'opus', schema: APPLY_SCHEMA })
    if (!applied.applied) throw new Error('winning attempt failed to apply to the shared worktree')
    return { auditPassed: !!applied.testsPassAfterApply, filesWritten: ['completeness.md', 'change-note.md'], summary: `judged winner ${judged.winnerIndex}/${attempts.length}` }
  }
  throw new Error(`unknown stage kind: ${spec.kind}`)
}

const results = []
for (const spec of args.stageList) {
  phase(spec.kind)
  const r = await runStage(spec)
  if (r && r.steerRequested) {
    // F7 (ADR-0029): the stage is ASKING, not failing. Return cleanly — the harness detects
    // the unanswered STEER-REQUEST.md on disk (the file is canonical, not this flag), parks
    // or pauses for the operator, and re-dispatches the remaining stages after the answer.
    log(`stage "${spec.kind}" paused on a steer-request — returning for the operator`)
    return { stagesRun: results.map((x) => x.kind), steerPaused: spec.kind, allAuditsPassed: false }
  }
  if (!r || r.auditPassed === false) {
    throw new Error(`stage "${spec.kind}" did not pass its Audit — stopping the arm before the gate`)
  }
  results.push({ kind: spec.kind, ...r })
}

return { stagesRun: results.map((r) => r.kind), allAuditsPassed: results.every((r) => r.auditPassed) }
