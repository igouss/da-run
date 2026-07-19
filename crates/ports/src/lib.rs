//! Ring 1: ports. Sync traits over domain types — I/O shape lives in the
//! adapters, never here.

mod artifact;
mod run_mirror;
mod snapshot;

pub use artifact::{ArtifactSink, ArtifactSource, MirrorSnapshot, RunArtifact};
pub use run_mirror::{MirrorError, RunMirror};
pub use snapshot::{SnapshotError, SnapshotSource};
