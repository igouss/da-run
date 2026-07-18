//! Ring 2: use cases. Load facts through a port, run the pure core, hand the
//! result to a boundary.

mod check_dispatch;
mod mirror;
mod status;

pub use check_dispatch::{Decision, check_dispatch};
pub use mirror::{PublishError, Published, publish_mirror};
pub use status::{StageStatus, StatusReport, status};
