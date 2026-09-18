use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error("Failed to read schema file {path}: {source}")]
    IoError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to parse YAML: {0}")]
    YamlParse(#[from] serde_yaml::Error),

    #[error("Failed to parse JSON: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Unsupported OpenAPI version: {version}. Supported: 3.0.x, 3.1.x")]
    UnsupportedVersion { version: String },

    #[error("Invalid schema: {0}")]
    Invalid(String),
}
