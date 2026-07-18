export const meta = {
  name: 'da-stage',
  description: 'Run exactly ONE named stage of the directory algorithm against a run instance — the single-stage entry point behind the /da-run skill.',
  whenToUse: 'Invoked by the da-run skill (or a harness) with {runDir, stage}. Routes agent stages through da-arm-pre.js and the commit stage through da-post-gate.js. Refuses "verify" — the mechanical gate is run by the caller via bash, never by a workflow (ADR-0028 §3).',
  phases: [
    { title: 'Stage', detail: 'the one requested stage, on its ADR-0009 model' },
  ],
}

// args:
//   runDir       absolute path to the run instance (holds CLAUDE.md, stages/, worktree/, run.edn, spec.md)
//   stage        'design' | 'design-review' | 'tests' | 'implement' |
//                'implement-parallel-attempt' | 'commit'
//   round        optional, labeling only (commit stage)
//   model        optional override; default is the ADR-0009 per-stage tier
//   attempts     optional, only for implement-parallel-attempt (2-4)
//   workflowsDir optional dir holding da-arm-pre.js / da-post-gate.js; defaults to the trial
//                repo's project-local '.claude/workflows'. The distributable skill passes its
//                own bundled workflows dir here so the bundle works from any install location.
//
// Deliberately NOT accepted: 'verify'. gate.sh is deterministic and zero-reasoning; wrapping it
// in an agent() call would launder the mechanical edge through an LLM. The caller runs it with
// bash and reads GATE GREEN / GATE RED itself.

const STAGE_MODELS = {
  design: 'opus',
  'design-review': 'opus',
  tests: 'sonnet',
  implement: 'opus',
  'implement-parallel-attempt': 'opus',
  commit: 'sonnet',
}

const PRE_GATE_KINDS = ['design', 'design-review', 'tests', 'implement', 'implement-parallel-attempt']

// Some harnesses deliver args as a JSON string — normalize once at the boundary.
const input = typeof args === 'string' ? JSON.parse(args) : (args ?? {})

if (!input.runDir) throw new Error('da-stage needs args.runDir (an absolute run-instance path)')
if (!input.stage) throw new Error('da-stage needs args.stage')

// The check JSON may itself arrive as a pasted string — same normalization.
if (typeof input.stateCheck === 'string') {
  try { input.stateCheck = JSON.parse(input.stateCheck) } catch { input.stateCheck = null }
}

if (input.stage === 'verify') {
  throw new Error(
    'da-stage refuses "verify": the mechanical gate is never a workflow (ADR-0028 §3). ' +
      'Run it yourself: (cd <runDir>/worktree && bash ../stages/04-verify/gate.sh), then read ' +
      'GATE GREEN / GATE RED from stages/04-verify/output/gate-report.md.'
  )
}

// ADR-0028-adjacent honesty scaffolding: the caller must have run the run-state
// authority and gotten "allowed". Advisory by design — the hard edges stay the
// mechanical gate and the adversarial reviewer.
if (!input.stateCheck || input.stateCheck.allowed !== true) {
  throw new Error(
    'da-stage refuses to dispatch without a passing run-state check — run: ' +
      `bash "$SKILL_DIR/algorithm/bin/state" check --run ${input.runDir} ${input.stage} ` +
      'and pass its printed JSON as args.stateCheck (SKILL.md §Ordering guards).'
  )
}

const model = input.model || STAGE_MODELS[input.stage]
if (!model) {
  throw new Error(
    `unknown stage "${input.stage}" — expected one of: ${PRE_GATE_KINDS.join(', ')}, commit`
  )
}

const wfDir = input.workflowsDir || '.claude/workflows'

phase('Stage')

if (input.stage === 'commit') {
  // the atomized adversarial reviewer + the scoped commit, as one hard-gated unit — a caller
  // cannot reach the commit agent without the review passing (ADR-0027 #3 / ADR-0028 §2).
  const result = await workflow(
    { scriptPath: `${wfDir}/da-post-gate.js` },
    { runDir: input.runDir, worktree: `${input.runDir}/worktree`, round: input.round || 'ad-hoc', commitModel: model }
  )
  return { stage: 'commit', ...result }
}

const stageSpec = { kind: input.stage, model }
if (input.stage === 'implement-parallel-attempt' && input.attempts) stageSpec.attempts = input.attempts

const result = await workflow(
  { scriptPath: `${wfDir}/da-arm-pre.js` },
  { runDir: input.runDir, worktree: `${input.runDir}/worktree`, stageList: [stageSpec] }
)

return { stage: input.stage, model, ...result }
