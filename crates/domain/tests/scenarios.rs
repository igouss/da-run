//! The Gherkin scenarios, one complexity-1 test each.
//! Given = a facts fixture, When = derive/check, Then = one assertion group.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{
    dispatch_ref, fresh_facts, implemented_facts, test_flow, with_drifted_worktree,
    with_empty_worktree, with_gated_worktree, with_output, with_steer,
};
use da_domain::{Anomaly, Flow, FsFacts, Refusal, RunState, Verdict, Warning, check, derive};

// Scenario: a fresh run holds only the spec
#[test]
fn fresh_run_is_specced() {
    let flow: Flow = test_flow();
    let facts: FsFacts = fresh_facts(&flow);
    assert_eq!(
        derive(&flow, &facts).state,
        RunState::Pending {
            label: "specced".to_string()
        }
    );
}

#[test]
fn fresh_run_refuses_tests() {
    let flow: Flow = test_flow();
    let facts: FsFacts = fresh_facts(&flow);
    assert_eq!(
        check(&flow, &facts, dispatch_ref(&flow, "tests")),
        Err(Refusal::OrderingViolation {
            code: "tests-before-design".to_string(),
            detail: "tests before stages/01-design/output/ has a design".to_string(),
        })
    );
}

#[test]
fn fresh_run_refuses_implement() {
    let flow: Flow = test_flow();
    let facts: FsFacts = fresh_facts(&flow);
    assert_eq!(
        check(&flow, &facts, dispatch_ref(&flow, "implement")),
        Err(Refusal::OrderingViolation {
            code: "implement-before-tests".to_string(),
            detail: "implement before stages/02-tests/output/ has a test plan".to_string(),
        })
    );
}

#[test]
fn fresh_run_warns_on_design_review() {
    let flow: Flow = test_flow();
    let facts: FsFacts = fresh_facts(&flow);
    let warnings: Vec<Warning> = check(&flow, &facts, dispatch_ref(&flow, "design-review"))
        .unwrap()
        .warnings;
    assert_eq!(
        warnings,
        vec![Warning::Advisory {
            code: "design-review-without-design".to_string()
        }]
    );
}

// Scenario: one design file unlocks tests
#[test]
fn one_design_file_is_designed() {
    let flow: Flow = test_flow();
    let facts: FsFacts = with_output(&flow, &fresh_facts(&flow), "design", "design.md");
    assert_eq!(
        derive(&flow, &facts).state,
        RunState::HandoffDone {
            label: "designed".to_string(),
            rank: 1
        }
    );
}

#[test]
fn one_design_file_allows_tests() {
    let flow: Flow = test_flow();
    let facts: FsFacts = with_output(&flow, &fresh_facts(&flow), "design", "design.md");
    assert!(check(&flow, &facts, dispatch_ref(&flow, "tests")).is_ok());
}

// Scenario: a green gate allows commit
#[test]
fn green_gate_is_gated_green() {
    let flow: Flow = test_flow();
    let mut facts: FsFacts = implemented_facts(&flow);
    facts.gate = Some(Verdict::Green);
    assert_eq!(derive(&flow, &facts).state, RunState::Gated(Verdict::Green));
}

#[test]
fn green_gate_on_the_verified_worktree_allows_commit() {
    let flow: Flow = test_flow();
    let mut facts: FsFacts = implemented_facts(&flow);
    facts.gate = Some(Verdict::Green);
    let facts: FsFacts = with_gated_worktree(&facts, "wt-verified");
    assert!(check(&flow, &facts, dispatch_ref(&flow, "commit")).is_ok());
}

// Scenario: a green gate over a worktree that has since moved refuses commit.
// This is the restore hazard — the gate report travels with the run, so its
// verdict outlives the code it described.
#[test]
fn green_gate_on_a_moved_worktree_refuses_commit() {
    let flow: Flow = test_flow();
    let mut facts: FsFacts = implemented_facts(&flow);
    facts.gate = Some(Verdict::Green);
    let facts: FsFacts = with_drifted_worktree(&facts, "wt-verified", "wt-now");
    assert!(matches!(
        check(&flow, &facts, dispatch_ref(&flow, "commit")),
        Err(Refusal::WorktreeMovedSinceGate { .. })
    ));
}

// Scenario: a run restored without its code carries a green gate but an empty
// worktree — the false green that would otherwise ship an empty commit.
#[test]
fn green_gate_over_an_empty_worktree_refuses_commit() {
    let flow: Flow = test_flow();
    let mut facts: FsFacts = implemented_facts(&flow);
    facts.gate = Some(Verdict::Green);
    let facts: FsFacts = with_empty_worktree(&facts, "wt-empty");
    assert!(matches!(
        check(&flow, &facts, dispatch_ref(&flow, "commit")),
        Err(Refusal::WorktreeEmpty)
    ));
}

// Scenario: no patch at all means the run dir cannot say what code it holds.
#[test]
fn green_gate_without_a_worktree_patch_refuses_commit() {
    let flow: Flow = test_flow();
    let mut facts: FsFacts = implemented_facts(&flow);
    facts.gate = Some(Verdict::Green);
    assert!(matches!(
        check(&flow, &facts, dispatch_ref(&flow, "commit")),
        Err(Refusal::WorktreeAbsent)
    ));
}

