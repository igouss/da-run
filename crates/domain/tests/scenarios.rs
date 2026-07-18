//! The Gherkin scenarios, one complexity-1 test each.
//! Given = a facts fixture, When = derive/check, Then = one assertion group.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{fresh_facts, implemented_facts, with_output, with_steer};
use da_domain::{
    Anomaly, Dispatch, FsFacts, Refusal, RunState, StageId, Verdict, Warning, check, derive,
};

// Scenario: a fresh run holds only the spec
#[test]
fn fresh_run_is_specced() {
    let facts: FsFacts = fresh_facts();
    assert_eq!(derive(&facts).state, RunState::Specced);
}

#[test]
fn fresh_run_refuses_tests() {
    let facts: FsFacts = fresh_facts();
    assert_eq!(
        check(&facts, &Dispatch::Tests),
        Err(Refusal::TestsBeforeDesign)
    );
}

#[test]
fn fresh_run_refuses_implement() {
    let facts: FsFacts = fresh_facts();
    assert_eq!(
        check(
            &facts,
            &Dispatch::Implement {
                parallel_attempts: None
            }
        ),
        Err(Refusal::ImplementBeforeTests)
    );
}

#[test]
fn fresh_run_warns_on_design_review() {
    let facts: FsFacts = fresh_facts();
    let warnings: Vec<Warning> = check(&facts, &Dispatch::DesignReview).unwrap().warnings;
    assert_eq!(warnings, vec![Warning::DesignReviewWithoutDesign]);
}

// Scenario: one design file unlocks tests
#[test]
fn one_design_file_is_designed() {
    let facts: FsFacts = with_output(&fresh_facts(), StageId::Design, "design.md");
    assert_eq!(derive(&facts).state, RunState::Designed);
}

#[test]
fn one_design_file_allows_tests() {
    let facts: FsFacts = with_output(&fresh_facts(), StageId::Design, "design.md");
    assert!(check(&facts, &Dispatch::Tests).is_ok());
}

// Scenario: a green gate allows commit
#[test]
fn green_gate_is_gated_green() {
    let mut facts: FsFacts = implemented_facts();
    facts.gate = Some(Verdict::Green);
    assert_eq!(derive(&facts).state, RunState::Gated(Verdict::Green));
}

#[test]
fn green_gate_allows_commit() {
    let mut facts: FsFacts = implemented_facts();
    facts.gate = Some(Verdict::Green);
    assert!(check(&facts, &Dispatch::Commit).is_ok());
}

// Scenario: a red gate refuses commit but allows rework
#[test]
fn red_gate_refuses_commit_with_typed_reason() {
    let mut facts: FsFacts = implemented_facts();
    facts.gate = Some(Verdict::Red);
    assert_eq!(
        check(&facts, &Dispatch::Commit),
        Err(Refusal::CommitBeforeGreenGate {
            gate: Some(Verdict::Red)
        })
    );
}

#[test]
fn red_gate_allows_implement_rework_with_warning() {
    let mut facts: FsFacts = implemented_facts();
    facts.gate = Some(Verdict::Red);
    let warnings: Vec<Warning> = check(
        &facts,
        &Dispatch::Implement {
            parallel_attempts: None,
        },
    )
    .unwrap()
    .warnings;
    assert!(warnings.contains(&Warning::RedGateRework));
}

// Scenario: an absent gate report fails closed
#[test]
fn absent_gate_report_refuses_commit() {
    let facts: FsFacts = implemented_facts();
    assert_eq!(
        check(&facts, &Dispatch::Commit),
        Err(Refusal::CommitBeforeGreenGate { gate: None })
    );
}

// Scenario: an unanswered steer parks the run
#[test]
fn unanswered_steer_parks_the_stage() {
    let facts: FsFacts = with_steer(&fresh_facts(), StageId::Tests, false);
    assert_eq!(derive(&facts).parked, vec![StageId::Tests]);
}

#[test]
fn unanswered_steer_refuses_design_too() {
    let facts: FsFacts = with_steer(&fresh_facts(), StageId::Tests, false);
    assert_eq!(
        check(&facts, &Dispatch::Design),
        Err(Refusal::SteerPending {
            stages: vec![StageId::Tests]
        })
    );
}

#[test]
fn answered_steer_clears_the_park() {
    let facts: FsFacts = with_steer(&fresh_facts(), StageId::Tests, true);
    assert_eq!(derive(&facts).parked, Vec::<StageId>::new());
}

// Scenario: zero, one, many pending steers (two counts as many)
#[test]
fn zero_steers_parks_nothing() {
    let facts: FsFacts = fresh_facts();
    assert_eq!(derive(&facts).parked, Vec::<StageId>::new());
}

#[test]
fn many_steers_park_in_pipeline_order() {
    let one: FsFacts = with_steer(&fresh_facts(), StageId::Implement, false);
    let many: FsFacts = with_steer(&one, StageId::Design, false);
    assert_eq!(
        derive(&many).parked,
        vec![StageId::Design, StageId::Implement]
    );
}

// Scenario: implementation without tests is an anomaly and still refused
#[test]
fn implement_output_without_tests_is_an_anomaly() {
    let facts: FsFacts = with_output(&fresh_facts(), StageId::Implement, "notes.md");
    assert_eq!(
        derive(&facts).anomalies,
        vec![Anomaly::LaterOutputWithoutEarlier {
            later: StageId::Implement,
            earlier: StageId::Tests
        }]
    );
}

#[test]
fn implement_output_without_tests_still_refuses_implement() {
    let facts: FsFacts = with_output(&fresh_facts(), StageId::Implement, "notes.md");
    assert_eq!(
        check(
            &facts,
            &Dispatch::Implement {
                parallel_attempts: None
            }
        ),
        Err(Refusal::ImplementBeforeTests)
    );
}

// Scenario: a commit record completes the run
#[test]
fn commit_record_is_committed() {
    let mut facts: FsFacts = implemented_facts();
    facts.commit_recorded = true;
    assert_eq!(derive(&facts).state, RunState::Committed);
}

// Scenario: steady-state re-dispatch warns, never refuses
#[test]
fn steady_state_redispatch_of_complete_stage_warns() {
    let facts: FsFacts = with_output(&fresh_facts(), StageId::Design, "design.md");
    let warnings: Vec<Warning> = check(&facts, &Dispatch::Design).unwrap().warnings;
    assert_eq!(
        warnings,
        vec![Warning::StageAlreadyComplete {
            stage: StageId::Design
        }]
    );
}

// Scenario: verify may always run, warning over an empty implementation
#[test]
fn verify_on_empty_implementation_warns() {
    let facts: FsFacts = fresh_facts();
    let warnings: Vec<Warning> = check(&facts, &Dispatch::Verify).unwrap().warnings;
    assert_eq!(warnings, vec![Warning::VerifyWithoutImplementation]);
}
