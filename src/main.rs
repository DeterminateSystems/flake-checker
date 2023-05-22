#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap};
use std::fs::{read_to_string, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use chrono::{Duration, Utc};
use clap::Parser;
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};

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
    owner: Option<String>,
    repo: Option<String>,
    r#type: String,
    r#ref: Option<String>,
}

#[derive(Clone, Deserialize)]
struct Locked {
    #[serde(alias = "lastModified")]
    last_modified: i64,
    #[serde(alias = "narHash")]
    nar_hash: String,
    owner: Option<String>,
    repo: Option<String>,
    rev: Option<String>,
    r#type: String,
}

#[derive(Clone, Deserialize)]
#[serde(untagged)]
enum Input {
    String(String),
    List(Vec<String>),
}

// TODO: make this an enum rather than a struct
#[derive(Clone, Deserialize)]
struct Node {
    inputs: Option<HashMap<String, Input>>,
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

fn write_to_summary(msg: &str) {
    let filepath = std::env::var("GITHUB_STEP_SUMMARY").unwrap();
    println!("Filepath: {filepath}");
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&filepath)
        .unwrap();
    file.write_all(msg.as_bytes()).unwrap();
}

fn check_for_outdated_nixpkgs(
    nodes: &HashMap<String, Node>,
    config: &Config,
) -> Vec<Issue> {
    let mut issues = vec![];
    let nixpkgs_deps = nixpkgs_deps(nodes);
    for (name, dep) in nixpkgs_deps {
        if let Some(locked) = &dep.locked {
            let num_days_old = nixpkgs_num_days_old(locked.last_modified);

            if num_days_old > config.max_days {
                issues.push(Issue {
                    kind: IssueKind::Outdated,
                    message: format!(
                        "dependency {name} is {num_days_old} days old, which is over the max of {}",
                        config.max_days
                    )
                });
            }
        }
    }
    issues
}

fn check_for_non_allowed_refs(
    nodes: &HashMap<String, Node>,
    config: &Config,
) -> Vec<Issue> {
    let mut issues = vec![];
    let nixpkgs_deps = nixpkgs_deps(nodes);
    for (name, dep) in nixpkgs_deps {
        if let Some(original) = &dep.original {
            if let Some(ref git_ref) = original.r#ref {
                if !config.allowed_refs.contains(git_ref) {
                    issues.push(Issue {
                        kind: IssueKind::Disallowed,
                        message: format!("dependency {name} has a Git ref of {git_ref} which is not explicitly allowed"),
                    });

                }
            }
        }
    }
    issues
}

fn check_flake_lock(flake_lock: &FlakeLock, config: &Config) -> Vec<Issue> {
    let mut is1 = check_for_outdated_nixpkgs(&flake_lock.nodes, config);
    let mut is2 = check_for_non_allowed_refs(&flake_lock.nodes, config);

    is1.append(&mut is2); // TODO: find a more elegant way to do this
    is1
}

fn nixpkgs_deps(nodes: &HashMap<String, Node>) -> HashMap<String, Node> {
    nodes
        .iter()
        .filter(|(k, _)| k.starts_with("nixpkgs"))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

fn warn(path: &str, message: &str) {
    println!("::warning file={path}::{message}");
}

#[derive(Serialize)]
enum IssueKind {
    Disallowed,
    Outdated,
}

#[derive(Serialize)]
struct Issue {
    kind: IssueKind,
    message: String,
}

struct Summary {
    issues: Vec<Issue>,
}

impl Summary {
    fn generate_markdown(&self) {
        let mut data = BTreeMap::new();
        data.insert("issues", &self.issues);
        let mut handlebars = Handlebars::new();
        handlebars.register_template_string("summary.md", include_str!("./templates/summary.md")).unwrap();
        let summary_md = handlebars.render("summary.md", &data).unwrap();
        let summary_md_filepath = std::env::var("GITHUB_STEP_SUMMARY").unwrap();
        let mut summary_md_file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&summary_md_filepath)
            .unwrap();
        summary_md_file.write_all(summary_md.as_bytes()).unwrap();
    }
}

fn main() -> Result<(), Error> {
    let Cli { flake_lock_path } = Cli::parse();
    let flake_lock_path = flake_lock_path.as_path().to_str().unwrap(); // TODO: handle this better
    let flake_lock_file = read_to_string(flake_lock_path)?;
    let flake_lock: FlakeLock = serde_json::from_str(&flake_lock_file)?;

    let config_file = include_str!("policy.json");
    let config: Config =
        serde_json::from_str(config_file).expect("inline policy.json file is malformed");

    let issues = check_flake_lock(&flake_lock, &config);
    let summary = Summary { issues };
    let _ = summary.generate_markdown();

    Ok(())
}
