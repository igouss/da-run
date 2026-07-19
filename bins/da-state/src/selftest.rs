//! The embedded smoke: build a scratch run dir carrying the canonical
//! flow.ron, walk the refusal ladder, assert every exit-code mapping.
//! Mirrors the `bin/run --selftest` ritual.

use crate::exec::{EXIT_OK, EXIT_ORDERING, EXIT_STEER_PENDING, Outcome};
use da_adapter_fs::{FsSnapshotSource, load_run_flow};
use da_app::{Decision, check_dispatch};
use da_domain::{DispatchRef, Flow, Refusal};
use da_ports::SnapshotError;
use std::fs;
use std::path::{Path, PathBuf};

/// The canonical flow, embedded so the selftest also proves it valid.
const CANONICAL_FLOW: &str = include_str!("../../../algorithm/flow.ron");

pub fn run() -> Outcome {
    match walk_ladder() {
        Ok(()) => Outcome {
            json: r#"{"selftest":"ok"}"#.to_string(),
            pretty: None,
            exit_code: EXIT_OK,
        },
        Err(failure) => Outcome {
            json: serde_json::json!({ "selftest": "failed", "detail": failure }).to_string(),
            pretty: None,
            exit_code: 1,
        },
    }
}

fn walk_ladder() -> Result<(), String> {
    let dir: tempfile::TempDir =
        tempfile::TempDir::new().map_err(|error: std::io::Error| error.to_string())?;
    let flow: Flow = scaffold(dir.path())?;

    ensure(
        decide(&flow, dir.path(), "tests")? == EXIT_ORDERING,
        "fresh run must refuse tests with exit 4",
    )?;
    write_stage(&flow, dir.path(), "design", "design.md", "# design")?;
    ensure(
        decide(&flow, dir.path(), "tests")? == EXIT_OK,
        "a designed run must allow tests",
    )?;
    write_stage(
        &flow,
        dir.path(),
        "verify",
        "gate-report.md",
        "GATE RED — do not ship\n",
    )?;
    ensure(
        decide(&flow, dir.path(), "commit")? == EXIT_ORDERING,
        "a red gate must refuse commit with exit 4",
    )?;
    write_stage(
        &flow,
        dir.path(),
        "verify",
        "gate-report.md",
        "GATE GREEN\n",
    )?;
    ensure(
        decide(&flow, dir.path(), "commit")? == EXIT_OK,
        "a green gate must allow commit",
    )?;
    write_stage(
        &flow,
        dir.path(),
        "tests",
        "STEER-REQUEST.md",
        "## Question\n\nq?\n\n## Answer\n\n",
    )?;
    ensure(
        decide(&flow, dir.path(), "design")? == EXIT_STEER_PENDING,
        "a pending steer must park every dispatch with exit 3",
    )?;
    ensure(
        flow.resolve_dispatch("warp-speed").is_none(),
        "an unknown dispatch kind must not resolve",
    )?;
    Ok(())
}

fn decide(flow: &Flow, run_dir: &Path, kind: &str) -> Result<u8, String> {
    let dispatch: DispatchRef = flow
        .resolve_dispatch(kind)
        .ok_or_else(|| format!("dispatch kind {kind:?} missing from the canonical flow"))?;
    let decision: Decision = check_dispatch(&FsSnapshotSource, flow, run_dir, dispatch)
        .map_err(|error: SnapshotError| error.to_string())?;
    Ok(match decision {
        Decision::Allowed(_) => EXIT_OK,
        Decision::Refused(refusal) => exit_of_refusal(&refusal),
    })
}

fn exit_of_refusal(refusal: &Refusal) -> u8 {
    match refusal {
        Refusal::SteerPending { .. } => EXIT_STEER_PENDING,
        _ => EXIT_ORDERING,
    }
}

/// Write run.edn + the canonical flow.ron, create every stage output dir,
/// and load the flow back — the same validation path real commands take.
fn scaffold(run_dir: &Path) -> Result<Flow, String> {
    fs::write(
        run_dir.join("run.edn"),
        "{:run-id \"selftest\" :phase \"steady-state\"}",
    )
    .map_err(|error: std::io::Error| error.to_string())?;
    fs::write(run_dir.join("flow.ron"), CANONICAL_FLOW)
        .map_err(|error: std::io::Error| error.to_string())?;
    let flow: Flow =
        load_run_flow(run_dir).map_err(|error: da_adapter_fs::FlowLoadError| error.to_string())?;
    for (_, stage) in flow.stages() {
        fs::create_dir_all(run_dir.join("stages").join(&stage.dir).join("output"))
            .map_err(|error: std::io::Error| error.to_string())?;
    }
    Ok(flow)
}

fn write_stage(
    flow: &Flow,
    run_dir: &Path,
    stage_name: &str,
    name: &str,
    content: &str,
) -> Result<(), String> {
    let stage_dir: String = flow
        .stage_named(stage_name)
        .map(|stage: da_domain::StageRef| flow.stage(stage).dir.clone())
        .ok_or_else(|| format!("stage {stage_name:?} missing from the canonical flow"))?;
    let path: PathBuf = run_dir
        .join("stages")
        .join(stage_dir)
        .join("output")
        .join(name);
    fs::write(path, content).map_err(|error: std::io::Error| error.to_string())
}

fn ensure(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_string())
    }
}
