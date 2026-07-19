//! The selftest ladder as a real test: the same walk the `selftest` CLI
//! subcommand runs, so the fixture flow and the ladder can never drift apart
//! without CI going red — the exact failure mode da-run shipped with.

#[test]
fn the_selftest_ladder_walks_green() {
    if let Err(failure) = da_state::selftest::walk_ladder() {
        panic!("selftest ladder failed: {failure}");
    }
}
