//! Use-case tests over fake ports — deterministic, no filesystem.

#![allow(clippy::unwrap_used)]

use da_app::check_dispatch;
use da_app::{Decision, Published, StatusReport, publish_mirror, status};
use da_domain::{
    AdviseRuleSpec, BlockRuleSpec, Derived, DispatchSpec, Flow, FlowSpec, FsFacts, Phase, Refusal,
    RoleSpec, RunId, RunState, StageFacts, StageFactsMap, StageRef, StageSpec, Verdict,
};
use da_ports::{
    ArtifactSource, MirrorError, MirrorSnapshot, RunArtifact, RunMirror, SnapshotError,
    SnapshotSource,
};
use std::cell::RefCell;
use std::path::{Path, PathBuf};

/// A minimal four-stage flow: two handoffs, the gate, the commit.
fn test_flow() -> Flow {
    Flow::from_spec(FlowSpec {
        initial_label: "specced".to_string(),
        stages: vec![
            StageSpec {
                name: "design".to_string(),
                dir: "01-design".to_string(),
                role: RoleSpec::Handoff {
                    done_label: "designed".to_string(),
                },
                artifact: None,
                dispatches: vec![plain_dispatch("design", vec![])],
            },
            StageSpec {
                name: "tests".to_string(),
                dir: "02-tests".to_string(),
                role: RoleSpec::Handoff {
                    done_label: "tested".to_string(),
                },
                artifact: None,
                dispatches: vec![plain_dispatch(
                    "tests",
                    vec![BlockRuleSpec {
                        stage: "design".to_string(),
                        code: "tests-before-design".to_string(),
                        detail: "tests before design".to_string(),
                    }],
                )],
            },
            StageSpec {
                name: "verify".to_string(),
                dir: "03-verify".to_string(),
                role: RoleSpec::Gate,
                artifact: Some("gate-report.md".to_string()),
                dispatches: vec![plain_dispatch("verify", vec![])],
            },
            StageSpec {
                name: "commit".to_string(),
                dir: "04-commit".to_string(),
                role: RoleSpec::Commit,
                artifact: None,
                dispatches: vec![plain_dispatch("commit", vec![])],
            },
        ],
    })
    .unwrap()
}

fn plain_dispatch(kind: &str, blocking: Vec<BlockRuleSpec>) -> DispatchSpec {
    DispatchSpec {
        kind: kind.to_string(),
        blocking,
        advisory: Vec::<AdviseRuleSpec>::new(),
        warn_on_red_gate: false,
        model: None,
        strategy: None,
        effort: None,
    }
}

struct FakeSnapshot {
    result: Result<FsFacts, SnapshotError>,
}

impl SnapshotSource for FakeSnapshot {
    fn snapshot(&self, _flow: &Flow, _run_dir: &Path) -> Result<FsFacts, SnapshotError> {
        self.result.clone()
    }
}

struct RecordingMirror {
    published: RefCell<Vec<(RunId, Derived)>>,
    artifacts: RefCell<Vec<Vec<RunArtifact>>>,
}

impl RunMirror for RecordingMirror {
    fn publish(&self, run_id: &RunId, derived: &Derived) -> Result<(), MirrorError> {
        self.published
            .borrow_mut()
            .push((run_id.clone(), derived.clone()));
        Ok(())
    }

    fn publish_artifacts(&self, _run_id: &RunId, files: &[RunArtifact]) -> Result<(), MirrorError> {
        self.artifacts.borrow_mut().push(files.to_vec());
        Ok(())
    }

    fn fetch_snapshot(&self, _run_id: &RunId) -> Result<MirrorSnapshot, MirrorError> {
        Ok(MirrorSnapshot {
            state_json: None,
            files: self.artifacts.borrow().last().cloned().unwrap_or_default(),
        })
    }
}

/// A fixed artifact set, standing in for the fs collector.
struct FixedArtifacts {
    files: Vec<RunArtifact>,
}

impl ArtifactSource for FixedArtifacts {
    fn collect(&self, _flow: &Flow, _run_dir: &Path) -> Result<Vec<RunArtifact>, SnapshotError> {
        Ok(self.files.clone())
    }
}

