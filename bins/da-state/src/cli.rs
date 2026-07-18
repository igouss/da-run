use clap::{Parser, Subcommand, ValueEnum};
use da_domain::Dispatch;
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
    Check {
        #[arg(long = "run")]
        run_dir: PathBuf,
        dispatch: DispatchArg,
    },
    /// Embedded smoke test over a scratch run dir.
    Selftest,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum DispatchArg {
    Design,
    DesignReview,
    Tests,
    Implement,
    Verify,
    Commit,
}

impl DispatchArg {
    pub fn to_dispatch(self) -> Dispatch {
        match self {
            DispatchArg::Design => Dispatch::Design,
            DispatchArg::DesignReview => Dispatch::DesignReview,
            DispatchArg::Tests => Dispatch::Tests,
            DispatchArg::Implement => Dispatch::Implement {
                parallel_attempts: None,
            },
            DispatchArg::Verify => Dispatch::Verify,
            DispatchArg::Commit => Dispatch::Commit,
        }
    }
}