// Scenario: a red gate refuses commit but allows rework
#[test]
fn red_gate_refuses_commit_with_typed_reason() {
    let flow: Flow = test_flow();
    let mut facts: FsFacts = implemented_facts(&flow);
    facts.gate = Some(Verdict::Red);
    assert_eq!(
        check(&flow, &facts, dispatch_ref(&flow, "commit")),
        Err(Refusal::CommitBeforeGreenGate {
            gate: Some(Verdict::Red),
            gate_report: "stages/04-verify/output/gate-report.md".to_string(),
        })
    );
}

#[test]
fn red_gate_allows_implement_rework_with_warning() {
    let flow: Flow = test_flow();
    let mut facts: FsFacts = implemented_facts(&flow);
    facts.gate = Some(Verdict::Red);
    let warnings: Vec<Warning> = check(&flow, &facts, dispatch_ref(&flow, "implement"))
        .unwrap()
        .warnings;
    assert!(warnings.contains(&Warning::RedGateRework));
}

// Scenario: an absent gate report fails closed
#[test]
fn absent_gate_report_refuses_commit() {
    let flow: Flow = test_flow();
    let facts: FsFacts = implemented_facts(&flow);
    assert_eq!(
        check(&flow, &facts, dispatch_ref(&flow, "commit")),
        Err(Refusal::CommitBeforeGreenGate {
            gate: None,
            gate_report: "stages/04-verify/output/gate-report.md".to_string(),
        })
    );
}

// Scenario: an unanswered steer parks the run
#[test]
fn unanswered_steer_parks_the_stage() {
    let flow: Flow = test_flow();
    let facts: FsFacts = with_steer(&flow, &fresh_facts(&flow), "tests", false);
    assert_eq!(derive(&flow, &facts).parked, vec!["02-tests".to_string()]);
}

#[test]
fn unanswered_steer_refuses_design_too() {
    let flow: Flow = test_flow();
    let facts: FsFacts = with_steer(&flow, &fresh_facts(&flow), "tests", false);
    assert_eq!(
        check(&flow, &facts, dispatch_ref(&flow, "design")),
        Err(Refusal::SteerPending {
            stages: vec!["02-tests".to_string()]
        })
    );
}

#[test]
fn answered_steer_clears_the_park() {
    let flow: Flow = test_flow();
    let facts: FsFacts = with_steer(&flow, &fresh_facts(&flow), "tests", true);
    assert_eq!(derive(&flow, &facts).parked, Vec::<String>::new());
}

// Scenario: zero, one, many pending steers (two counts as many)
#[test]
fn zero_steers_parks_nothing() {
    let flow: Flow = test_flow();
    let facts: FsFacts = fresh_facts(&flow);
    assert_eq!(derive(&flow, &facts).parked, Vec::<String>::new());
}

#[test]
fn many_steers_park_in_pipeline_order() {
    let flow: Flow = test_flow();
    let one: FsFacts = with_steer(&flow, &fresh_facts(&flow), "implement", false);
    let many: FsFacts = with_steer(&flow, &one, "design", false);
    assert_eq!(
        derive(&flow, &many).parked,
        vec!["01-design".to_string(), "03-implement".to_string()]
    );
}

// Scenario: implementation without tests is an anomaly and still refused
#[test]
fn implement_output_without_tests_is_an_anomaly() {
    let flow: Flow = test_flow();
    let facts: FsFacts = with_output(&flow, &fresh_facts(&flow), "implement", "notes.md");
    assert_eq!(
        derive(&flow, &facts).anomalies,
        vec![Anomaly::LaterOutputWithoutEarlier {
            later: "03-implement".to_string(),
            earlier: "02-tests".to_string(),
        }]
    );
}

#[test]
fn implement_output_without_tests_still_refuses_implement() {
    let flow: Flow = test_flow();
    let facts: FsFacts = with_output(&flow, &fresh_facts(&flow), "implement", "notes.md");
    assert_eq!(
        check(&flow, &facts, dispatch_ref(&flow, "implement")),
        Err(Refusal::OrderingViolation {
            code: "implement-before-tests".to_string(),
            detail: "implement before stages/02-tests/output/ has a test plan".to_string(),
        })
    );
}

// Scenario: a commit record completes the run
#[test]
fn commit_record_is_committed() {
    let flow: Flow = test_flow();
    let mut facts: FsFacts = implemented_facts(&flow);
    facts.commit_recorded = true;
    assert_eq!(derive(&flow, &facts).state, RunState::Committed);
}

// Scenario: steady-state re-dispatch warns, never refuses
#[test]
fn steady_state_redispatch_of_complete_stage_warns() {
    let flow: Flow = test_flow();
    let facts: FsFacts = with_output(&flow, &fresh_facts(&flow), "design", "design.md");
    let warnings: Vec<Warning> = check(&flow, &facts, dispatch_ref(&flow, "design"))
        .unwrap()
        .warnings;
    assert_eq!(
        warnings,
        vec![Warning::StageAlreadyComplete {
            stage: "01-design".to_string()
        }]
    );
}

// Scenario: verify may always run, warning over an empty implementation
#[test]
fn verify_on_empty_implementation_warns() {
    let flow: Flow = test_flow();
    let facts: FsFacts = fresh_facts(&flow);
    let warnings: Vec<Warning> = check(&flow, &facts, dispatch_ref(&flow, "verify"))
        .unwrap()
        .warnings;
    assert_eq!(
        warnings,
        vec![Warning::Advisory {
            code: "verify-without-implementation".to_string()
        }]
    );
}
