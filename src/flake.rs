#![allow(dead_code)]

use std::collections::HashMap;

use crate::issue::{Disallowed, Issue, IssueKind, NonUpstream, Outdated};
use crate::FlakeCheckerError;

use chrono::{Duration, Utc};
use parse_flake_lock::{FlakeLock, Node};

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

fn nixpkgs_deps(
    flake_lock: &FlakeLock,
    keys: Vec<String>,
) -> Result<HashMap<String, Node>, FlakeCheckerError> {
    let mut deps: HashMap<String, Node> = HashMap::new();

    for (ref key, node) in flake_lock.root.clone() {
        if let Node::Repo(_) = &node {
            if keys.contains(key) {
                deps.insert(key.to_string(), node.clone());
            }
        }

        if let Node::Indirect(indirect_node) = &node {
            if &indirect_node.original.id == key {
                deps.insert(key.to_string(), node);
            }
        }

        // NOTE: it's unclear that a path node for Nixpkgs should be accepted
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

pub(crate) fn check_flake_lock(
    flake_lock: &FlakeLock,
    config: &FlakeCheckConfig,
) -> Result<Vec<Issue>, FlakeCheckerError> {
    let mut issues = vec![];

    let deps = nixpkgs_deps(flake_lock, config.nixpkgs_keys.clone())?;

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
        for n in 0..=7 {
            let path = PathBuf::from(format!("tests/flake.clean.{n}.lock"));
            let flake_lock = FlakeLock::new(&path).expect("couldn't create flake.lock");
            let config = FlakeCheckConfig {
                check_outdated: false,
                ..Default::default()
            };
            let issues = check_flake_lock(&flake_lock, &config).expect(&format!(
                "couldn't run check_flake_lock function in {path:?}"
            ));
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
