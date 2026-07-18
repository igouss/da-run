use da_domain::{Derived, RunId};

/// Why the mirror could not be published.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("mirror publish failed: {detail}")]
pub struct MirrorError {
    pub detail: String,
}

/// Publishes a run's derived state to the durable mirror (non-authoritative —
/// the filesystem stays canonical, ADR-0029).
pub trait RunMirror {
    fn publish(&self, run_id: &RunId, derived: &Derived) -> Result<(), MirrorError>;
}
