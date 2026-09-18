/// Auto-detect schema files in a directory.
#[must_use]
pub fn detect_schemas(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let patterns = [
        "openapi.yaml",
        "openapi.yml",
        "openapi.json",
        "swagger.yaml",
        "swagger.yml",
        "swagger.json",
        "api.yaml",
        "api.yml",
        "api.json",
    ];
    let mut found = Vec::new();
    for pattern in &patterns {
        let path = dir.join(pattern);
        if path.exists() {
            found.push(path);
        }
    }
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
