#![allow(dead_code)]
use std::collections::HashMap;

use crate::{Issue, IssueKind, ALLOWED_REFS, MAX_DAYS};

use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::json;

#[derive(Clone, Deserialize)]
pub struct FlakeLock {
    nodes: HashMap<String, Node>,
    root: String,
    version: usize,
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
#[serde(untagged)]
enum Node {
    Root(RootNode),
    Dependency(Box<DependencyNode>),
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

#[derive(Clone, Deserialize)]
#[serde(untagged)]
enum Input {
    String(String),
    List(Vec<String>),
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
struct Original {
    owner: String,
    repo: String,
    #[serde(alias = "type")]
    node_type: String,
    #[serde(alias = "ref")]
    git_ref: Option<String>,
}
