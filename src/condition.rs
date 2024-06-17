use cel_interpreter::{Context, Program, Value};
use parse_flake_lock::{FlakeLock, Node, RepoNode};

use crate::{
    error::FlakeCheckerError,
    flake::{nixpkgs_deps, num_days_old},
    issue::{Issue, IssueKind},
};

pub(super) fn evaluate_condition(
    flake_lock: &FlakeLock,
    nixpkgs_keys: &[String],
    condition: &str,
    allowed_refs: Vec<String>,
) -> Result<Vec<Issue>, FlakeCheckerError> {
    let mut issues: Vec<Issue> = vec![];
    let mut ctx = Context::default();

    let deps = nixpkgs_deps(flake_lock, nixpkgs_keys)?;

    for (name, dep) in deps {
        if let Node::Repo(repo) = dep {
            let allowed_refs: Value = Value::from(
                allowed_refs
                    .iter()
                    .map(|r| Value::from(r.to_string()))
                    .collect::<Vec<Value>>(),
            );

            ctx.add_variable_from_value("supported_refs", allowed_refs);

            for (k, v) in nixpkgs_cel_values(repo) {
                ctx.add_variable_from_value(k, v);
            }

            let program = Program::compile(condition)?;
            match program.execute(&ctx) {
                Ok(result) => match result {
                    Value::Bool(b) if !b => {
                        issues.push(Issue {
                            input: name.clone(),
                            kind: IssueKind::Violation,
                        });
                    }
                    _ => continue,
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
            "git_ref",
            repo.original
                .git_ref
                .map_or_else(|| Value::Null, Value::from),
        ),
        (
            "days_old",
            Value::from(num_days_old(repo.locked.last_modified)),
        ),
        ("owner", Value::from(repo.original.owner)),
    ]
}
