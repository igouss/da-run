use da_domain::{Derived, FsFacts, derive};
use da_ports::{MirrorError, RunMirror, SnapshotError, SnapshotSource};
use std::path::Path;

/// Why a mirror publish did not happen.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    Snapshot(#[from] SnapshotError),
    #[error(transparent)]
    Mirror(#[from] MirrorError),
}

/// Snapshot a run dir and publish its derived state to the mirror.
/// Best-effort policy (ignore an unreachable mirror) is the caller's call.
pub fn publish_mirror<S: SnapshotSource, M: RunMirror>(
    source: &S,
    mirror: &M,
    run_dir: &Path,
) -> Result<Derived, PublishError> {
    let facts: FsFacts = source.snapshot(run_dir)?;
    let derived: Derived = derive(&facts);
    mirror.publish(&facts.run_id, &derived)?;
    Ok(derived)
}
