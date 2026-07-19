use da_domain::{Allowed, Flow, FsFacts, Refusal, check};
use da_ports::{SnapshotError, SnapshotSource};
use std::path::Path;

/// The machine's verdict on a dispatch — a value, not an error: a refusal is
/// a correct answer, not a failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    Allowed(Allowed),
    Refused(Refusal),
}

/// Snapshot a run dir and decide a dispatch, named by kind, against it.
pub fn check_dispatch<S: SnapshotSource>(
    source: &S,
    flow: &Flow,
    run_dir: &Path,
    dispatch: &str,
) -> Result<Decision, SnapshotError> {
    let facts: FsFacts = source.snapshot(flow, run_dir)?;
    Ok(match check(flow, &facts, dispatch) {
        Ok(allowed) => Decision::Allowed(allowed),
        Err(refusal) => Decision::Refused(refusal),
    })
}
