//! P5: `derive` is total and monotone — adding output files never moves the
//! summary state earlier.

mod common;

use common::{arb_facts, with_output};
use da_domain::{Derived, FsFacts, StageId, derive};
use proptest::prelude::*;

proptest! {
    #[test]
    fn derive_is_total(facts in arb_facts()) {
        let _derived: Derived = derive(&facts);
    }

    #[test]
    fn adding_output_never_moves_state_earlier(
        facts in arb_facts(),
        stage_index in 0usize..5,
    ) {
        let stage: StageId = StageId::ALL[stage_index];
        let grown: FsFacts = with_output(&facts, stage, "extra.md");
        let before: u8 = derive(&facts).state.progress();
        let after: u8 = derive(&grown).state.progress();
        prop_assert!(after >= before);
    }

    #[test]
    fn parked_lists_exactly_the_unanswered_steers(facts in arb_facts()) {
        let derived: Derived = derive(&facts);
        for stage in StageId::ALL {
            let pending: bool = facts.stages.get(stage).steer_pending();
            prop_assert_eq!(derived.parked.contains(&stage), pending);
        }
    }
}
