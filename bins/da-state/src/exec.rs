use crate::cli::{Cli, Command};
use crate::pretty;
use crate::selftest;
use da_adapter_fs::FsSnapshotSource;
use da_app::{Decision, check_dispatch, status};
use da_domain::{Derived, Dispatch, Refusal, derive};
use da_ports::{SnapshotError, SnapshotSource};
use da_wire::{CheckWire, DerivedWire, StatusWire};
use std::path::Path;

/// Exit codes — bin/steer's convention, extended.
pub const EXIT_OK: u8 = 0;
pub const EXIT_USAGE: u8 = 2;
pub const EXIT_STEER_PENDING: u8 = 3;
pub const EXIT_ORDERING: u8 = 4;

pub struct Outcome {
    pub json: String,
    pub pretty: Option<String>,
    pub exit_code: u8,
}

pub fn execute(args: &Cli) -> Outcome {
    match &args.command {
        Command::Derive { run_dir } => run_derive(run_dir),
        Command::Status { run_dir } => run_status(run_dir),
        Command::Check { run_dir, dispatch } => run_check(run_dir, &dispatch.to_dispatch()),
        Command::Selftest => selftest::run(),
    }
}

fn run_derive(run_dir: &Path) -> Outcome {
    match FsSnapshotSource.snapshot(run_dir) {
        Ok(facts) => {
            let derived: Derived = derive(&facts);
            let wire: DerivedWire = DerivedWire::from_domain(&facts.run_id, &derived);
            Outcome {
                json: to_json(&wire),
                pretty: None,
                exit_code: EXIT_OK,
            }
        }
        Err(error) => snapshot_failure(&error),
    }
}

fn run_status(run_dir: &Path) -> Outcome {
    match status(&FsSnapshotSource, run_dir) {
        Ok(report) => {
            let wire: StatusWire = StatusWire::from_report(&report);
            Outcome {
                pretty: Some(pretty::render_status(&wire)),
                json: to_json(&wire),
                exit_code: EXIT_OK,
            }
        }
        Err(error) => snapshot_failure(&error),
    }
}

fn run_check(run_dir: &Path, dispatch: &Dispatch) -> Outcome {
    match check_dispatch(&FsSnapshotSource, run_dir, dispatch) {
        Ok(Decision::Allowed(allowed)) => Outcome {
            json: to_json(&CheckWire::allowed(&allowed)),
            pretty: None,
            exit_code: EXIT_OK,
        },
        Ok(Decision::Refused(refusal)) => Outcome {
            json: to_json(&CheckWire::refused(&refusal)),
            pretty: None,
            exit_code: refusal_exit(&refusal),
        },
        Err(error) => snapshot_failure(&error),
    }
}

fn refusal_exit(refusal: &Refusal) -> u8 {
    match refusal {
        Refusal::SteerPending { .. } => EXIT_STEER_PENDING,
        _ => EXIT_ORDERING,
    }
}

fn snapshot_failure(error: &SnapshotError) -> Outcome {
    let payload: serde_json::Value = serde_json::json!({
        "error": error.to_string(),
    });
    Outcome {
        json: payload.to_string(),
        pretty: None,
        exit_code: EXIT_USAGE,
    }
}

fn to_json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|error: serde_json::Error| {
        serde_json::json!({ "error": format!("serialize: {error}") }).to_string()
    })
}
