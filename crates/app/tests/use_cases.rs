//! Use-case tests over fake ports — deterministic, no filesystem.

#![allow(clippy::unwrap_used)]

use da_app::check_dispatch;
use da_app::{Decision, StatusReport, publish_mirror, status};
use da_domain::{
    Derived, Dispatch, FsFacts, Phase, Refusal, RunId, RunState, StageFacts, StageFactsMap,
    StageId, Verdict,
};
use da_ports::{MirrorError, RunMirror, SnapshotError, SnapshotSource};
use std::cell::RefCell;
use std::path::{Path, PathBuf};

struct FakeSnapshot {
    result: Result<FsFacts, SnapshotError>,
}

impl SnapshotSource for FakeSnapshot {
    fn snapshot(&self, _run_dir: &Path) -> Result<FsFacts, SnapshotError> {
        self.result.clone()
    }
}

struct RecordingMirror {
    published: RefCell<Vec<(RunId, Derived)>>,
}

impl RunMirror for RecordingMirror {
    fn publish(&self, run_id: &RunId, derived: &Derived) -> Result<(), MirrorError> {
        self.published
            .borrow_mut()
            .push((run_id.clone(), derived.clone()));
        Ok(())
    }
}

fn facts_with_design() -> FsFacts {
    FsFacts {
        stages: StageFactsMap::from_fn(|id: StageId| {
            if id == StageId::Design {
                StageFacts {
                    output_files: vec!["design.md".to_string()],
                    steer: None,
                }
            } else {
                StageFacts::empty()
            }
        }),
        gate: None,
        commit_recorded: false,
        phase: Phase::SteadyState,
        run_id: RunId::new("use-case-run").unwrap(),
    }
}

// Scenario: status reflects the snapshot
#[test]
fn status_marks_the_designed_stage_complete() {
    let source: FakeSnapshot = FakeSnapshot {
        result: Ok(facts_with_design()),
    };
    let report: StatusReport = status(&source, Path::new("unused")).unwrap();
    assert!(report.stages[0].complete);
    assert_eq!(report.derived.state, RunState::Designed);
}

#[test]
fn status_marks_empty_stages_pending() {
    let source: FakeSnapshot = FakeSnapshot {
        result: Ok(facts_with_design()),
    };
    let report: StatusReport = status(&source, Path::new("unused")).unwrap();
    assert!(!report.stages[1].complete);
}

// Scenario: check relays the domain decision as a value
#[test]
fn check_dispatch_allows_tests_after_design() {
    let source: FakeSnapshot = FakeSnapshot {
        result: Ok(facts_with_design()),
    };
    let decision: Decision =
        check_dispatch(&source, Path::new("unused"), &Dispatch::Tests).unwrap();
    assert!(matches!(decision, Decision::Allowed(_)));
}

#[test]
fn check_dispatch_relays_a_refusal_as_a_value() {
    let source: FakeSnapshot = FakeSnapshot {
        result: Ok(facts_with_design()),
    };
    let decision: Decision =
        check_dispatch(&source, Path::new("unused"), &Dispatch::Commit).unwrap();
    assert_eq!(
        decision,
        Decision::Refused(Refusal::CommitBeforeGreenGate { gate: None })
    );
}

// Scenario: a broken run dir surfaces the snapshot error
#[test]
fn check_dispatch_surfaces_snapshot_errors() {
    let source: FakeSnapshot = FakeSnapshot {
        result: Err(SnapshotError::NotARunDir {
            path: PathBuf::from("nowhere"),
        }),
    };
    let error: SnapshotError =
        check_dispatch(&source, Path::new("nowhere"), &Dispatch::Design).unwrap_err();
    assert!(matches!(error, SnapshotError::NotARunDir { .. }));
}

// Scenario: the mirror receives exactly the derived state
#[test]
fn publish_mirror_sends_run_id_and_derived_state() {
    let source: FakeSnapshot = FakeSnapshot {
        result: Ok(facts_with_design()),
    };
    let mirror: RecordingMirror = RecordingMirror {
        published: RefCell::new(Vec::new()),
    };
    let derived: Derived = publish_mirror(&source, &mirror, Path::new("unused")).unwrap();
    let published: Vec<(RunId, Derived)> = mirror.published.into_inner();
    assert_eq!(
        published,
        vec![(RunId::new("use-case-run").unwrap(), derived)]
    );
}

// Scenario: a gated run reports its verdict through status
#[test]
fn status_reports_the_gate_verdict() {
    let mut facts: FsFacts = facts_with_design();
    facts.gate = Some(Verdict::Red);
    let source: FakeSnapshot = FakeSnapshot { result: Ok(facts) };
    let report: StatusReport = status(&source, Path::new("unused")).unwrap();
    assert_eq!(report.derived.state, RunState::Gated(Verdict::Red));
}
