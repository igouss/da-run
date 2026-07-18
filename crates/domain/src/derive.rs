use crate::anomaly::Anomaly;
use crate::facts::FsFacts;
use crate::phase::Phase;
use crate::run_state::RunState;
use crate::stage::StageId;

/// The derived view of a run: summary state, parked stages, anomalies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Derived {
    pub state: RunState,
    /// Stages holding an unanswered steer-request, in pipeline order.
    pub parked: Vec<StageId>,
    pub phase: Phase,
    pub anomalies: Vec<Anomaly>,
}

/// Fold a facts snapshot into the derived view. Total — never panics.
pub fn derive(facts: &FsFacts) -> Derived {
    Derived {
        state: summarize(facts),
        parked: pending_steers(facts),
        phase: facts.phase,
        anomalies: find_anomalies(facts),
    }
}

/// Stages with an unanswered steer-request, in pipeline order.
pub(crate) fn pending_steers(facts: &FsFacts) -> Vec<StageId> {
    StageId::ALL
        .into_iter()
        .filter(|id: &StageId| facts.stages.get(*id).steer_pending())
        .collect()
}

fn summarize(facts: &FsFacts) -> RunState {
    if facts.commit_recorded {
        return RunState::Committed;
    }
    if let Some(verdict) = facts.gate {
        return RunState::Gated(verdict);
    }
    if facts.stages.get(StageId::Implement).has_output() {
        return RunState::Implemented;
    }
    if facts.stages.get(StageId::Tests).has_output() {
        return RunState::Tested;
    }
    if facts.stages.get(StageId::Design).has_output() {
        return RunState::Designed;
    }
    RunState::Specced
}

/// A later handoff with output while an earlier one is empty, over the
/// design → tests → implement chain.
fn find_anomalies(facts: &FsFacts) -> Vec<Anomaly> {
    const HANDOFF_CHAIN: [StageId; 3] = [StageId::Design, StageId::Tests, StageId::Implement];
    let mut anomalies: Vec<Anomaly> = Vec::new();
    for pair in HANDOFF_CHAIN.windows(2) {
        let (earlier, later): (StageId, StageId) = (pair[0], pair[1]);
        if facts.stages.get(later).has_output() && !facts.stages.get(earlier).has_output() {
            anomalies.push(Anomaly::LaterOutputWithoutEarlier { later, earlier });
        }
    }
    anomalies
}
