use crate::gate_report::{gate_verdict, gate_worktree};
use crate::run_json::{ManifestFacts, parse_manifest, parse_phase};
use crate::steer_file::steer_answered;
use crate::worktree_patch::{WORKTREE_PATCH, worktree_facts};
use da_domain::{
    Flow, FsFacts, Phase, RunId, StageDef, StageFacts, StageFactsMap, StageRef, SteerFacts, Verdict,
    WorktreeFacts, WorktreeId,
};
use da_ports::{SnapshotError, SnapshotSource};
use std::path::{Path, PathBuf};

/// Files never counted as stage output.
const GITKEEP: &str = ".gitkeep";
const STEER_FILE: &str = "STEER-REQUEST.md";
/// The orchestrator-verified commit marker (`bin/run record-commit`).
pub const COMMIT_VERIFIED: &str = "commit-verified";
/// The run manifest `bin/run setup` writes — its presence defines a run dir.
pub const RUN_MANIFEST: &str = "run.json";

/// Reads a run dir (as laid out by `bin/run setup`) into [`FsFacts`].
pub struct FsSnapshotSource;

impl SnapshotSource for FsSnapshotSource {
    fn snapshot(&self, flow: &Flow, run_dir: &Path) -> Result<FsFacts, SnapshotError> {
        let manifest_path: PathBuf = run_dir.join(RUN_MANIFEST);
        if !manifest_path.is_file() {
            return Err(SnapshotError::NotARunDir {
                path: run_dir.to_path_buf(),
            });
        }
        let manifest_text: String = read_file(&manifest_path)?;
        let manifest: ManifestFacts =
            parse_manifest(&manifest_text).map_err(|detail: String| SnapshotError::Malformed {
                path: manifest_path.clone(),
                detail,
            })?;
        let run_id: RunId = refine_run_id(&manifest_path, manifest.run_id.as_deref())?;
        let phase: Phase = refine_phase(&manifest_path, manifest.phase.as_deref())?;

        let mut stage_facts: Vec<StageFacts> = Vec::new();
        for (_, stage) in flow.stages() {
            stage_facts.push(read_stage(run_dir, &stage.dir)?);
        }
        let stages: StageFactsMap = StageFactsMap::from_fn(flow, |stage: StageRef| {
            stage_facts
                .get(stage_index(flow, stage))
                .cloned()
                .unwrap_or_else(StageFacts::empty)
        });

        let report: Option<String> = read_gate_report(flow, run_dir)?;
        let gate: Option<Verdict> = report.as_deref().and_then(gate_verdict);
        let gate_worktree: Option<WorktreeId> = report.as_deref().and_then(gate_worktree);
        let worktree: Option<WorktreeFacts> = read_worktree(run_dir)?;
        // The commit fact is the orchestrator's verified marker, written by
        // `bin/run record-commit` only after the sha resolves in the
        // worktree's git — never the commit agent's own output. An agent
        // that wrote commit.md over a failed `git commit` used to derive
        // Committed with no commit on the branch (da-run's recorded
        // commit-record-trust gap, closed here).
        let commit_recorded: bool = run_dir.join(COMMIT_VERIFIED).is_file();
        Ok(FsFacts {
            stages,
            gate,
            commit_recorded,
            worktree,
            gate_worktree,
            phase,
            run_id,
        })
    }
}

fn read_stage(run_dir: &Path, stage_dir: &str) -> Result<StageFacts, SnapshotError> {
    let output_dir: PathBuf = run_dir.join("stages").join(stage_dir).join("output");
    if !output_dir.is_dir() {
        return Ok(StageFacts::empty());
    }
    let mut output_files: Vec<String> = Vec::new();
    let mut steer: Option<SteerFacts> = None;
    let entries =
        std::fs::read_dir(&output_dir).map_err(|error: std::io::Error| SnapshotError::Io {
            path: output_dir.clone(),
            detail: error.to_string(),
        })?;
    for entry in entries {
        let entry: std::fs::DirEntry =
            entry.map_err(|error: std::io::Error| SnapshotError::Io {
                path: output_dir.clone(),
                detail: error.to_string(),
            })?;
        let name: String = entry.file_name().to_string_lossy().into_owned();
        if name == STEER_FILE {
            let content: String = read_file(&entry.path())?;
            steer = Some(SteerFacts {
                answered: steer_answered(&content),
            });
        } else if name != GITKEEP {
            output_files.push(name);
        }
    }
    output_files.sort();
    Ok(StageFacts {
        output_files,
        steer,
    })
}

fn read_gate_report(flow: &Flow, run_dir: &Path) -> Result<Option<String>, SnapshotError> {
    let report_path: PathBuf = run_dir.join(flow.gate_report_path());
    if !report_path.is_file() {
        return Ok(None);
    }
    read_file(&report_path).map(Some)
}

fn read_worktree(run_dir: &Path) -> Result<Option<WorktreeFacts>, SnapshotError> {
    let patch_path: PathBuf = run_dir.join(WORKTREE_PATCH);
    if !patch_path.is_file() {
        return Ok(None);
    }
    Ok(worktree_facts(&read_file(&patch_path)?))
}

fn refine_run_id(manifest_path: &Path, raw: Option<&str>) -> Result<RunId, SnapshotError> {
    let raw: &str = raw.ok_or_else(|| SnapshotError::Malformed {
        path: manifest_path.to_path_buf(),
        detail: "run.json has no \"run-id\"".to_string(),
    })?;
    RunId::new(raw).map_err(|error: da_domain::RunIdError| SnapshotError::Malformed {
        path: manifest_path.to_path_buf(),
        detail: format!("run.json \"run-id\" refused: {error}"),
    })
}

fn refine_phase(manifest_path: &Path, raw: Option<&str>) -> Result<Phase, SnapshotError> {
    parse_phase(raw).ok_or_else(|| SnapshotError::Malformed {
        path: manifest_path.to_path_buf(),
        detail: format!(
            "run.json \"phase\" {:?} is neither convergence nor steady-state",
            raw
        ),
    })
}

fn read_file(path: &Path) -> Result<String, SnapshotError> {
    crate::read::read_utf8(path)
}

fn stage_index(flow: &Flow, target: StageRef) -> usize {
    flow.stages()
        .position(|(stage, _): (StageRef, &StageDef)| stage == target)
        .unwrap_or(0)
}
