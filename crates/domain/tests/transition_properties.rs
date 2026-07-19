//! P1–P4: the transition laws, over the canonical flow fixture.

#![allow(clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::{arb_facts, arb_stage_facts, test_flow};
use da_domain::{Flow, FsFacts, Refusal, StageFacts, StageFactsMap, StageRef, Verdict, check};
use proptest::prelude::*;

fn arb_dispatch_kind() -> impl Strategy<Value = String> {
    let kinds: Vec<String> = test_flow()
        .dispatch_kinds()
        .into_iter()
        .map(str::to_string)
        .collect();
    prop::sample::select(kinds)
}

fn arb_stage_name() -> impl Strategy<Value = String> {
    let names: Vec<String> = test_flow()
        .stages()
        .map(|(_, stage): (StageRef, &da_domain::StageDef)| stage.name.clone())
        .collect();
    prop::sample::select(names)
}

proptest! {
    // P1 — the headline law: commit allowed implies a green gate and no
    // unanswered steer anywhere.
    #[test]
    fn commit_allowed_implies_green_gate_and_no_steer(facts in arb_facts()) {
        let flow: Flow = test_flow();
        if check(&flow, &facts, "commit").is_ok() {
            prop_assert_eq!(facts.gate, Some(Verdict::Green));
            for (stage, _) in flow.stages() {
                prop_assert!(!facts.stages.get(stage).steer_pending());
            }
        }
    }

    // P2 — any unanswered steer refuses every dispatch, naming the stage dir.
    #[test]
    fn pending_steer_refuses_every_dispatch(
        facts in arb_facts(),
        kind in arb_dispatch_kind(),
        steer_stage in arb_stage_name(),
    ) {
        let flow: Flow = test_flow();
        let parked: FsFacts = common::with_steer(&flow, &facts, &steer_stage, false);
        let refusal: Refusal = check(&flow, &parked, &kind)
            .expect_err("a pending steer must refuse every dispatch");
        let steer_dir: String = flow
            .stage(common::stage_ref(&flow, &steer_stage))
            .expect("the fixture flow resolves its own stages")
            .dir
            .clone();
        match refusal {
            Refusal::SteerPending { stages } => prop_assert!(stages.contains(&steer_dir)),
            other => prop_assert!(false, "expected SteerPending, got {other:?}"),
        }
    }

    // P3 — handoff order: no tests without a design, no implementation
    // without tests.
    #[test]
    fn empty_design_refuses_tests(facts in arb_facts(), design in arb_stage_facts()) {
        let flow: Flow = test_flow();
        let mut empty_design: StageFacts = design;
        empty_design.output_files.clear();
        let facts: FsFacts = rebuild_stage(&flow, &facts, "design", empty_design);
        if check(&flow, &facts, "tests").is_ok() {
            prop_assert!(false, "tests must be refused while the design is empty");
        }
    }

    #[test]
    fn empty_tests_refuse_implement(facts in arb_facts()) {
        let flow: Flow = test_flow();
        let replacement: StageFacts = StageFacts {
            output_files: Vec::new(),
            steer: facts.stages.get(common::stage_ref(&flow, "tests")).steer.clone(),
        };
        let facts: FsFacts = rebuild_stage(&flow, &facts, "tests", replacement);
        if check(&flow, &facts, "implement").is_ok() {
            prop_assert!(false, "implement must be refused while tests are empty");
        }
    }

    // P4 — with no steer pending, design and design-review always run.
    #[test]
    fn design_always_allowed_without_steer(facts in arb_facts()) {
        let flow: Flow = test_flow();
        let calm: FsFacts = clear_steers(&flow, &facts);
        prop_assert!(check(&flow, &calm, "design").is_ok());
        prop_assert!(check(&flow, &calm, "design-review").is_ok());
    }

    // P6 — an unknown dispatch kind is its own typed refusal (or a steer
    // park), never a silent resolution to some other dispatch.
    #[test]
    fn unknown_kind_never_resolves_to_another_dispatch(
        facts in arb_facts(),
        kind in "[a-z]{1,12}",
    ) {
        let flow: Flow = test_flow();
        prop_assume!(flow.resolve_dispatch(&kind).is_none());
        match check(&flow, &facts, &kind) {
            Err(Refusal::UnknownDispatch { kind: named }) => prop_assert_eq!(named, kind),
            Err(Refusal::SteerPending { .. }) => {} // steers park everything, even typos
            other => prop_assert!(false, "expected UnknownDispatch, got {other:?}"),
        }
    }
}

fn rebuild_stage(flow: &Flow, base: &FsFacts, name: &str, replacement: StageFacts) -> FsFacts {
    let target: StageRef = common::stage_ref(flow, name);
    let mut facts: FsFacts = base.clone();
    facts.stages = StageFactsMap::from_fn(flow, |stage: StageRef| {
        if stage == target {
            replacement.clone()
        } else {
            base.stages.get(stage).clone()
        }
    });
    facts
}

fn clear_steers(flow: &Flow, base: &FsFacts) -> FsFacts {
    let mut facts: FsFacts = base.clone();
    facts.stages = StageFactsMap::from_fn(flow, |stage: StageRef| StageFacts {
        output_files: base.stages.get(stage).output_files.clone(),
        steer: None,
    });
    facts
}
