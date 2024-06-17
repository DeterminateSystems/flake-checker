use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Issue {
    pub input: String,
    pub kind: IssueKind,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub(crate) enum IssueKind {
    Disallowed(Disallowed),
    Outdated(Outdated),
    NonUpstream(NonUpstream),
    Violation,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Disallowed {
    pub(crate) reference: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Outdated {
    pub(crate) num_days_old: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct NonUpstream {
    pub(crate) owner: String,
}

impl IssueKind {
    pub(crate) fn is_disallowed(&self) -> bool {
        matches!(self, Self::Disallowed(_))
    }

    pub(crate) fn is_outdated(&self) -> bool {
        matches!(self, Self::Outdated(_))
    }

    pub(crate) fn is_non_upstream(&self) -> bool {
        matches!(self, Self::NonUpstream(_))
    }

    pub(crate) fn is_violation(&self) -> bool {
        matches!(self, Self::Violation)
    }
}
