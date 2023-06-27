mod error;
mod flake;
mod issue;
mod summary;
pub mod telemetry;

pub use error::FlakeCheckerError;
pub use flake::{check_flake_lock, FlakeCheckConfig, FlakeLock};
pub use summary::Summary;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

/// A flake.lock checker for Nix projects.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Don't send aggregate sums of each issue type.
    ///
    /// See: https://github.com/determinateSystems/flake-checker.
    #[arg(long, env = "NIX_FLAKE_CHECKER_NO_TELEMETRY", default_value_t = false)]
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

    /// Fail with an exit code of 1 if any issues are encountered.
    #[arg(
        long,
        short,
        env = "NIX_FLAKE_CHECKER_FAIL_MODE",
        default_value_t = false
    )]
    fail_mode: bool,

    /// Nixpkgs input keys as a comma-separated list.
    #[arg(
        long,
        short,
        env = "NIX_FLAKE_CHECKER_NIXPKGS_KEYS",
        default_value = "nixpkgs",
        value_delimiter = ',',
        name = "KEY_LIST"
    )]
    nixpkgs_keys: Vec<String>,
}

fn main() -> Result<ExitCode, FlakeCheckerError> {
    let Cli {
        no_telemetry,
        check_outdated,
        check_owner,
        check_supported,
        ignore_missing_flake_lock,
        flake_lock_path,
        fail_mode,
        nixpkgs_keys,
    } = Cli::parse();

    if !flake_lock_path.exists() {
        if ignore_missing_flake_lock {
            println!("no flake lockfile found at {:?}; ignoring", flake_lock_path);
            return Ok(ExitCode::SUCCESS);
        } else {
            println!("no flake lockfile found at {:?}", flake_lock_path);
            return Ok(ExitCode::FAILURE);
        }
    }

    let flake_lock = FlakeLock::new(&flake_lock_path)?;

    let flake_check_config = FlakeCheckConfig {
        check_supported,
        check_outdated,
        check_owner,
        nixpkgs_keys,
    };

    let issues = check_flake_lock(&flake_lock, &flake_check_config)?;

    if !no_telemetry {
        telemetry::TelemetryReport::make_and_send(&issues);
    }

    let summary = Summary::new(&issues);

    if std::env::var("GITHUB_ACTIONS").is_ok() {
        summary.generate_markdown()?;
    } else {
        summary.generate_text()?;
    }

    if fail_mode && !issues.is_empty() {
        return Ok(ExitCode::FAILURE);
    }

    Ok(ExitCode::SUCCESS)
}
