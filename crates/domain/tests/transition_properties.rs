//! P1–P4: the transition laws.

#![allow(clippy::expect_used)]

mod common;

use common::{arb_facts, arb_stage_facts};
use da_domain::{Dispatch, FsFacts, Refusal, StageId, Verdict, check};
use proptest::prelude::*;

fn arb_dispatch() -> impl Strategy<Value = Dispatch> {
    prop_oneof![
        Just(Dispatch::Design),
        Just(Dispatch::DesignReview),
        Just(Dispatch::Tests),
        prop::option::of(1u8..4).prop_map(|parallel_attempts: Option<u8>| {
            Dispatch::Implement { parallel_attempts }
        }),
        Just(Dispatch::Verify),
        Just(Dispatch::Commit),
    ]
}

proptest! {
    // P1 — the headline law: commit allowed implies a green gate and no
    // unanswered steer anywhere.
    #[test]
    fn commit_allowed_implies_green_gate_and_no_steer(facts in arb_facts()) {
        if check(&facts, &Dispatch::Commit).is_ok() {
            prop_assert_eq!(facts.gate, Some(Verdict::Green));
            for stage in StageId::ALL {
                prop_assert!(!facts.stages.get(stage).steer_pending());
            }
        }
    }

    // P2 — any unanswered steer refuses every dispatch, naming the stage.
    #[test]
    fn pending_steer_refuses_every_dispatch(
        facts in arb_facts(),
        dispatch in arb_dispatch(),
        steer_stage in 0usize..5,
    ) {
        let stage: StageId = StageId::ALL[steer_stage];
        let parked: FsFacts = common::with_steer(&facts, stage, false);
        let refusal: Refusal = check(&parked, &dispatch)
            .expect_err("a pending steer must refuse every dispatch");
        match refusal {
            Refusal::SteerPending { stages } => prop_assert!(stages.contains(&stage)),
            other => prop_assert!(false, "expected SteerPending, got {other:?}"),
        }
    }

    // P3 — handoff order: no tests without a design, no implementation
    // without tests.
    #[test]
    fn empty_design_refuses_tests(facts in arb_facts(), design in arb_stage_facts()) {
        let mut empty_design = design;
        empty_design.output_files.clear();
        let facts: FsFacts = rebuild_stage(&facts, StageId::Design, empty_design);
        if check(&facts, &Dispatch::Tests).is_ok() {
            prop_assert!(false, "tests must be refused while the design is empty");
        }
    }

    #[test]
    fn empty_tests_refuse_implement(facts in arb_facts()) {
        let facts: FsFacts = rebuild_stage(
            &facts,
            StageId::Tests,
            da_domain::StageFacts {
                output_files: Vec::new(),
                steer: facts.stages.get(StageId::Tests).steer.clone(),
            },
        );
        if check(&facts, &Dispatch::Implement { parallel_attempts: None }).is_ok() {
            prop_assert!(false, "implement must be refused while tests are empty");
        }
    }

    // P4 — with no steer pending, design and design-review always run.
    #[test]
    fn design_always_allowed_without_steer(facts in arb_facts()) {
        let calm: FsFacts = clear_steers(&facts);
        prop_assert!(check(&calm, &Dispatch::Design).is_ok());
        prop_assert!(check(&calm, &Dispatch::DesignReview).is_ok());
    }
}

fn rebuild_stage(base: &FsFacts, target: StageId, replacement: da_domain::StageFacts) -> FsFacts {
    let mut facts: FsFacts = base.clone();
    facts.stages = da_domain::StageFactsMap::from_fn(|id: StageId| {
        if id == target {
            replacement.clone()
        } else {
            base.stages.get(id).clone()
        }
    });
    facts
}

fn clear_steers(base: &FsFacts) -> FsFacts {
    let mut facts: FsFacts = base.clone();
    facts.stages = da_domain::StageFactsMap::from_fn(|id: StageId| da_domain::StageFacts {
        output_files: base.stages.get(id).output_files.clone(),
        steer: None,
    });
    facts
}
