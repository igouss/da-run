//! Ring 3: the filesystem adapter. Parses a run dir into [`da_domain::FsFacts`],
//! byte-compatible with the bb scripts' reading of the same files, and loads
//! the pipeline definition from `flow.ron`.

mod events;
mod flow_ron;
mod read;
mod fs_snapshot;
mod gate_report;
mod run_artifacts;
mod run_json;
mod steer_file;
mod worktree_patch;

pub use events::{EVENTS_FILE, append_event, fingerprint_of, outputs_fingerprint};
pub use flow_ron::{FLOW_FILE, FlowLoadError, load_flow_file, load_run_flow};
pub use fs_snapshot::{COMMIT_VERIFIED, FsSnapshotSource};
pub use gate_report::{gate_verdict, gate_worktree};
pub use run_artifacts::{FsArtifactSink, FsArtifactSource};
pub use run_json::{ManifestFacts, parse_manifest};
pub use steer_file::steer_answered;
pub use worktree_patch::{WORKTREE_PATCH, worktree_facts};
