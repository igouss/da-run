//! The embedded smoke: build a scratch run dir, walk the refusal ladder,
//! assert every exit-code mapping. Mirrors the `bin/run --selftest` ritual.

use crate::exec::{EXIT_OK, EXIT_ORDERING, EXIT_STEER_PENDING, Outcome};
use da_adapter_fs::FsSnapshotSource;
use da_app::{Decision, check_dispatch};
use da_domain::{Dispatch, Refusal};
use da_ports::SnapshotError;
use std::fs;
use std::path::{Path, PathBuf};

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
    scaffold(dir.path()).map_err(|error: std::io::Error| error.to_string())?;

    ensure(
        decide(dir.path(), &Dispatch::Tests)? == exit_of_refusal(&Refusal::TestsBeforeDesign),
        "fresh run must refuse tests with exit 4",
    )?;
    write_stage(dir.path(), "01-design", "design.md", "# design")?;
    ensure(
        decide(dir.path(), &Dispatch::Tests)? == EXIT_OK,
        "a designed run must allow tests",
    )?;
    write_stage(
        dir.path(),
        "04-verify",
        "gate-report.md",
        "GATE RED — do not ship\n",
    )?;
    ensure(
        decide(dir.path(), &Dispatch::Commit)? == EXIT_ORDERING,
        "a red gate must refuse commit with exit 4",
    )?;
    write_stage(dir.path(), "04-verify", "gate-report.md", "GATE GREEN\n")?;
    ensure(
        decide(dir.path(), &Dispatch::Commit)? == EXIT_OK,
        "a green gate must allow commit",
    )?;
    write_stage(
        dir.path(),
        "02-tests",
        "STEER-REQUEST.md",
        "## Question\n\nq?\n\n## Answer\n\n",
    )?;
    ensure(
        decide(dir.path(), &Dispatch::Design)? == EXIT_STEER_PENDING,
        "a pending steer must park every dispatch with exit 3",
    )?;
    Ok(())
}

fn decide(run_dir: &Path, dispatch: &Dispatch) -> Result<u8, String> {
    let decision: Decision = check_dispatch(&FsSnapshotSource, run_dir, dispatch)
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

fn scaffold(run_dir: &Path) -> std::io::Result<()> {
    fs::write(
        run_dir.join("run.edn"),
        "{:run-id \"selftest\" :phase \"steady-state\"}",
    )?;
    for stage in [
        "01-design",
        "02-tests",
        "03-implement",
        "04-verify",
        "05-commit",
    ] {
        fs::create_dir_all(run_dir.join("stages").join(stage).join("output"))?;
    }
    Ok(())
}

fn write_stage(run_dir: &Path, stage: &str, name: &str, content: &str) -> Result<(), String> {
    let path: PathBuf = run_dir.join("stages").join(stage).join("output").join(name);
    fs::write(path, content).map_err(|error: std::io::Error| error.to_string())
}

fn ensure(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_string())
    }
}
