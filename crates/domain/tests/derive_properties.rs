//! P5: `derive` is total and monotone — adding output files never moves the
//! summary state earlier.

#![allow(clippy::unwrap_used)]

mod common;

use common::{arb_facts, test_flow, with_output};
use da_domain::{Derived, Flow, FsFacts, StageDef, StageRef, derive};
use proptest::prelude::*;

fn arb_stage_name() -> impl Strategy<Value = String> {
    let names: Vec<String> = test_flow()
        .stages()
        .map(|(_, stage): (StageRef, &StageDef)| stage.name.clone())
        .collect();
    prop::sample::select(names)
}

proptest! {
    #[test]
    fn derive_is_total(facts in arb_facts()) {
        let flow: Flow = test_flow();
        let _derived: Derived = derive(&flow, &facts);
    }

    #[test]
    fn adding_output_never_moves_state_earlier(
        facts in arb_facts(),
        stage_name in arb_stage_name(),
    ) {
        let flow: Flow = test_flow();
        let grown: FsFacts = with_output(&flow, &facts, &stage_name, "extra.md");
        let before: u8 = derive(&flow, &facts).state.progress();
        let after: u8 = derive(&flow, &grown).state.progress();
        prop_assert!(after >= before);
    }

    #[test]
    fn parked_lists_exactly_the_unanswered_steers(facts in arb_facts()) {
        let flow: Flow = test_flow();
        let derived: Derived = derive(&flow, &facts);
        for (stage, def) in flow.stages() {
            let pending: bool = facts.stages.get(stage).steer_pending();
            prop_assert_eq!(derived.parked.contains(&def.dir), pending);
        }
    }
}
