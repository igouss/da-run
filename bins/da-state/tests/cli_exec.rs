//! CLI-level tests: drive the compiled binary and pin the exit-code
//! contract the engine scripts (bin/state, bin/steer, da-stage.js) build on.
//! In-process tests cannot vary the environment safely, so these use the
//! real process boundary — env cleared or set per case.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const FIXTURE_FLOW: &str = include_str!("../../../engine/fixtures/minimal-flow.ron");

fn scaffold(run_dir: &Path) {
    fs::write(
        run_dir.join("run.json"),
        r#"{"run-id":"cli-test","phase":"steady-state"}"#,
    )
    .unwrap();
    fs::write(run_dir.join("flow.ron"), FIXTURE_FLOW).unwrap();
    for dir in ["01-plan", "02-build", "03-check", "04-land"] {
        fs::create_dir_all(run_dir.join("stages").join(dir).join("output")).unwrap();
    }
}

fn da_state(run_dir: &Path, args: &[&str], with_ingress: Option<&str>) -> Output {
    let mut command: Command = Command::new(env!("CARGO_BIN_EXE_da-state"));
    command.args(args).env_remove("DA_STEER_INGRESS");
    if let Some(ingress) = with_ingress {
        command.env("DA_STEER_INGRESS", ingress);
    }
    command.current_dir(run_dir).output().expect("binary runs")
}

fn exit_code(output: &Output) -> i32 {
    output.status.code().expect("no signal death")
}

#[test]
fn check_allowed_is_exit_0_with_allowed_true() {
    let dir: tempfile::TempDir = tempfile::TempDir::new().unwrap();
    scaffold(dir.path());
    let run: &str = dir.path().to_str().unwrap();
    let output: Output = da_state(dir.path(), &["check", "--run", run, "plan"], None);
    assert_eq!(exit_code(&output), 0, "{output:?}");
    let stdout: String = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"allowed\":true"), "{stdout}");
}

// Scenario: an allowed check journals its dispatch (ADR-0004) — one entry
// per check, trigger `dispatch:<kind>`, fingerprint included.
#[test]
fn check_allowed_journals_the_dispatch() {
    let dir: tempfile::TempDir = tempfile::TempDir::new().unwrap();
    scaffold(dir.path());
    let run: &str = dir.path().to_str().unwrap();
    let output: Output = da_state(dir.path(), &["check", "--run", run, "plan"], None);
    assert_eq!(exit_code(&output), 0, "{output:?}");
    let journal: String = fs::read_to_string(dir.path().join("events.jsonl")).unwrap();
    assert_eq!(journal.lines().count(), 1, "{journal}");
    assert!(journal.contains("\"trigger\":\"dispatch:plan\""), "{journal}");
    assert!(journal.contains("\"fingerprint\":\""), "{journal}");
}

#[test]
fn check_with_no_journal_writes_nothing() {
    let dir: tempfile::TempDir = tempfile::TempDir::new().unwrap();
    scaffold(dir.path());
    let run: &str = dir.path().to_str().unwrap();
    let output: Output = da_state(
        dir.path(),
        &["check", "--run", run, "plan", "--no-journal"],
        None,
    );
    assert_eq!(exit_code(&output), 0, "{output:?}");
    assert!(!dir.path().join("events.jsonl").exists());
}

// Scenario: a refused check journals nothing — no dispatch will follow, so
// an entry would wrongly claim the next window as that dispatch's work.
#[test]
fn check_refused_journals_nothing() {
    let dir: tempfile::TempDir = tempfile::TempDir::new().unwrap();
    scaffold(dir.path());
    let run: &str = dir.path().to_str().unwrap();
    let output: Output = da_state(dir.path(), &["check", "--run", run, "build"], None);
    assert_eq!(exit_code(&output), 4, "{output:?}");
    assert!(!dir.path().join("events.jsonl").exists());
}

// Scenario: many checks append — the journal is a log, never a rewrite.
#[test]
fn two_checks_append_two_entries() {
    let dir: tempfile::TempDir = tempfile::TempDir::new().unwrap();
    scaffold(dir.path());
    let run: &str = dir.path().to_str().unwrap();
    let first: Output = da_state(dir.path(), &["check", "--run", run, "plan"], None);
    let second: Output = da_state(dir.path(), &["check", "--run", run, "plan"], None);
    assert_eq!(exit_code(&first), 0, "{first:?}");
    assert_eq!(exit_code(&second), 0, "{second:?}");
    let journal: String = fs::read_to_string(dir.path().join("events.jsonl")).unwrap();
    assert_eq!(journal.lines().count(), 2, "{journal}");
}

