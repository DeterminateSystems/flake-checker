use cel_interpreter::{Context, Program, Value};
use parse_flake_lock::{FlakeLock, Node, RepoNode};

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
    allowed_refs: Vec<String>,
) -> Result<Vec<Issue>, FlakeCheckerError> {
    let mut issues: Vec<Issue> = vec![];

    let allowed_refs: Value = Value::from(
        allowed_refs
            .iter()
            .map(|r| Value::from(r.to_string()))
            .collect::<Vec<Value>>(),
    );

    let deps = nixpkgs_deps(flake_lock, nixpkgs_keys)?;

    for (name, dep) in deps {
        if let Node::Repo(repo) = dep {
            let mut ctx = Context::default();
            ctx.add_variable_from_value(KEY_SUPPORTED_REFS, allowed_refs.clone());
            for (k, v) in nixpkgs_cel_values(repo) {
                ctx.add_variable_from_value(k, v);
            }

            match Program::compile(condition)?.execute(&ctx) {
                Ok(result) => match result {
                    Value::Bool(b) if !b => {
                        issues.push(Issue {
                            input: name.clone(),
                            kind: IssueKind::Violation,
                        });
                    }
                    Value::Bool(b) if b => continue,
                    result => return Err(FlakeCheckerError::InvalidCelCondition(format!("CEL conditions must return a Boolean but your supplied condition returned a {}", result.type_of()))),
                },
                Err(e) => return Err(FlakeCheckerError::CelExecution(e)),
            }
        }
    }

    Ok(issues)
}

fn nixpkgs_cel_values(repo: Box<RepoNode>) -> Vec<(&'static str, Value)> {
    vec![
        (
            KEY_GIT_REF,
            repo.original
                .git_ref
                .map_or_else(|| Value::Null, Value::from),
        ),
        (
            KEY_NUM_DAYS_OLD,
            Value::from(num_days_old(repo.locked.last_modified)),
        ),
        (KEY_OWNER, Value::from(repo.original.owner)),
    ]
}
