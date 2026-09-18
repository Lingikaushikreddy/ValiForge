# ValiForge — Rust Core Implementation Guide

**For: Senior Rust Core Engineer (CTO)**
**Goal: Build the MVP validation engine from scratch**

---

## 1. Cargo Workspace Setup

### `Cargo.toml` (workspace root)

```toml
[workspace]
resolver = "2"
members = [
    "crates/valiforge-cli",
    "crates/valiforge-core",
    "crates/valiforge-schema",
    "crates/valiforge-diff",
    "crates/valiforge-report",
    "crates/valiforge-datagen",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
license = "Apache-2.0"
repository = "https://github.com/valiforge/valiforge"

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
thiserror = "2"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
clap = { version = "4", features = ["derive", "env", "string"] }
openapiv3 = "2"
jsonschema = "0.22"
toml = "0.8"
url = "2"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
indicatif = "0.17"
owo-colors = "4"
proptest = "1"
fake = { version = "3", features = ["derive"] }

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
unwrap_used = "deny"
panic = "deny"
missing_errors_doc = "warn"

[profile.release]
lto = true
codegen-units = 1
strip = true
opt-level = "s"
panic = "abort"
```

### `rust-toolchain.toml`

```toml
[toolchain]
channel = "1.75"
components = ["rustfmt", "clippy"]
```

### `deny.toml`

```toml
[advisories]
vulnerability = "deny"
unmaintained = "warn"

[licenses]
allow = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Unicode-DFS-2016"]
confidence-threshold = 0.8
unlicensed = "deny"
copyleft = "deny"

[bans]
multiple-versions = "warn"
wildcards = "deny"
```

---

## 2. Core Types & Traits (`valiforge-core`)

### `crates/valiforge-core/Cargo.toml`

```toml
[package]
name = "valiforge-core"
version.workspace = true
edition.workspace = true

[dependencies]
tokio.workspace = true
reqwest.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tracing.workspace = true
url.workspace = true
chrono.workspace = true
jsonschema.workspace = true

[lints]
workspace = true
```

### `crates/valiforge-core/src/lib.rs`

```rust
pub mod engine;
pub mod error;
pub mod types;

pub use engine::ValidationEngine;
pub use error::CoreError;
pub use types::*;
```

### `crates/valiforge-core/src/error.rs`

```rust
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
```

### `crates/valiforge-core/src/types.rs`

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Outcome of validating a single endpoint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Pass,
    Fail,
    Skip,
    Error,
}

/// A single validation violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub rule: String,
    pub message: String,
    pub severity: Severity,
    pub path: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub suggestion: Option<String>,
    pub docs_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Result of validating a single endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointResult {
    pub method: String,
    pub path: String,
    pub status: ValidationStatus,
    pub response_status: Option<u16>,
    pub latency: Duration,
    pub violations: Vec<Violation>,
    pub request_url: String,
}

/// Full validation report across all endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub schema_name: String,
    pub schema_version: Option<String>,
    pub target_url: String,
    pub timestamp: DateTime<Utc>,
    pub duration: Duration,
    pub results: Vec<EndpointResult>,
    pub summary: ReportSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub errors: usize,
    pub violation_count: usize,
}

impl ValidationReport {
    #[must_use]
    pub fn compute_summary(results: &[EndpointResult]) -> ReportSummary {
        let mut summary = ReportSummary {
            total: results.len(),
            passed: 0,
            failed: 0,
            skipped: 0,
            errors: 0,
            violation_count: 0,
        };
        for r in results {
            match r.status {
                ValidationStatus::Pass => summary.passed += 1,
                ValidationStatus::Fail => summary.failed += 1,
                ValidationStatus::Skip => summary.skipped += 1,
                ValidationStatus::Error => summary.errors += 1,
            }
            summary.violation_count += r.violations.len();
        }
        summary
    }

    #[must_use]
    pub fn is_success(&self) -> bool {
        self.summary.failed == 0 && self.summary.errors == 0
    }
}

