#![allow(dead_code)]
extern crate flake_checker;

use std::collections::{BTreeMap, HashMap};
use std::fs::{read_to_string, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use chrono::{Duration, Utc};
use clap::Parser;
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};

use flake_checker::TopLevel;

const ALLOWED_REFS_ENDPOINT: &str = "https://monitoring.nixos.org/prometheus/api/v1/query?query=channel_revision";
const MAX_DAYS: i64 = 30;

fn get_allowed_refs() -> Result<Vec<String>, FlakeCheckerError> {
    let resp: TopLevel = reqwest::blocking::get(ALLOWED_REFS_ENDPOINT)?
        .json::<TopLevel>()?;

    let mut branches = vec![];
    for result in resp.data.result {
        if result.metric.current == "1" {
            branches.push(result.metric.channel);
        }
    }
    Ok(branches)
}

/// A flake.lock checker for Nix projects.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The path to the flake.lock file to check.
    #[clap(default_value = "flake.lock")]
    flake_lock_path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
enum FlakeCheckerError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("couldn't access flake.lock: {0}")]
    Io(#[from] std::io::Error),
    #[error("couldn't parse flake.lock: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Clone, Deserialize)]
struct Original {
    owner: String,
    repo: String,
    #[serde(alias = "type")]
    node_type: String,
    #[serde(alias = "ref")]
    git_ref: Option<String>,
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
    #[serde(alias = "type")]
    node_type: String,
}

#[derive(Clone, Deserialize)]
#[serde(untagged)]
enum Input {
    String(String),
    List(Vec<String>),
}

#[derive(Clone, Deserialize)]
#[serde(untagged)]
enum Node {
    Dependency(DependencyNode),
    Root(RootNode),
}

impl Node {
    fn is_nixpkgs(&self) -> bool {
        match self {
            Self::Dependency(dep) => dep.locked.node_type == "github" && dep.original.repo == "nixpkgs",
            _ => false,
        }
    }
}

#[derive(Clone, Deserialize)]
struct RootNode {
    inputs: HashMap<String, String>,
}

// TODO: make this an enum rather than a struct
#[derive(Clone, Deserialize)]
struct DependencyNode {
    inputs: Option<HashMap<String, Input>>,
    locked: Locked,
    original: Original,
}

#[derive(Clone, Deserialize)]
struct FlakeLock {
    nodes: HashMap<String, Node>,
    root: String,
    version: usize,
}


trait Check {
    fn run(&self, flake_lock: &FlakeLock) -> Vec<Issue>;
}

struct Refs;

impl Check for Refs {
    fn run(&self, flake_lock: &FlakeLock) -> Vec<Issue> {
        let allowed_refs = get_allowed_refs().unwrap(); // TODO: handle this better

        let mut issues = vec![];
        let nixpkgs_deps = nixpkgs_deps(&flake_lock.nodes);
        for (name, dep) in nixpkgs_deps {
            if let Node::Dependency(dep) = dep {
                if let Some(ref git_ref) = dep.original.git_ref {
                    if !allowed_refs.contains(git_ref) {
                        issues.push(Issue {
                        kind: IssueKind::Disallowed,
                        message: format!("dependency `{name}` has a Git ref of `{git_ref}` which is not explicitly allowed"),
                    });
                    }
                }
            }
        }
        issues
    }
}

struct MaxAge;

impl Check for MaxAge {
    fn run(&self, flake_lock: &FlakeLock) -> Vec<Issue> {
        let mut issues = vec![];
        let nixpkgs_deps = nixpkgs_deps(&flake_lock.nodes);
        for (name, dep) in nixpkgs_deps {
            if let Node::Dependency(dep) = dep {
                let now_timestamp = Utc::now().timestamp();
                let diff = now_timestamp - dep.locked.last_modified;
                let num_days_old = Duration::seconds(diff).num_days();

                if num_days_old > MAX_DAYS {
                    issues.push(Issue {
                        kind: IssueKind::Outdated,
                        message: format!(
                            "dependency `{name}` is **{num_days_old}** days old, which is over the max of **{}**",
                            MAX_DAYS
                        ),
                    });
                }
            }
        }
        issues
    }
}

#[derive(Deserialize)]
struct Config {
    allowed_refs: Vec<String>,
    max_days: i64,
}

fn check_flake_lock(flake_lock: &FlakeLock) -> Vec<Issue> {
    let mut is1 = (MaxAge)
    .run(flake_lock);

    let mut is2 = (Refs)
    .run(flake_lock);

    // TODO: find a more elegant way to concat results
    is1.append(&mut is2);
    is1
}

fn nixpkgs_deps(nodes: &HashMap<String, Node>) -> HashMap<String, Node> {
    // TODO: select based on locked.type="github" and original.repo="nixpkgs"
    nodes
        .iter()
        .filter(|(_, v)| v.is_nixpkgs())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

// TODO: re-introduce logging
fn warn(path: &str, message: &str) {
    println!("::warning file={path}::{message}");
}

#[derive(Serialize)]
enum IssueKind {
    #[serde(rename = "disallowed")]
    Disallowed,
    #[serde(rename = "outdated")]
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
        handlebars
            .register_template_string("summary.md", include_str!("./templates/summary.md"))
            .expect("summary template not found");
        let summary_md = handlebars
            .render("summary.md", &data)
            .expect("markdown render error");
        let summary_md_filepath =
            std::env::var("GITHUB_STEP_SUMMARY").expect("summary markdown file not found");
        let mut summary_md_file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(summary_md_filepath)
            .expect("error creating/reading summary markdown file");
        summary_md_file
            .write_all(summary_md.as_bytes())
            .expect("error writing summary markdown to file");
    }
}

fn main() -> Result<(), FlakeCheckerError> {
    let Cli { flake_lock_path } = Cli::parse();
    let flake_lock_path = flake_lock_path
        .as_path()
        .to_str()
        .expect("flake.lock file not found based on supplied path"); // TODO: handle this better
    let flake_lock_file = read_to_string(flake_lock_path)?;
    let flake_lock: FlakeLock = serde_json::from_str(&flake_lock_file)?;

    let issues = check_flake_lock(&flake_lock);
    let summary = Summary { issues };
    summary.generate_markdown();

    Ok(())
}
