use crate::derive::pending_steers;
use crate::facts::FsFacts;
use crate::flow::{AdviseRule, BlockRule, DispatchDef, DispatchRef, Flow, Role, StageDef};
use crate::phase::Phase;
use crate::refusal::Refusal;
use crate::verdict::Verdict;
use crate::warning::Warning;

/// A dispatch the machine allows, with advisory warnings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Allowed {
    pub warnings: Vec<Warning>,
}

/// Proof that the commit precondition held: gate green, no pending steer.
/// The private field makes it constructible only by [`commit_precondition`] —
/// no path reports "commit allowed" without one.
#[derive(Debug)]
pub struct GateGreenProof(());

/// Decide a dispatch against the facts. The ordering guards are the flow's
/// blocking rules; the steer and gate laws are the machine's own; warnings
/// never block.
pub fn check(flow: &Flow, facts: &FsFacts, dispatch: DispatchRef) -> Result<Allowed, Refusal> {
    let parked: Vec<String> = pending_steers(flow, facts);
    if !parked.is_empty() {
        return Err(Refusal::SteerPending { stages: parked });
    }
    let (stage, spec): (&StageDef, &DispatchDef) = flow.dispatch(dispatch);
    for rule in &spec.blocking {
        let missing: bool = !facts.stages.get(rule.stage).has_output();
        if missing {
            return Err(ordering_violation(rule));
        }
    }
    let mut warnings: Vec<Warning> = Vec::new();
    for rule in &spec.advisory {
        let missing: bool = !facts.stages.get(rule.stage).has_output();
        if missing {
            warnings.push(advisory(rule));
        }
    }
    if spec.warn_on_red_gate && facts.gate == Some(Verdict::Red) {
        warnings.push(Warning::RedGateRework);
    }
    if matches!(stage.role, Role::Commit) {
        let _proof: GateGreenProof = commit_precondition(flow, facts)?;
    }
    if facts.phase == Phase::SteadyState && facts.stages.get(dispatch.stage()).has_output() {
        warnings.push(Warning::StageAlreadyComplete {
            stage: stage.dir.clone(),
        });
    }
    Ok(Allowed { warnings })
}

/// The commit precondition: no pending steer and a green gate. Absent or
/// unparseable gate report fails closed.
pub fn commit_precondition(flow: &Flow, facts: &FsFacts) -> Result<GateGreenProof, Refusal> {
    let parked: Vec<String> = pending_steers(flow, facts);
    if !parked.is_empty() {
        return Err(Refusal::SteerPending { stages: parked });
    }
    match facts.gate {
        Some(Verdict::Green) => Ok(GateGreenProof(())),
        gate => Err(Refusal::CommitBeforeGreenGate {
            gate,
            gate_report: flow.gate_report_path(),
        }),
    }
}

fn ordering_violation(rule: &BlockRule) -> Refusal {
    Refusal::OrderingViolation {
        code: rule.code.clone(),
        detail: rule.detail.clone(),
    }
}

fn advisory(rule: &AdviseRule) -> Warning {
    Warning::Advisory {
        code: rule.code.clone(),
    }
}
