#![allow(clippy::unwrap_used)]
#![allow(dead_code)]

use da_domain::{FsFacts, Phase, RunId, StageFacts, StageFactsMap, StageId, SteerFacts, Verdict};
use proptest::prelude::*;

/// A fresh run: spec only, nothing produced, steady-state.
pub fn fresh_facts() -> FsFacts {
    FsFacts {
        stages: StageFactsMap::from_fn(|_id: StageId| StageFacts::empty()),
        gate: None,
        commit_recorded: false,
        phase: Phase::SteadyState,
        run_id: RunId::new("test-run").unwrap(),
    }
}

/// `base` with one output file added to `stage`.
pub fn with_output(base: &FsFacts, stage: StageId, file: &str) -> FsFacts {
    let mut facts: FsFacts = base.clone();
    facts.stages = StageFactsMap::from_fn(|id: StageId| {
        let mut stage_facts: StageFacts = base.stages.get(id).clone();
        if id == stage {
            stage_facts.output_files.push(file.to_string());
        }
        stage_facts
    });
    facts
}

/// `base` with a steer-request at `stage`.
pub fn with_steer(base: &FsFacts, stage: StageId, answered: bool) -> FsFacts {
    let mut facts: FsFacts = base.clone();
    facts.stages = StageFactsMap::from_fn(|id: StageId| {
        let mut stage_facts: StageFacts = base.stages.get(id).clone();
        if id == stage {
            stage_facts.steer = Some(SteerFacts { answered });
        }
        stage_facts
    });
    facts
}

/// A run with design, tests, and implementation outputs present.
pub fn implemented_facts() -> FsFacts {
    let designed: FsFacts = with_output(&fresh_facts(), StageId::Design, "design.md");
    let tested: FsFacts = with_output(&designed, StageId::Tests, "test-plan.md");
    with_output(&tested, StageId::Implement, "notes.md")
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
    (
        prop::collection::vec(arb_stage_facts(), 5),
        prop::option::of(arb_verdict()),
        any::<bool>(),
        arb_phase(),
    )
        .prop_map(
            |(stages, gate, commit_recorded, phase): (
                Vec<StageFacts>,
                Option<Verdict>,
                bool,
                Phase,
            )| {
                FsFacts {
                    stages: StageFactsMap::from_fn(|id: StageId| {
                        let index: usize = StageId::ALL
                            .iter()
                            .position(|candidate: &StageId| *candidate == id)
                            .unwrap();
                        stages[index].clone()
                    }),
                    gate,
                    commit_recorded,
                    phase,
                    run_id: RunId::new("prop-run").unwrap(),
                }
            },
        )
}
