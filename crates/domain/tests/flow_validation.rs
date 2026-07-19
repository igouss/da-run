//! Load-time validation of a flow spec: every invariant refused with a typed
//! error, the canonical fixture accepted.

#![allow(clippy::unwrap_used)]

mod common;

use common::{advise, block, dispatch, stage, test_spec};
use da_domain::{Flow, FlowError, FlowSpec, RoleSpec, StageSpec};

fn spec_with(mutate: impl FnOnce(&mut FlowSpec)) -> FlowSpec {
    let mut spec: FlowSpec = test_spec();
    mutate(&mut spec);
    spec
}

// Scenario: the canonical spec validates
#[test]
fn canonical_spec_is_valid() {
    assert!(Flow::from_spec(test_spec()).is_ok());
}

// Scenario: zero stages
#[test]
fn zero_stages_is_refused() {
    let spec: FlowSpec = spec_with(|spec: &mut FlowSpec| spec.stages.clear());
    assert_eq!(Flow::from_spec(spec), Err(FlowError::Empty));
}

// Scenario: blank identities
#[test]
fn blank_initial_label_is_refused() {
    let spec: FlowSpec = spec_with(|spec: &mut FlowSpec| spec.initial_label = "  ".to_string());
    assert_eq!(Flow::from_spec(spec), Err(FlowError::BlankInitialLabel));
}

#[test]
fn blank_stage_name_is_refused() {
    let spec: FlowSpec = spec_with(|spec: &mut FlowSpec| spec.stages[0].name = "".to_string());
    assert_eq!(
        Flow::from_spec(spec),
        Err(FlowError::BlankStageName { index: 0 })
    );
}

// Scenario: duplicate identities
#[test]
fn duplicate_stage_name_is_refused() {
    let spec: FlowSpec =
        spec_with(|spec: &mut FlowSpec| spec.stages[1].name = "design".to_string());
    assert_eq!(
        Flow::from_spec(spec),
        Err(FlowError::DuplicateStageName {
            name: "design".to_string()
        })
    );
}

#[test]
fn duplicate_dir_is_refused() {
    let spec: FlowSpec = spec_with(|spec: &mut FlowSpec| {
        spec.stages[1].dir = "01-design".to_string();
    });
    assert!(matches!(
        Flow::from_spec(spec),
        Err(FlowError::BadDirPrefix { .. } | FlowError::DuplicateDir { .. })
    ));
}

// Scenario: dirs must carry their position
#[test]
fn out_of_order_dir_prefix_is_refused() {
    let spec: FlowSpec =
        spec_with(|spec: &mut FlowSpec| spec.stages[0].dir = "02-design".to_string());
    assert_eq!(
        Flow::from_spec(spec),
        Err(FlowError::BadDirPrefix {
            name: "design".to_string(),
            dir: "02-design".to_string(),
            expected: "01-".to_string(),
        })
    );
}

#[test]
fn bare_numeric_dir_is_refused() {
    let spec: FlowSpec = spec_with(|spec: &mut FlowSpec| spec.stages[0].dir = "01-".to_string());
    assert!(matches!(
        Flow::from_spec(spec),
        Err(FlowError::BadDirPrefix { .. })
    ));
}

// Scenario: run-state labels stay unique
#[test]
fn duplicate_done_label_is_refused() {
    let spec: FlowSpec = spec_with(|spec: &mut FlowSpec| {
        spec.stages[1].role = RoleSpec::Handoff {
            done_label: "designed".to_string(),
        };
    });
    assert_eq!(
        Flow::from_spec(spec),
        Err(FlowError::DuplicateLabel {
            label: "designed".to_string()
        })
    );
}

#[test]
fn done_label_clashing_with_initial_is_refused() {
    let spec: FlowSpec = spec_with(|spec: &mut FlowSpec| {
        spec.stages[0].role = RoleSpec::Handoff {
            done_label: "specced".to_string(),
        };
    });
    assert_eq!(
        Flow::from_spec(spec),
        Err(FlowError::DuplicateLabel {
            label: "specced".to_string()
        })
    );
}

#[test]
fn blank_done_label_is_refused() {
    let spec: FlowSpec = spec_with(|spec: &mut FlowSpec| {
        spec.stages[0].role = RoleSpec::Handoff {
            done_label: " ".to_string(),
        };
    });
    assert_eq!(
        Flow::from_spec(spec),
        Err(FlowError::BlankDoneLabel {
            name: "design".to_string()
        })
    );
}

// Scenario: dispatches exist and stay unique
#[test]
fn stage_without_dispatches_is_refused() {
    let spec: FlowSpec = spec_with(|spec: &mut FlowSpec| spec.stages[0].dispatches.clear());
    assert_eq!(
        Flow::from_spec(spec),
        Err(FlowError::NoDispatches {
            name: "design".to_string()
        })
    );
}

#[test]
fn duplicate_dispatch_kind_across_stages_is_refused() {
    let spec: FlowSpec = spec_with(|spec: &mut FlowSpec| {
        spec.stages[1].dispatches[0].kind = "design".to_string();
    });
    assert_eq!(
        Flow::from_spec(spec),
        Err(FlowError::DuplicateDispatchKind {
            kind: "design".to_string()
        })
    );
}

#[test]
fn blank_dispatch_kind_is_refused() {
    let spec: FlowSpec = spec_with(|spec: &mut FlowSpec| {
        spec.stages[0].dispatches[0].kind = "".to_string();
    });
    assert_eq!(
        Flow::from_spec(spec),
        Err(FlowError::BlankDispatchKind {
            name: "design".to_string()
        })
    );
}

