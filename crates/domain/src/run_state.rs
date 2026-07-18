use crate::verdict::Verdict;

/// The run's summary state — reporting shorthand for humans and JSON.
/// Transition rules test the underlying facts, never this summary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunState {
    Specced,
    Designed,
    Tested,
    Implemented,
    Gated(Verdict),
    Committed,
}

impl RunState {
    /// Pipeline progress rank; `Gated(Red)` and `Gated(Green)` tie.
    pub fn progress(self) -> u8 {
        match self {
            RunState::Specced => 0,
            RunState::Designed => 1,
            RunState::Tested => 2,
            RunState::Implemented => 3,
            RunState::Gated(_) => 4,
            RunState::Committed => 5,
        }
    }
}
