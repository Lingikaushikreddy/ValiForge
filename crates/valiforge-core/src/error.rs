use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("HTTP request to {url} failed: {source}")]
    HttpRequest {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("Schema validation failed for {endpoint}: {message}")]
    SchemaViolation { endpoint: String, message: String },

    #[error("Failed to parse response body from {url}: {source}")]
    ResponseParse {
        url: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("Request to {url} timed out after {timeout_ms}ms")]
    Timeout { url: String, timeout_ms: u64 },

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Schema file not found: {0}")]
    SchemaNotFound(PathBuf),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, CoreError>;
