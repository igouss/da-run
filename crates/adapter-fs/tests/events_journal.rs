//! The events journal against real temp dirs: the fs walk must land on the
//! same fingerprint the pure core (and the babashka selftest) pin.

#![allow(clippy::unwrap_used)]

use da_adapter_fs::{append_event, outputs_fingerprint};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// The shared parity fixture digest — also pinned in `events.rs`'s unit
/// tests and in `bin/run --selftest`.
const PARITY_FINGERPRINT: &str =
    "ec51ef27d4bf77772dcb1f3c68107219e72d4d72eac7420899d54f0064abb0a7";
const EMPTY_FINGERPRINT: &str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

fn scaffold() -> TempDir {
    let dir: TempDir = TempDir::new().unwrap();
    fs::write(dir.path().join("spec.md"), "# spec\n").unwrap();
    let output: PathBuf = dir.path().join("stages/01-plan/output");
    fs::create_dir_all(&output).unwrap();
    fs::write(output.join("plan.md"), "the plan\n").unwrap();
    dir
}

// Scenario: zero fingerprintable files — the empty digest, matching bb
#[test]
fn empty_run_dir_is_the_empty_digest() {
    let dir: TempDir = TempDir::new().unwrap();
    assert_eq!(outputs_fingerprint(dir.path()).unwrap(), EMPTY_FINGERPRINT);
}

// Scenario: the parity fixture through the real fs walk
#[test]
fn fs_walk_matches_the_parity_pin() {
    let dir: TempDir = scaffold();
    assert_eq!(outputs_fingerprint(dir.path()).unwrap(), PARITY_FINGERPRINT);
}

// Scenario: steer files are excluded — answering must not move the surface
#[test]
fn steer_request_does_not_move_the_fingerprint() {
    let dir: TempDir = scaffold();
    fs::write(
        dir.path().join("stages/01-plan/output/STEER-REQUEST.md"),
        "## Question\n\nq?\n",
    )
    .unwrap();
    assert_eq!(outputs_fingerprint(dir.path()).unwrap(), PARITY_FINGERPRINT);
}

// Scenario: hidden files are outside the surface — bb's fs/glob never
// matches them, and every run ships .gitkeep in each output dir. This is
// the parity case a pure-fixture pin cannot catch.
#[test]
fn gitkeep_does_not_move_the_fingerprint() {
    let dir: TempDir = scaffold();
    fs::write(dir.path().join("stages/01-plan/output/.gitkeep"), "").unwrap();
    assert_eq!(outputs_fingerprint(dir.path()).unwrap(), PARITY_FINGERPRINT);
}

// Scenario: files outside output/ (contracts, references) are excluded
#[test]
fn stage_contract_files_do_not_move_the_fingerprint() {
    let dir: TempDir = scaffold();
    fs::write(dir.path().join("stages/01-plan/CONTEXT.md"), "contract").unwrap();
    assert_eq!(outputs_fingerprint(dir.path()).unwrap(), PARITY_FINGERPRINT);
}

// Scenario: one appended event holds ts, trigger, and the fingerprint
#[test]
fn append_event_writes_one_line() {
    let dir: TempDir = scaffold();
    append_event(dir.path(), "dispatch:plan").unwrap();
    let journal: String = fs::read_to_string(dir.path().join("events.jsonl")).unwrap();
    assert_eq!(journal.lines().count(), 1);
    assert!(journal.contains("\"trigger\":\"dispatch:plan\""), "{journal}");
    assert!(journal.contains(PARITY_FINGERPRINT), "{journal}");
    assert!(journal.contains("\"ts\":\""), "{journal}");
}

// Scenario: many events append in order — the journal is a log
#[test]
fn append_event_appends_in_order() {
    let dir: TempDir = scaffold();
    append_event(dir.path(), "dispatch:plan").unwrap();
    append_event(dir.path(), "dispatch:build").unwrap();
    let journal: String = fs::read_to_string(dir.path().join("events.jsonl")).unwrap();
    let triggers: Vec<bool> = journal
        .lines()
        .map(|line: &str| line.contains("dispatch:"))
        .collect();
    assert_eq!(triggers, vec![true, true]);
    assert!(journal.lines().next().unwrap().contains("dispatch:plan"));
}
