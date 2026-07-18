use da_domain::{Derived, FsFacts, StageId, derive};
use da_ports::{SnapshotError, SnapshotSource};
use std::path::Path;

/// One stage's line in the status render.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageStatus {
    pub stage: StageId,
    pub complete: bool,
    pub files: Vec<String>,
    pub steer_pending: bool,
}

/// The full status view: derived summary plus per-stage detail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusReport {
    pub derived: Derived,
    pub stages: Vec<StageStatus>,
}

/// Snapshot a run dir and render its status.
pub fn status<S: SnapshotSource>(
    source: &S,
    run_dir: &Path,
) -> Result<StatusReport, SnapshotError> {
    let facts: FsFacts = source.snapshot(run_dir)?;
    let stages: Vec<StageStatus> = StageId::ALL
        .into_iter()
        .map(|stage: StageId| StageStatus {
            stage,
            complete: facts.stages.get(stage).has_output(),
            files: facts.stages.get(stage).output_files.clone(),
            steer_pending: facts.stages.get(stage).steer_pending(),
        })
        .collect();
    Ok(StatusReport {
        derived: derive(&facts),
        stages,
    })
}
