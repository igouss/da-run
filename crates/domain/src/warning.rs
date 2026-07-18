use crate::stage::StageId;

/// Advisory notes on an allowed dispatch — never blocking.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Warning {
    /// Reviewing a design that does not exist yet.
    DesignReviewWithoutDesign,
    /// Running the gate over an empty implementation stage.
    VerifyWithoutImplementation,
    /// Re-dispatching a stage whose output already exists (steady-state only —
    /// operator steering between stages is the point).
    StageAlreadyComplete { stage: StageId },
    /// Dispatching implement again after a red gate — legitimate rework.
    RedGateRework,
}
