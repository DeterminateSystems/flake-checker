#![allow(dead_code)]

use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::PathBuf;

use chrono::{Duration, Utc};
use clap::Parser;
use serde::Deserialize;

/// A flake.lock checker for Nix projects.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The path to the flake.lock file to check.
    #[clap(default_value = "flake.lock")]
    flake_lock_path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("couldn't access flake.lock: {0}")]
    Io(#[from] std::io::Error),

    #[error("couldn't parse flake.lock: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Clone, Deserialize)]
struct Original {
    owner: String,
    repo: String,
    r#type: String,
    r#ref: Option<String>,
}

#[derive(Clone, Deserialize)]
struct Locked {
    #[serde(alias = "lastModified")]
    last_modified: i64,
    #[serde(alias = "narHash")]
    nar_hash: String,
    owner: String,
    repo: String,
    rev: String,
    r#type: String,
}

#[derive(Clone, Deserialize)]
struct Node {
    inputs: Option<HashMap<String, String>>,
    locked: Option<Locked>,
    original: Option<Original>,
}

#[derive(Clone, Deserialize)]
struct FlakeLock {
    nodes: HashMap<String, Node>,
    root: String,
    version: usize,
}

#[derive(Deserialize)]
struct Config {
    allowed_refs: Vec<String>,
    max_days: i64,
}

fn nixpkgs_num_days_old(timestamp: i64) -> i64 {
    let now_timestamp = Utc::now().timestamp();
    let diff = now_timestamp - timestamp;
    Duration::seconds(diff).num_days()
}

fn check_for_outdated_nixpkgs(nodes: &HashMap<String, Node>, config: &Config) {
    let nixpkgs_deps = nixpkgs_deps(nodes);
    for (name, dep) in nixpkgs_deps {
        if let Some(locked) = &dep.locked {
            let num_days_old = nixpkgs_num_days_old(locked.last_modified);

            if num_days_old > config.max_days {
                println!(
                    "dependency {} is {} days old, which is over the max of {}",
                    name, num_days_old, config.max_days
                );
            }
        }
    }
}

fn check_for_non_allowed_refs(nodes: &HashMap<String, Node>, config: &Config) {
    let nixpkgs_deps = nixpkgs_deps(nodes);
    for (name, dep) in nixpkgs_deps {
        if let Some(original) = &dep.original {
            if let Some(ref git_ref) = original.r#ref {
                if !config.allowed_refs.contains(&git_ref) {
                    println!(
                        "dependency {} has a Git ref of {} which is not explicitly allowed",
                        name, git_ref
                    );
                }
            }
        }
    }
}

fn check_flake_lock(flake_lock: &FlakeLock, config: &Config) {
    check_for_outdated_nixpkgs(&flake_lock.nodes, config);
    check_for_non_allowed_refs(&flake_lock.nodes, config);
}

fn nixpkgs_deps(nodes: &HashMap<String, Node>) -> HashMap<String, Node> {
    nodes
        .iter()
        .filter(|(k, _)| k.starts_with("nixpkgs"))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

fn main() -> Result<(), Error> {
    let Cli { flake_lock_path } = Cli::parse();
    let flake_lock_file = read_to_string(flake_lock_path)?;
    let flake_lock: FlakeLock = serde_json::from_str(&flake_lock_file)?;

    let config_file = include_str!("policy.json");
    let config: Config = serde_json::from_str(&config_file)?;

    check_flake_lock(&flake_lock, &config);

    Ok(())
}
