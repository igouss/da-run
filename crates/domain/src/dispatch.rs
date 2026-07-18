use crate::stage::StageId;

/// A stage dispatch the operator (or the skill) wants to run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dispatch {
    Design,
    DesignReview,
    Tests,
    Implement { parallel_attempts: Option<u8> },
    Verify,
    Commit,
}

impl Dispatch {
    /// The stage a dispatch operates on (`DesignReview` reviews the design).
    pub fn stage(&self) -> StageId {
        match self {
            Dispatch::Design | Dispatch::DesignReview => StageId::Design,
            Dispatch::Tests => StageId::Tests,
            Dispatch::Implement { .. } => StageId::Implement,
            Dispatch::Verify => StageId::Verify,
            Dispatch::Commit => StageId::Commit,
        }
    }
}
