//! Ring 3: the filesystem adapter. Parses a run dir into [`da_domain::FsFacts`],
//! byte-compatible with the bb scripts' reading of the same files.

mod fs_snapshot;
mod gate_report;
mod run_edn;
mod steer_file;

pub use fs_snapshot::FsSnapshotSource;
pub use gate_report::gate_verdict;
pub use run_edn::{EdnFacts, extract_edn_facts};
pub use steer_file::steer_answered;
