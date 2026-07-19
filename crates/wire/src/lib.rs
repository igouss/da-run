//! The published language. Wire shapes are versioned, kebab-cased, and
//! separate from the domain enums — adapters translate, consumers read
//! tolerantly. `v` marks the payload version.

mod artifact;
mod check;
mod derived;
mod flow;
mod status;

pub use artifact::{ArtifactWire, MirrorSnapshotWire, RunSnapshotWire};
pub use check::{CheckWire, ReasonWire, WarningWire};
pub use derived::{AnomalyWire, DerivedWire};
pub use flow::{FlowDispatchWire, FlowStageWire, FlowWire};
pub use status::{StageWire, StatusWire};

/// Current wire payload version.
pub const WIRE_VERSION: u32 = 1;
