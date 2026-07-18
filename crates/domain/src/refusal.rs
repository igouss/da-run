use crate::stage::StageId;
use crate::verdict::Verdict;

/// A typed reason a dispatch is refused. Mirrors SKILL.md's ordering guards
/// plus the steer law. The Display text is relayed to the operator verbatim.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Refusal {
    #[error("tests before stages/01-design/output/ has a design")]
    TestsBeforeDesign,
    #[error("implement before stages/02-tests/output/ has a test plan")]
    ImplementBeforeTests,
    #[error(
        "commit before stages/04-verify/output/gate-report.md shows GATE GREEN (gate: {})",
        gate_label(gate)
    )]
    CommitBeforeGreenGate { gate: Option<Verdict> },
    #[error("a steer-request awaits the operator at {}", stage_list(stages))]
    SteerPending { stages: Vec<StageId> },
}

fn gate_label(gate: &Option<Verdict>) -> &'static str {
    match gate {
        Some(Verdict::Green) => "green",
        Some(Verdict::Red) => "red",
        None => "no verdict",
    }
}

fn stage_list(stages: &[StageId]) -> String {
    stages
        .iter()
        .map(|stage: &StageId| stage.dir_name())
        .collect::<Vec<&str>>()
        .join(", ")
}
