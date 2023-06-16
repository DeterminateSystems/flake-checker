use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct Issue {
    pub dependency: String,
    pub kind: IssueKind,
    pub details: serde_json::Value,
}

#[derive(Clone, PartialEq, Serialize)]
pub enum IssueKind {
    #[serde(rename = "disallowed")]
    Disallowed,
    #[serde(rename = "outdated")]
    Outdated,
    #[serde(rename = "non-upstream")]
    NonUpstream,
}
