#![allow(dead_code)]

use std::collections::{HashMap, VecDeque};
use std::fs::read_to_string;
use std::fmt;
use std::path::Path;

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

#[derive(Debug, thiserror::Error)]
pub enum FlakeLockParseError {
    #[error("invalid flake.lock file: {0}")]
    Invalid(String),
    #[error("couldn't access flake.lock: {0}")]
    Io(#[from] std::io::Error),
    #[error("couldn't parse flake.lock as json: {0}")]
    Json(#[from] serde_json::Error)
}

#[derive(Clone, Debug)]
pub struct FlakeLock {
    pub nodes: HashMap<String, Node>,
    pub root: HashMap<String, Node>,
    pub version: usize,
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
) -> Result<&Node, FlakeLockParseError> {
    let Some(next_input) = inputs.pop_front() else {
        unreachable!("there should always be at least one input");
    };

    let mut node = &nodes[&next_input];
    for input in inputs {
        let maybe_node_inputs = match node {
            Node::Repo(node) => node.inputs.to_owned(),
            Node::Fallthrough(node) => match node.get("inputs") {
                Some(node_inputs) => {
                    serde_json::from_value(node_inputs.clone()).map_err(FlakeLockParseError::Json)?
                }
                None => None,
            },
            Node::Root(_) => None,
        };

        let node_inputs = match maybe_node_inputs {
            Some(node_inputs) => node_inputs,
            None => {
                return Err(FlakeLockParseError::Invalid(format!(
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
    pub fn new(path: &Path) -> Result<Self, FlakeLockParseError> {
        let flake_lock_file = read_to_string(path)?;
        let flake_lock: FlakeLock = serde_json::from_str(&flake_lock_file)?;
        Ok(flake_lock)
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum Node {
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
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum Input {
    String(String),
    List(Vec<String>),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootNode {
    pub inputs: HashMap<String, Input>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepoNode {
    pub inputs: Option<HashMap<String, Input>>,
    pub locked: RepoLocked,
    pub original: RepoOriginal,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RepoLocked {
    #[serde(alias = "lastModified")]
    pub last_modified: i64,
    #[serde(alias = "narHash")]
    pub nar_hash: String,
    pub owner: String,
    pub repo: String,
    pub rev: String,
    #[serde(alias = "type")]
    pub node_type: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RepoOriginal {
    pub owner: String,
    pub repo: String,
    #[serde(alias = "type")]
    pub node_type: String,
    #[serde(alias = "ref")]
    pub git_ref: Option<String>,
}

