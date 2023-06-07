extern crate flake_checker;

use flake_checker::{check_flake_lock, telemetry, FlakeCheckerError, FlakeLock, Summary};

use std::fs::read_to_string;
use std::path::PathBuf;
use std::process::exit;

use clap::Parser;

/// A flake.lock checker for Nix projects.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Send aggregate sums of each issue type.
    ///
    /// See: https://github.com/determinateSystems/flake-checker.
    #[arg(long, env = "NIX_FLAKE_CHECKER_NO_TELEMETRY", default_value_t = true)]
    no_telemetry: bool,

    /// Check for outdated Nixpkgs inputs.
    #[arg(long, env = "NIX_FLAKE_CHECKER_CHECK_OUTDATED", default_value_t = true)]
    check_outdated: bool,

    /// Check that Nixpkgs inputs have "NixOS" as the GitHub owner.
    #[arg(long, env = "NIX_FLAKE_CHECKER_CHECK_OWNER", default_value_t = true)]
    check_owner: bool,

    /// Check that Git refs for Nixpkgs inputs are supported.
    #[arg(
        long,
        env = "NIX_FLAKE_CHECKER_CHECK_SUPPORTED",
        default_value_t = true
    )]
    check_supported: bool,

    /// Ignore a missing flake.lock file.
    #[arg(
        long,
        env = "NIX_FLAKE_CHECKER_IGNORE_MISSING_FLAKE_LOCK",
        default_value_t = true
    )]
    ignore_missing_flake_lock: bool,

    /// The path to the flake.lock file to check.
    #[arg(
        env = "NIX_FLAKE_CHECKER_FLAKE_LOCK_PATH",
        default_value = "flake.lock"
    )]
    flake_lock_path: PathBuf,
}

fn main() -> Result<(), FlakeCheckerError> {
    let Cli {
        no_telemetry,
        check_outdated,
        check_owner,
        check_supported,
        ignore_missing_flake_lock,
        flake_lock_path,
    } = Cli::parse();

    if !flake_lock_path.exists() {
        if ignore_missing_flake_lock {
            println!("no flake lockfile found at {:?}; ignoring", flake_lock_path);
            exit(0);
        } else {
            println!("no flake lockfile found at {:?}", flake_lock_path);
            exit(1);
        }
    }

    let flake_lock_file = read_to_string(flake_lock_path)?;
    let flake_lock: FlakeLock = serde_json::from_str(&flake_lock_file)?;

    let issues = check_flake_lock(&flake_lock, check_supported, check_outdated, check_owner);

    if !no_telemetry {
        telemetry::TelemetryReport::make_and_send(&issues);
    }

    let summary = Summary::new(issues);

    if std::env::var("GITHUB_ACTIONS").is_ok() {
        summary.generate_markdown()?;
    } else {
        summary.generate_text()?;
    }

    Ok(())
}
