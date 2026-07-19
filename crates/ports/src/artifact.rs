use crate::snapshot::SnapshotError;
use da_domain::Flow;
use std::path::Path;

/// One run artifact: a run-dir-relative path and its UTF-8 content.
/// Artifacts are the run's durable ephemera — run.edn, flow.ron, spec.md,
/// every stage's output/ files, and `worktree.patch`.
///
/// The patch is what makes the mirror sufficient on its own. Branch and base
/// commit alone are not enough: the run branch is never pushed, so it does
/// not exist on another host, and between the first stage and the commit
/// stage the code is intermediate history that no shared ref names. The
/// patch is base-relative text, so any clone holding the base commit can
/// reconstitute the run's code exactly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunArtifact {
    pub path: String,
    pub content: String,
}

/// What the mirror holds for a run: the last published state (raw wire JSON)
/// and the artifact set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirrorSnapshot {
    pub state_json: Option<String>,
    pub files: Vec<RunArtifact>,
}

/// Collects a run dir's artifacts for publishing.
pub trait ArtifactSource {
    fn collect(&self, flow: &Flow, run_dir: &Path) -> Result<Vec<RunArtifact>, SnapshotError>;
}

/// Materializes fetched artifacts into a run dir (restore on another host).
pub trait ArtifactSink {
    fn materialize(&self, run_dir: &Path, files: &[RunArtifact]) -> Result<(), SnapshotError>;
}
