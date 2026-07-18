use da_domain::{Allowed, Dispatch, FsFacts, Refusal, check};
use da_ports::{SnapshotError, SnapshotSource};
use std::path::Path;

/// The machine's verdict on a dispatch — a value, not an error: a refusal is
/// a correct answer, not a failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    Allowed(Allowed),
    Refused(Refusal),
}

/// Snapshot a run dir and decide a dispatch against it.
pub fn check_dispatch<S: SnapshotSource>(
    source: &S,
    run_dir: &Path,
    dispatch: &Dispatch,
) -> Result<Decision, SnapshotError> {
    let facts: FsFacts = source.snapshot(run_dir)?;
    Ok(match check(&facts, dispatch) {
        Ok(allowed) => Decision::Allowed(allowed),
        Err(refusal) => Decision::Refused(refusal),
    })
}
