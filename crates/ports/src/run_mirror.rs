use crate::artifact::{MirrorSnapshot, RunArtifact};
use da_domain::{Derived, RunId};

/// Why the mirror could not be reached or read.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("mirror: {detail}")]
pub struct MirrorError {
    pub detail: String,
}

/// The durable run mirror (non-authoritative for a live run — the filesystem
/// stays canonical, ADR-0029). Holds the derived state and the artifact set,
/// so a run can be restored from the mirror on another host.
///
/// State and artifacts travel in ONE publish: two calls would let a half
/// failure leave the mirror advertising a state its artifacts do not
/// support (da-run's recorded publish-atomicity gap, closed here by shape).
pub trait RunMirror {
    fn publish_snapshot(
        &self,
        run_id: &RunId,
        derived: &Derived,
        files: &[RunArtifact],
    ) -> Result<(), MirrorError>;
    fn fetch_snapshot(&self, run_id: &RunId) -> Result<MirrorSnapshot, MirrorError>;
}
