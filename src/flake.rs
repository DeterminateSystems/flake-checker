#![allow(dead_code)]
use std::collections::HashMap;
use std::fmt;

use crate::{Issue, IssueKind, ALLOWED_REFS, MAX_DAYS};

use chrono::{Duration, Utc};
use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::Deserialize;
use serde_json::json;

pub fn check_flake_lock(
    flake_lock: &FlakeLock,
    check_supported: bool,
    check_outdated: bool,
    check_owner: bool,
) -> Vec<Issue> {
    let mut issues = vec![];

    for (name, dep) in flake_lock.nixpkgs_deps() {
        if let Node::Repo(repo) = dep {
            // Check if not explicitly supported
            if check_supported {
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
            }

            // Check if outdated
            if check_outdated {
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
            }

            // Check that the GitHub owner is NixOS
            if check_owner {
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
    }
    issues
}

#[derive(Clone)]
pub struct FlakeLock {
    nodes: HashMap<String, Node>,
    root: HashMap<String, Node>,
    version: usize,
}

impl<'de> Deserialize<'de> for FlakeLock {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Nodes,
            Root,
            Version,
        }

        struct FlakeLockVisitor;

        impl<'de> Visitor<'de> for FlakeLockVisitor {
            type Value = FlakeLock;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct FlakeLock")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut nodes = None;
                let mut root = None;
                let mut version = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Nodes => {
                            if nodes.is_some() {
                                return Err(de::Error::duplicate_field("nodes"));
                            }
                            nodes = Some(map.next_value()?);
                        }
                        Field::Root => {
                            if root.is_some() {
                                return Err(de::Error::duplicate_field("root"));
                            }
                            root = Some(map.next_value()?);
                        }
                        Field::Version => {
                            if version.is_some() {
                                return Err(de::Error::duplicate_field("version"));
                            }
                            version = Some(map.next_value()?);
                        }
                    }
                }
                let nodes: HashMap<String, Node> =
                    nodes.ok_or_else(|| de::Error::missing_field("nodes"))?;
                let root: String = root.ok_or_else(|| de::Error::missing_field("root"))?;
                let version: usize = version.ok_or_else(|| de::Error::missing_field("version"))?;

                let mut root_nodes = HashMap::new();
                let root_node = &nodes[&root];
                let Node::Root(root_node) = root_node else {
                    panic!("root node was not a Root node, but was a {} node", root_node.variant());
                };
                for (root_name, root_reference) in root_node.inputs.iter() {
                    let root_reference = root_reference.as_str();
                    let root_node = nodes[root_reference].to_owned();
                    root_nodes.insert(root_name.to_owned(), root_node);
                }

                Ok(FlakeLock {
                    nodes,
                    root: root_nodes,
                    version,
                })
            }
        }

        deserializer.deserialize_any(FlakeLockVisitor)
    }
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
}

#[derive(Clone, Deserialize)]
#[serde(untagged)]
enum Node {
    Root(RootNode),
    Repo(Box<RepoNode>),
    Fallthrough(serde_json::value::Value), // Covers all other node types
}

impl Node {
    fn variant(&self) -> &'static str {
        match self {
            Node::Root(_) => "Root",
            Node::Repo(_) => "Repo",
            Node::Fallthrough(_) => "Fallthrough", // Covers all other node types
        }
    }

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
#[serde(untagged)]
enum Input {
    String(String),
    List(Vec<String>),
}

#[derive(Clone, Deserialize)]
struct RootNode {
    inputs: HashMap<String, String>,
}

#[derive(Clone, Deserialize)]
struct RepoNode {
    inputs: Option<HashMap<String, Input>>,
    locked: RepoLocked,
    original: RepoOriginal,
}

#[derive(Clone, Deserialize)]
struct RepoLocked {
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
struct RepoOriginal {
    owner: String,
    repo: String,
    #[serde(alias = "type")]
    node_type: String,
    #[serde(alias = "ref")]
    git_ref: Option<String>,
}
