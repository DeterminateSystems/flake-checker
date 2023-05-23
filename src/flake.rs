#![allow(dead_code)]
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;

use chrono::{Duration, Utc};
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::FlakeCheckerError;

// Update this when necessary by running the get-allowed-refs.sh script to fetch
// the current values from monitoring.nixos.org
const ALLOWED_REFS: &[&str; 6] = &[
    "nixos-22.11",
    "nixos-22.11-small",
    "nixos-unstable",
    "nixos-unstable-small",
    "nixpkgs-22.11-darwin",
    "nixpkgs-unstable",
];
const MAX_DAYS: i64 = 30;

#[derive(Clone, Deserialize)]
pub struct FlakeLock {
    nodes: HashMap<String, Node>,
    root: String,
    version: usize,
}

#[derive(Serialize)]
pub struct Issue {
    dependency: String,
    kind: IssueKind,
    details: serde_json::Value,
}

#[derive(Serialize)]
enum IssueKind {
    #[serde(rename = "disallowed")]
    Disallowed,
    #[serde(rename = "outdated")]
    Outdated,
    #[serde(rename = "non-upstream")]
    NonUpstream,
}

pub struct Summary {
    pub issues: Vec<Issue>,
}

impl Summary {
    pub fn generate_markdown(&self) -> Result<(), FlakeCheckerError> {
        let summary_md = if !self.issues.is_empty() {
            // TODO: make this more elegant
            let has_disallowed = !&self.disallowed().is_empty();
            let has_outdated = !&self.outdated().is_empty();
            let has_non_upstream = !&self.non_upstream().is_empty();

            let supported_ref_names = ALLOWED_REFS.map(|r| format!("* `{r}`")).join("\n");

            let data = json!({
                "issues": &self.issues,
                "disallowed": &self.disallowed(),
                "outdated": &self.outdated(),
                "non_upstream": &self.non_upstream(),
                "has_disallowed": has_disallowed,
                "has_outdated": has_outdated,
                "has_non_upstream": has_non_upstream,
                // Constants
                "max_days": MAX_DAYS,
                "supported_ref_names": supported_ref_names,
                // Text snippets
                "supported_refs_explainer": include_str!("./explainers/supported_refs.md"),
                "outdated_deps_explainer": include_str!("./explainers/outdated_deps.md"),
                "upstream_nixpkgs_explainer": include_str!("./explainers/upstream_nixpkgs.md"),
            });

            let mut handlebars = Handlebars::new();
            handlebars
                .register_template_string("summary.md", include_str!("./templates/summary.md"))
                .map_err(Box::new)?;
            handlebars.render("summary.md", &data)?
        } else {
            String::from("## Nix flake dependency check\n\n:check: Your `flake.lock` has a clean bill of health.")
        };

        let summary_md_filepath = std::env::var("GITHUB_STEP_SUMMARY")?;
        let mut summary_md_file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(summary_md_filepath)?;
        summary_md_file.write_all(summary_md.as_bytes())?;

        Ok(())
    }

    fn disallowed(&self) -> Vec<&Issue> {
        self.issues
            .iter()
            .filter(|i| matches!(i.kind, IssueKind::Disallowed))
            .collect()
    }

    fn outdated(&self) -> Vec<&Issue> {
        self.issues
            .iter()
            .filter(|i| matches!(i.kind, IssueKind::Outdated))
            .collect()
    }

    fn non_upstream(&self) -> Vec<&Issue> {
        self.issues
            .iter()
            .filter(|i| matches!(i.kind, IssueKind::NonUpstream))
            .collect()
    }
}

impl FlakeLock {
    fn nixpkgs_deps(&self) -> HashMap<String, Node> {
        self.nodes
            .iter()
            .filter(|(_, v)| v.is_nixpkgs())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub fn check(&self) -> Vec<Issue> {
        let mut issues = vec![];

        for (name, dep) in self.nixpkgs_deps() {
            if let Node::Dependency(dep) = dep {
                // Check if not explicitly supported
                if let Some(ref git_ref) = dep.original.git_ref {
                    if !ALLOWED_REFS.contains(&git_ref.as_str()) {
                        issues.push(Issue {
                            dependency: name.clone(),
                            kind: IssueKind::Disallowed,
                            details: json!({
                                "input": name,
                                "ref": git_ref
                            }),
                        });
                    }
                }

                // Check if outdated
                let now_timestamp = Utc::now().timestamp();
                let diff = now_timestamp - dep.locked.last_modified;
                let num_days_old = Duration::seconds(diff).num_days();

                if num_days_old > MAX_DAYS {
                    issues.push(Issue {
                        dependency: name.clone(),
                        kind: IssueKind::Outdated,
                        details: json!({
                            "input": name,
                            "num_days_old": num_days_old,
                        }),
                    });
                }

                // Check that the GitHub owner is NixOS
                let owner = dep.original.owner;
                if owner.to_lowercase() != "nixos" {
                    issues.push(Issue {
                        dependency: name.clone(),
                        kind: IssueKind::NonUpstream,
                        details: json!({
                            "input": name,
                            "owner": owner,
                        }),
                    });
                }
            }
        }
        issues
    }
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

#[derive(Clone, Deserialize)]
struct DependencyNode {
    inputs: Option<HashMap<String, Input>>,
    locked: Locked,
    original: Original,
}
