#![allow(clippy::unwrap_used)]
#![allow(dead_code)]

use da_domain::{
    AdviseRuleSpec, BlockRuleSpec, DispatchRef, DispatchSpec, Flow, FlowSpec, FsFacts, Phase,
    RoleSpec, RunId, StageFacts, StageFactsMap, StageRef, StageSpec, SteerFacts, Verdict,
    WorktreeFacts, WorktreeId,
};
use proptest::prelude::*;

/// The canonical five-stage flow, mirrored as a code fixture so the domain
/// tests stay filesystem-free.
pub fn test_flow() -> Flow {
    Flow::from_spec(test_spec()).unwrap()
}

pub fn test_spec() -> FlowSpec {
    FlowSpec {
        initial_label: "specced".to_string(),
        stages: vec![
            stage(
                "design",
                "01-design",
                RoleSpec::Handoff {
                    done_label: "designed".to_string(),
                },
                vec![
                    dispatch("design", vec![], vec![], false),
                    dispatch(
                        "design-review",
                        vec![],
                        vec![advise("design", "design-review-without-design")],
                        false,
                    ),
                ],
            ),
            stage(
                "tests",
                "02-tests",
                RoleSpec::Handoff {
                    done_label: "tested".to_string(),
                },
                vec![dispatch(
                    "tests",
                    vec![block(
                        "design",
                        "tests-before-design",
                        "tests before stages/01-design/output/ has a design",
                    )],
                    vec![],
                    false,
                )],
            ),
            stage(
                "implement",
                "03-implement",
                RoleSpec::Handoff {
                    done_label: "implemented".to_string(),
                },
                vec![dispatch(
                    "implement",
                    vec![block(
                        "tests",
                        "implement-before-tests",
                        "implement before stages/02-tests/output/ has a test plan",
                    )],
                    vec![],
                    true,
                )],
            ),
            stage(
                "verify",
                "04-verify",
                RoleSpec::Gate,
                vec![dispatch(
                    "verify",
                    vec![],
                    vec![advise("implement", "verify-without-implementation")],
                    false,
                )],
            ),
            stage(
                "commit",
                "05-commit",
                RoleSpec::Commit,
                vec![dispatch("commit", vec![], vec![], false)],
            ),
        ],
    }
}

pub fn stage(name: &str, dir: &str, role: RoleSpec, dispatches: Vec<DispatchSpec>) -> StageSpec {
    StageSpec {
        name: name.to_string(),
        dir: dir.to_string(),
        role,
        artifact: if name == "verify" {
            Some("gate-report.md".to_string())
        } else {
            None
        },
        dispatches,
    }
}

pub fn dispatch(
    kind: &str,
    blocking: Vec<BlockRuleSpec>,
    advisory: Vec<AdviseRuleSpec>,
    warn_on_red_gate: bool,
) -> DispatchSpec {
    DispatchSpec {
        kind: kind.to_string(),
        blocking,
        advisory,
        warn_on_red_gate,
        model: None,
        strategy: None,
        effort: None,
        design_from: None,
        tests_from: None,
        judge_reference: None,
    }
}

pub fn block(stage: &str, code: &str, detail: &str) -> BlockRuleSpec {
    BlockRuleSpec {
        stage: stage.to_string(),
        code: code.to_string(),
        detail: detail.to_string(),
    }
}

pub fn advise(stage: &str, code: &str) -> AdviseRuleSpec {
    AdviseRuleSpec {
        stage: stage.to_string(),
        code: code.to_string(),
    }
}

pub fn stage_ref(flow: &Flow, name: &str) -> StageRef {
    flow.stage_named(name).unwrap()
}

pub fn dispatch_ref(flow: &Flow, kind: &str) -> DispatchRef {
    flow.resolve_dispatch(kind).unwrap()
}

/// A fresh run: spec only, nothing produced, steady-state.
pub fn fresh_facts(flow: &Flow) -> FsFacts {
    FsFacts {
        stages: StageFactsMap::from_fn(flow, |_stage: StageRef| StageFacts::empty()),
        gate: None,
        commit_recorded: false,
        worktree: None,
        gate_worktree: None,
        phase: Phase::SteadyState,
        run_id: RunId::new("test-run").unwrap(),
    }
}

/// `base` carrying code the gate has seen — the honest case, where the
/// worktree holds a change and the gate report names that same change.
pub fn with_gated_worktree(base: &FsFacts, id: &str) -> FsFacts {
    let mut facts: FsFacts = base.clone();
    facts.worktree = Some(WorktreeFacts {
        id: WorktreeId::new(id).unwrap(),
        empty: false,
    });
    facts.gate_worktree = Some(WorktreeId::new(id).unwrap());
    facts
}