#[test]
fn check_ordering_violation_is_exit_4() {
    let dir: tempfile::TempDir = tempfile::TempDir::new().unwrap();
    scaffold(dir.path());
    let run: &str = dir.path().to_str().unwrap();
    let output: Output = da_state(dir.path(), &["check", "--run", run, "build"], None);
    assert_eq!(exit_code(&output), 4, "{output:?}");
    let stdout: String = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("build-before-plan"), "{stdout}");
}

#[test]
fn check_pending_steer_is_exit_3() {
    let dir: tempfile::TempDir = tempfile::TempDir::new().unwrap();
    scaffold(dir.path());
    fs::write(
        dir.path().join("stages/02-build/output/STEER-REQUEST.md"),
        "## Question\n\nq?\n\n## Answer\n\n",
    )
    .unwrap();
    let run: &str = dir.path().to_str().unwrap();
    let output: Output = da_state(dir.path(), &["check", "--run", run, "plan"], None);
    assert_eq!(exit_code(&output), 3, "{output:?}");
}

#[test]
fn check_unknown_dispatch_is_exit_2_listing_kinds() {
    let dir: tempfile::TempDir = tempfile::TempDir::new().unwrap();
    scaffold(dir.path());
    let run: &str = dir.path().to_str().unwrap();
    let output: Output = da_state(dir.path(), &["check", "--run", run, "warp-speed"], None);
    assert_eq!(exit_code(&output), 2, "{output:?}");
    let stdout: String = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"kinds\""), "{stdout}");
    assert!(stdout.contains("plan"), "{stdout}");
}

#[test]
fn check_broken_run_dir_is_exit_2() {
    let dir: tempfile::TempDir = tempfile::TempDir::new().unwrap();
    let run: &str = dir.path().to_str().unwrap();
    let output: Output = da_state(dir.path(), &["check", "--run", run, "plan"], None);
    assert_eq!(exit_code(&output), 2, "{output:?}");
}

#[test]
fn notify_without_ingress_is_a_clean_no_op() {
    let dir: tempfile::TempDir = tempfile::TempDir::new().unwrap();
    scaffold(dir.path());
    let run: &str = dir.path().to_str().unwrap();
    let output: Output = da_state(dir.path(), &["notify", "--run", run], None);
    assert_eq!(exit_code(&output), 0, "{output:?}");
    let stdout: String = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"published\":false"), "{stdout}");
}

#[test]
fn notify_with_unreachable_ingress_is_exit_1() {
    let dir: tempfile::TempDir = tempfile::TempDir::new().unwrap();
    scaffold(dir.path());
    let run: &str = dir.path().to_str().unwrap();
    let output: Output = da_state(
        dir.path(),
        &["notify", "--run", run],
        Some("http://127.0.0.1:9"), // discard port: nothing listens
    );
    assert_eq!(exit_code(&output), 1, "{output:?}");
    let stdout: String = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"published\":false"), "{stdout}");
}

#[test]
fn restore_refuses_to_overwrite_an_existing_run() {
    let dir: tempfile::TempDir = tempfile::TempDir::new().unwrap();
    scaffold(dir.path());
    let into: &str = dir.path().to_str().unwrap();
    let output: Output = da_state(
        dir.path(),
        &["restore", "--run-id", "cli-test", "--into", into],
        Some("http://127.0.0.1:9"),
    );
    assert_eq!(exit_code(&output), 2, "{output:?}");
    let stdout: String = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("refusing to overwrite"), "{stdout}");
}

#[test]
fn restore_without_ingress_is_a_usage_error() {
    let dir: tempfile::TempDir = tempfile::TempDir::new().unwrap();
    let into_path: std::path::PathBuf = dir.path().join("fresh");
    let into: &str = into_path.to_str().unwrap();
    let output: Output = da_state(dir.path(), &["restore", "--run-id", "x", "--into", into], None);
    assert_eq!(exit_code(&output), 2, "{output:?}");
}
