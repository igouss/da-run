//! The fs adapter against real (temp) run dirs shaped like `bin/run setup`
//! makes them.

#![allow(clippy::unwrap_used)]

use da_adapter_fs::FsSnapshotSource;
use da_domain::{FsFacts, Phase, StageId, Verdict};
use da_ports::{SnapshotError, SnapshotSource};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

const RUN_EDN: &str = "{:run-id \"250718-fixture\"\n :arm \"pre\"\n :phase \"steady-state\"\n :target {:project \"/p\"}}";

fn scaffold_run_dir() -> TempDir {
    let dir: TempDir = TempDir::new().unwrap();
    fs::write(dir.path().join("run.edn"), RUN_EDN).unwrap();
    for stage in StageId::ALL {
        let output: PathBuf = output_dir(&dir, stage);
        fs::create_dir_all(&output).unwrap();
        fs::write(output.join(".gitkeep"), "").unwrap();
    }
    dir
}

fn output_dir(dir: &TempDir, stage: StageId) -> PathBuf {
    dir.path()
        .join("stages")
        .join(stage.dir_name())
        .join("output")
}

fn write_output(dir: &TempDir, stage: StageId, name: &str, content: &str) {
    fs::write(output_dir(dir, stage).join(name), content).unwrap();
}

fn snapshot(dir: &TempDir) -> Result<FsFacts, SnapshotError> {
    FsSnapshotSource.snapshot(dir.path())
}

// Scenario: a fresh scaffold is empty facts
#[test]
fn fresh_scaffold_has_no_output_anywhere() {
    let dir: TempDir = scaffold_run_dir();
    let facts: FsFacts = snapshot(&dir).unwrap();
    assert!(!facts.stages.get(StageId::Design).has_output());
    assert_eq!(facts.gate, None);
    assert!(!facts.commit_recorded);
    assert_eq!(facts.phase, Phase::SteadyState);
    assert_eq!(facts.run_id.as_str(), "250718-fixture");
}

// Scenario: .gitkeep never counts as output
#[test]
fn gitkeep_is_not_output() {
    let dir: TempDir = scaffold_run_dir();
    let facts: FsFacts = snapshot(&dir).unwrap();
    assert_eq!(
        facts.stages.get(StageId::Design).output_files,
        Vec::<String>::new()
    );
}

// Scenario: one design file
#[test]
fn one_design_file_is_design_output() {
    let dir: TempDir = scaffold_run_dir();
    write_output(&dir, StageId::Design, "design.md", "# design");
    let facts: FsFacts = snapshot(&dir).unwrap();
    assert_eq!(
        facts.stages.get(StageId::Design).output_files,
        vec!["design.md".to_string()]
    );
}

// Scenario: many output files arrive sorted
#[test]
fn many_output_files_are_sorted() {
    let dir: TempDir = scaffold_run_dir();
    write_output(&dir, StageId::Design, "b.md", "b");
    write_output(&dir, StageId::Design, "a.md", "a");
    let facts: FsFacts = snapshot(&dir).unwrap();
    assert_eq!(
        facts.stages.get(StageId::Design).output_files,
        vec!["a.md".to_string(), "b.md".to_string()]
    );
}

// Scenario: a green gate report
#[test]
fn green_gate_report_reads_green() {
    let dir: TempDir = scaffold_run_dir();
    write_output(
        &dir,
        StageId::Verify,
        "gate-report.md",
        "=== gate ===\nall ok\n\nGATE GREEN\n",
    );
    let facts: FsFacts = snapshot(&dir).unwrap();
    assert_eq!(facts.gate, Some(Verdict::Green));
}

// Scenario: a red gate report
#[test]
fn red_gate_report_reads_red() {
    let dir: TempDir = scaffold_run_dir();
    write_output(
        &dir,
        StageId::Verify,
        "gate-report.md",
        "tests failed\n\nGATE RED — do not ship\n",
    );
    let facts: FsFacts = snapshot(&dir).unwrap();
    assert_eq!(facts.gate, Some(Verdict::Red));
}

// Scenario: a garbage gate report fails closed
#[test]
fn garbage_gate_report_reads_none() {
    let dir: TempDir = scaffold_run_dir();
    write_output(&dir, StageId::Verify, "gate-report.md", "inconclusive");
    let facts: FsFacts = snapshot(&dir).unwrap();
    assert_eq!(facts.gate, None);
}

// Scenario: an unanswered steer-request is not stage output
#[test]
fn unanswered_steer_is_pending_and_not_output() {
    let dir: TempDir = scaffold_run_dir();
    write_output(
        &dir,
        StageId::Tests,
        "STEER-REQUEST.md",
        "# STEER-REQUEST — 02-tests\n\n## Question\n\nWhich port?\n\n## Answer\n\n",
    );
    let facts: FsFacts = snapshot(&dir).unwrap();
    assert!(facts.stages.get(StageId::Tests).steer_pending());
    assert!(!facts.stages.get(StageId::Tests).has_output());
}

// Scenario: an answered steer-request clears
#[test]
fn answered_steer_is_not_pending() {
    let dir: TempDir = scaffold_run_dir();
    write_output(
        &dir,
        StageId::Tests,
        "STEER-REQUEST.md",
        "## Question\n\nWhich port?\n\n## Answer\n\nuse 9080\n",
    );
    let facts: FsFacts = snapshot(&dir).unwrap();
    assert!(!facts.stages.get(StageId::Tests).steer_pending());
}

// Scenario: a commit record marks the run committed
#[test]
fn commit_output_marks_commit_recorded() {
    let dir: TempDir = scaffold_run_dir();
    write_output(&dir, StageId::Commit, "commit.md", "scoped: message");
    let facts: FsFacts = snapshot(&dir).unwrap();
    assert!(facts.commit_recorded);
}

// Scenario: not a run dir
#[test]
fn missing_run_edn_is_not_a_run_dir() {
    let dir: TempDir = TempDir::new().unwrap();
    assert!(matches!(
        snapshot(&dir),
        Err(SnapshotError::NotARunDir { .. })
    ));
}

// Scenario: a malformed run.edn is refused loudly
#[test]
fn unknown_phase_is_malformed() {
    let dir: TempDir = scaffold_run_dir();
    fs::write(
        dir.path().join("run.edn"),
        "{:run-id \"r1\" :phase \"warp-speed\"}",
    )
    .unwrap();
    assert!(matches!(
        snapshot(&dir),
        Err(SnapshotError::Malformed { .. })
    ));
}

#[test]
fn missing_run_id_is_malformed() {
    let dir: TempDir = scaffold_run_dir();
    fs::write(dir.path().join("run.edn"), "{:phase \"steady-state\"}").unwrap();
    assert!(matches!(
        snapshot(&dir),
        Err(SnapshotError::Malformed { .. })
    ));
}

// Scenario: a missing output dir is simply empty
#[test]
fn missing_output_dir_is_empty_stage() {
    let dir: TempDir = TempDir::new().unwrap();
    fs::write(dir.path().join("run.edn"), RUN_EDN).unwrap();
    let facts: FsFacts = snapshot(&dir).unwrap();
    assert!(!facts.stages.get(StageId::Design).has_output());
}
