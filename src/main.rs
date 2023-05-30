extern crate flake_checker;

use flake_checker::{check_flake_lock, telemetry, FlakeCheckerError, FlakeLock, Summary};

use std::fs::read_to_string;
use std::path::PathBuf;

use clap::Parser;

/// A flake.lock checker for Nix projects.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Send aggregate sums of each issue type,
    /// see: https://github.com/determinateSystems/flake-checker
    #[clap(long, env = "NIX_FLAKE_CHECKER_NO_TELEMETRY", default_value = "false")]
    no_telemetry: bool,

    #[clap(long, env = "NIX_FLAKE_CHECKER_CHECK_OUTDATED", default_value = "true")]
    check_outdated: bool,

    #[clap(
        long,
        env = "NIX_FLAKE_CHECKER_CHECK_SUPPORTED",
        default_value = "true"
    )]
    check_supported: bool,

    #[clap(long, env = "NIX_FLAKE_CHECKER_CHECK_OWNER", default_value = "true")]
    check_owner: bool,

    /// The path to the flake.lock file to check.
    #[clap(
        env = "NIX_FLAKE_CHECKER_FLAKE_LOCK_PATH",
        default_value = "flake.lock"
    )]
    flake_lock_path: PathBuf,
}

fn main() -> Result<(), FlakeCheckerError> {
    let Cli {
        flake_lock_path,
        no_telemetry,
        check_supported,
        check_outdated,
        check_owner,
    } = Cli::parse();
    let flake_lock_file = read_to_string(flake_lock_path)?;
    let flake_lock: FlakeLock = serde_json::from_str(&flake_lock_file)?;
    let issues = check_flake_lock(&flake_lock, check_supported, check_outdated, check_owner);

    if !no_telemetry {
        telemetry::TelemetryReport::make_and_send(&issues);
    }

    let summary = Summary { issues };
    summary.generate_markdown()?;

    Ok(())
}