/// Context passed to the validation engine
#[derive(Debug, Clone)]
pub struct ValidationContext {
    pub target_url: url::Url,
    pub headers: HashMap<String, String>,
    pub timeout: Duration,
    pub max_concurrency: usize,
    pub retry_count: u32,
    pub fail_on: Vec<FailCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum FailCondition {
    SchemaViolations,
    BreakingChanges,
    SecurityIssues,
    PerformanceRegression,
}

impl Default for ValidationContext {
    fn default() -> Self {
        Self {
            target_url: "http://localhost:3000".parse().expect("valid default URL"),
            headers: HashMap::new(),
            timeout: Duration::from_secs(10),
            max_concurrency: 8,
            retry_count: 1,
            fail_on: vec![FailCondition::SchemaViolations],
        }
    }
}

/// Breaking change detected between two schema versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakingChange {
    pub change_type: ChangeType,
    pub severity: Severity,
    pub path: String,
    pub message: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    EndpointRemoved,
    MethodRemoved,
    RequiredFieldAdded,
    FieldRemoved,
    TypeChanged,
    EnumValueRemoved,
    ResponseStatusRemoved,
    ParameterRequired,
    PathChanged,
}
```

### `crates/valiforge-core/src/engine.rs`

```rust
use crate::error::{CoreError, Result};
use crate::types::*;
use chrono::Utc;
use reqwest::Client;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;
use tracing::{info, warn, instrument};

pub struct ValidationEngine {
    client: Client,
    ctx: ValidationContext,
}

impl ValidationEngine {
    /// Create a new engine with the given context.
    ///
    /// # Errors
    /// Returns error if the HTTP client cannot be built.
    pub fn new(ctx: ValidationContext) -> Result<Self> {
        let mut builder = Client::builder()
            .timeout(ctx.timeout)
            .redirect(reqwest::redirect::Policy::limited(5));

        // Use rustls, no native TLS dependency
        builder = builder.use_rustls_tls();

        let client = builder
            .build()
            .map_err(|e| CoreError::Config(format!("Failed to build HTTP client: {e}")))?;

        Ok(Self { client, ctx })
    }

    /// Validate all endpoints in a parsed schema concurrently.
    ///
    /// # Errors
    /// Returns error if validation cannot proceed (config issues).
    #[instrument(skip(self, endpoints))]
    pub async fn validate_all(
        &self,
        endpoints: &[ParsedEndpoint],
        schema_name: &str,
    ) -> Result<ValidationReport> {
        let start = Instant::now();
        let semaphore = Arc::new(Semaphore::new(self.ctx.max_concurrency));
        let mut handles = Vec::with_capacity(endpoints.len());

        for endpoint in endpoints {
            let permit = semaphore.clone().acquire_owned().await
                .map_err(|e| CoreError::Config(format!("Semaphore error: {e}")))?;
            let client = self.client.clone();
            let ctx = self.ctx.clone();
            let ep = endpoint.clone();

            let handle = tokio::spawn(async move {
                let result = validate_endpoint(&client, &ctx, &ep).await;
                drop(permit);
                result
            });
            handles.push(handle);
        }

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    warn!("Validation task panicked: {e}");
                    results.push(EndpointResult {
                        method: "UNKNOWN".into(),
                        path: "UNKNOWN".into(),
                        status: ValidationStatus::Error,
                        response_status: None,
                        latency: std::time::Duration::ZERO,
                        violations: vec![Violation {
                            rule: "internal_error".into(),
                            message: format!("Task panicked: {e}"),
                            severity: Severity::Critical,
                            path: String::new(),
                            expected: None,
                            actual: None,
                            suggestion: Some("Please report this bug.".into()),
                            docs_url: None,
                        }],
                        request_url: String::new(),
                    });
                }
            }
        }

        let summary = ValidationReport::compute_summary(&results);
        let duration = start.elapsed();

        info!(
            endpoints = results.len(),
            passed = summary.passed,
            failed = summary.failed,
            duration_ms = duration.as_millis(),
            "Validation complete"
        );

        Ok(ValidationReport {
            schema_name: schema_name.to_string(),
            schema_version: None,
            target_url: ctx.target_url.to_string(),
            timestamp: Utc::now(),
            duration,
            results,
            summary,
        })
    }
}

