/// The five pipeline stages, in handoff order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StageId {
    Design,
    Tests,
    Implement,
    Verify,
    Commit,
}

impl StageId {
    /// Every stage, in pipeline order.
    pub const ALL: [StageId; 5] = [
        StageId::Design,
        StageId::Tests,
        StageId::Implement,
        StageId::Verify,
        StageId::Commit,
    ];

    /// The stage's directory under `stages/`.
    pub fn dir_name(self) -> &'static str {
        match self {
            StageId::Design => "01-design",
            StageId::Tests => "02-tests",
            StageId::Implement => "03-implement",
            StageId::Verify => "04-verify",
            StageId::Commit => "05-commit",
        }
    }
}
