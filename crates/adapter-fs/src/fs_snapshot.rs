use crate::gate_report::gate_verdict;
use crate::run_edn::{EdnFacts, extract_edn_facts, parse_phase};
use crate::steer_file::steer_answered;
use da_domain::{
    Flow, FsFacts, Phase, RunId, StageDef, StageFacts, StageFactsMap, StageRef, SteerFacts, Verdict,
};
use da_ports::{SnapshotError, SnapshotSource};
use std::path::{Path, PathBuf};

/// Files never counted as stage output.
const GITKEEP: &str = ".gitkeep";
const STEER_FILE: &str = "STEER-REQUEST.md";

/// Reads a run dir (as laid out by `bin/run setup`) into [`FsFacts`].
pub struct FsSnapshotSource;

impl SnapshotSource for FsSnapshotSource {
    fn snapshot(&self, flow: &Flow, run_dir: &Path) -> Result<FsFacts, SnapshotError> {
        let edn_path: PathBuf = run_dir.join("run.edn");
        if !edn_path.is_file() {
            return Err(SnapshotError::NotARunDir {
                path: run_dir.to_path_buf(),
            });
        }
        let edn_text: String = read_file(&edn_path)?;
        let edn: EdnFacts = extract_edn_facts(&edn_text);
        let run_id: RunId = refine_run_id(&edn_path, edn.run_id.as_deref())?;
        let phase: Phase = refine_phase(&edn_path, edn.phase.as_deref())?;

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

        let gate: Option<Verdict> = read_gate(flow, run_dir)?;
        let (commit_ref, _): (StageRef, &StageDef) = flow.commit();
        let commit_recorded: bool = stages.get(commit_ref).has_output();
        Ok(FsFacts {
            stages,
            gate,
            commit_recorded,
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

fn read_gate(flow: &Flow, run_dir: &Path) -> Result<Option<Verdict>, SnapshotError> {
    let report_path: PathBuf = run_dir.join(flow.gate_report_path());
    if !report_path.is_file() {
        return Ok(None);
    }
    let report: String = read_file(&report_path)?;
    Ok(gate_verdict(&report))
}

fn refine_run_id(edn_path: &Path, raw: Option<&str>) -> Result<RunId, SnapshotError> {
    let raw: &str = raw.ok_or_else(|| SnapshotError::Malformed {
        path: edn_path.to_path_buf(),
        detail: "run.edn has no :run-id".to_string(),
    })?;
    RunId::new(raw).map_err(|_blank: da_domain::BlankRunId| SnapshotError::Malformed {
        path: edn_path.to_path_buf(),
        detail: "run.edn :run-id is blank".to_string(),
    })
}

fn refine_phase(edn_path: &Path, raw: Option<&str>) -> Result<Phase, SnapshotError> {
    parse_phase(raw).ok_or_else(|| SnapshotError::Malformed {
        path: edn_path.to_path_buf(),
        detail: format!(
            "run.edn :phase {:?} is neither convergence nor steady-state",
            raw
        ),
    })
}

fn read_file(path: &Path) -> Result<String, SnapshotError> {
    std::fs::read_to_string(path).map_err(|error: std::io::Error| SnapshotError::Io {
        path: path.to_path_buf(),
        detail: error.to_string(),
    })
}

fn stage_index(flow: &Flow, target: StageRef) -> usize {
    flow.stages()
        .position(|(stage, _): (StageRef, &StageDef)| stage == target)
        .unwrap_or(0)
}
