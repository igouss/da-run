//! The fs adapter against real (temp) run dirs shaped like `bin/run setup`
//! makes them — flow.ron included.

#![allow(clippy::unwrap_used)]

use da_adapter_fs::{FlowLoadError, FsSnapshotSource, load_run_flow};
use da_domain::{Flow, FsFacts, Phase, StageRef, Verdict};
use da_ports::{SnapshotError, SnapshotSource};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

const RUN_JSON: &str = r#"{"run-id":"250718-fixture","arm":"pre","phase":"steady-state","target":{"project":"/p"}}"#;

/// The canonical flow — the adapter must read exactly what bin/run copies.
const FLOW_RON: &str = include_str!("../../../engine/fixtures/minimal-flow.ron");

fn scaffold_run_dir() -> (TempDir, Flow) {
    let dir: TempDir = TempDir::new().unwrap();
    fs::write(dir.path().join("run.json"), RUN_JSON).unwrap();
    fs::write(dir.path().join("flow.ron"), FLOW_RON).unwrap();
    let flow: Flow = load_run_flow(dir.path()).unwrap();
    for (_, stage) in flow.stages() {
        let output: PathBuf = dir.path().join("stages").join(&stage.dir).join("output");
        fs::create_dir_all(&output).unwrap();
        fs::write(output.join(".gitkeep"), "").unwrap();
    }
    (dir, flow)
}

fn stage_ref(flow: &Flow, name: &str) -> StageRef {
    flow.stage_named(name).unwrap()
}

fn write_output(dir: &TempDir, flow: &Flow, name: &str, file: &str, content: &str) {
    let stage_dir: String = flow.stage(stage_ref(flow, name)).unwrap().dir.clone();
    let output: PathBuf = dir.path().join("stages").join(stage_dir).join("output");
    fs::write(output.join(file), content).unwrap();
}

fn snapshot(dir: &TempDir, flow: &Flow) -> Result<FsFacts, SnapshotError> {
    FsSnapshotSource.snapshot(flow, dir.path())
}

// Scenario: a fresh scaffold is empty facts
#[test]
fn fresh_scaffold_has_no_output_anywhere() {
    let (dir, flow): (TempDir, Flow) = scaffold_run_dir();
    let facts: FsFacts = snapshot(&dir, &flow).unwrap();
    assert!(!facts.stages.get(stage_ref(&flow, "plan")).has_output());
    assert_eq!(facts.gate, None);
    assert!(!facts.commit_recorded);
    assert_eq!(facts.phase, Phase::SteadyState);
    assert_eq!(facts.run_id.as_str(), "250718-fixture");
}

// Scenario: .gitkeep never counts as output
#[test]
fn gitkeep_is_not_output() {
    let (dir, flow): (TempDir, Flow) = scaffold_run_dir();
    let facts: FsFacts = snapshot(&dir, &flow).unwrap();
    assert_eq!(
        facts.stages.get(stage_ref(&flow, "plan")).output_files,
        Vec::<String>::new()
    );
}

// Scenario: one design file
#[test]
fn one_design_file_is_design_output() {
    let (dir, flow): (TempDir, Flow) = scaffold_run_dir();
    write_output(&dir, &flow, "plan", "plan.md", "# design");
    let facts: FsFacts = snapshot(&dir, &flow).unwrap();
    assert_eq!(
        facts.stages.get(stage_ref(&flow, "plan")).output_files,
        vec!["plan.md".to_string()]
    );
}

// Scenario: many output files arrive sorted
#[test]
fn many_output_files_are_sorted() {
    let (dir, flow): (TempDir, Flow) = scaffold_run_dir();
    write_output(&dir, &flow, "plan", "b.md", "b");
    write_output(&dir, &flow, "plan", "a.md", "a");
    let facts: FsFacts = snapshot(&dir, &flow).unwrap();
    assert_eq!(
        facts.stages.get(stage_ref(&flow, "plan")).output_files,
        vec!["a.md".to_string(), "b.md".to_string()]
    );
}

// Scenario: a green gate report
#[test]
fn green_gate_report_reads_green() {
    let (dir, flow): (TempDir, Flow) = scaffold_run_dir();
    write_output(
        &dir,
        &flow,
        "check",
        "gate-report.md",
        "=== gate ===\nall ok\n\nGATE GREEN\n",
    );
    let facts: FsFacts = snapshot(&dir, &flow).unwrap();
    assert_eq!(facts.gate, Some(Verdict::Green));
}

// Scenario: a red gate report
#[test]
fn red_gate_report_reads_red() {
    let (dir, flow): (TempDir, Flow) = scaffold_run_dir();
    write_output(
        &dir,
        &flow,
        "check",
        "gate-report.md",
        "tests failed\n\nGATE RED — do not ship\n",
    );
    let facts: FsFacts = snapshot(&dir, &flow).unwrap();
    assert_eq!(facts.gate, Some(Verdict::Red));
}

// Scenario: a garbage gate report fails closed
#[test]
fn garbage_gate_report_reads_none() {
    let (dir, flow): (TempDir, Flow) = scaffold_run_dir();
    write_output(&dir, &flow, "check", "gate-report.md", "inconclusive");
    let facts: FsFacts = snapshot(&dir, &flow).unwrap();
    assert_eq!(facts.gate, None);
}