/// `base` with a worktree the gate never saw — the drift case.
pub fn with_drifted_worktree(base: &FsFacts, verified: &str, current: &str) -> FsFacts {
    let mut facts: FsFacts = with_gated_worktree(base, current);
    facts.gate_worktree = Some(WorktreeId::new(verified).unwrap());
    facts
}

/// `base` whose worktree holds no change at all.
pub fn with_empty_worktree(base: &FsFacts, id: &str) -> FsFacts {
    let mut facts: FsFacts = with_gated_worktree(base, id);
    facts.worktree = Some(WorktreeFacts {
        id: WorktreeId::new(id).unwrap(),
        empty: true,
    });
    facts
}

/// `base` with one output file added to the stage named `name`.
pub fn with_output(flow: &Flow, base: &FsFacts, name: &str, file: &str) -> FsFacts {
    let target: StageRef = stage_ref(flow, name);
    let mut facts: FsFacts = base.clone();
    facts.stages = StageFactsMap::from_fn(flow, |stage: StageRef| {
        let mut stage_facts: StageFacts = base.stages.get(stage).clone();
        if stage == target {
            stage_facts.output_files.push(file.to_string());
        }
        stage_facts
    });
    facts
}

/// `base` with a steer-request at the stage named `name`.
pub fn with_steer(flow: &Flow, base: &FsFacts, name: &str, answered: bool) -> FsFacts {
    let target: StageRef = stage_ref(flow, name);
    let mut facts: FsFacts = base.clone();
    facts.stages = StageFactsMap::from_fn(flow, |stage: StageRef| {
        let mut stage_facts: StageFacts = base.stages.get(stage).clone();
        if stage == target {
            stage_facts.steer = Some(SteerFacts { answered });
        }
        stage_facts
    });
    facts
}

/// A run with design, tests, and implementation outputs present.
pub fn implemented_facts(flow: &Flow) -> FsFacts {
    let designed: FsFacts = with_output(flow, &fresh_facts(flow), "design", "design.md");
    let tested: FsFacts = with_output(flow, &designed, "tests", "test-plan.md");
    with_output(flow, &tested, "implement", "notes.md")
}

pub fn arb_stage_facts() -> impl Strategy<Value = StageFacts> {
    (
        prop::collection::vec("[a-z]{1,8}\\.md", 0..3),
        prop::option::of(any::<bool>()),
    )
        .prop_map(
            |(output_files, steer): (Vec<String>, Option<bool>)| StageFacts {
                output_files,
                steer: steer.map(|answered: bool| SteerFacts { answered }),
            },
        )
}

pub fn arb_verdict() -> impl Strategy<Value = Verdict> {
    prop_oneof![Just(Verdict::Green), Just(Verdict::Red)]
}

pub fn arb_phase() -> impl Strategy<Value = Phase> {
    prop_oneof![Just(Phase::Convergence), Just(Phase::SteadyState)]
}

pub fn arb_facts() -> impl Strategy<Value = FsFacts> {
    let flow: Flow = test_flow();
    (
        prop::collection::vec(arb_stage_facts(), flow.stage_count()),
        prop::option::of(arb_verdict()),
        any::<bool>(),
        arb_phase(),
        prop::option::of(arb_worktree_facts()),
        prop::option::of(arb_worktree_id()),
    )
        .prop_map(
            move |(stages, gate, commit_recorded, phase, worktree, gate_worktree): (
                Vec<StageFacts>,
                Option<Verdict>,
                bool,
                Phase,
                Option<WorktreeFacts>,
                Option<WorktreeId>,
            )| {
                let mut remaining: std::vec::IntoIter<StageFacts> = stages.into_iter();
                FsFacts {
                    stages: StageFactsMap::from_fn(&flow, |_stage: StageRef| {
                        remaining.next().unwrap_or_else(StageFacts::empty)
                    }),
                    gate,
                    commit_recorded,
                    worktree,
                    gate_worktree,
                    phase,
                    run_id: RunId::new("prop-run").unwrap(),
                }
            },
        )
}

/// A small identity alphabet, so drift and agreement both occur often enough
/// to exercise the commit law rather than always disagreeing by chance.
fn arb_worktree_id() -> impl Strategy<Value = WorktreeId> {
    prop::sample::select(vec!["wt-a", "wt-b", "wt-c"])
        .prop_map(|raw: &str| WorktreeId::new(raw).unwrap())
}

fn arb_worktree_facts() -> impl Strategy<Value = WorktreeFacts> {
    (arb_worktree_id(), any::<bool>()).prop_map(|(id, empty): (WorktreeId, bool)| WorktreeFacts {
        id,
        empty,
    })
}
