use crate::derive::pending_steers;
use crate::facts::FsFacts;
use crate::flow::{AdviseRule, BlockRule, DispatchDef, DispatchRef, Flow, Role, StageDef};
use crate::phase::Phase;
use crate::refusal::Refusal;
use crate::verdict::Verdict;
use crate::warning::Warning;
use crate::worktree::{WorktreeFacts, WorktreeId};

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

/// The commit precondition: no pending steer, a green gate, and a worktree
/// that still holds the very code the gate went green on.
///
/// The gate verdict alone is not enough. It is text in a file, and a run
/// restored onto another host carries that text with it — so a verdict read
/// in isolation will happily bless a worktree that lost its code in transit.
/// Every unknown fails closed: absent patch, absent gate identity, and an
/// unparseable verdict all refuse.
pub fn commit_precondition(flow: &Flow, facts: &FsFacts) -> Result<GateGreenProof, Refusal> {
    let parked: Vec<String> = pending_steers(flow, facts);
    if !parked.is_empty() {
        return Err(Refusal::SteerPending { stages: parked });
    }
    if !matches!(facts.gate, Some(Verdict::Green)) {
        return Err(Refusal::CommitBeforeGreenGate {
            gate: facts.gate,
            gate_report: flow.gate_report_path(),
        });
    }
    let worktree: &WorktreeFacts = facts.worktree.as_ref().ok_or(Refusal::WorktreeAbsent)?;
    if worktree.empty {
        return Err(Refusal::WorktreeEmpty);
    }
    // An unrecorded gate identity is a mismatch, not a pass: a report that
    // cannot name the code it verified has not verified this code.
    let verified: &WorktreeId = facts
        .gate_worktree
        .as_ref()
        .ok_or_else(|| Refusal::WorktreeMovedSinceGate {
            verified: "(unrecorded)".to_string(),
            current: worktree.id.to_string(),
            gate_report: flow.gate_report_path(),
        })?;
    if verified != &worktree.id {
        return Err(Refusal::WorktreeMovedSinceGate {
            verified: verified.to_string(),
            current: worktree.id.to_string(),
            gate_report: flow.gate_report_path(),
        });
    }
    Ok(GateGreenProof(()))
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
