use da_domain::{Derived, Flow, FsFacts, RunId, StageDef, StageRef, Verdict, derive};
use da_ports::{SnapshotError, SnapshotSource};
use std::path::Path;

/// One stage's line in the status render, keyed by the stage's dir.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageStatus {
    pub stage: String,
    pub complete: bool,
    pub files: Vec<String>,
    pub steer_pending: bool,
}

/// The full status view: derived summary plus per-stage detail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusReport {
    pub run_id: RunId,
    pub derived: Derived,
    /// The raw gate verdict — visible even after the run reaches Committed.
    pub gate: Option<Verdict>,
    pub stages: Vec<StageStatus>,
}

/// Snapshot a run dir and render its status.
pub fn status<S: SnapshotSource>(
    source: &S,
    flow: &Flow,
    run_dir: &Path,
) -> Result<StatusReport, SnapshotError> {
    let facts: FsFacts = source.snapshot(flow, run_dir)?;
    let stages: Vec<StageStatus> = flow
        .stages()
        .map(|(stage, def): (StageRef, &StageDef)| StageStatus {
            stage: def.dir.clone(),
            complete: facts.stages.get(stage).has_output(),
            files: facts.stages.get(stage).output_files.clone(),
            steer_pending: facts.stages.get(stage).steer_pending(),
        })
        .collect();
    Ok(StatusReport {
        run_id: facts.run_id.clone(),
        derived: derive(flow, &facts),
        gate: facts.gate,
        stages,
    })
}
