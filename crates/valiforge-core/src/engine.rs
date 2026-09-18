use crate::error::{CoreError, Result};
use crate::types::{
    EndpointResult, Severity, ValidationContext, ValidationReport, ValidationStatus, Violation,
};
use chrono::Utc;
use reqwest::Client;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;
use tracing::{info, instrument, warn};

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
            let permit = semaphore
                .clone()
                .acquire_owned()
                .await
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
            target_url: self.ctx.target_url.to_string(),
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
#[allow(clippy::unwrap_used, clippy::too_many_lines)]
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

    for (key, value) in &ctx.headers {
        request = request.header(key, value);
    }

    let mut last_error = None;
    for attempt in 0..=ctx.retry_count {
        if attempt > 0 {
            let delay = std::time::Duration::from_millis(100 * 2u64.pow(attempt - 1));
            tokio::time::sleep(delay).await;
        }

        match request
            .try_clone()
            .unwrap_or_else(|| client.get(&url))
            .send()
            .await
        {
            Ok(response) => {
                let status_code = response.status().as_u16();
                let latency = start.elapsed();
                let mut violations = Vec::new();

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

                if let Some(ref schema) = endpoint.response_schema {
                    if let Ok(body) = response.json::<serde_json::Value>().await {
                        if let Ok(validator) = jsonschema::validator_for(schema) {
                            if let Err(errors) = validator.validate(&body) {
                                for err in errors {
                                    violations.push(Violation {
                                        rule: "response_schema_violation".into(),
                                        message: err.to_string(),
                                        severity: Severity::Error,
                                        path: format!(
                                            "{} {} -> response.body.{}",
                                            endpoint.method, endpoint.path, err.instance_path
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