/// Parsed endpoint from schema — used as input to engine
#[derive(Debug, Clone)]
pub struct ParsedEndpoint {
    pub method: String,
    pub path: String,
    pub parameters: Vec<EndpointParam>,
    pub expected_status: u16,
    pub response_schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct EndpointParam {
    pub name: String,
    pub location: ParamLocation,
    pub required: bool,
    pub schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamLocation {
    Path,
    Query,
    Header,
    Cookie,
}

/// Validate a single endpoint against the live server.
async fn validate_endpoint(
    client: &Client,
    ctx: &ValidationContext,
    endpoint: &ParsedEndpoint,
) -> EndpointResult {
    let url = format!("{}{}", ctx.target_url, endpoint.path);
    let start = Instant::now();

    let mut request = match endpoint.method.to_uppercase().as_str() {
        "GET" => client.get(&url),
        "POST" => client.post(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        "PATCH" => client.patch(&url),
        "HEAD" => client.head(&url),
        method => {
            return EndpointResult {
                method: method.to_string(),
                path: endpoint.path.clone(),
                status: ValidationStatus::Skip,
                response_status: None,
                latency: start.elapsed(),
                violations: vec![],
                request_url: url,
            };
        }
    };

    // Add custom headers
    for (key, value) in &ctx.headers {
        request = request.header(key, value);
    }

    // Execute with retry
    let mut last_error = None;
    for attempt in 0..=ctx.retry_count {
        if attempt > 0 {
            let delay = std::time::Duration::from_millis(100 * 2u64.pow(attempt - 1));
            tokio::time::sleep(delay).await;
        }

        match request.try_clone().unwrap_or_else(|| client.get(&url)).send().await {
            Ok(response) => {
                let status_code = response.status().as_u16();
                let latency = start.elapsed();
                let mut violations = Vec::new();

                // Check status code
                if status_code != endpoint.expected_status {
                    violations.push(Violation {
                        rule: "status_code_mismatch".into(),
                        message: format!(
                            "Expected status {}, got {}",
                            endpoint.expected_status, status_code
                        ),
                        severity: Severity::Error,
                        path: endpoint.path.clone(),
                        expected: Some(endpoint.expected_status.to_string()),
                        actual: Some(status_code.to_string()),
                        suggestion: Some(format!(
                            "Ensure {} {} returns status {}",
                            endpoint.method, endpoint.path, endpoint.expected_status
                        )),
                        docs_url: None,
                    });
                }

                // Validate response body against schema
                if let Some(ref schema) = endpoint.response_schema {
                    if let Ok(body) = response.json::<serde_json::Value>().await {
                        if let Err(errs) = jsonschema::validate(schema, &body) {
                            for err in errs {
                                violations.push(Violation {
                                    rule: "response_schema_violation".into(),
                                    message: err.to_string(),
                                    severity: Severity::Error,
                                    path: format!(
                                        "{} {} -> response.body.{}",
                                        endpoint.method,
                                        endpoint.path,
                                        err.instance_path
                                    ),
                                    expected: None,
                                    actual: None,
                                    suggestion: Some(
                                        "Response body does not match the declared schema."
                                            .into(),
                                    ),
                                    docs_url: None,
                                });
                            }
                        }
                    }
                }

                let status = if violations.is_empty() {
                    ValidationStatus::Pass
                } else {
                    ValidationStatus::Fail
                };

                return EndpointResult {
                    method: endpoint.method.clone(),
                    path: endpoint.path.clone(),
                    status,
                    response_status: Some(status_code),
                    latency,
                    violations,
                    request_url: url,
                };
            }
            Err(e) => {
                last_error = Some(e);
            }
        }
    }

    // All retries exhausted
    EndpointResult {
        method: endpoint.method.clone(),
        path: endpoint.path.clone(),
        status: ValidationStatus::Error,
        response_status: None,
        latency: start.elapsed(),
        violations: vec![Violation {
            rule: "request_failed".into(),
            message: format!(
                "Request failed after {} attempts: {}",
                ctx.retry_count + 1,
                last_error.map_or_else(|| "unknown".to_string(), |e| e.to_string())
            ),
            severity: Severity::Critical,
            path: endpoint.path.clone(),
            expected: None,
            actual: None,
            suggestion: Some(format!("Verify the server is running at {url}")),
            docs_url: None,
        }],
        request_url: url,
    }
}
```

---

## 3. Schema Parser (`valiforge-schema`)

### `crates/valiforge-schema/Cargo.toml`

```toml
[package]
name = "valiforge-schema"
version.workspace = true
edition.workspace = true

[dependencies]
openapiv3.workspace = true
serde.workspace = true
serde_json.workspace = true
serde_yaml.workspace = true
thiserror.workspace = true
tracing.workspace = true
valiforge-core = { path = "../valiforge-core" }

[dev-dependencies]
proptest.workspace = true
insta = { version = "1", features = ["yaml"] }
```

### `crates/valiforge-schema/src/lib.rs`

```rust
pub mod detect;
pub mod error;
pub mod openapi;

pub use error::SchemaError;

use valiforge_core::engine::ParsedEndpoint;

/// Parse an OpenAPI spec file into a list of endpoints for validation.
///
/// # Errors
/// Returns error if the file cannot be read or parsed.
pub fn parse_openapi_file(path: &std::path::Path) -> Result<Vec<ParsedEndpoint>, SchemaError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| SchemaError::IoError { path: path.to_owned(), source: e })?;
    openapi::parse_openapi_string(&content)
}
```

### `crates/valiforge-schema/src/error.rs`

```rust
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
```

### `crates/valiforge-schema/src/openapi.rs`

```rust
use crate::SchemaError;
use openapiv3::{OpenAPI, PathItem, ReferenceOr, StatusCode};
use valiforge_core::engine::{EndpointParam, ParamLocation, ParsedEndpoint};

/// Parse an OpenAPI 3.x spec string into endpoints.
///
/// # Errors
/// Returns error if the YAML/JSON is invalid or the OpenAPI version is unsupported.
pub fn parse_openapi_string(content: &str) -> Result<Vec<ParsedEndpoint>, SchemaError> {
    let spec: OpenAPI = serde_yaml::from_str(content)
        .or_else(|_| serde_json::from_str::<OpenAPI>(content).map_err(SchemaError::JsonParse))?;

    // Validate version
    if !spec.openapi.starts_with("3.0") && !spec.openapi.starts_with("3.1") {
        return Err(SchemaError::UnsupportedVersion {
            version: spec.openapi.clone(),
        });
    }

    let mut endpoints = Vec::new();

    for (path, path_item_ref) in &spec.paths.paths {
        let path_item = match path_item_ref {
            ReferenceOr::Item(item) => item,
            ReferenceOr::Reference { .. } => continue, // Skip $ref paths for MVP
        };

        extract_operations(path, path_item, &mut endpoints);
    }

    Ok(endpoints)
}

fn extract_operations(path: &str, item: &PathItem, endpoints: &mut Vec<ParsedEndpoint>) {
    let ops = [
        ("GET", &item.get),
        ("POST", &item.post),
        ("PUT", &item.put),
        ("DELETE", &item.delete),
        ("PATCH", &item.patch),
        ("HEAD", &item.head),
        ("OPTIONS", &item.options),
    ];

    for (method, op) in ops {
        if let Some(operation) = op {
            let params: Vec<EndpointParam> = operation
                .parameters
                .iter()
                .filter_map(|p| match p {
                    ReferenceOr::Item(param) => Some(EndpointParam {
                        name: param_name(param),
                        location: param_location(param),
                        required: param_required(param),
                        schema: param_schema(param),
                    }),
                    _ => None,
                })
                .collect();

            // Get the primary success response schema
            let (expected_status, response_schema) =
                extract_response_info(&operation.responses);

            endpoints.push(ParsedEndpoint {
                method: method.to_string(),
                path: path.to_string(),
                parameters: params,
                expected_status,
                response_schema,
            });
        }
    }
}

fn extract_response_info(
    responses: &openapiv3::Responses,
) -> (u16, Option<serde_json::Value>) {
    // Look for 200, 201, 204 in order
    for code in [200, 201, 204] {
        let key = StatusCode::Code(code);
        if let Some(ReferenceOr::Item(resp)) = responses.responses.get(&key) {
            if let Some(content) = resp.content.get("application/json") {
                if let Some(ReferenceOr::Item(schema)) = &content.schema {
                    if let Ok(json) = serde_json::to_value(schema) {
                        return (code, Some(json));
                    }
                }
            }
            return (code, None);
        }
    }
    // Default: expect 200 with no schema validation
    (200, None)
}

fn param_name(param: &openapiv3::Parameter) -> String {
    match param {
        openapiv3::Parameter::Query { parameter_data, .. } => parameter_data.name.clone(),
        openapiv3::Parameter::Header { parameter_data, .. } => parameter_data.name.clone(),
        openapiv3::Parameter::Path { parameter_data, .. } => parameter_data.name.clone(),
        openapiv3::Parameter::Cookie { parameter_data, .. } => parameter_data.name.clone(),
    }
}

fn param_location(param: &openapiv3::Parameter) -> ParamLocation {
    match param {
        openapiv3::Parameter::Query { .. } => ParamLocation::Query,
        openapiv3::Parameter::Header { .. } => ParamLocation::Header,
        openapiv3::Parameter::Path { .. } => ParamLocation::Path,
        openapiv3::Parameter::Cookie { .. } => ParamLocation::Cookie,
    }
}

fn param_required(param: &openapiv3::Parameter) -> bool {
    match param {
        openapiv3::Parameter::Path { .. } => true, // Path params always required
        openapiv3::Parameter::Query { parameter_data, .. }
        | openapiv3::Parameter::Header { parameter_data, .. }
        | openapiv3::Parameter::Cookie { parameter_data, .. } => parameter_data.required,
    }
}

fn param_schema(param: &openapiv3::Parameter) -> Option<serde_json::Value> {
    let data = match param {
        openapiv3::Parameter::Query { parameter_data, .. }
        | openapiv3::Parameter::Header { parameter_data, .. }
        | openapiv3::Parameter::Path { parameter_data, .. }
        | openapiv3::Parameter::Cookie { parameter_data, .. } => parameter_data,
    };
    match &data.format {
        openapiv3::ParameterSchemaOrContent::Schema(ReferenceOr::Item(schema)) => {
            serde_json::to_value(schema).ok()
        }
        _ => None,
    }
}

/// Auto-detect schema files in a directory.
pub fn detect_schemas(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let patterns = [
        "openapi.yaml", "openapi.yml", "openapi.json",
        "swagger.yaml", "swagger.yml", "swagger.json",
        "api.yaml", "api.yml", "api.json",
    ];
    let mut found = Vec::new();
    for pattern in &patterns {
        let path = dir.join(pattern);
        if path.exists() {
            found.push(path);
        }
    }
    // Also check common subdirs
    for subdir in ["docs", "api", "spec", "schemas"] {
        let sub = dir.join(subdir);
        if sub.is_dir() {
            for pattern in &patterns {
                let path = sub.join(pattern);
                if path.exists() {
                    found.push(path);
                }
            }
        }
    }
    found
}
```

---

## 4. CLI (`valiforge-cli`)

### `crates/valiforge-cli/Cargo.toml`

```toml
[package]
name = "valiforge"
version.workspace = true
edition.workspace = true

[[bin]]
name = "valiforge"
path = "src/main.rs"

[dependencies]
valiforge-core = { path = "../valiforge-core" }
valiforge-schema = { path = "../valiforge-schema" }
valiforge-diff = { path = "../valiforge-diff" }
valiforge-report = { path = "../valiforge-report" }
clap.workspace = true
tokio.workspace = true
anyhow.workspace = true
serde.workspace = true
toml.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
indicatif.workspace = true
owo-colors.workspace = true
url.workspace = true
```

### `crates/valiforge-cli/src/main.rs`

```rust
mod config;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "valiforge",
    version,
    about = "Headless API validation engine for the AI coding era",
    long_about = "ValiForge validates API contracts, detects breaking changes,\n\
                  and generates test data — all from a single Rust binary.\n\n\
                  Validate APIs, not pixels."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to configuration file
    #[arg(long, global = true, default_value = "valiforge.toml")]
    config: PathBuf,

