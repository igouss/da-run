use crate::verdict::Verdict;

/// A typed reason a dispatch is refused. Ordering guards come from the flow's
/// blocking rules; the steer and gate laws are the machine's own. The Display
/// text is relayed to the operator verbatim.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Refusal {
    /// A flow blocking rule fired — its code and detail come from the flow.
    #[error("{detail}")]
    OrderingViolation { code: String, detail: String },
    #[error(
        "commit before {gate_report} shows GATE GREEN (gate: {})",
        gate_label(gate)
    )]
    CommitBeforeGreenGate {
        gate: Option<Verdict>,
        /// The gate report's run-dir-relative path, from the flow.
        gate_report: String,
    },
    /// Stage dirs holding an unanswered steer-request, in pipeline order.
    #[error("a steer-request awaits the operator at {}", stages.join(", "))]
    SteerPending { stages: Vec<String> },
    /// No `worktree.patch` — the run dir cannot say what code it holds, so a
    /// green gate proves nothing about anything.
    #[error("commit without a worktree.patch — the run dir holds no record of its code")]
    WorktreeAbsent,
    /// The patch carries no change: committing would record an empty change
    /// against a green gate.
    #[error("commit with an empty worktree — the run produced no code to ship")]
    WorktreeEmpty,
    /// The worktree moved after the gate ran, so the green verdict describes
    /// code that is no longer there.
    #[error(
        "commit against a worktree the gate never saw — {gate_report} verified {verified}, \
         the worktree is now {current} (re-run the gate)"
    )]
    WorktreeMovedSinceGate {
        verified: String,
        current: String,
        /// The gate report's run-dir-relative path, from the flow.
        gate_report: String,
    },
}

fn gate_label(gate: &Option<Verdict>) -> &'static str {
    match gate {
        Some(Verdict::Green) => "green",
        Some(Verdict::Red) => "red",
        None => "no verdict",
    }
}
