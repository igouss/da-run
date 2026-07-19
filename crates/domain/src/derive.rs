use crate::anomaly::Anomaly;
use crate::facts::FsFacts;
use crate::flow::{Flow, Role, StageDef, StageRef};
use crate::phase::Phase;
use crate::run_state::RunState;

/// The derived view of a run: summary state, parked stages, anomalies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Derived {
    pub state: RunState,
    /// Dirs of stages holding an unanswered steer-request, in pipeline order.
    pub parked: Vec<String>,
    pub phase: Phase,
    pub anomalies: Vec<Anomaly>,
}

/// Fold a facts snapshot into the derived view. Total — never panics.
pub fn derive(flow: &Flow, facts: &FsFacts) -> Derived {
    Derived {
        state: summarize(flow, facts),
        parked: pending_steers(flow, facts),
        phase: facts.phase,
        anomalies: find_anomalies(flow, facts),
    }
}

/// Dirs of stages with an unanswered steer-request, in pipeline order.
pub(crate) fn pending_steers(flow: &Flow, facts: &FsFacts) -> Vec<String> {
    flow.stages()
        .filter(|(stage, _): &(StageRef, &StageDef)| facts.stages.get(*stage).steer_pending())
        .map(|(_, def): (StageRef, &StageDef)| def.dir.clone())
        .collect()
}

fn summarize(flow: &Flow, facts: &FsFacts) -> RunState {
    if facts.commit_recorded {
        return RunState::Committed;
    }
    if let Some(verdict) = facts.gate {
        return RunState::Gated(verdict);
    }
    let furthest: Option<(String, u8)> = flow
        .handoffs()
        .into_iter()
        .rev()
        .find(|(stage, _, _): &(StageRef, &StageDef, u8)| facts.stages.get(*stage).has_output())
        .and_then(
            |(_, def, rank): (StageRef, &StageDef, u8)| match &def.role {
                Role::Handoff { done_label } => Some((done_label.clone(), rank)),
                _ => None,
            },
        );
    match furthest {
        Some((label, rank)) => RunState::HandoffDone { label, rank },
        None => RunState::Pending {
            label: flow.initial_label().to_string(),
        },
    }
}

/// A later handoff with output while an earlier one is empty, over the
/// flow's handoff chain.
fn find_anomalies(flow: &Flow, facts: &FsFacts) -> Vec<Anomaly> {
    let chain: Vec<(StageRef, &StageDef, u8)> = flow.handoffs();
    let mut anomalies: Vec<Anomaly> = Vec::new();
    for pair in chain.windows(2) {
        let (earlier, earlier_def, _): (StageRef, &StageDef, u8) = pair[0];
        let (later, later_def, _): (StageRef, &StageDef, u8) = pair[1];
        if facts.stages.get(later).has_output() && !facts.stages.get(earlier).has_output() {
            anomalies.push(Anomaly::LaterOutputWithoutEarlier {
                later: later_def.dir.clone(),
                earlier: earlier_def.dir.clone(),
            });
        }
    }
    anomalies
}