    /// Output format
    #[arg(long, global = true, default_value = "terminal")]
    format: OutputFormat,

    /// Suppress all output except errors
    #[arg(long, global = true)]
    quiet: bool,

    /// Enable verbose output
    #[arg(long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize ValiForge in the current project
    Init {
        /// Accept all defaults without prompting
        #[arg(long)]
        yes: bool,
    },
    /// Validate API endpoints against schema
    Validate {
        /// Path to API schema file
        #[arg(long)]
        schema: Option<PathBuf>,
        /// Base URL of the API to validate
        #[arg(long)]
        target: Option<String>,
    },
    /// Detect breaking changes between schema versions
    Diff {
        /// Path to the old (current) schema
        old: PathBuf,
        /// Path to the new (proposed) schema
        new: PathBuf,
    },
    /// Generate test data from schema
    Generate {
        /// Path to API schema
        #[arg(long)]
        schema: PathBuf,
        /// Number of payloads to generate
        #[arg(long, default_value = "10")]
        count: usize,
        /// Include negative test cases
        #[arg(long)]
        include_negative: bool,
    },
}

#[derive(Clone, clap::ValueEnum)]
enum OutputFormat {
    Terminal,
    Json,
    Junit,
    Markdown,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    // Setup tracing
    let filter = if cli.verbose {
        "valiforge=debug"
    } else if cli.quiet {
        "error"
    } else {
        "valiforge=info"
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    match run(cli).await {
        Ok(success) => {
            if success {
                ExitCode::from(0)
            } else {
                ExitCode::from(1) // Validation failures
            }
        }
        Err(e) => {
            eprintln!("{} {e:#}", "error:".red().bold());
            ExitCode::from(2) // Config/runtime error
        }
    }
}

async fn run(cli: Cli) -> Result<bool> {
    match cli.command {
        Commands::Init { yes } => {
            let schemas = valiforge_schema::openapi::detect_schemas(
                &std::env::current_dir()?,
            );

            if schemas.is_empty() {
                println!(
                    "{} No API schemas detected. Create an openapi.yaml and re-run.",
                    "warning:".yellow().bold()
                );
                return Ok(true);
            }

            println!(
                "{} Detected {} schema(s):",
                "info:".blue().bold(),
                schemas.len()
            );
            for s in &schemas {
                println!("  {}", s.display().green());
            }

            // Generate valiforge.toml
            let config_content = format!(
                r#"# ValiForge Configuration
# Generated by `valiforge init`

[schema]
path = "{}"

[target]
base_url = "http://localhost:3000"
timeout = "10s"

[validation]
fail_on = ["schema-violations"]

[reporting]
formats = ["terminal"]
"#,
                schemas[0].display()
            );

            std::fs::write("valiforge.toml", &config_content)
                .context("Failed to write valiforge.toml")?;

            println!(
                "\n{} Created valiforge.toml",
                "success:".green().bold()
            );
            println!("\nNext steps:");
            println!("  1. Start your API server");
            println!(
                "  2. Run {} to validate",
                "valiforge validate".cyan().bold()
            );

            Ok(true)
        }

        Commands::Validate { schema, target } => {
            let cfg = config::load_config(&cli.config)?;
            let schema_path = schema
                .or(cfg.schema_path)
                .context("No schema specified. Use --schema or set [schema].path in valiforge.toml")?;
            let target_url = target
                .or(cfg.target_url)
                .unwrap_or_else(|| "http://localhost:3000".into());

            // Parse schema
            let endpoints = valiforge_schema::parse_openapi_file(&schema_path)
                .context("Failed to parse schema")?;

            println!(
                "{} Validating {} endpoints against {}",
                "info:".blue().bold(),
                endpoints.len().to_string().cyan(),
                target_url.cyan()
            );

            // Build engine context
            let ctx = valiforge_core::ValidationContext {
                target_url: target_url.parse().context("Invalid target URL")?,
                ..Default::default()
            };

            let engine = valiforge_core::ValidationEngine::new(ctx)
                .context("Failed to create validation engine")?;

            let report = engine
                .validate_all(&endpoints, &schema_path.display().to_string())
                .await
                .context("Validation failed")?;

            // Print results
            println!("\n{}", "Results:".bold());
            for r in &report.results {
                let icon = match r.status {
                    valiforge_core::ValidationStatus::Pass => "PASS".green().to_string(),
                    valiforge_core::ValidationStatus::Fail => "FAIL".red().to_string(),
                    valiforge_core::ValidationStatus::Skip => "SKIP".yellow().to_string(),
                    valiforge_core::ValidationStatus::Error => "ERR ".red().bold().to_string(),
                };
                println!(
                    "  {} {} {} ({}ms)",
                    icon,
                    r.method.bold(),
                    r.path,
                    r.latency.as_millis()
                );
                for v in &r.violations {
                    println!("       {} {}", "->".dimmed(), v.message);
                }
            }

            // Summary
            println!(
                "\n{}: {} total, {} passed, {} failed, {} errors",
                "Summary".bold(),
                report.summary.total,
                report.summary.passed.to_string().green(),
                report.summary.failed.to_string().red(),
                report.summary.errors.to_string().red(),
            );

            Ok(report.is_success())
        }

        Commands::Diff { old, new } => {
            let old_content = std::fs::read_to_string(&old)
                .context(format!("Failed to read {}", old.display()))?;
            let new_content = std::fs::read_to_string(&new)
                .context(format!("Failed to read {}", new.display()))?;

            let old_endpoints = valiforge_schema::openapi::parse_openapi_string(&old_content)?;
            let new_endpoints = valiforge_schema::openapi::parse_openapi_string(&new_content)?;

            println!(
                "{} Comparing {} -> {}",
                "info:".blue().bold(),
                old.display().to_string().cyan(),
                new.display().to_string().cyan()
            );

            // Basic diff: find removed endpoints
            let mut breaking = Vec::new();
            for old_ep in &old_endpoints {
                let key = format!("{} {}", old_ep.method, old_ep.path);
                let exists = new_endpoints
                    .iter()
                    .any(|n| n.method == old_ep.method && n.path == old_ep.path);
                if !exists {
                    breaking.push(valiforge_core::BreakingChange {
                        change_type: valiforge_core::ChangeType::EndpointRemoved,
                        severity: valiforge_core::Severity::Critical,
                        path: key,
                        message: format!(
                            "Endpoint {} {} was removed",
                            old_ep.method, old_ep.path
                        ),
                        old_value: Some(format!("{} {}", old_ep.method, old_ep.path)),
                        new_value: None,
                    });
                }
            }

            if breaking.is_empty() {
                println!("\n{} No breaking changes detected.", "success:".green().bold());
                Ok(true)
            } else {
                println!(
                    "\n{} {} breaking change(s) detected:",
                    "warning:".red().bold(),
                    breaking.len()
                );
                for change in &breaking {
                    println!(
                        "  {} [{}] {}",
                        "BREAK".red().bold(),
                        format!("{:?}", change.change_type).yellow(),
                        change.message
                    );
                }
                Ok(false)
            }
        }

        Commands::Generate { schema, count, include_negative } => {
            println!(
                "{} Generating {} test payloads from {}",
                "info:".blue().bold(),
                count.to_string().cyan(),
                schema.display().to_string().cyan()
            );
            // Placeholder — DATAGEN module implements this
            println!("{} Test data generation will be available with valiforge-datagen", "todo:".yellow().bold());
            Ok(true)
        }
    }
}
```

### `crates/valiforge-cli/src/config.rs`

```rust
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub schema: Option<SchemaConfig>,
    pub target: Option<TargetConfig>,
    pub validation: Option<ValidationConfig>,
    pub reporting: Option<ReportingConfig>,
}

