//! The published language. Wire shapes are versioned, kebab-cased, and
//! separate from the domain enums — adapters translate, consumers read
//! tolerantly. `v` marks the payload version.

mod check;
mod derived;
mod status;

pub use check::{CheckWire, ReasonWire, WarningWire};
pub use derived::{AnomalyWire, DerivedWire};
pub use status::{StageWire, StatusWire};

/// Current wire payload version.
pub const WIRE_VERSION: u32 = 1;
