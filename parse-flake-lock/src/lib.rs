#![allow(dead_code)]

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::fs::read_to_string;
use std::path::Path;

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

/// A custom error type for the `parse-flake-lock` crate.
#[derive(Debug, thiserror::Error)]
pub enum FlakeLockParseError {
    /// The `flake.lock` can be parsed as JSON but is nonetheless invalid.
    #[error("invalid flake.lock file: {0}")]
    Invalid(String),
    /// The `flake.lock` file couldn't be found.
    #[error("couldn't find the flake.lock file: {0}")]
    NotFound(#[from] std::io::Error),
    /// The specified `flake.lock` file couldn't be parsed as JSON.
    #[error("couldn't parse the flake.lock file as json: {0}")]
    Json(#[from] serde_json::Error),
}

/// A Rust representation of a Nix [`flake.lock`
/// file](https://zero-to-nix.com/concepts/flakes#lockfile).
#[derive(Clone, Debug)]
pub struct FlakeLock {
    /// The `nodes` field of the `flake.lock`, representing all input [Node]s for the flake.
    pub nodes: HashMap<String, Node>,
    /// The `root` of the `flake.lock` with all input references resolved into the corresponding
    /// [Node]s represented by the `nodes` field.
    pub root: HashMap<String, Node>,
    /// The version of the `flake.lock` (incremented whenever the `flake.nix` dependencies are
    /// updated).
    pub version: usize,
}

/// A custom [Deserializer] for `flake.lock` files, which are standard JSON but require some special
/// logic to create a meaningful Rust representation.
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
            Node::Root(_) => None,
            Node::Repo(node) => node.inputs.to_owned(),
            Node::Indirect(_) => None,
            Node::Fallthrough(node) => match node.get("inputs") {
                Some(node_inputs) => serde_json::from_value(node_inputs.clone())
                    .map_err(FlakeLockParseError::Json)?,
                None => None,
            },
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
    /// Instantiate a new [FlakeLock] from the provided [Path].
    pub fn new(path: &Path) -> Result<Self, FlakeLockParseError> {
        let flake_lock_file = read_to_string(path)?;
        let flake_lock: FlakeLock = serde_json::from_str(&flake_lock_file)?;
        Ok(flake_lock)
    }
}

/// A flake input [node]. This enum represents two concrete node types, [RepoNode] and [RootNode],
/// and uses the `Fallthrough` variant to capture node types that don't have explicitly defined
/// structs in this library, representing them as raw [Value][serde_json::value::Value]s.
///
/// [node]: https://nixos.org/manual/nix/stable/command-ref/new-cli/nix3-flake.html#lock-files
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum Node {
    /// A [RootNode] specifying an [Input] map.
    Root(RootNode),
    /// A [RepoNode] flake input for a [Git](https://git-scm.com) repository (or another version
    /// control system).
    Repo(Box<RepoNode>),
    Indirect(IndirectNode),
    /// A "catch-all" variant for node types that don't (yet) have explicit struct definitions in
    /// this crate.
    Fallthrough(serde_json::value::Value), // Covers all other node types
}

impl Node {
    fn variant(&self) -> &'static str {
        match self {
            Node::Root(_) => "Root",
            Node::Repo(_) => "Repo",
            Node::Indirect(_) => "Indirect",
            Node::Fallthrough(_) => "Fallthrough", // Covers all other node types
        }
    }
}

/// An enum type representing node input references.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum Input {
    String(String),
    List(Vec<String>),
}

/// A flake [Node] representing a raw mapping of strings to [Input]s.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootNode {
    /// A mapping of the flake's input [Node]s.
    pub inputs: HashMap<String, Input>,
}

/// A [Node] representing a [Git](https://git-scm.com) repository (or another version control
/// system).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepoNode {
    /// Whether the input is itself a flake.
    pub flake: Option<bool>,
    /// The node's inputs.
    pub inputs: Option<HashMap<String, Input>>,
    /// The "locked" attributes of the input (set by Nix).
    pub locked: Locked,
    /// The "original" (user-supplied) attributes of the input.
    pub original: RepoOriginal,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Locked {
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

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndirectNode {
    pub locked: Locked,
    pub original: IndirectOriginal,
}

#[derive(Clone, Debug, Deserialize)]
pub struct IndirectOriginal {
    id: String,
    #[serde(alias = "type")]
    pub node_type: String,
}
