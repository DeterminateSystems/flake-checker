use crate::error::FlakeCheckerError;
use crate::flake::{ALLOWED_REFS, MAX_DAYS};
use crate::issue::{Issue, IssueKind};

use std::fs::OpenOptions;
use std::io::Write;

use handlebars::Handlebars;
use serde_json::json;

pub struct Summary {
    pub issues: Vec<Issue>,
    data: serde_json::Value,
}

impl Summary {
    pub fn new(issues: &Vec<Issue>) -> Self {
        let disallowed: Vec<&Issue> = issues
            .iter()
            .filter(|i| matches!(i.kind, IssueKind::Disallowed))
            .collect();

        let outdated: Vec<&Issue> = issues
            .iter()
            .filter(|i| matches!(i.kind, IssueKind::Outdated))
            .collect();

        let non_upstream: Vec<&Issue> = issues
            .iter()
            .filter(|i| matches!(i.kind, IssueKind::NonUpstream))
            .collect();

        let data = json!({
            "issues": issues,
            "num_issues": issues.len(),
            "clean": issues.is_empty(),
            "dirty": !issues.is_empty(),
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
