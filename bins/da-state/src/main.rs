//! `da-state` — the run-state authority for da-run.
//!
//! Robot JSON on stdout always; `--pretty` adds a human render on stderr so
//! pipelines never break. Exit codes extend `bin/steer`'s convention:
//! 0 allowed/ok, 2 usage or broken run dir, 3 steer pending, 4 ordering
//! violation.

use clap::Parser;
use da_state::{cli, exec};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: cli::Cli = cli::Cli::parse();
    let outcome: exec::Outcome = exec::execute(&args);
    println!("{}", outcome.json);
    if args.pretty
        && let Some(rendered) = outcome.pretty
    {
        eprintln!("{rendered}");
    }
    ExitCode::from(outcome.exit_code)
}
