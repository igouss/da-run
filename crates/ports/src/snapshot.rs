use da_domain::FsFacts;
use std::path::{Path, PathBuf};

/// Why a run dir could not be snapshotted into facts.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SnapshotError {
    #[error("{path}: not a run dir (no run.edn)")]
    NotARunDir { path: PathBuf },
    #[error("{path}: {detail}")]
    Malformed { path: PathBuf, detail: String },
    #[error("{path}: {detail}")]
    Io { path: PathBuf, detail: String },
}

/// Reads a run dir into a refined [`FsFacts`] snapshot.
pub trait SnapshotSource {
    fn snapshot(&self, run_dir: &Path) -> Result<FsFacts, SnapshotError>;
}