#[derive(Debug, Deserialize)]
pub struct SchemaConfig {
    pub path: Option<PathBuf>,
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TargetConfig {
    pub base_url: Option<String>,
    pub timeout: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ValidationConfig {
    pub fail_on: Option<Vec<String>>,
    pub ignore_paths: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct ReportingConfig {
    pub formats: Option<Vec<String>>,
}

pub struct ResolvedConfig {
    pub schema_path: Option<PathBuf>,
    pub target_url: Option<String>,
}

pub fn load_config(path: &Path) -> Result<ResolvedConfig> {
    if !path.exists() {
        return Ok(ResolvedConfig {
            schema_path: None,
            target_url: None,
        });
    }

    let content = std::fs::read_to_string(path)
        .context(format!("Failed to read config: {}", path.display()))?;

    let config: Config = toml::from_str(&content)
        .context(format!("Failed to parse config: {}", path.display()))?;

    Ok(ResolvedConfig {
        schema_path: config.schema.and_then(|s| s.path),
        target_url: config.target.and_then(|t| t.base_url),
    })
}
```

---

## 5. Report Generation (`valiforge-report`)

### `crates/valiforge-report/src/lib.rs`

```rust
use valiforge_core::ValidationReport;

pub trait ReportFormatter {
    fn format_id(&self) -> &str;
    fn format(&self, report: &ValidationReport) -> String;
}

pub struct JsonFormatter;

impl ReportFormatter for JsonFormatter {
    fn format_id(&self) -> &str { "json" }
    fn format(&self, report: &ValidationReport) -> String {
        serde_json::to_string_pretty(report).unwrap_or_else(|e| format!("{{\"error\": \"{e}\"}}"))
    }
}

pub struct JunitFormatter;

impl ReportFormatter for JunitFormatter {
    fn format_id(&self) -> &str { "junit" }
    fn format(&self, report: &ValidationReport) -> String {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str(&format!(
            "<testsuites tests=\"{}\" failures=\"{}\" errors=\"{}\" time=\"{:.3}\">\n",
            report.summary.total,
            report.summary.failed,
            report.summary.errors,
            report.duration.as_secs_f64()
        ));
        xml.push_str(&format!(
            "  <testsuite name=\"{}\" tests=\"{}\">\n",
            report.schema_name, report.summary.total
        ));
        for result in &report.results {
            let name = format!("{} {}", result.method, result.path);
            xml.push_str(&format!(
                "    <testcase name=\"{name}\" time=\"{:.3}\"",
                result.latency.as_secs_f64()
            ));
            if result.violations.is_empty() {
                xml.push_str(" />\n");
            } else {
                xml.push_str(">\n");
                for v in &result.violations {
                    xml.push_str(&format!(
                        "      <failure message=\"{}\" type=\"{}\"/>\n",
                        xml_escape(&v.message),
                        v.rule
                    ));
                }
                xml.push_str("    </testcase>\n");
            }
        }
        xml.push_str("  </testsuite>\n</testsuites>\n");
        xml
    }
}

pub struct MarkdownFormatter;

impl ReportFormatter for MarkdownFormatter {
    fn format_id(&self) -> &str { "markdown" }
    fn format(&self, report: &ValidationReport) -> String {
        let mut md = format!("## ValiForge Validation Report\n\n");
        md.push_str(&format!(
            "**Schema:** {} | **Target:** {} | **Duration:** {:.1}s\n\n",
            report.schema_name, report.target_url, report.duration.as_secs_f64()
        ));
        md.push_str(&format!(
            "| Status | Total | Passed | Failed | Errors |\n|---|---|---|---|---|\n| {} | {} | {} | {} | {} |\n\n",
            if report.is_success() { "PASS" } else { "FAIL" },
            report.summary.total, report.summary.passed,
            report.summary.failed, report.summary.errors,
        ));
        if !report.results.iter().any(|r| !r.violations.is_empty()) {
            md.push_str("All endpoints passed validation.\n");
        } else {
            md.push_str("### Violations\n\n");
            for r in report.results.iter().filter(|r| !r.violations.is_empty()) {
                md.push_str(&format!("**{} {}**\n", r.method, r.path));
                for v in &r.violations {
                    md.push_str(&format!("- `{}`: {}\n", v.rule, v.message));
                }
                md.push('\n');
            }
        }
        md
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}
```

---

## 6. GitHub Actions CI (``.github/workflows/ci.yml``)

```yaml
name: CI
on:
  push:
    branches: [main]
  pull_request:

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-D warnings"

jobs:
  check:
    name: Check & Lint
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all --check
      - run: cargo clippy --workspace --all-targets

  test:
    name: Test (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --workspace

  audit:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: rustsec/audit-check@v2
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
```

---

## 7. Quick Start Commands

```bash
# Create workspace
mkdir valiforge && cd valiforge
cargo init --name valiforge-cli crates/valiforge-cli
cargo init --lib --name valiforge-core crates/valiforge-core
cargo init --lib --name valiforge-schema crates/valiforge-schema
cargo init --lib --name valiforge-diff crates/valiforge-diff
cargo init --lib --name valiforge-report crates/valiforge-report
cargo init --lib --name valiforge-datagen crates/valiforge-datagen

# Verify it compiles
cargo check --workspace

# Run tests
cargo test --workspace

# Build release
cargo build --release

# Binary is at target/release/valiforge
```

---

*This guide provides the foundation. Each module can be extended independently.*
*Best practice: get `cargo check` passing on day 1, `cargo test` on day 2, first real validation on day 5.*