fn facts_with_design(flow: &Flow) -> FsFacts {
    let design: StageRef = flow.stage_named("design").unwrap();
    FsFacts {
        stages: StageFactsMap::from_fn(flow, |stage: StageRef| {
            if stage == design {
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
        worktree: None,
        gate_worktree: None,
        phase: Phase::SteadyState,
        run_id: RunId::new("use-case-run").unwrap(),
    }
}

// Scenario: status reflects the snapshot
#[test]
fn status_marks_the_designed_stage_complete() {
    let flow: Flow = test_flow();
    let source: FakeSnapshot = FakeSnapshot {
        result: Ok(facts_with_design(&flow)),
    };
    let report: StatusReport = status(&source, &flow, Path::new("unused")).unwrap();
    assert!(report.stages[0].complete);
    assert_eq!(
        report.derived.state,
        RunState::HandoffDone {
            label: "designed".to_string(),
            rank: 1
        }
    );
}

#[test]
fn status_marks_empty_stages_pending() {
    let flow: Flow = test_flow();
    let source: FakeSnapshot = FakeSnapshot {
        result: Ok(facts_with_design(&flow)),
    };
    let report: StatusReport = status(&source, &flow, Path::new("unused")).unwrap();
    assert!(!report.stages[1].complete);
}

// Scenario: check relays the domain decision as a value
#[test]
fn check_dispatch_allows_tests_after_design() {
    let flow: Flow = test_flow();
    let source: FakeSnapshot = FakeSnapshot {
        result: Ok(facts_with_design(&flow)),
    };
    let decision: Decision = check_dispatch(
        &source,
        &flow,
        Path::new("unused"),
        flow.resolve_dispatch("tests").unwrap(),
    )
    .unwrap();
    assert!(matches!(decision, Decision::Allowed(_)));
}

#[test]
fn check_dispatch_relays_a_refusal_as_a_value() {
    let flow: Flow = test_flow();
    let source: FakeSnapshot = FakeSnapshot {
        result: Ok(facts_with_design(&flow)),
    };
    let decision: Decision = check_dispatch(
        &source,
        &flow,
        Path::new("unused"),
        flow.resolve_dispatch("commit").unwrap(),
    )
    .unwrap();
    assert_eq!(
        decision,
        Decision::Refused(Refusal::CommitBeforeGreenGate {
            gate: None,
            gate_report: "stages/03-verify/output/gate-report.md".to_string(),
        })
    );
}

// Scenario: a broken run dir surfaces the snapshot error
#[test]
fn check_dispatch_surfaces_snapshot_errors() {
    let flow: Flow = test_flow();
    let source: FakeSnapshot = FakeSnapshot {
        result: Err(SnapshotError::NotARunDir {
            path: PathBuf::from("nowhere"),
        }),
    };
    let error: SnapshotError = check_dispatch(
        &source,
        &flow,
        Path::new("nowhere"),
        flow.resolve_dispatch("design").unwrap(),
    )
    .unwrap_err();
    assert!(matches!(error, SnapshotError::NotARunDir { .. }));
}

// Scenario: the mirror receives exactly the derived state and the artifacts
#[test]
fn publish_mirror_sends_run_id_and_derived_state() {
    let flow: Flow = test_flow();
    let source: FakeSnapshot = FakeSnapshot {
        result: Ok(facts_with_design(&flow)),
    };
    let artifacts: FixedArtifacts = FixedArtifacts {
        files: vec![RunArtifact {
            path: "run.edn".to_string(),
            content: "{}".to_string(),
        }],
    };
    let mirror: RecordingMirror = RecordingMirror {
        published: RefCell::new(Vec::new()),
        artifacts: RefCell::new(Vec::new()),
    };
    let sent: Published =
        publish_mirror(&source, &artifacts, &mirror, &flow, Path::new("unused")).unwrap();
    assert_eq!(sent.artifact_count, 1);
    assert_eq!(mirror.artifacts.borrow().len(), 1);
    let published: Vec<(RunId, Derived)> = mirror.published.into_inner();
    assert_eq!(published, vec![(sent.run_id, sent.derived)]);
}

// Scenario: a gated run reports its verdict through status
#[test]
fn status_reports_the_gate_verdict() {
    let flow: Flow = test_flow();
    let mut facts: FsFacts = facts_with_design(&flow);
    facts.gate = Some(Verdict::Red);
    let source: FakeSnapshot = FakeSnapshot { result: Ok(facts) };
    let report: StatusReport = status(&source, &flow, Path::new("unused")).unwrap();
    assert_eq!(report.derived.state, RunState::Gated(Verdict::Red));
}
