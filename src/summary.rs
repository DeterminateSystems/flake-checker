use crate::FlakeCheckConfig;
use crate::error::FlakeCheckerError;
use crate::flake::MAX_DAYS;
use crate::issue::{Issue, IssueKind};

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use handlebars::Handlebars;
use serde_json::json;

static CEL_MARKDOWN_TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/templates/summary.cel.md.hbs"
));

static CEL_TEXT_TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/templates/summary.cel.txt.hbs"
));

static STANDARD_MARKDOWN_TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/templates/summary.standard.md.hbs"
));

static STANDARD_TEXT_TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/templates/summary.standard.txt.hbs"
));

pub(crate) struct Summary {
    pub issues: Vec<Issue>,
    data: serde_json::Value,
    flake_lock_path: PathBuf,
    flake_check_config: FlakeCheckConfig,
    condition: Option<String>,
}

impl Summary {
    pub(crate) fn new(
        issues: &Vec<Issue>,
        flake_lock_path: PathBuf,
        flake_check_config: FlakeCheckConfig,
        allowed_refs: Vec<String>,
        condition: Option<String>,
    ) -> Self {
        let num_issues = issues.len();
        let clean = issues.is_empty();
        let issue_word = if issues.len() == 1 { "issue" } else { "issues" };

        let data = if let Some(condition) = &condition {
            let inputs_with_violations: Vec<String> = issues
                .iter()
                .filter(|i| i.kind.is_violation())
                .map(|i| i.input.to_owned())
                .collect();

            json!({
                "issues": issues,
                "num_issues": num_issues,
                "clean": clean,
                "dirty": !clean,
                "issue_word": issue_word,
                "condition": condition,
                "inputs_with_violations": inputs_with_violations,
            })
        } else {
            let disallowed: Vec<&Issue> =
                issues.iter().filter(|i| i.kind.is_disallowed()).collect();
            let outdated: Vec<&Issue> = issues.iter().filter(|i| i.kind.is_outdated()).collect();
            let non_upstream: Vec<&Issue> =
                issues.iter().filter(|i| i.kind.is_non_upstream()).collect();

            json!({
                "issues": issues,
                "num_issues": num_issues,
                "clean": clean,
                "dirty": !clean,
                "issue_word": issue_word,
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
                "supported_ref_names": allowed_refs,
            })
        };

        Self {
            issues: issues.to_vec(),
            data,
            flake_lock_path,
            flake_check_config,
            condition,
        }
    }

    pub fn console_log_errors(&self) -> Result<(), FlakeCheckerError> {
        let file = self.flake_lock_path.to_string_lossy();

        if self.issues.is_empty() {
            println!("The Determinate Nix Flake Checker scanned {file} and found no issues");
            return Ok(());
        }

        if let Some(condition) = &self.condition {
            println!("You supplied this CEL condition for your flake:\n\n{condition}");
            println!("The following inputs violate that condition:\n");
            for issue in self.issues.iter() {
                println!("* {}", issue.input);
            }
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
                    IssueKind::Violation => Some(String::from("policy violation")),
                };

                if let Some(message) = message {
                    println!("{}: {}", level.to_uppercase(), message);
                }
            }
        }
        Ok(())
    }

    pub fn generate_markdown(&self) -> Result<(), FlakeCheckerError> {
        let template = if self.condition.is_some() {
            CEL_MARKDOWN_TEMPLATE
        } else {
            STANDARD_MARKDOWN_TEMPLATE
        };

        let mut handlebars = Handlebars::new();

        handlebars
            .register_template_string("summary.md", template)
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
        let template = if self.condition.is_some() {
            CEL_TEXT_TEMPLATE
        } else {
            STANDARD_TEXT_TEMPLATE
        };

        let mut handlebars = Handlebars::new();
        handlebars
            .register_template_string("summary.txt", template)
            .map_err(Box::new)?;

        let summary_txt = handlebars.render("summary.txt", &self.data)?;

        print!("{summary_txt}");

        Ok(())
    }
}
