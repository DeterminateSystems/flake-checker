mod condition;
mod error;
mod flake;
mod issue;
mod summary;
mod telemetry;

#[cfg(feature = "allowed-refs")]
mod allowed_refs;

use error::FlakeCheckerError;
use flake::{check_flake_lock, FlakeCheckConfig};
use summary::Summary;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use parse_flake_lock::FlakeLock;

use crate::condition::evaluate_condition;

/// A flake.lock checker for Nix projects.
#[cfg(not(feature = "allowed-refs"))]
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Don't send aggregate sums of each issue type.
    ///
    /// See <https://github.com/determinateSystems/flake-checker>.
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
        value_delimiter = ',',
        name = "KEY_LIST"
    )]
    nixpkgs_keys: Option<Vec<String>>,

    /// Display Markdown summary (in GitHub Actions).
    #[arg(
        long,
        short,
        env = "NIX_FLAKE_CHECKER_MARKDOWN_SUMMARY",
        default_value_t = true
    )]
    markdown_summary: bool,

    /// The Common Expression Language (CEL) policy to apply to each Nixpkgs input.
    #[arg(long, short, env = "NIX_FLAKE_CHECKER_CONDITION")]
    condition: Option<String>,
}

#[cfg(not(feature = "allowed-refs"))]
fn main() -> Result<ExitCode, FlakeCheckerError> {
    let allowed_refs: Vec<String> =
        serde_json::from_str(include_str!("../allowed-refs.json")).unwrap();

    let Cli {
        no_telemetry,
        check_outdated,
        check_owner,
        check_supported,
        ignore_missing_flake_lock,
        flake_lock_path,
        fail_mode,
        nixpkgs_keys,
        markdown_summary,
        condition,
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
        nixpkgs_keys: nixpkgs_keys.clone(),
        fail_mode,
    };

    let issues = if let Some(condition) = &condition {
        evaluate_condition(
            &flake_lock,
            &flake_check_config,
            condition,
            allowed_refs.clone(),
        )?
    } else {
        check_flake_lock(&flake_lock, &flake_check_config, allowed_refs.clone())?
    };

    if !no_telemetry {
        telemetry::TelemetryReport::make_and_send(&issues);
    }

    let summary = Summary::new(
        &issues,
        flake_lock_path,
        flake_check_config,
        allowed_refs,
        condition,
    );

    if std::env::var("GITHUB_ACTIONS").is_ok() {
        if markdown_summary {
            summary.generate_markdown()?;
        }
        summary.console_log_errors()?;
    } else {
        summary.generate_text()?;
    }

    if fail_mode && !issues.is_empty() {
        return Ok(ExitCode::FAILURE);
    }

    Ok(ExitCode::SUCCESS)
}

#[cfg(feature = "allowed-refs")]
#[derive(Parser)]
struct Cli {
    // Check to make sure that Flake Checker is aware of the current supported branches.
    #[arg(long, hide = true)]
    check_allowed_refs: bool,

    // Check to make sure that Flake Checker is aware of the current supported branches.
    #[arg(long, hide = true)]
    get_allowed_refs: bool,
}

#[cfg(feature = "allowed-refs")]
fn main() -> Result<ExitCode, FlakeCheckerError> {
    let Cli {
        check_allowed_refs,
        get_allowed_refs,
    } = Cli::parse();

    if !get_allowed_refs && !check_allowed_refs {
        panic!("You must select either --get-allowed-refs or --check-allowed-refs");
    }

    if get_allowed_refs {
        match allowed_refs::fetch_allowed_refs() {
            Ok(refs) => {
                let json_refs = serde_json::to_string(&refs)?;
                println!("{json_refs}");
                return Ok(ExitCode::SUCCESS);
            }
            Err(e) => {
                println!("Error fetching allowed refs: {}", e);
                return Ok(ExitCode::FAILURE);
            }
        }
    }

    if check_allowed_refs {
        let mut allowed_refs: Vec<String> =
            serde_json::from_str(include_str!("../allowed-refs.json")).unwrap();

        allowed_refs.sort();

        match allowed_refs::check_allowed_refs(allowed_refs) {
            Ok(equals) => {
                if equals {
                    println!("The allowed reference sets are up to date.");
                    return Ok(ExitCode::SUCCESS);
                } else {
                    println!("The allowed reference sets are NOT up to date. Make sure to update.");
                    return Ok(ExitCode::FAILURE);
                }
            }
            Err(e) => {
                println!("Error checking allowed refs: {}", e);
                return Ok(ExitCode::FAILURE);
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}
