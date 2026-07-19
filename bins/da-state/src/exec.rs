use crate::cli::{Cli, Command};
use crate::pretty;
use crate::selftest;
use da_adapter_fs::{
    FLOW_FILE, FlowLoadError, FsArtifactSink, FsArtifactSource, FsSnapshotSource, load_flow_file,
    load_run_flow,
};
use da_adapter_restate::RestateIngressMirror;
use da_app::{
    Decision, PublishError, RestoreError, Restored, check_dispatch, publish_mirror, restore_run,
    status,
};
use da_domain::{Derived, Flow, FsFacts, Refusal, RunId, derive};
use da_ports::{SnapshotError, SnapshotSource};
use da_wire::{CheckWire, DerivedWire, FlowWire, StatusWire};
use std::path::{Path, PathBuf};

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
        Command::Derive { run_dir } => with_flow(run_dir, run_derive),
        Command::Status { run_dir } => with_flow(run_dir, run_status),
        Command::Check { run_dir, dispatch } => {
            with_flow(run_dir, |flow: &Flow, run_dir: &Path| {
                run_check(flow, run_dir, dispatch)
            })
        }
        Command::Notify { run_dir } => with_flow(run_dir, run_notify),
        Command::Restore { run_id, into } => run_restore(run_id, into),
        Command::Flow { run_dir, file } => run_flow(run_dir.as_deref(), file.as_deref()),
        Command::Selftest => selftest::run(),
    }
}

/// Load the run dir's flow.ron — the load-time validation gate every command
/// passes through — then run the command over it.
fn with_flow(run_dir: &Path, run: impl FnOnce(&Flow, &Path) -> Outcome) -> Outcome {
    match load_run_flow(run_dir) {
        Ok(flow) => run(&flow, run_dir),
        Err(error) => flow_failure(&error),
    }
}

