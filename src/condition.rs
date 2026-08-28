use cel::{Context, Program, Value};
use parse_flake_lock::FlakeLock;

use std::collections::{BTreeMap, HashMap};

use crate::{
    error::FlakeCheckerError,
    flake::{nixpkgs_deps, num_days_old},
    issue::{Issue, IssueKind},
};

const KEY_GIT_REF: &str = "gitRef";
const KEY_NUM_DAYS_OLD: &str = "numDaysOld";
const KEY_OWNER: &str = "owner";
const KEY_REF_STATUSES: &str = "refStatuses";
const KEY_SUPPORTED_REFS: &str = "supportedRefs";

pub(super) fn evaluate_condition(
    flake_lock: &FlakeLock,
    nixpkgs_keys: &[String],
    condition: &str,
    ref_statuses: BTreeMap<String, String>,
    supported_refs: Vec<String>,
) -> Result<Vec<Issue>, FlakeCheckerError> {
    let mut issues: Vec<Issue> = vec![];
    let mut ctx = Context::default();

    let ref_statuses = ref_statuses
        .into_iter()
        .collect::<HashMap<String, String>>();
    ctx.add_variable_from_value(KEY_REF_STATUSES, ref_statuses);
    ctx.add_variable_from_value(KEY_SUPPORTED_REFS, supported_refs);

    let deps = nixpkgs_deps(flake_lock, nixpkgs_keys)?;

    for (name, node) in deps {
        let git_ref = node.git_ref();
        let last_modified = node.last_modified();
        let owner = node.owner();

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
                    ));
                }
            },
            Err(e) => return Err(FlakeCheckerError::CelExecution(e)),
        }
    }

    Ok(issues)
}

fn add_cel_variables(
    ctx: &mut Context,
    git_ref: Option<&str>,
    last_modified: Option<i64>,
    owner: Option<&str>,
) {
    ctx.add_variable_from_value(KEY_GIT_REF, Value::from(git_ref.unwrap_or("")));
    ctx.add_variable_from_value(
        KEY_NUM_DAYS_OLD,
        Value::from(last_modified.map(num_days_old).unwrap_or(0)),
    );
    ctx.add_variable_from_value(KEY_OWNER, Value::from(owner.unwrap_or("")));
}
