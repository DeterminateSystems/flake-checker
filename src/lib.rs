use serde::Serialize;

mod error;
mod flake;
mod summary;

pub use error::FlakeCheckerError;
pub use flake::FlakeLock;
pub use summary::Summary;

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

// MAYBE: re-introduce logging
fn _warn(path: &str, message: &str) {
    println!("::warning file={path}::{message}");
}
