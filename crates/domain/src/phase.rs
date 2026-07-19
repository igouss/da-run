/// The run's lifecycle phase, from `run.json` `"phase"`. Advisory only — it
/// shapes warnings, never refusals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Convergence,
    SteadyState,
}
