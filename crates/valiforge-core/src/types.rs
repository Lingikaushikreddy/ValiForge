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
    #[allow(clippy::unwrap_used)]
    fn default() -> Self {
        Self {
            target_url: "http://localhost:3000".parse().unwrap(),
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
