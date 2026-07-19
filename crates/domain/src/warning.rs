/// Advisory notes on an allowed dispatch — never blocking.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Warning {
    /// A flow advisory rule fired — its code comes from the flow
    /// (e.g. reviewing a design that does not exist yet).
    Advisory { code: String },
    /// Re-dispatching a stage whose output already exists (steady-state only —
    /// operator steering between stages is the point). Names the stage dir.
    StageAlreadyComplete { stage: String },
    /// Re-dispatching over a red gate — legitimate rework.
    RedGateRework,
}
