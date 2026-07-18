/// The mechanical gate's verdict, parsed from the final line of
/// `stages/04-verify/output/gate-report.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Green,
    Red,
}
