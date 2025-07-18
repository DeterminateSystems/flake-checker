mod condition;
mod error;
mod flake;
mod issue;
mod summary;

#[cfg(feature = "ref-statuses")]
mod ref_statuses;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use parse_flake_lock::FlakeLock;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::condition::evaluate_condition;
use error::FlakeCheckerError;
use flake::{check_flake_lock, FlakeCheckConfig};
use summary::Summary;

/// A flake.lock checker for Nix projects.
#[cfg(not(feature = "ref-statuses"))]
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
        default_value = "nixpkgs",
        value_delimiter = ',',
        name = "KEY_LIST"
    )]
    nixpkgs_keys: Vec<String>,

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

#[cfg(not(feature = "ref-statuses"))]
pub(crate) fn supported_refs(ref_statuses: BTreeMap<String, String>) -> Vec<String> {
    let mut return_value: Vec<String> = ref_statuses
        .iter()
        .filter_map(|(channel, status)| {
            if ["rolling", "stable", "deprecated"].contains(&status.as_str()) {
                Some(channel.clone())
            } else {
                None
            }
        })
        .collect();
    return_value.sort();
    return_value
}

#[cfg(not(feature = "ref-statuses"))]
#[tokio::main]
async fn main() -> Result<ExitCode, FlakeCheckerError> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let ref_statuses: BTreeMap<String, String> =
        serde_json::from_str(include_str!("../ref-statuses.json")).unwrap();

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

    let (reporter, worker) = detsys_ids_client::builder!()
        .enable_reporting(!no_telemetry)
        .fact("check_owner", check_owner)
        .fact("check_outdated", check_outdated)
        .fact("check_supported", check_supported)
        .fact("ignore_missing_flake_lock", ignore_missing_flake_lock)
        .fact("flake_lock_path", flake_lock_path.to_string_lossy())
        .fact("fail_mode", fail_mode)
        .fact("condition", condition.as_deref())
        .build_or_default()
        .await;

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

    let allowed_refs = supported_refs(ref_statuses.clone());

    let issues = if let Some(condition) = &condition {
        evaluate_condition(
            &flake_lock,
            &nixpkgs_keys,
            condition,
            ref_statuses,
            allowed_refs.clone(),
        )?
    } else {
        check_flake_lock(&flake_lock, &flake_check_config, allowed_refs.clone())?
    };

    reporter
        .record(
            "flake_issues",
            Some(detsys_ids_client::Map::from_iter([
                (
                    "disallowed".into(),
                    issues
                        .iter()
                        .filter(|issue| issue.kind.is_disallowed())
                        .count()
                        .into(),
                ),
                (
                    "outdated".into(),
                    issues
                        .iter()
                        .filter(|issue| issue.kind.is_outdated())
                        .count()
                        .into(),
                ),
                (
                    "non_upstream".into(),
                    issues
                        .iter()
                        .filter(|issue| issue.kind.is_non_upstream())
                        .count()
                        .into(),
                ),
            ])),
        )
        .await;

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

    drop(reporter);
    worker.wait().await;

    if fail_mode && !issues.is_empty() {
        return Ok(ExitCode::FAILURE);
    }

    Ok(ExitCode::SUCCESS)
}

#[cfg(feature = "ref-statuses")]
#[derive(Parser)]
struct Cli {
    // Check to make sure that Flake Checker is aware of the current supported branches.
    #[arg(long, hide = true)]
    check_ref_statuses: bool,

    // Check to make sure that Flake Checker is aware of the current supported branches.
    #[arg(long, hide = true)]
    get_ref_statuses: bool,
}

#[cfg(feature = "ref-statuses")]
fn main() -> Result<ExitCode, FlakeCheckerError> {
    let Cli {
        check_ref_statuses,
        get_ref_statuses,
    } = Cli::parse();

    if !get_ref_statuses && !check_ref_statuses {
        panic!("You must select either --get-ref-statuses or --check-ref-statuses");
    }

    if get_ref_statuses {
        match ref_statuses::fetch_ref_statuses() {
            Ok(refs) => {
                let json_refs = serde_json::to_string(&refs)?;
                println!("{json_refs}");
                return Ok(ExitCode::SUCCESS);
            }
            Err(e) => {
                println!("Error fetching ref statuses: {}", e);
                return Ok(ExitCode::FAILURE);
            }
        }
    }

    if check_ref_statuses {
        let mut ref_statuses: BTreeMap<String, String> =
            serde_json::from_str(include_str!("../ref-statuses.json")).unwrap();

        match ref_statuses::check_ref_statuses(ref_statuses) {
            Ok(equals) => {
                if equals {
                    println!("The reference statuses sets are up to date.");
                    return Ok(ExitCode::SUCCESS);
                } else {
                    println!(
                        "The reference statuses sets are NOT up to date. Make sure to update."
                    );
                    return Ok(ExitCode::FAILURE);
                }
            }
            Err(e) => {
                println!("Error checking ref statuses: {}", e);
                return Ok(ExitCode::FAILURE);
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}
