use da_domain::{Derived, Flow, FsFacts, RunId, derive};
use da_ports::{
    ArtifactSink, ArtifactSource, MirrorError, MirrorSnapshot, RunArtifact, RunMirror,
    SnapshotError, SnapshotSource,
};
use std::path::Path;

/// Why a mirror publish did not happen.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    Snapshot(#[from] SnapshotError),
    #[error(transparent)]
    Mirror(#[from] MirrorError),
}

/// What a successful publish sent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Published {
    pub run_id: RunId,
    pub derived: Derived,
    /// Artifact files pushed alongside the state.
    pub artifact_count: usize,
}

/// Snapshot a run dir and publish its derived state AND its artifacts to the
/// mirror — after this, the mirror can restore the run on another host.
/// Best-effort policy (ignore an unreachable mirror) is the caller's call.
pub fn publish_mirror<S: SnapshotSource, A: ArtifactSource, M: RunMirror>(
    source: &S,
    artifacts: &A,
    mirror: &M,
    flow: &Flow,
    run_dir: &Path,
) -> Result<Published, PublishError> {
    let facts: FsFacts = source.snapshot(flow, run_dir)?;
    let derived: Derived = derive(flow, &facts);
    let files: Vec<RunArtifact> = artifacts.collect(flow, run_dir)?;
    mirror.publish(&facts.run_id, &derived)?;
    mirror.publish_artifacts(&facts.run_id, &files)?;
    Ok(Published {
        run_id: facts.run_id,
        derived,
        artifact_count: files.len(),
    })
}

/// Why a restore did not happen.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RestoreError {
    #[error(transparent)]
    Mirror(#[from] MirrorError),
    #[error(transparent)]
    Sink(#[from] SnapshotError),
    #[error("the mirror holds no artifacts for run {run_id:?}")]
    NothingMirrored { run_id: String },
}

/// What a restore materialized.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Restored {
    pub file_count: usize,
    pub state_json: Option<String>,
}

/// Fetch a run's mirror snapshot and materialize its artifacts into
/// `run_dir` — the restart-on-another-host path. The worktree is NOT
/// restored: recreate it from run.edn's project/branch/base-commit.
pub fn restore_run<M: RunMirror, K: ArtifactSink>(
    mirror: &M,
    sink: &K,
    run_id: &RunId,
    run_dir: &Path,
) -> Result<Restored, RestoreError> {
    let snapshot: MirrorSnapshot = mirror.fetch_snapshot(run_id)?;
    if snapshot.files.is_empty() {
        return Err(RestoreError::NothingMirrored {
            run_id: run_id.as_str().to_string(),
        });
    }
    sink.materialize(run_dir, &snapshot.files)?;
    Ok(Restored {
        file_count: snapshot.files.len(),
        state_json: snapshot.state_json,
    })
}
