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

if (!args.runDir) throw new Error('da-stage needs args.runDir (an absolute run-instance path)')
if (!args.stage) throw new Error('da-stage needs args.stage')

if (args.stage === 'verify') {
  throw new Error(
    'da-stage refuses "verify": the mechanical gate is never a workflow (ADR-0028 §3). ' +
      'Run it yourself: (cd <runDir>/worktree && bash ../stages/04-verify/gate.sh), then read ' +
      'GATE GREEN / GATE RED from stages/04-verify/output/gate-report.md.'
  )
}

const model = args.model || STAGE_MODELS[args.stage]
if (!model) {
  throw new Error(
    `unknown stage "${args.stage}" — expected one of: ${PRE_GATE_KINDS.join(', ')}, commit`
  )
}

const wfDir = args.workflowsDir || '.claude/workflows'

phase('Stage')

if (args.stage === 'commit') {
  // the atomized adversarial reviewer + the scoped commit, as one hard-gated unit — a caller
  // cannot reach the commit agent without the review passing (ADR-0027 #3 / ADR-0028 §2).
  const result = await workflow(
    { scriptPath: `${wfDir}/da-post-gate.js` },
    { runDir: args.runDir, worktree: `${args.runDir}/worktree`, round: args.round || 'ad-hoc', commitModel: model }
  )
  return { stage: 'commit', ...result }
}

const stageSpec = { kind: args.stage, model }
if (args.stage === 'implement-parallel-attempt' && args.attempts) stageSpec.attempts = args.attempts

const result = await workflow(
  { scriptPath: `${wfDir}/da-arm-pre.js` },
  { runDir: args.runDir, worktree: `${args.runDir}/worktree`, stageList: [stageSpec] }
)

return { stage: args.stage, model, ...result }
