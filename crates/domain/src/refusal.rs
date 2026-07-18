use crate::stage::StageId;
use crate::verdict::Verdict;

/// A typed reason a dispatch is refused. Mirrors SKILL.md's ordering guards
/// plus the steer law.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Refusal {
    #[error("tests before stages/01-design/output/ has a design")]
    TestsBeforeDesign,
    #[error("implement before stages/02-tests/output/ has a test plan")]
    ImplementBeforeTests,
    #[error(
        "commit before stages/04-verify/output/gate-report.md shows GATE GREEN (gate: {gate:?})"
    )]
    CommitBeforeGreenGate { gate: Option<Verdict> },
    #[error("a steer-request awaits the operator at {stages:?}")]
    SteerPending { stages: Vec<StageId> },
}