fn run_derive(flow: &Flow, run_dir: &Path) -> Outcome {
    match FsSnapshotSource.snapshot(flow, run_dir) {
        Ok(facts) => {
            let facts: FsFacts = facts;
            let derived: Derived = derive(flow, &facts);
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

fn run_status(flow: &Flow, run_dir: &Path) -> Outcome {
    match status(&FsSnapshotSource, flow, run_dir) {
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

fn run_check(flow: &Flow, run_dir: &Path, dispatch: &str) -> Outcome {
    match check_dispatch(&FsSnapshotSource, flow, run_dir, dispatch) {
        Ok(Decision::Allowed(allowed)) => Outcome {
            json: to_json(&CheckWire::allowed(&allowed)),
            pretty: None,
            exit_code: EXIT_OK,
        },
        // The CLI contract for a typo'd kind predates the typed refusal:
        // an error payload listing the flow's kinds, exit 2.
        Ok(Decision::Refused(Refusal::UnknownDispatch { .. })) => {
            unknown_dispatch(flow, dispatch)
        }
        Ok(Decision::Refused(refusal)) => Outcome {
            json: to_json(&CheckWire::refused(&refusal)),
            pretty: None,
            exit_code: refusal_exit(&refusal),
        },
        Err(error) => snapshot_failure(&error),
    }
}

fn run_flow(run_dir: Option<&Path>, file: Option<&Path>) -> Outcome {
    let path: PathBuf = match (run_dir, file) {
        (Some(run_dir), None) => run_dir.join(FLOW_FILE),
        (None, Some(file)) => file.to_path_buf(),
        _ => {
            return Outcome {
                json: serde_json::json!({
                    "error": "flow needs exactly one of --run RUNDIR or --file FLOW.RON",
                })
                .to_string(),
                pretty: None,
                exit_code: EXIT_USAGE,
            };
        }
    };
    match load_flow_file(&path) {
        Ok(flow) => Outcome {
            json: to_json(&FlowWire::from_domain(&flow)),
            pretty: None,
            exit_code: EXIT_OK,
        },
        Err(error) => flow_failure(&error),
    }
}

/// Best-effort by configuration: no `DA_STEER_INGRESS` means the mirror is
/// off — a clean no-op, exit 0. A configured-but-failing publish is exit 1.
fn run_notify(flow: &Flow, run_dir: &Path) -> Outcome {
    let ingress: Option<String> = std::env::var("DA_STEER_INGRESS").ok();
    let Some(ingress) = ingress else {
        return Outcome {
            json: serde_json::json!({
                "published": false,
                "note": "DA_STEER_INGRESS unset — mirror disabled",
            })
            .to_string(),
            pretty: None,
            exit_code: EXIT_OK,
        };
    };
    let mirror: RestateIngressMirror = RestateIngressMirror {
        ingress,
        ca_path: restate_ca(),
    };
    match publish_mirror(&FsSnapshotSource, &FsArtifactSource, &mirror, flow, run_dir) {
        Ok(published) => Outcome {
            json: serde_json::json!({
                "published": true,
                "artifacts": published.artifact_count,
                "state": DerivedWire::from_domain(&published.run_id, &published.derived),
            })
            .to_string(),
            pretty: None,
            exit_code: EXIT_OK,
        },
        Err(PublishError::Snapshot(error)) => snapshot_failure(&error),
        Err(PublishError::Mirror(error)) => Outcome {
            json: serde_json::json!({ "published": false, "error": error.to_string() }).to_string(),
            pretty: None,
            exit_code: 1,
        },
    }
}

/// Restore a mirrored run's artifacts into a directory. Refuses to overwrite
/// an existing run (a run.edn already present in the target).
fn run_restore(run_id: &str, into: &Path) -> Outcome {
    let Ok(run_id) = RunId::new(run_id) else {
        return usage_failure("restore needs a non-blank --run-id");
    };
    let Some(ingress) = std::env::var("DA_STEER_INGRESS").ok() else {
        return usage_failure("restore needs DA_STEER_INGRESS (the mirror holds the artifacts)");
    };
    if into.join("run.edn").is_file() {
        return usage_failure("target already holds a run.edn — refusing to overwrite a run");
    }
    let mirror: RestateIngressMirror = RestateIngressMirror {
        ingress,
        ca_path: restate_ca(),
    };
    match restore_run(&mirror, &FsArtifactSink, &run_id, into) {
        Ok(restored) => {
            let restored: Restored = restored;
            Outcome {
                json: serde_json::json!({
                    "restored": true,
                    "run_id": run_id.as_str(),
                    "run_dir": into.display().to_string(),
                    "files": restored.file_count,
                    "state": restored
                        .state_json
                        .and_then(|raw: String| serde_json::from_str::<serde_json::Value>(&raw).ok()),
                    "note": "worktree not restored — recreate from run.edn's project/branch/base-commit",
                })
                .to_string(),
                pretty: None,
                exit_code: EXIT_OK,
            }
        }
        Err(RestoreError::Sink(error)) => snapshot_failure(&error),
        Err(error) => Outcome {
            json: serde_json::json!({ "restored": false, "error": error.to_string() }).to_string(),
            pretty: None,
            exit_code: 1,
        },
    }
}

fn usage_failure(detail: &str) -> Outcome {
    Outcome {
        json: serde_json::json!({ "error": detail }).to_string(),
        pretty: None,
        exit_code: EXIT_USAGE,
    }
}

/// `RESTATE_CA`, defaulting to the homelab caddy root exactly like
/// `bin/steer` does; only an existing file is passed to curl.
fn restate_ca() -> Option<PathBuf> {
    let from_env: Option<PathBuf> = std::env::var("RESTATE_CA").ok().map(PathBuf::from);
    let default: Option<PathBuf> = std::env::var("HOME").ok().map(|home: String| {
        PathBuf::from(home).join(".local/share/caddy/pki/authorities/local/root.crt")
    });
    from_env.or(default).filter(|path: &PathBuf| path.is_file())
}

fn refusal_exit(refusal: &Refusal) -> u8 {
    match refusal {
        Refusal::SteerPending { .. } => EXIT_STEER_PENDING,
        _ => EXIT_ORDERING,
    }
}

fn unknown_dispatch(flow: &Flow, dispatch: &str) -> Outcome {
    Outcome {
        json: serde_json::json!({
            "error": format!("unknown dispatch {dispatch:?} — not in the run's flow.ron"),
            "kinds": flow.dispatch_kinds(),
        })
        .to_string(),
        pretty: None,
        exit_code: EXIT_USAGE,
    }
}

fn flow_failure(error: &FlowLoadError) -> Outcome {
    let payload: serde_json::Value = serde_json::json!({
        "error": error.to_string(),
    });
    Outcome {
        json: payload.to_string(),
        pretty: None,
        exit_code: EXIT_USAGE,
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
