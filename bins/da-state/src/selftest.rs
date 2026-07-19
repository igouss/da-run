//! The embedded smoke: build a scratch run dir carrying the engine's fixture
//! flow, walk the refusal ladder — ordering, gate, the worktree-identity
//! commit law, steer parking, unknown kinds — and assert every exit-code
//! mapping. One walk, two callers: the `selftest` CLI subcommand and a
//! `cargo test` in `tests/selftest_ladder.rs`, so the ladder cannot drift
//! out from under CI again.

use crate::exec::{EXIT_OK, EXIT_ORDERING, EXIT_STEER_PENDING, Outcome};
use da_adapter_fs::{FsSnapshotSource, load_run_flow, worktree_facts};
use da_app::{Decision, check_dispatch};
use da_domain::{Flow, Refusal};
use da_ports::SnapshotError;
use std::fs;
use std::path::{Path, PathBuf};

/// The engine's fixture flow (plan → build → check → land), embedded so the
/// selftest also proves it valid — and so a fixture rename breaks this file
/// at compile time, not silently at run time.
const FIXTURE_FLOW: &str = include_str!("../../../engine/fixtures/minimal-flow.ron");

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

/// The full ladder. Public so `cargo test` runs the identical walk.
pub fn walk_ladder() -> Result<(), String> {
    let dir: tempfile::TempDir =
        tempfile::TempDir::new().map_err(|error: std::io::Error| error.to_string())?;
    let flow: Flow = scaffold(dir.path())?;

    ensure(
        decide(&flow, dir.path(), "build")? == EXIT_ORDERING,
        "a fresh run must refuse build with exit 4",
    )?;
    write_stage(&flow, dir.path(), "plan", "plan.md", "# plan")?;
    ensure(
        decide(&flow, dir.path(), "build")? == EXIT_OK,
        "a planned run must allow build",
    )?;
    write_stage(
        &flow,
        dir.path(),
        "check",
        "gate-report.md",
        "GATE RED — do not ship\n",
    )?;
    ensure(
        decide(&flow, dir.path(), "land")? == EXIT_ORDERING,
        "a red gate must refuse land with exit 4",
    )?;
    // A green verdict alone is not enough: with no worktree.patch the run
    // cannot say what code the gate judged, and the commit law fails closed.
    write_stage(&flow, dir.path(), "check", "gate-report.md", "GATE GREEN\n")?;
    ensure(
        decide(&flow, dir.path(), "land")? == EXIT_ORDERING,
        "a green gate without a worktree record must still refuse land",
    )?;
    let patch: &str = "diff --git a/lib.rs b/lib.rs\n+pub fn answer() -> u8 { 42 }\n";
    fs::write(dir.path().join("worktree.patch"), patch)
        .map_err(|error: std::io::Error| error.to_string())?;
    let identity: String = worktree_facts(patch)
        .map(|facts: da_domain::WorktreeFacts| facts.id.to_string())
        .ok_or_else(|| "a non-empty patch must yield a worktree identity".to_string())?;
    write_stage(
        &flow,
        dir.path(),
        "check",
        "gate-report.md",
        &format!("Worktree: {identity}\nGATE GREEN\n"),
    )?;
    ensure(
        decide(&flow, dir.path(), "land")? == EXIT_OK,
        "a green gate over the recorded worktree must allow land",
    )?;
    // The worktree moves after the gate: the recorded identity no longer
    // matches, so the stale green verdict must stop blessing the new code.
    fs::write(
        dir.path().join("worktree.patch"),
        "diff --git a/lib.rs b/lib.rs\n+pub fn answer() -> u8 { 41 }\n",
    )
    .map_err(|error: std::io::Error| error.to_string())?;
    ensure(
        decide(&flow, dir.path(), "land")? == EXIT_ORDERING,
        "a worktree that moved since the gate must refuse land",
    )?;
    write_stage(
        &flow,
        dir.path(),
        "build",
        "STEER-REQUEST.md",
        "## Question\n\nq?\n\n## Answer\n\n",
    )?;
    ensure(
        decide(&flow, dir.path(), "plan")? == EXIT_STEER_PENDING,
        "a pending steer must park every dispatch with exit 3",
    )?;
    ensure(
        flow.resolve_dispatch("warp-speed").is_none(),
        "an unknown dispatch kind must not resolve",
    )?;
    Ok(())
}

fn decide(flow: &Flow, run_dir: &Path, kind: &str) -> Result<u8, String> {
    let decision: Decision = check_dispatch(&FsSnapshotSource, flow, run_dir, kind)
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

/// Write run.edn + the fixture flow.ron, create every stage output dir,
/// and load the flow back — the same validation path real commands take.
fn scaffold(run_dir: &Path) -> Result<Flow, String> {
    fs::write(
        run_dir.join("run.edn"),
        "{:run-id \"selftest\" :phase \"steady-state\"}",
    )
    .map_err(|error: std::io::Error| error.to_string())?;
    fs::write(run_dir.join("flow.ron"), FIXTURE_FLOW)
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
        .stages()
        .find(|(_, stage): &(da_domain::StageRef, &da_domain::StageDef)| stage.name == stage_name)
        .map(|(_, stage): (da_domain::StageRef, &da_domain::StageDef)| stage.dir.clone())
        .ok_or_else(|| format!("stage {stage_name:?} missing from the fixture flow"))?;
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
