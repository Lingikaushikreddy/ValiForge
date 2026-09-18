pub mod detect;
pub mod error;
pub mod openapi;

pub use error::SchemaError;

use valiforge_core::engine::ParsedEndpoint;

/// Parse an `OpenAPI` spec file into a list of endpoints for validation.
///
/// # Errors
/// Returns error if the file cannot be read or parsed.
pub fn parse_openapi_file(path: &std::path::Path) -> Result<Vec<ParsedEndpoint>, SchemaError> {
    let content = std::fs::read_to_string(path).map_err(|e| SchemaError::IoError {
        path: path.to_owned(),
        source: e,
    })?;
    openapi::parse_openapi_string(&content)
}
