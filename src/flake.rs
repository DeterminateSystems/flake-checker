#![allow(dead_code)]
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::fs::read_to_string;
use std::path::Path;

use crate::issue::{Disallowed, Issue, IssueKind, NonUpstream, Outdated};
use crate::FlakeCheckerError;

use chrono::{Duration, Utc};
use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::Deserialize;

// Update this when necessary by running the get-allowed-refs.sh script to fetch
// the current values from monitoring.nixos.org
pub const ALLOWED_REFS: &[&str] = &[
    "nixos-22.11",
    "nixos-22.11-small",
    "nixos-23.05",
    "nixos-23.05-small",
    "nixos-unstable",
    "nixos-unstable-small",
    "nixpkgs-22.11-darwin",
    "nixpkgs-23.05-darwin",
    "nixpkgs-unstable",
];
pub const MAX_DAYS: i64 = 30;

pub(crate) struct FlakeCheckConfig {
    pub check_supported: bool,
    pub check_outdated: bool,
    pub check_owner: bool,
    pub fail_mode: bool,
    pub nixpkgs_keys: Vec<String>,
}

impl Default for FlakeCheckConfig {
    fn default() -> Self {
        Self {
            check_supported: true,
            check_outdated: true,
            check_owner: true,
            fail_mode: false,
            nixpkgs_keys: vec![String::from("nixpkgs")],
        }
    }
}

pub(crate) fn check_flake_lock(
    flake_lock: &FlakeLock,
    config: &FlakeCheckConfig,
) -> Result<Vec<Issue>, FlakeCheckerError> {
    let mut issues = vec![];

    let deps = flake_lock.nixpkgs_deps(config.nixpkgs_keys.clone())?;

    for (name, dep) in deps {
        if let Node::Repo(repo) = dep {
            // Check if not explicitly supported
            if config.check_supported {
                if let Some(ref git_ref) = repo.original.git_ref {
                    if !ALLOWED_REFS.contains(&git_ref.as_str()) {
                        issues.push(Issue {
                            input: name.clone(),
                            kind: IssueKind::Disallowed(Disallowed {
                                reference: git_ref.to_string(),
                            }),
                        });
                    }
                }
            }

            // Check if outdated
            if config.check_outdated {
                let now_timestamp = Utc::now().timestamp();
                let diff = now_timestamp - repo.locked.last_modified;
                let num_days_old = Duration::seconds(diff).num_days();

                if num_days_old > MAX_DAYS {
                    issues.push(Issue {
                        input: name.clone(),
                        kind: IssueKind::Outdated(Outdated { num_days_old }),
                    });
                }
            }

            // Check that the GitHub owner is NixOS
            if config.check_owner {
                let owner = repo.original.owner;
                if owner.to_lowercase() != "nixos" {
                    issues.push(Issue {
                        input: name.clone(),
                        kind: IssueKind::NonUpstream(NonUpstream { owner }),
                    });
                }
            }
        }
    }
    Ok(issues)
}

