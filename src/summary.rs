use crate::error::FlakeCheckerError;
use crate::flake::{ALLOWED_REFS, MAX_DAYS};
use crate::issue::{Issue, IssueKind};
use crate::FlakeCheckConfig;

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use handlebars::Handlebars;
use serde_json::json;

pub struct Summary {
    pub issues: Vec<Issue>,
    data: serde_json::Value,
    flake_lock_path: PathBuf,
    flake_check_config: FlakeCheckConfig,
    fail_mode: bool,
}

impl Summary {
    pub fn new(
        issues: &Vec<Issue>,
        flake_lock_path: PathBuf,
        flake_check_config: FlakeCheckConfig,
        fail_mode: bool,
    ) -> Self {
        let by_kind =
            |kind: IssueKind| -> Vec<&Issue> { issues.iter().filter(|i| i.kind == kind).collect() };

        let disallowed = by_kind(IssueKind::Disallowed);
        let outdated = by_kind(IssueKind::Outdated);
        let non_upstream = by_kind(IssueKind::NonUpstream);

        let data = json!({
            "issues": issues,
            "num_issues": issues.len(),
            "clean": issues.is_empty(),
            "dirty": !issues.is_empty(),
            "issue_word": if issues.len() == 1 { "issue" } else { "issues" },
            // Disallowed refs
            "has_disallowed": !disallowed.is_empty(),
            "disallowed": disallowed,
            // Outdated refs
            "has_outdated": !outdated.is_empty(),
            "outdated": outdated,
            // Non-upstream refs
            "has_non_upstream": !non_upstream.is_empty(),
            "non_upstream": non_upstream,
            // Constants
            "max_days": MAX_DAYS,
            "supported_ref_names": ALLOWED_REFS,
        });

        Self {
            issues: issues.to_vec(),
            data,
            flake_lock_path,
            flake_check_config,
            fail_mode,
        }
    }

    pub fn console_log_errors(&self) {
        let file = self.flake_lock_path.to_string_lossy();

        let level = if self.fail_mode { "error" } else { "warning" };

        if self.issues.is_empty() {
            // This is only logged if ACTIONS_STEP_DEBUG is set to true. See here:
            // https://docs.github.com/en/actions/using-workflows/workflow-commands-for-github-actions#setting-a-debug-message
            println!(
                "::debug::The Determinate Nix Flake Checker scanned {file} and found no issues"
            );
        } else {
            println!("::group::Nix Flake Checker results");
            for issue in self.issues.iter() {
                let message: Option<String> = if self.flake_check_config.check_supported
                    && matches!(issue.kind, IssueKind::Disallowed)
                {
                    let input = issue.details.get("input").unwrap();
                    let reference = issue.details.get("ref").unwrap();
                    Some(format!(
                        "the {input} input uses the non-supported git branch {reference} for nixpkgs"
                    ))
                } else if self.flake_check_config.check_outdated
                    && matches!(issue.kind, IssueKind::Outdated)
                {
                    let input = issue.details.get("input").unwrap();
                    let num_days_old = issue.details.get("num_days_old").unwrap();
                    Some(format!(
                        "the {input} input is {num_days_old} days old (the max allowed is {MAX_DAYS})"
                    ))
                } else if self.flake_check_config.check_owner
                    && matches!(issue.kind, IssueKind::NonUpstream)
                {
                    let input: &serde_json::Value = issue.details.get("input").unwrap();
                    let owner = issue.details.get("owner").unwrap();
                    Some(format!(
                        "the {input} input has {owner} as an owner rather than the NixOS org"
                    ))
                } else {
                    None
                };

                if let Some(message) = message {
                    println!("::{level} file={file}::{message}");
                }
            }
            println!("::endgroup::");
        }
    }

    pub fn generate_markdown(&self) -> Result<(), FlakeCheckerError> {
        let mut handlebars = Handlebars::new();

        handlebars
            .register_template_string("summary.md", include_str!("templates/summary_md.hbs"))
            .map_err(Box::new)?;
        let summary_md = handlebars.render("summary.md", &self.data)?;

        let summary_md_filepath = std::env::var("GITHUB_STEP_SUMMARY")?;
        let mut summary_md_file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(summary_md_filepath)?;
        summary_md_file.write_all(summary_md.as_bytes())?;

        Ok(())
    }

    pub fn generate_text(&self) -> Result<(), FlakeCheckerError> {
        let mut handlebars = Handlebars::new();
        handlebars
            .register_template_string("summary.txt", include_str!("templates/summary_txt.hbs"))
            .map_err(Box::new)?;

        let summary_txt = handlebars.render("summary.txt", &self.data)?;

        std::io::stdout().write_all(summary_txt.as_bytes())?;

        Ok(())
    }
}
