#[derive(Debug, thiserror::Error)]
pub enum FlakeCheckerError {
    #[error("env var error: {0}")]
    EnvVar(#[from] std::env::VarError),
    #[error("couldn't access flake.lock: {0}")]
    Io(#[from] std::io::Error),
    #[error("couldn't parse flake.lock: {0}")]
    Json(#[from] serde_json::Error),
    #[error("handlebars render error: {0}")]
    Render(#[from] handlebars::RenderError),
    #[error("handlebars template error: {0}")]
    Template(#[from] Box<handlebars::TemplateError>),
}
