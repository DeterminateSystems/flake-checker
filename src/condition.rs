use cel_interpreter::{Context, Program, Value};
use parse_flake_lock::{FlakeLock, Node};

use crate::{
    error::FlakeCheckerError,
    flake::{nixpkgs_deps, num_days_old},
    issue::{Issue, IssueKind},
};

const KEY_GIT_REF: &str = "gitRef";
const KEY_NUM_DAYS_OLD: &str = "numDaysOld";
const KEY_OWNER: &str = "owner";
const KEY_SUPPORTED_REFS: &str = "supportedRefs";

pub(super) fn evaluate_condition(
    flake_lock: &FlakeLock,
    nixpkgs_keys: &[String],
    condition: &str,
    supported_refs: Vec<String>,
) -> Result<Vec<Issue>, FlakeCheckerError> {
    let mut issues: Vec<Issue> = vec![];
    let mut ctx = Context::default();
    ctx.add_variable_from_value(KEY_SUPPORTED_REFS, supported_refs);

    let deps = nixpkgs_deps(flake_lock, nixpkgs_keys)?;

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

        add_cel_variables(&mut ctx, git_ref, last_modified, owner);

        match Program::compile(condition)?.execute(&ctx) {
            Ok(result) => match result {
                Value::Bool(b) if !b => {
                    issues.push(Issue {
                        input: name.clone(),
                        kind: IssueKind::Violation,
                    });
                }
                Value::Bool(b) if b => continue,
                result => {
                    return Err(FlakeCheckerError::NonBooleanCondition(
                        result.type_of().to_string(),
                    ))
                }
            },
            Err(e) => return Err(FlakeCheckerError::CelExecution(e)),
        }
    }

    Ok(issues)
}

fn add_cel_variables(
    ctx: &mut Context,
    git_ref: Option<String>,
    last_modified: Option<i64>,
    owner: Option<String>,
) {
    ctx.add_variable_from_value(KEY_GIT_REF, value_or_empty_string(git_ref));
    ctx.add_variable_from_value(
        KEY_NUM_DAYS_OLD,
        value_or_zero(last_modified.map(num_days_old)),
    );
    ctx.add_variable_from_value(KEY_OWNER, value_or_empty_string(owner));
}

fn value_or_empty_string(value: Option<String>) -> Value {
    Value::from(value.unwrap_or(String::from("")))
}

fn value_or_zero(value: Option<i64>) -> Value {
    Value::from(value.unwrap_or(0))
}