#[derive(Clone, Debug)]
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
                    return Err(de::Error::custom(format!("root node was not a Root node, but was a {} node", root_node.variant())));
                };

                for (root_name, root_input) in root_node.inputs.iter() {
                    let inputs: VecDeque<String> = match root_input.clone() {
                        Input::String(s) => [s].into(),
                        Input::List(keys) => keys.into(),
                    };

                    let real_node = chase_input_node(&nodes, inputs).map_err(|e| {
                        de::Error::custom(format!("failed to chase input {}: {:?}", root_name, e))
                    })?;
                    root_nodes.insert(root_name.clone(), real_node.clone());
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

fn chase_input_node(
    nodes: &HashMap<String, Node>,
    mut inputs: VecDeque<String>,
) -> Result<&Node, FlakeCheckerError> {
    let Some(next_input) = inputs.pop_front() else {
        unreachable!("there should always be at least one input");
    };

    let mut node = &nodes[&next_input];
    for input in inputs {
        let maybe_node_inputs = match node {
            Node::Repo(node) => node.inputs.to_owned(),
            Node::Fallthrough(node) => match node.get("inputs") {
                Some(node_inputs) => {
                    serde_json::from_value(node_inputs.clone()).map_err(FlakeCheckerError::Json)?
                }
                None => None,
            },
            Node::Root(_) => None,
        };

        let node_inputs = match maybe_node_inputs {
            Some(node_inputs) => node_inputs,
            None => {
                return Err(FlakeCheckerError::Invalid(format!(
                    "lock node should have had some inputs but had none:\n{:?}",
                    node
                )));
            }
        };

        let next_inputs = &node_inputs[&input];
        node = match next_inputs {
            Input::String(s) => &nodes[s],
            Input::List(inputs) => chase_input_node(nodes, inputs.to_owned().into())?,
        };
    }

    Ok(node)
}

impl FlakeLock {
    pub fn new(path: &Path) -> Result<Self, FlakeCheckerError> {
        let flake_lock_file = read_to_string(path)?;
        let flake_lock: FlakeLock = serde_json::from_str(&flake_lock_file)?;
        Ok(flake_lock)
    }

    fn nixpkgs_deps(&self, keys: Vec<String>) -> Result<HashMap<String, Node>, FlakeCheckerError> {
        let mut deps: HashMap<String, Node> = HashMap::new();

        for (key, node) in self.root.clone() {
            if let Node::Repo(_) = node {
                if keys.contains(&key) {
                    deps.insert(key, node);
                }
            }
        }
        let missing: Vec<String> = keys
            .iter()
            .filter(|k| !deps.contains_key(*k))
            .map(String::from)
            .collect();

        if !missing.is_empty() {
            let error_msg = format!(
                "no nixpkgs dependency found for specified {}: {}",
                if missing.len() > 1 { "keys" } else { "key" },
                missing.join(", ")
            );
            return Err(FlakeCheckerError::Invalid(error_msg));
        }

        Ok(deps)
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum Node {
    Repo(Box<RepoNode>),
    Root(RootNode),
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

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum Input {
    String(String),
    List(Vec<String>),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RootNode {
    pub(crate) inputs: HashMap<String, Input>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RepoNode {
    pub(crate) inputs: Option<HashMap<String, Input>>,
    pub(crate) locked: RepoLocked,
    pub(crate) original: RepoOriginal,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct RepoLocked {
    #[serde(alias = "lastModified")]
    pub(crate) last_modified: i64,
    #[serde(alias = "narHash")]
    pub(crate) nar_hash: String,
    pub(crate) owner: String,
    pub(crate) repo: String,
    pub(crate) rev: String,
    #[serde(alias = "type")]
    pub(crate) node_type: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct RepoOriginal {
    pub(crate) owner: String,
    pub(crate) repo: String,
    #[serde(alias = "type")]
    pub(crate) node_type: String,
    #[serde(alias = "ref")]
    pub(crate) git_ref: Option<String>,
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use crate::{
        check_flake_lock,
        issue::{Disallowed, Issue, IssueKind, NonUpstream},
        FlakeCheckConfig, FlakeLock,
    };

    #[test]
    fn test_clean_flake_locks() {
        for n in 0..=4 {
            let path = PathBuf::from(format!("tests/flake.clean.{n}.lock"));
            let flake_lock = FlakeLock::new(&path).expect("couldn't create flake.lock");
            let config = FlakeCheckConfig {
                check_outdated: false,
                ..Default::default()
            };
            let issues = check_flake_lock(&flake_lock, &config)
                .expect("couldn't run check_flake_lock function");
            assert!(issues.is_empty());
        }
    }

    #[test]
    fn test_dirty_flake_locks() {
        let cases: Vec<(&str, Vec<Issue>)> = vec![
            (
                "flake.dirty.0.lock",
                vec![
                    Issue {
                        input: String::from("nixpkgs"),
                        kind: IssueKind::Disallowed(Disallowed {
                            reference: String::from("this-should-fail"),
                        }),
                    },
                    Issue {
                        input: String::from("nixpkgs"),
                        kind: IssueKind::NonUpstream(NonUpstream {
                            owner: String::from("bitcoin-miner-org"),
                        }),
                    },
                ],
            ),
            (
                "flake.dirty.1.lock",
                vec![
                    Issue {
                        input: String::from("nixpkgs"),
                        kind: IssueKind::Disallowed(Disallowed {
                            reference: String::from("probably-nefarious"),
                        }),
                    },
                    Issue {
                        input: String::from("nixpkgs"),
                        kind: IssueKind::NonUpstream(NonUpstream {
                            owner: String::from("pretty-shady"),
                        }),
                    },
                ],
            ),
        ];

        for (file, expected_issues) in cases {
            let path = PathBuf::from(format!("tests/{file}"));
            let flake_lock = FlakeLock::new(&path).expect("couldn't create flake.lock");
            let config = FlakeCheckConfig {
                check_outdated: false,
                ..Default::default()
            };
            let issues = check_flake_lock(&flake_lock, &config)
                .expect("couldn't run check_flake_lock function");
            dbg!(&path);
            assert_eq!(issues, expected_issues);
        }
    }

    #[test]
    fn test_explicit_nixpkgs_keys() {
        let cases: Vec<(&str, Vec<String>, Vec<Issue>)> = vec![(
            "flake.explicit-keys.0.lock",
            vec![String::from("nixpkgs"), String::from("nixpkgs-alt")],
            vec![Issue {
                input: String::from("nixpkgs-alt"),
                kind: IssueKind::NonUpstream(NonUpstream {
                    owner: String::from("seems-pretty-shady"),
                }),
            }],
        )];

        for (file, nixpkgs_keys, expected_issues) in cases {
            let path = PathBuf::from(format!("tests/{file}"));
            let flake_lock = FlakeLock::new(&path).expect("couldn't create flake.lock");
            let config = FlakeCheckConfig {
                check_outdated: false,
                nixpkgs_keys,
                ..Default::default()
            };
            let issues = check_flake_lock(&flake_lock, &config)
                .expect("couldn't run check_flake_lock function");
            assert_eq!(issues, expected_issues);
        }
    }

    #[test]
    fn test_missing_nixpkgs_keys() {
        let cases: Vec<(&str, Vec<String>, String)> = vec![(
            "flake.clean.0.lock",
            vec![String::from("nixpkgs"), String::from("foo"), String::from("bar")],
            String::from("invalid flake.lock: no nixpkgs dependency found for specified keys: foo, bar"),
        ),
        (
            "flake.clean.1.lock",
            vec![String::from("nixpkgs"), String::from("nixpkgs-other")],
            String::from("invalid flake.lock: no nixpkgs dependency found for specified key: nixpkgs-other"),
        )];
        for (file, nixpkgs_keys, expected_err) in cases {
            let path = PathBuf::from(format!("tests/{file}"));
            let flake_lock = FlakeLock::new(&path).expect("couldn't create flake.lock");
            let config = FlakeCheckConfig {
                check_outdated: false,
                nixpkgs_keys,
                ..Default::default()
            };

            let result = check_flake_lock(&flake_lock, &config);

            assert!(result.is_err());
            assert_eq!(result.unwrap_err().to_string(), expected_err);
        }
    }
}
