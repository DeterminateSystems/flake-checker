#[derive(Debug, thiserror::Error)]
pub enum FlakeCheckerError {
    #[error("CEL execution error: {0}")]
    CelExecution(#[from] cel_interpreter::ExecutionError),
    #[error("CEL parsing error: {0}")]
    CelParse(#[from] cel_interpreter::ParseError),
    #[error("env var error: {0}")]
    EnvVar(#[from] std::env::VarError),
    #[error("couldn't parse flake.lock: {0}")]
    FlakeLock(#[from] parse_flake_lock::FlakeLockParseError),
    #[error("http client error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("CEL conditions must return a Boolean but returned {0} instead")]
    NonBooleanCondition(String),
    #[error("couldn't access flake.lock: {0}")]
    Io(#[from] std::io::Error),
    #[error("couldn't parse flake.lock: {0}")]
    Json(#[from] serde_json::Error),
    #[error("handlebars render error: {0}")]
    Render(#[from] handlebars::RenderError),
    #[error("handlebars template error: {0}")]
    Template(#[from] Box<handlebars::TemplateError>),
    #[error("invalid flake.lock: {0}")]
    Invalid(String),
}
