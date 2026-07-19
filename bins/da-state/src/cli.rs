use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "da-state", about = "The run-state authority for da-run")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
    /// Also render a human-readable view on stderr.
    #[arg(long, global = true)]
    pub pretty: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Full derived state as JSON.
    Derive {
        #[arg(long = "run")]
        run_dir: PathBuf,
    },
    /// Pipeline status: per-stage detail plus the derived summary.
    Status {
        #[arg(long = "run")]
        run_dir: PathBuf,
    },
    /// Decide a dispatch: exit 0 allowed, 3 steer pending, 4 ordering violation.
    /// Dispatch kinds come from the run's flow.ron — unknown kinds are exit 2.
    /// An allowed check journals `dispatch:<kind>` to events.jsonl (ADR-0004)
    /// — check is the one mandatory pre-dispatch touchpoint, so the journal
    /// rides it structurally instead of relying on a remembered `bin/run mark`.
    Check {
        #[arg(long = "run")]
        run_dir: PathBuf,
        dispatch: String,
        /// Skip the events.jsonl entry (status probes, dry runs).
        #[arg(long = "no-journal")]
        no_journal: bool,
    },
    /// Publish the derived state AND the run's artifacts to the DaRun mirror
    /// (needs DA_STEER_INGRESS) — after this the run is restorable elsewhere.
    Notify {
        #[arg(long = "run")]
        run_dir: PathBuf,
    },
    /// Materialize a mirrored run's artifacts into a directory — the
    /// restart-on-another-host path (needs DA_STEER_INGRESS). The worktree is
    /// not restored: recreate it from run.json's project/branch/base-commit.
    Restore {
        #[arg(long = "run-id")]
        run_id: String,
        #[arg(long = "into")]
        into: PathBuf,
    },
    /// Load, validate, and print a flow definition as JSON — the bridge for
    /// consumers that cannot parse RON (bb scripts, workflow JS).
    Flow {
        /// A run dir holding flow.ron.
        #[arg(long = "run", conflicts_with = "file")]
        run_dir: Option<PathBuf>,
        /// A flow.ron path directly (pre-run validation).
        #[arg(long = "file")]
        file: Option<PathBuf>,
    },
    /// Embedded smoke test over a scratch run dir.
    Selftest,
}
