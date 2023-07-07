use crate::error::FlakeCheckerError;
use crate::flake::{ALLOWED_REFS, MAX_DAYS};
use crate::issue::{Issue, IssueKind};
use crate::FlakeCheckConfig;

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use handlebars::Handlebars;
use serde_json::json;

pub(crate) struct Summary {
    pub issues: Vec<Issue>,
    data: serde_json::Value,
    flake_lock_path: PathBuf,
    flake_check_config: FlakeCheckConfig,
}

impl Summary {
    pub(crate) fn new(
        issues: &Vec<Issue>,
        flake_lock_path: PathBuf,
        flake_check_config: FlakeCheckConfig,
    ) -> Self {
        let disallowed: Vec<&Issue> = issues.iter().filter(|i| i.kind.is_disallowed()).collect();
        let outdated: Vec<&Issue> = issues.iter().filter(|i| i.kind.is_outdated()).collect();
        let non_upstream: Vec<&Issue> =
            issues.iter().filter(|i| i.kind.is_non_upstream()).collect();

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
        }
    }

    pub fn console_log_errors(&self) -> Result<(), FlakeCheckerError> {
        let file = self.flake_lock_path.to_string_lossy();

        if self.issues.is_empty() {
            println!("The Determinate Nix Flake Checker scanned {file} and found no issues");
        } else {
            let level = if self.flake_check_config.fail_mode {
                "error"
            } else {
                "warning"
            };

            for issue in self.issues.iter() {
                let input = &issue.input;

                let message: Option<String> = match &issue.kind {
                    IssueKind::Disallowed(disallowed) => {
                        if self.flake_check_config.check_supported {
                            let reference = &disallowed.reference;
                            Some(format!(
                                "the `{input}` input uses the non-supported Git branch `{reference}` for Nixpkgs"
                             ))
                        } else {
                            None
                        }
                    }
                    IssueKind::Outdated(outdated) => {
                        if self.flake_check_config.check_outdated {
                            let num_days_old = outdated.num_days_old;
                            Some(format!(
                                "the `{input}` input is {num_days_old} days old (the max allowed is {MAX_DAYS})"
                            ))
                        } else {
                            None
                        }
                    }
                    IssueKind::NonUpstream(non_upstream) => {
                        if self.flake_check_config.check_owner {
                            let owner = &non_upstream.owner;
                            Some(format!(
                                "the `{input}` input has the non-upstream owner `{owner}` rather than `NixOS` (upstream)"
                            ))
                        } else {
                            None
                        }
                    }
                };

                if let Some(message) = message {
                    println!("{}: {}", level.to_uppercase(), message);
                }
            }
        }
        Ok(())
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

        println!("{}", summary_txt);

        Ok(())
    }
}
