/// An inconsistency in the run dir worth surfacing. Anomalies never change
/// what `check` refuses — refusals come from the facts directly.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Anomaly {
    /// A later stage has output while an earlier handoff is empty (e.g. the
    /// operator deleted the design after tests were written). Names stage dirs.
    LaterOutputWithoutEarlier { later: String, earlier: String },
}