// Scenario: rules resolve to real, correctly ordered stages
#[test]
fn rule_naming_an_unknown_stage_is_refused() {
    let spec: FlowSpec = spec_with(|spec: &mut FlowSpec| {
        spec.stages[1].dispatches[0].blocking = vec![block("warp", "c", "d")];
    });
    assert_eq!(
        Flow::from_spec(spec),
        Err(FlowError::UnknownRuleStage {
            kind: "tests".to_string(),
            stage: "warp".to_string()
        })
    );
}

#[test]
fn blocking_rule_on_own_stage_is_refused() {
    let spec: FlowSpec = spec_with(|spec: &mut FlowSpec| {
        spec.stages[1].dispatches[0].blocking = vec![block("tests", "c", "d")];
    });
    assert_eq!(
        Flow::from_spec(spec),
        Err(FlowError::BlockRuleNotEarlier {
            kind: "tests".to_string(),
            stage: "tests".to_string()
        })
    );
}

#[test]
fn blocking_rule_on_later_stage_is_refused() {
    let spec: FlowSpec = spec_with(|spec: &mut FlowSpec| {
        spec.stages[1].dispatches[0].blocking = vec![block("commit", "c", "d")];
    });
    assert_eq!(
        Flow::from_spec(spec),
        Err(FlowError::BlockRuleNotEarlier {
            kind: "tests".to_string(),
            stage: "commit".to_string()
        })
    );
}

#[test]
fn advisory_rule_on_own_stage_is_allowed() {
    let spec: FlowSpec = spec_with(|spec: &mut FlowSpec| {
        spec.stages[0].dispatches[1].advisory = vec![advise("design", "still-fine")];
    });
    assert!(Flow::from_spec(spec).is_ok());
}

#[test]
fn advisory_rule_on_later_stage_is_refused() {
    let spec: FlowSpec = spec_with(|spec: &mut FlowSpec| {
        spec.stages[0].dispatches[1].advisory = vec![advise("commit", "c")];
    });
    assert_eq!(
        Flow::from_spec(spec),
        Err(FlowError::AdviseRuleLater {
            kind: "design-review".to_string(),
            stage: "commit".to_string()
        })
    );
}

#[test]
fn blank_rule_code_is_refused() {
    let spec: FlowSpec = spec_with(|spec: &mut FlowSpec| {
        spec.stages[1].dispatches[0].blocking = vec![block("design", " ", "d")];
    });
    assert!(matches!(
        Flow::from_spec(spec),
        Err(FlowError::BlankRuleCode { .. })
    ));
}

#[test]
fn blank_rule_detail_is_refused() {
    let spec: FlowSpec = spec_with(|spec: &mut FlowSpec| {
        spec.stages[1].dispatches[0].blocking = vec![block("design", "c", " ")];
    });
    assert!(matches!(
        Flow::from_spec(spec),
        Err(FlowError::BlankRuleDetail { .. })
    ));
}

// Scenario: exactly one gate, exactly one commit (zero, one, many)
#[test]
fn zero_gates_is_refused() {
    let spec: FlowSpec = spec_with(|spec: &mut FlowSpec| {
        spec.stages[3].role = RoleSpec::Handoff {
            done_label: "verified".to_string(),
        };
    });
    assert_eq!(
        Flow::from_spec(spec),
        Err(FlowError::GateCount { count: 0 })
    );
}

#[test]
fn many_gates_are_refused() {
    let spec: FlowSpec = spec_with(|spec: &mut FlowSpec| {
        spec.stages[2].role = RoleSpec::Gate;
    });
    assert_eq!(
        Flow::from_spec(spec),
        Err(FlowError::GateCount { count: 2 })
    );
}

#[test]
fn zero_commits_is_refused() {
    let spec: FlowSpec = spec_with(|spec: &mut FlowSpec| {
        spec.stages[4].role = RoleSpec::Handoff {
            done_label: "wrapped".to_string(),
        };
    });
    assert_eq!(
        Flow::from_spec(spec),
        Err(FlowError::CommitCount { count: 0 })
    );
}

#[test]
fn many_commits_are_refused() {
    let spec: FlowSpec = spec_with(|spec: &mut FlowSpec| {
        spec.stages[2].role = RoleSpec::Commit;
    });
    assert_eq!(
        Flow::from_spec(spec),
        Err(FlowError::CommitCount { count: 2 })
    );
}

#[test]
fn commit_not_last_is_refused() {
    let spec: FlowSpec = spec_with(|spec: &mut FlowSpec| {
        spec.stages[2].role = RoleSpec::Commit;
        spec.stages[4].role = RoleSpec::Handoff {
            done_label: "wrapped".to_string(),
        };
    });
    assert_eq!(
        Flow::from_spec(spec),
        Err(FlowError::CommitNotLast {
            name: "implement".to_string()
        })
    );
}

// Scenario: the pipeline still needs handoffs and a gate artifact
#[test]
fn no_handoffs_is_refused() {
    let stages: Vec<StageSpec> = vec![
        stage(
            "verify",
            "01-verify",
            RoleSpec::Gate,
            vec![dispatch("verify", vec![], vec![], false)],
        ),
        stage(
            "commit",
            "02-commit",
            RoleSpec::Commit,
            vec![dispatch("commit", vec![], vec![], false)],
        ),
    ];
    let mut spec: FlowSpec = test_spec();
    spec.stages = stages;
    spec.stages[0].artifact = Some("gate-report.md".to_string());
    assert_eq!(Flow::from_spec(spec), Err(FlowError::NoHandoffs));
}

#[test]
fn gate_without_artifact_is_refused() {
    let spec: FlowSpec = spec_with(|spec: &mut FlowSpec| spec.stages[3].artifact = None);
    assert_eq!(
        Flow::from_spec(spec),
        Err(FlowError::MissingGateArtifact {
            name: "verify".to_string()
        })
    );
}
