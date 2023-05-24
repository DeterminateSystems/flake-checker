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
        // TODO: make this more robust for real-world use cases
        self.nodes
            .iter()
            .filter(|(k, v)| matches!(v, Node::Repo(_)) && k == &"nixpkgs")
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub fn check(&self) -> Vec<Issue> {
        let mut issues = vec![];

        for (name, dep) in self.nixpkgs_deps() {
            if let Node::Repo(repo) = dep {
                // Check if not explicitly supported
                if let Some(ref git_ref) = repo.original.git_ref {
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
                let diff = now_timestamp - repo.locked.last_modified;
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
                let owner = repo.original.owner;
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
#[non_exhaustive]
enum Node {
    Root(RootNode),
    Repo(RepoNode),
    Path(PathNode),
    Url(UrlNode),
}

impl Node {
    fn is_nixpkgs(&self) -> bool {
        match self {
            Self::Repo(repo) => {
                repo.locked.node_type == "github" && repo.original.repo == "nixpkgs"
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
struct RepoNode {
    inputs: Option<HashMap<String, Input>>,
    locked: LockedRepo,
    original: OriginalRepo,
}

#[derive(Clone, Deserialize)]
struct PathNode {
    inputs: Option<HashMap<String, Input>>,
    locked: LockedPath,
    original: OriginalPath,
}

#[derive(Clone, Deserialize)]
struct UrlNode {
    inputs: Option<HashMap<String, Input>>,
    locked: LockedUrl,
    original: OriginalUrl,
}

#[derive(Clone, Deserialize)]
#[serde(untagged)]
enum Input {
    String(String),
    List(Vec<String>),
}

#[derive(Clone, Deserialize)]
struct LockedRepo {
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
struct LockedPath {
    #[serde(alias = "lastModified")]
    last_modified: i64,
    #[serde(alias = "narHash")]
    nar_hash: String,
    path: String,
    #[serde(alias = "type")]
    node_type: String,
}

#[derive(Clone, Deserialize)]
struct LockedUrl {
    #[serde(alias = "lastModified")]
    last_modified: i64,
    #[serde(alias = "narHash")]
    nar_hash: String,
    #[serde(alias = "ref")]
    git_ref: String,
    rev: String,
    #[serde(alias = "revCount")]
    rev_count: usize,
    #[serde(alias = "type")]
    node_type: String,
    url: String,
}

#[derive(Clone, Deserialize)]
struct OriginalRepo {
    owner: String,
    repo: String,
    #[serde(alias = "type")]
    node_type: String,
    #[serde(alias = "ref")]
    git_ref: Option<String>,
}

#[derive(Clone, Deserialize)]
struct OriginalPath {
    path: String,
    #[serde(alias = "type")]
    node_type: String,
}

#[derive(Clone, Deserialize)]
struct OriginalUrl {
    #[serde(alias = "type")]
    node_type: String,
    url: String,
}
