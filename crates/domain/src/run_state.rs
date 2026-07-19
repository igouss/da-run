use crate::verdict::Verdict;

/// The run's summary state — reporting shorthand for humans and JSON.
/// Transition rules test the underlying facts, never this summary.
/// Handoff labels ("designed", "tested", …) come from the flow.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunState {
    /// No handoff stage has output yet; the flow's initial label.
    Pending {
        label: String,
    },
    /// The furthest handoff stage with output; `rank` is its 1-based
    /// position among the flow's handoff stages.
    HandoffDone {
        label: String,
        rank: u8,
    },
    Gated(Verdict),
    Committed,
}

impl RunState {
    /// Pipeline progress rank; `Gated(Red)` and `Gated(Green)` tie.
    pub fn progress(&self) -> u8 {
        match self {
            RunState::Pending { .. } => 0,
            RunState::HandoffDone { rank, .. } => *rank,
            RunState::Gated(_) => u8::MAX - 1,
            RunState::Committed => u8::MAX,
        }
    }

    /// The wire label: handoff labels come from the flow; the gate and
    /// commit summaries are the machine's own.
    pub fn label(&self) -> String {
        match self {
            RunState::Pending { label } | RunState::HandoffDone { label, .. } => label.clone(),
            RunState::Gated(Verdict::Green) => "gated-green".to_string(),
            RunState::Gated(Verdict::Red) => "gated-red".to_string(),
            RunState::Committed => "committed".to_string(),
        }
    }
}
