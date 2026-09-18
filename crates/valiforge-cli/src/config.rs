use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

// Every documented `valiforge.toml` key is accepted, but only `schema.path` and
// `target.base_url` are used so far; the rest are wired up as features land.
#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
pub struct Config {
    pub schema: Option<SchemaConfig>,
    pub target: Option<TargetConfig>,
    pub validation: Option<ValidationConfig>,
    pub reporting: Option<ReportingConfig>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct SchemaConfig {
    pub path: Option<PathBuf>,
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct TargetConfig {
    pub base_url: Option<String>,
    pub timeout: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ValidationConfig {
    pub fail_on: Option<Vec<String>>,
    pub ignore_paths: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
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

    let config: Config =
        toml::from_str(&content).context(format!("Failed to parse config: {}", path.display()))?;

    Ok(ResolvedConfig {
        schema_path: config.schema.and_then(|s| s.path),
        target_url: config.target.and_then(|t| t.base_url),
    })
}
