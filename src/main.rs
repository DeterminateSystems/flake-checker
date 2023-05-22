#![allow(dead_code)]
use std::collections::{BTreeMap, HashMap};
use std::fs::{read_to_string, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use chrono::{Duration, Utc};
use clap::Parser;
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};

const ALLOWED_REFS: &[&str; 6] = &[
    "nixos-22.11",
    "nixos-22.11-small",
    "nixos-unstable",
    "nixos-unstable-small",
    "nixpkgs-22.11-darwin",
    "nixpkgs-unstable",
];
const MAX_DAYS: i64 = 30;

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
    Dependency(Box<DependencyNode>),
    Root(RootNode),
}

impl Node {
    fn is_nixpkgs(&self) -> bool {
        match self {
            Self::Dependency(dep) => {
                dep.locked.node_type == "github" && dep.original.repo == "nixpkgs"
            }
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

struct AllowedRefs;

impl Check for AllowedRefs {
    fn run(&self, flake_lock: &FlakeLock) -> Vec<Issue> {
        let mut issues = vec![];
        let nixpkgs_deps = nixpkgs_deps(&flake_lock.nodes);
        for (name, dep) in nixpkgs_deps {
            if let Node::Dependency(dep) = dep {
                if let Some(ref git_ref) = dep.original.git_ref {
                    if !ALLOWED_REFS.contains(&git_ref.as_str()) {
                        let supported_ref_names = ALLOWED_REFS.map(|r| format!("`{r}`")).join(", ");
                        issues.push(Issue {
                            dependency: name.clone(),
                            kind: IssueKind::Disallowed,
                            message: format!("The flake input named `{name}` has a Git ref of `{git_ref}` which is not a supported branch. Consider updating to one of these: {supported_ref_names}."),
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
                        dependency: name.clone(),
                        kind: IssueKind::Outdated,
                        message: format!(
                            "The flake input named `{name}` hasn't been updated in **{num_days_old}** days, which is over the allowed {MAX_DAYS}. Consider automating `flake.lock` updates with the [`update-flake-lock` Action](https://github.com/DeterminateSystems/update-flake-lock).",

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
    let mut is1 = MaxAge.run(flake_lock);
    let mut is2 = AllowedRefs.run(flake_lock);

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
    dependency: String,
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
