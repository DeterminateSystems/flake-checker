#![allow(dead_code)]

use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::PathBuf;

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use clap::Parser;
use serde::Deserialize;

const MAX_DAYS: usize = 30;

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    path: PathBuf,
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

fn nixpkgs_too_old(timestamp: i64) -> bool {
    let date_from_timestamp = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp(timestamp, 0),
        Utc,
    );

    let thirty_days_ago = Utc::now() - Duration::days(30);

    date_from_timestamp < thirty_days_ago
}

fn check_for_outdated_nixpkgs(nodes: &HashMap<String, Node>) {
    let nixpkgs_deps = nixpkgs_deps(nodes);
    for dep in nixpkgs_deps.values() {
       if let Some(locked) = &dep.locked {
            if nixpkgs_too_old(locked.last_modified) {
                println!("TOO OLD");
            }
       }
    }
}

fn check_flake_lock(flake_lock: &FlakeLock) {
    check_for_outdated_nixpkgs(&flake_lock.nodes);
}

fn nixpkgs_deps(nodes: &HashMap<String, Node>) -> HashMap<String, Node> {
    nodes
        .iter()
        .filter(|(k, _)| k.starts_with("nixpkgs"))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

fn main() -> Result<(), Error> {
    let Cli { path } = Cli::parse();
    let file = read_to_string(path)?;
    let flake_lock: FlakeLock = serde_json::from_str(&file)?;
    check_flake_lock(&flake_lock);

    Ok(())
}
