//! The artifact bridge against real temp dirs: collect a run dir's durable
//! ephemera, materialize them elsewhere, refuse hostile paths.

#![allow(clippy::unwrap_used)]

use da_adapter_fs::{FsArtifactSink, FsArtifactSource, load_run_flow};
use da_domain::Flow;
use da_ports::{ArtifactSink, ArtifactSource, RunArtifact, SnapshotError};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

const RUN_EDN: &str = "{:run-id \"250718-artifacts\" :phase \"steady-state\"}";
const FLOW_RON: &str = include_str!("../../../algorithm/flow.ron");

fn scaffold() -> (TempDir, Flow) {
    let dir: TempDir = TempDir::new().unwrap();
    fs::write(dir.path().join("run.edn"), RUN_EDN).unwrap();
    fs::write(dir.path().join("flow.ron"), FLOW_RON).unwrap();
    let flow: Flow = load_run_flow(dir.path()).unwrap();
    for (_, stage) in flow.stages() {
        let output: PathBuf = dir.path().join("stages").join(&stage.dir).join("output");
        fs::create_dir_all(&output).unwrap();
        fs::write(output.join(".gitkeep"), "").unwrap();
    }
    (dir, flow)
}

fn paths(files: &[RunArtifact]) -> Vec<&str> {
    files
        .iter()
        .map(|f: &RunArtifact| f.path.as_str())
        .collect()
}

// Scenario: a fresh run collects only its root files (zero outputs)
#[test]
fn fresh_run_collects_root_files_only() {
    let (dir, flow): (TempDir, Flow) = scaffold();
    let files: Vec<RunArtifact> = FsArtifactSource.collect(&flow, dir.path()).unwrap();
    assert_eq!(paths(&files), vec!["run.edn", "flow.ron"]);
}

// Scenario: one stage output is collected under its stage path
#[test]
fn one_output_file_is_collected() {
    let (dir, flow): (TempDir, Flow) = scaffold();
    fs::write(
        dir.path().join("stages/01-design/output/design.md"),
        "# design",
    )
    .unwrap();
    let files: Vec<RunArtifact> = FsArtifactSource.collect(&flow, dir.path()).unwrap();
    assert!(paths(&files).contains(&"stages/01-design/output/design.md"));
}

// Scenario: many outputs across stages, .gitkeep never included
#[test]
fn many_outputs_are_collected_and_gitkeep_is_not() {
    let (dir, flow): (TempDir, Flow) = scaffold();
    fs::write(dir.path().join("spec.md"), "the spec").unwrap();
    fs::write(dir.path().join("stages/01-design/output/design.md"), "d").unwrap();
    fs::write(
        dir.path().join("stages/02-tests/output/STEER-REQUEST.md"),
        "## Question\n\nq?\n\n## Answer\n\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("stages/04-verify/output/gate-report.md"),
        "GATE GREEN",
    )
    .unwrap();
    let files: Vec<RunArtifact> = FsArtifactSource.collect(&flow, dir.path()).unwrap();
    let collected: Vec<&str> = paths(&files);
    assert_eq!(
        collected,
        vec![
            "run.edn",
            "flow.ron",
            "spec.md",
            "stages/01-design/output/design.md",
            "stages/02-tests/output/STEER-REQUEST.md",
            "stages/04-verify/output/gate-report.md",
        ]
    );
    assert!(!collected.iter().any(|p: &&str| p.contains(".gitkeep")));
}

// Scenario: a collect -> materialize round trip reproduces the run dir
#[test]
fn collect_then_materialize_round_trips() {
    let (dir, flow): (TempDir, Flow) = scaffold();
    fs::write(dir.path().join("stages/01-design/output/design.md"), "# d").unwrap();
    let files: Vec<RunArtifact> = FsArtifactSource.collect(&flow, dir.path()).unwrap();

    let target: TempDir = TempDir::new().unwrap();
    FsArtifactSink.materialize(target.path(), &files).unwrap();
    assert_eq!(
        fs::read_to_string(target.path().join("run.edn")).unwrap(),
        RUN_EDN
    );
    assert_eq!(
        fs::read_to_string(target.path().join("stages/01-design/output/design.md")).unwrap(),
        "# d"
    );
    let restored_flow: Flow = load_run_flow(target.path()).unwrap();
    assert_eq!(restored_flow, flow);
}

// Scenario: hostile mirror paths are refused (zero writes)
#[test]
fn parent_traversal_path_is_refused() {
    let target: TempDir = TempDir::new().unwrap();
    let hostile: Vec<RunArtifact> = vec![RunArtifact {
        path: "../escape.md".to_string(),
        content: "x".to_string(),
    }];
    assert!(matches!(
        FsArtifactSink.materialize(target.path(), &hostile),
        Err(SnapshotError::Malformed { .. })
    ));
}

#[test]
fn absolute_path_is_refused() {
    let target: TempDir = TempDir::new().unwrap();
    let hostile: Vec<RunArtifact> = vec![RunArtifact {
        path: "/etc/escape.md".to_string(),
        content: "x".to_string(),
    }];
    assert!(matches!(
        FsArtifactSink.materialize(target.path(), &hostile),
        Err(SnapshotError::Malformed { .. })
    ));
}
