//! The run-state machine for a da-run instance.
//!
//! State is re-derived from filesystem facts on every invocation, so the core
//! is a pure validator/deriver over an immutable [`FsFacts`] snapshot — the
//! "events" of this machine are file mutations made by other actors (agents,
//! the operator, gate.sh). Adapters parse the run dir into facts; this crate
//! never touches I/O.
//!
//! Which stages exist, their dirs, dispatch kinds, and ordering guards are
//! not compiled in — they arrive as a [`Flow`], validated by
//! [`Flow::from_spec`] at load time (canonically parsed from `flow.ron` by
//! the fs adapter). The machine's own laws — steer parks everything, commit
//! demands a green gate — stay in code.

mod anomaly;
mod derive;
mod facts;
mod flow;
mod phase;
mod refusal;
mod run_state;
mod transition;
mod verdict;
mod warning;
mod worktree;

pub use anomaly::Anomaly;
pub use derive::{Derived, derive};
pub use facts::{BlankRunId, FsFacts, RunId, StageFacts, StageFactsMap, SteerFacts};
pub use flow::{
    AdviseRule, AdviseRuleSpec, BlockRule, BlockRuleSpec, DispatchDef, DispatchRef, DispatchSpec,
    Flow, FlowError, FlowSpec, Role, RoleSpec, StageDef, StageRef, StageSpec,
};
pub use phase::Phase;
pub use refusal::Refusal;
pub use run_state::RunState;
pub use transition::{Allowed, GateGreenProof, check, commit_precondition};
pub use verdict::Verdict;
pub use warning::Warning;
pub use worktree::{BlankWorktreeId, WorktreeFacts, WorktreeId};
