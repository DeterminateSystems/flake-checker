#![allow(dead_code)]

use std::collections::HashMap;

use crate::issue::{Disallowed, Issue, IssueKind, NonUpstream, Outdated};
use crate::FlakeCheckerError;

use chrono::{Duration, Utc};
use parse_flake_lock::{FlakeLock, Node};

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

pub(super) fn nixpkgs_deps(
    flake_lock: &FlakeLock,
    keys: &[String],
) -> Result<HashMap<String, Node>, FlakeCheckerError> {
    let mut deps: HashMap<String, Node> = HashMap::new();

    for (ref key, node) in flake_lock.root.clone() {
        match &node {
            Node::Repo(_) => {
                if keys.contains(key) {
                    deps.insert(key.to_string(), node);
                }
            }
            Node::Tarball(_) => {
                if keys.contains(key) {
                    deps.insert(key.to_string(), node);
                }
            }
            Node::Indirect(indirect_node) => {
                if keys.contains(key) && &indirect_node.original.id == key {
                    deps.insert(key.to_string(), node);
                }
            }
            _ => {
                // NOTE: it's unclear that a path node for Nixpkgs should be accepted
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

pub(crate) fn check_flake_lock(
    flake_lock: &FlakeLock,
    config: &FlakeCheckConfig,
    allowed_refs: Vec<String>,
) -> Result<Vec<Issue>, FlakeCheckerError> {
    let mut issues = vec![];

    let deps = nixpkgs_deps(flake_lock, &config.nixpkgs_keys)?;

    for (name, node) in deps {
        let (git_ref, last_modified, owner) = match node {
            Node::Repo(repo) => (
                repo.original.git_ref,
                Some(repo.locked.last_modified),
                Some(repo.original.owner),
            ),
            Node::Tarball(tarball) => (None, tarball.locked.last_modified, None),
            _ => (None, None, None),
        };

        // Check if not explicitly supported
        if let Some(git_ref) = git_ref {
            // Check if not explicitly supported
            if config.check_supported && !allowed_refs.contains(&git_ref) {
                issues.push(Issue {
                    input: name.clone(),
                    kind: IssueKind::Disallowed(Disallowed {
                        reference: git_ref.to_string(),
                    }),
                });
            }
        }

        if let Some(last_modified) = last_modified {
            // Check if outdated
            if config.check_outdated {
                let num_days_old = num_days_old(last_modified);

                if num_days_old > MAX_DAYS {
                    issues.push(Issue {
                        input: name.clone(),
                        kind: IssueKind::Outdated(Outdated { num_days_old }),
                    });
                }
            }
        }

        if let Some(owner) = owner {
            // Check that the GitHub owner is NixOS
            if config.check_owner && owner.to_lowercase() != "nixos" {
                issues.push(Issue {
                    input: name.clone(),
                    kind: IssueKind::NonUpstream(NonUpstream { owner }),
                });
            }
        }
    }
    Ok(issues)
}

pub(super) fn num_days_old(timestamp: i64) -> i64 {
    let now_timestamp = Utc::now().timestamp();
    let diff = now_timestamp - timestamp;
    Duration::seconds(diff).num_days()
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use crate::{
        check_flake_lock,
        condition::evaluate_condition,
        issue::{Disallowed, Issue, IssueKind, NonUpstream},
        FlakeCheckConfig, FlakeLock,
    };

    #[test]
    fn cel_conditions() {
        // (condition, expected)
        let cases: Vec<(&str, bool)> = vec![
            (include_str!("../tests/cel-condition.txt"), true),
            (

                "has(gitRef) && has(numDaysOld) && has(owner) && has(supportedRefs) && supportedRefs.contains(gitRef) && owner != 'NixOS'",
                false,
            ),
            (

                "has(gitRef) && has(numDaysOld) && has(owner) && has(supportedRefs) && supportedRefs.contains(gitRef) && owner != 'NixOS'",
                false,
            ),
        ];

        let supported_refs: Vec<String> =
            serde_json::from_str(include_str!("../allowed-refs.json")).unwrap();
        let path = PathBuf::from("tests/flake.cel.0.lock");

        for (condition, expected) in cases {
            let flake_lock = FlakeLock::new(&path).unwrap();
            let config = FlakeCheckConfig {
                nixpkgs_keys: vec![String::from("nixpkgs")],
                ..Default::default()
            };

            let result = evaluate_condition(
                &flake_lock,
                &config.nixpkgs_keys,
                condition,
                supported_refs.clone(),
            );

            if expected {
                assert!(result.is_ok());
                assert!(result.unwrap().is_empty());
            } else {
                assert!(!result.unwrap().is_empty());
            }
        }
    }

    #[test]
    fn clean_flake_locks() {
        let allowed_refs: Vec<String> =
            serde_json::from_str(include_str!("../allowed-refs.json")).unwrap();
        for n in 0..=7 {
            let path = PathBuf::from(format!("tests/flake.clean.{n}.lock"));
            let flake_lock = FlakeLock::new(&path).unwrap();
            let config = FlakeCheckConfig {
                check_outdated: false,
                ..Default::default()
            };
            let issues = check_flake_lock(&flake_lock, &config, allowed_refs.clone())
                .unwrap_or_else(|_| panic!("couldn't run check_flake_lock function in {path:?}"));
            assert!(
                issues.is_empty(),
                "expected clean flake.lock in tests/flake.clean.{n}.lock but encountered an issue"
            );
        }
    }

    #[test]
    fn dirty_flake_locks() {
        let allowed_refs: Vec<String> =
            serde_json::from_str(include_str!("../allowed-refs.json")).unwrap();
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
            let flake_lock = FlakeLock::new(&path).unwrap();
            let config = FlakeCheckConfig {
                check_outdated: false,
                ..Default::default()
            };
            let issues = check_flake_lock(&flake_lock, &config, allowed_refs.clone()).unwrap();
            dbg!(&path);
            assert_eq!(issues, expected_issues);
        }
    }

    #[test]
    fn explicit_nixpkgs_keys() {
        let allowed_refs: Vec<String> =
            serde_json::from_str(include_str!("../allowed-refs.json")).unwrap();
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
            let flake_lock = FlakeLock::new(&path).unwrap();
            let config = FlakeCheckConfig {
                check_outdated: false,
                nixpkgs_keys,
                ..Default::default()
            };
            let issues = check_flake_lock(&flake_lock, &config, allowed_refs.clone()).unwrap();
            assert_eq!(issues, expected_issues);
        }
    }

    #[test]
    fn missing_nixpkgs_keys() {
        let allowed_refs: Vec<String> =
            serde_json::from_str(include_str!("../allowed-refs.json")).unwrap();
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
            let flake_lock = FlakeLock::new(&path).unwrap();
            let config = FlakeCheckConfig {
                check_outdated: false,
                nixpkgs_keys,
                ..Default::default()
            };

            let result = check_flake_lock(&flake_lock, &config, allowed_refs.clone());

            assert!(result.is_err());
            assert_eq!(result.unwrap_err().to_string(), expected_err);
        }
    }
}
