export const meta = {
  name: 'da-stage',
  description: 'Run exactly ONE named stage of the directory algorithm against a run instance — the single-stage entry point behind the /da-run skill.',
  whenToUse: 'Invoked by the da-run skill (or a harness) with {runDir, stage, flow}. Routes agent stages through da-arm-pre.js and the commit stage through da-post-gate.js. Refuses the gate stage — the mechanical gate is run by the caller via bash, never by a workflow (ADR-0028 §3).',
  phases: [
    { title: 'Stage', detail: 'the one requested stage, on its flow-configured model' },
  ],
}

// args:
//   runDir       absolute path to the run instance (holds CLAUDE.md, stages/, worktree/, run.edn, spec.md)
//   stage        a dispatch kind from the run's flow.ron (e.g. 'design', 'tests', 'commit')
//   flow         REQUIRED: the validated pipeline JSON printed by
//                `bash "$SKILL_DIR/engine/bin/state" flow --run <runDir>` — stage names,
//                dirs, dispatch kinds, models, and strategies come from there, never from
//                this script. A JSON string is normalized like the other args.
//   round        optional, labeling only (commit stage)
//   model        optional override; default is the flow's per-dispatch tier (ADR-0009)
//   attempts     optional, only for the parallel-attempts strategy (2-4)
//   workflowsDir REQUIRED dir holding da-arm-pre.js / da-post-gate.js — the skill passes its
//                own bundled workflows dir so the bundle works from any install location.
//
// Deliberately NOT accepted: the gate stage. gate.sh is deterministic and zero-reasoning;
// wrapping it in an agent() call would launder the mechanical edge through an LLM. The caller
// runs it with bash and reads GATE GREEN / GATE RED itself.

// Some harnesses deliver args as a JSON string — normalize once at the boundary.
const input = typeof args === 'string' ? JSON.parse(args) : (args ?? {})

if (!input.runDir) throw new Error('da-stage needs args.runDir (an absolute run-instance path)')
if (!input.stage) throw new Error('da-stage needs args.stage')

// The check and flow JSON may themselves arrive as pasted strings — same normalization.
if (typeof input.stateCheck === 'string') {
  try { input.stateCheck = JSON.parse(input.stateCheck) } catch { input.stateCheck = null }
}
if (typeof input.flow === 'string') {
  try { input.flow = JSON.parse(input.flow) } catch { input.flow = null }
}

if (!input.flow || !Array.isArray(input.flow.stages)) {
  throw new Error(
    'da-stage needs args.flow — the pipeline definition. Run: ' +
      `bash "$SKILL_DIR/engine/bin/state" flow --run ${input.runDir} ` +
      'and pass its printed JSON as args.flow (flow.ron is the single source of truth).'
  )
}

const stages = input.flow.stages
const allKinds = stages.flatMap((s) => s.dispatches.map((d) => d.kind))

const owner = stages.find((s) => s.dispatches.some((d) => d.kind === input.stage))
const dispatch = owner && owner.dispatches.find((d) => d.kind === input.stage)
if (!owner) {
  throw new Error(`unknown stage "${input.stage}" — the flow defines: ${allKinds.join(', ')}`)
}

if (owner.role === 'gate') {
  throw new Error(
    `da-stage refuses "${input.stage}": the mechanical gate is never a workflow (ADR-0028 §3). ` +
      `Run it yourself: bash "$SKILL_DIR/engine/bin/run" gate --run <runDir> (which seals the ` +
      `worktree and stamps the report with its identity), then read ` +
      `GATE GREEN / GATE RED from stages/${owner.dir}/output/${owner.artifact}.`
  )
}

// ADR-0028-adjacent honesty scaffolding: the caller must have run the run-state
// authority and gotten "allowed". Advisory by design — the hard edges stay the
// mechanical gate and the adversarial reviewer.
if (!input.stateCheck || input.stateCheck.allowed !== true) {
  throw new Error(
    'da-stage refuses to dispatch without a passing run-state check — run: ' +
      `bash "$SKILL_DIR/engine/bin/state" check --run ${input.runDir} ${input.stage} ` +
      'and pass its printed JSON as args.stateCheck (SKILL.md §Ordering guards).'
  )
}

const model = input.model || dispatch.model
if (!model) {
  throw new Error(
    `the flow gives no model for dispatch "${input.stage}" and args.model was not passed`
  )
}

if (!input.workflowsDir) {
  throw new Error(
    'da-stage needs args.workflowsDir (the skill bundle\'s workflows/ directory — ' +
      'the engines da-arm-pre.js and da-post-gate.js live beside this script)'
  )
}
const wfDir = input.workflowsDir

const handoffs = stages.filter((s) => s.role === 'handoff')

phase('Stage')

if (owner.role === 'commit') {
  // the atomized adversarial reviewer + the scoped commit, as one hard-gated unit — a caller
  // cannot reach the commit agent without the review passing (ADR-0027 #3 / ADR-0028 §2).
  // The reviewer reads the test plan of the last pre-implementation handoff.
  const gate = stages.find((s) => s.role === 'gate')
  const testsStage = handoffs[handoffs.length - 2]
  const result = await workflow(
    { scriptPath: `${wfDir}/da-post-gate.js` },
    {
      runDir: input.runDir,
      worktree: `${input.runDir}/worktree`,
      round: input.round || 'ad-hoc',
      commitModel: model,
      testPlanPath: testsStage && `${input.runDir}/stages/${testsStage.dir}/output/${testsStage.artifact}`,
      gateReportPath: `${input.runDir}/stages/${gate.dir}/output/${gate.artifact}`,
      reviewPath: `${input.runDir}/stages/${gate.dir}/output/adversarial-review.md`,
      commitRecordPath: `${input.runDir}/stages/${owner.dir}/output/${owner.artifact}`,
    }
  )
  return { stage: input.stage, ...result }
}

// Pre-gate handoff stage: enrich the spec with everything da-arm-pre needs so it
// re-derives nothing — dirs, artifacts, and the earlier handoffs it may reference.
const prior = handoffs
  .filter((s) => stages.indexOf(s) < stages.indexOf(owner))
  .map((s) => ({ dir: s.dir, artifact: s.artifact }))

const stageSpec = {
  kind: input.stage,
  model,
  strategy: dispatch.strategy || 'single',
  effort: dispatch.effort,
  dir: owner.dir,
  artifact: owner.artifact,
  prior,
}
if (stageSpec.strategy === 'parallel-attempts' && input.attempts) stageSpec.attempts = input.attempts

const result = await workflow(
  { scriptPath: `${wfDir}/da-arm-pre.js` },
  { runDir: input.runDir, worktree: `${input.runDir}/worktree`, stageList: [stageSpec] }
)

return { stage: input.stage, model, ...result }
