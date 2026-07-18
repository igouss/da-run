//! The run-state machine for a da-run instance.
//!
//! State is re-derived from filesystem facts on every invocation, so the core
//! is a pure validator/deriver over an immutable [`FsFacts`] snapshot — the
//! "events" of this machine are file mutations made by other actors (agents,
//! the operator, gate.sh). Adapters parse the run dir into facts; this crate
//! never touches I/O.

mod anomaly;
mod derive;
mod dispatch;
mod facts;
mod phase;
mod refusal;
mod run_state;
mod stage;
mod transition;
mod verdict;
mod warning;

pub use anomaly::Anomaly;
pub use derive::{Derived, derive};
pub use dispatch::Dispatch;
pub use facts::{BlankRunId, FsFacts, RunId, StageFacts, StageFactsMap, SteerFacts};
pub use phase::Phase;
pub use refusal::Refusal;
pub use run_state::RunState;
pub use stage::StageId;
pub use transition::{Allowed, GateGreenProof, check, commit_precondition};
pub use verdict::Verdict;
pub use warning::Warning;
