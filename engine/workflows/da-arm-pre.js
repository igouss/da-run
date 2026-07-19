export const meta = {
  name: 'da-arm-pre',
  description: 'Drive the pre-gate handoff subsequence of one directory-algorithm arm from a stage list — fixed (System A) or generated (System A prime) — up to but not including the mechanical gate.',
  whenToUse: 'Called by da-stage.js with a stage list enriched from the flow. Never call with a stage list that includes the gate or commit stages — those are hard invariants driven outside this script (ADR-0028).',
  phases: [
    { title: 'Stages', detail: 'each handoff stage in list order, on its flow-configured model and strategy' },
  ],
}

// args:
//   runDir, worktree      absolute paths (worktree = runDir + '/worktree')
//   stageList             array of {kind, model, strategy, dir, artifact, attempts?, effort?,
//                          design?, tests?, judgeReference?}
//                          — every field resolved by the CALLER from the run's flow.ron
//                          (via `bin/state flow`). strategy in:
//                          'single' | 'review' | 'parallel-attempts'
//                          design/tests = the flow-NAMED input handoffs as {dir, artifact}
//                          (from design_from / tests_from — never positional guesses);
//                          judgeReference = run-dir-relative house standard for the judge.
//   The list is DATA, never re-derived by this script. This script only ever executes what
//   it is given: no stage name, dir, model, or reference filename lives here.

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

function reviewPrompt(runDir, spec) {
  return (
    stagePrompt(runDir, spec.dir) +
    `\n\nThis is a REVIEW pass: this stage's output already exists at ` +
    `${runDir}/stages/${spec.dir}/output/${spec.artifact}. Re-read it against your contract; ` +
    `revise it in place where the Audit finds a gap, leave it unchanged where it already ` +
    `passes — then re-run the Audit on the final file.`
  )
}

// The parallel-attempts strategy reads the design and tests handoffs the
// flow NAMED on its dispatch (design_from / tests_from) — never a positional
// guess over the stage list, which breaks silently on a differently-shaped
// flow. Attempt worktrees live under the run dir, not /tmp: /tmp collides
// across concurrent runs and cargo builds overflow tmpfs (the run driver's
// own rule); bin/run gate prunes any leftovers.
function attemptWorktree(runDir, index) {
  return `${runDir}/attempts/da-attempt-${index}`
}

function attemptPrompt(runDir, spec, index) {
  const design = spec.design
  const tests = spec.tests
  const wt = attemptWorktree(runDir, index)
  return (
    `Stage ${spec.dir}, ATTEMPT ${index}, one of several independent parallel attempts that ` +
    `will be judged. Create your own throwaway git worktree off the SAME base commit as ` +
    `${runDir}/worktree (read the base commit from ${runDir}/run.edn): \n` +
    `  git -C <the target project repo, same repo as ${runDir}/worktree> worktree add ` +
    `${wt} <base-commit>\n` +
    `Implement the change there per the design at ` +
    `${runDir}/stages/${design.dir}/output/${design.artifact} and the tests at ` +
    `${runDir}/stages/${tests.dir} (copy that stage's test files into your attempt ` +
    `worktree first, from ${runDir}/worktree). Run them; report testsPass honestly. Output the ` +
    `unified diff of your attempt against the base commit as \`patch\` — do NOT touch ` +
    `${runDir}/worktree itself, another attempt or the judge may still be using it. Before ` +
    `finishing, remove your throwaway worktree ` +
    `(\`git worktree remove --force ${wt}\`).`
  )
}

function judgePrompt(runDir, spec, attempts) {
  const design = spec.design
  const tests = spec.tests
  const standards = spec.judgeReference
    ? `the house standards at ${runDir}/${spec.judgeReference}`
    : `the house standards under ${runDir}/references/`
  const body = attempts
    .map((a, i) => `--- ATTEMPT ${i} (testsPass=${a.testsPass}) ---\n${a.selfAssessment}\n\nPATCH:\n${a.patch}\n`)
    .join('\n')
  return (
    `${attempts.length} independent implementation attempts were made for the same design and ` +
    `tests (design at ${runDir}/stages/${design.dir}/output/${design.artifact}, tests at ` +
    `${runDir}/stages/${tests.dir}/output/${tests.artifact}). Judge them against the design's ` +
    `requirement ledger, the tests, and ${standards}. Reject any attempt whose testsPass is ` +
    `false unless every attempt failed. Pick exactly one winner by index (0-based).\n\n${body}`
  )
}

function applyWinnerPrompt(runDir, spec, winnerPatch) {
  const tests = spec.tests
  return (
    `Apply exactly this patch to ${runDir}/worktree (the real, shared worktree — nothing else has ` +
    `touched it):\n\n${winnerPatch}\n\nUse \`git -C ${runDir}/worktree apply\`; if it fails to ` +
    `apply cleanly because of path drift, re-create the equivalent change by hand from the patch's ` +
    `intent rather than giving up. Then re-run the ${tests.dir} tests in the worktree and confirm ` +
    `green. Write this stage's output files exactly as its contract ` +
    `(${runDir}/stages/${spec.dir}/CONTEXT.md) requires.`
  )
}

function agentOpts(spec, label, schema) {
  const opts = { label, model: spec.model, schema }
  if (spec.effort) opts.effort = spec.effort
  return opts
}

async function runStage(spec) {
  if (spec.strategy === 'single') {
    return agent(stagePrompt(args.runDir, spec.dir), agentOpts(spec, spec.kind, STAGE_SCHEMA))
  }
  if (spec.strategy === 'review') {
    return agent(reviewPrompt(args.runDir, spec), agentOpts(spec, spec.kind, STAGE_SCHEMA))
  }
  if (spec.strategy === 'parallel-attempts') {
    if (!spec.design || !spec.tests) {
      throw new Error(
        `parallel-attempts for "${spec.kind}" arrived without named design/tests input stages — ` +
          'the caller resolves them from design_from / tests_from in flow.ron'
      )
    }
    const n = Math.max(2, Math.min(4, spec.attempts || 3))
    phase(`${spec.kind} (parallel attempts)`)
    const attempts = (
      await parallel(
        Array.from({ length: n }, (_, i) => () =>
          agent(attemptPrompt(args.runDir, spec, i), agentOpts(spec, `attempt-${i}`, ATTEMPT_SCHEMA)).catch(() => null)
        )
      )
    ).filter(Boolean)
    if (attempts.length === 0) throw new Error('all parallel implementation attempts errored')
    const judged = await agent(judgePrompt(args.runDir, spec, attempts), agentOpts(spec, 'judge-attempts', JUDGE_SCHEMA))
    const winner = attempts[Math.max(0, Math.min(attempts.length - 1, judged.winnerIndex))]
    log(`${spec.kind}: ${attempts.length} attempt(s) judged, winner=${judged.winnerIndex} (${judged.rationale})`)
    const applied = await agent(applyWinnerPrompt(args.runDir, spec, winner.patch), agentOpts(spec, 'apply-winner', APPLY_SCHEMA))
    if (!applied.applied) throw new Error('winning attempt failed to apply to the shared worktree')
    return { auditPassed: !!applied.testsPassAfterApply, filesWritten: [], summary: `judged winner ${judged.winnerIndex}/${attempts.length}` }
  }
  throw new Error(`unknown stage strategy: ${spec.strategy} (kind: ${spec.kind})`)
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