// Scenario: an unanswered steer-request is not stage output
#[test]
fn unanswered_steer_is_pending_and_not_output() {
    let (dir, flow): (TempDir, Flow) = scaffold_run_dir();
    write_output(
        &dir,
        &flow,
        "build",
        "STEER-REQUEST.md",
        "# STEER-REQUEST — 02-build\n\n## Question\n\nWhich port?\n\n## Answer\n\n",
    );
    let facts: FsFacts = snapshot(&dir, &flow).unwrap();
    assert!(facts.stages.get(stage_ref(&flow, "build")).steer_pending());
    assert!(!facts.stages.get(stage_ref(&flow, "build")).has_output());
}

// Scenario: an answered steer-request clears
#[test]
fn answered_steer_is_not_pending() {
    let (dir, flow): (TempDir, Flow) = scaffold_run_dir();
    write_output(
        &dir,
        &flow,
        "build",
        "STEER-REQUEST.md",
        "## Question\n\nWhich port?\n\n## Answer\n\nuse 9080\n",
    );
    let facts: FsFacts = snapshot(&dir, &flow).unwrap();
    assert!(!facts.stages.get(stage_ref(&flow, "build")).steer_pending());
}

// Scenario: only the orchestrator's verified marker marks the run committed
#[test]
fn verified_marker_marks_commit_recorded() {
    let (dir, flow): (TempDir, Flow) = scaffold_run_dir();
    fs::write(dir.path().join("commit-verified"), "Commit: abc123\n").unwrap();
    let facts: FsFacts = snapshot(&dir, &flow).unwrap();
    assert!(facts.commit_recorded);
}

// Scenario: a commit agent's own output is a claim, not evidence — without
// the verified marker the run must not derive as committed
#[test]
fn commit_output_alone_is_not_commit_recorded() {
    let (dir, flow): (TempDir, Flow) = scaffold_run_dir();
    write_output(&dir, &flow, "land", "commit.md", "scoped: message");
    let facts: FsFacts = snapshot(&dir, &flow).unwrap();
    assert!(!facts.commit_recorded);
}

// Scenario: not a run dir
#[test]
fn missing_run_json_is_not_a_run_dir() {
    let (_, flow): (TempDir, Flow) = scaffold_run_dir();
    let bare: TempDir = TempDir::new().unwrap();
    assert!(matches!(
        snapshot(&bare, &flow),
        Err(SnapshotError::NotARunDir { .. })
    ));
}

// Scenario: a malformed run.json is refused loudly
#[test]
fn unknown_phase_is_malformed() {
    let (dir, flow): (TempDir, Flow) = scaffold_run_dir();
    fs::write(
        dir.path().join("run.json"),
        r#"{"run-id":"r1","phase":"warp-speed"}"#,
    )
    .unwrap();
    assert!(matches!(
        snapshot(&dir, &flow),
        Err(SnapshotError::Malformed { .. })
    ));
}

#[test]
fn missing_run_id_is_malformed() {
    let (dir, flow): (TempDir, Flow) = scaffold_run_dir();
    fs::write(dir.path().join("run.json"), r#"{"phase":"steady-state"}"#).unwrap();
    assert!(matches!(
        snapshot(&dir, &flow),
        Err(SnapshotError::Malformed { .. })
    ));
}

// Scenario: an EDN-era manifest (or any non-JSON text) is refused loudly
#[test]
fn edn_manifest_is_malformed() {
    let (dir, flow): (TempDir, Flow) = scaffold_run_dir();
    fs::write(
        dir.path().join("run.json"),
        "{:run-id \"r1\" :phase \"steady-state\"}",
    )
    .unwrap();
    assert!(matches!(
        snapshot(&dir, &flow),
        Err(SnapshotError::Malformed { .. })
    ));
}

// Scenario: a missing output dir is simply empty
#[test]
fn missing_output_dir_is_empty_stage() {
    let (_, flow): (TempDir, Flow) = scaffold_run_dir();
    let dir: TempDir = TempDir::new().unwrap();
    fs::write(dir.path().join("run.json"), RUN_JSON).unwrap();
    let facts: FsFacts = snapshot(&dir, &flow).unwrap();
    assert!(!facts.stages.get(stage_ref(&flow, "plan")).has_output());
}

// Scenario: flow loading fails loudly — zero file, garbage, invalid
#[test]
fn missing_flow_ron_is_an_io_error() {
    let dir: TempDir = TempDir::new().unwrap();
    assert!(matches!(
        load_run_flow(dir.path()),
        Err(FlowLoadError::Io { .. })
    ));
}

#[test]
fn garbage_flow_ron_is_a_parse_error() {
    let dir: TempDir = TempDir::new().unwrap();
    fs::write(dir.path().join("flow.ron"), "Flow(nonsense: true)").unwrap();
    assert!(matches!(
        load_run_flow(dir.path()),
        Err(FlowLoadError::Parse { .. })
    ));
}

#[test]
fn structurally_invalid_flow_ron_is_an_invalid_error() {
    let dir: TempDir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("flow.ron"),
        "Flow(initial_label: \"specced\", stages: [])",
    )
    .unwrap();
    assert!(matches!(
        load_run_flow(dir.path()),
        Err(FlowLoadError::Invalid { .. })
    ));
}
