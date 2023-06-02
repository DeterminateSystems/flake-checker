use crate::{FlakeCheckerError, Issue, IssueKind, ALLOWED_REFS, MAX_DAYS};

use std::fs::OpenOptions;
use std::io::Write;

use handlebars::Handlebars;
use serde_json::json;

pub struct Summary {
    pub issues: Vec<Issue>,
}

impl Summary {
    pub fn generate_markdown(&self) -> Result<(), FlakeCheckerError> {
        let mut handlebars = Handlebars::new();
        let supported_ref_names = ALLOWED_REFS
            .iter()
            .map(|r| format!("* `{r}`"))
            .collect::<Vec<_>>()
            .join("\n");
        let data = json!({
            "issues": &self.issues,
            "clean": self.issues.is_empty(),
            "dirty": !self.issues.is_empty(),
            // Disallowed refs
            "has_disallowed": !&self.disallowed().is_empty(),
            "disallowed": &self.disallowed(),
            // Outdated refs
            "has_outdated": !&self.outdated().is_empty(),
            "outdated": &self.outdated(),
            // Non-upstream refs
            "has_non_upstream": !&self.non_upstream().is_empty(),
            "non_upstream": &self.non_upstream(),
            // Constants
            "max_days": MAX_DAYS,
            "supported_ref_names": supported_ref_names,
            // Text snippets
            "supported_refs_explainer": include_str!("explainers/supported_refs.md"),
            "outdated_deps_explainer": include_str!("explainers/outdated_deps.md"),
            "upstream_nixpkgs_explainer": include_str!("explainers/upstream_nixpkgs.md"),
        });

        handlebars
            .register_template_string("summary.md", include_str!("templates/summary.md"))
            .map_err(Box::new)?;
        let summary_md = handlebars.render("summary.md", &data)?;

        let summary_md_filepath =
            std::env::var("GITHUB_STEP_SUMMARY").unwrap_or("/dev/stdout".to_string());
        let mut summary_md_file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(summary_md_filepath)?;
        summary_md_file.write_all(summary_md.as_bytes())?;

        Ok(())
    }

    fn disallowed(&self) -> Vec<&Issue> {
        self.issues
            .iter()
            .filter(|i| matches!(i.kind, IssueKind::Disallowed))
            .collect()
    }

    fn outdated(&self) -> Vec<&Issue> {
        self.issues
            .iter()
            .filter(|i| matches!(i.kind, IssueKind::Outdated))
            .collect()
    }

    fn non_upstream(&self) -> Vec<&Issue> {
        self.issues
            .iter()
            .filter(|i| matches!(i.kind, IssueKind::NonUpstream))
            .collect()
    }
}
