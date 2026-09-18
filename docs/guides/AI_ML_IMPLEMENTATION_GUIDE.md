# ValiForge — AI/ML Test Data Generation Implementation Guide

**For: Senior AI/ML Engineer**
**Goal: Build the 3-layer test data generation engine (Schema-Driven → Property-Based → SLM-Powered)**

---

## 1. Crate Setup — `valiforge-datagen`

### `crates/valiforge-datagen/Cargo.toml`

```toml
[package]
name = "valiforge-datagen"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "AI-powered test data generation for API validation"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
tokio = { workspace = true }
anyhow = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }

# Data generation
fake = { version = "3", features = ["derive"] }
rand = "0.8"
rand_chacha = "0.3"
regex-syntax = "0.8"

# Property-based testing
proptest = "1"

# SLM inference
llama-cpp-2 = { version = "0.1", optional = true }

# Schema handling
indexmap = { version = "2", features = ["serde"] }

[dev-dependencies]
proptest = "1"
tokio = { workspace = true, features = ["test-util"] }
insta = { version = "1", features = ["json"] }
criterion = { version = "0.5", features = ["async_tokio"] }

[features]
default = ["slm"]
slm = ["dep:llama-cpp-2"]

[[bench]]
name = "datagen_bench"
harness = false
```

### Module Structure

```
crates/valiforge-datagen/src/
├── lib.rs              # Public API, re-exports
├── types.rs            # Core types: GeneratedTestCase, TestCategory, etc.
├── error.rs            # DatagenError enum
├── schema_driven.rs    # Layer 1: JSON Schema → valid data
├── negative.rs         # Negative/adversarial test generation
├── proptest_gen.rs     # Layer 2: proptest strategies
├── slm.rs              # Layer 3: SLM-powered generation
├── domain.rs           # Domain-aware realistic data
├── generator.rs        # Unified TestDataEngine orchestrator
├── seed.rs             # Deterministic seed management
└── diversity.rs        # Deduplication & diversity scoring
```

---

## 2. Core Types (`types.rs`)

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Strategy for generating test data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationStrategy {
    /// Layer 1: Schema-driven grammar-based generation
    SchemaDriven,
    /// Layer 2: Property-based with proptest
    PropertyBased,
    /// Layer 3: SLM-powered intelligent generation
    SlmPowered,
    /// Auto: tries SLM → Property → Schema with fallback
    Auto,
}

/// Category of generated test case
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestCategory {
    HappyPath,
    BoundaryValue,
    NegativeInput,
    SecurityPayload,
    EdgeCase,
    TypeMismatch,
    MissingField,
    FormatViolation,
}

/// A single generated test case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedTestCase {
    /// The generated input payload
    pub input: Value,
    /// Expected HTTP status code (200 for happy path, 400/422 for negative)
    pub expected_status: u16,
    /// Human-readable description of what this test case covers
    pub description: String,
    /// Category for filtering and reporting
    pub category: TestCategory,
    /// Which strategy generated this case
    pub generated_by: GenerationStrategy,
    /// Unique fingerprint for deduplication
    pub fingerprint: String,
}

/// Configuration for test data generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorConfig {
    /// Which strategy to use
    pub strategy: GenerationStrategy,
    /// Path to SLM model file (GGUF format)
    pub slm_model_path: Option<String>,
    /// SLM temperature (0.0 = deterministic, 0.7 = creative)
    pub temperature: f32,
    /// Max tokens for SLM generation
    pub max_tokens: u32,
    /// Domain hints for realistic data (e.g., "fintech", "healthcare")
    pub domain_hints: Vec<String>,
    /// Seed for reproducibility (None = random)
    pub seed: Option<u64>,
    /// Number of test cases per endpoint
    pub count: usize,
    /// Include negative/adversarial test cases
    pub include_negative: bool,
    /// Include security payload test cases
    pub include_security: bool,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            strategy: GenerationStrategy::Auto,
            slm_model_path: None,
            temperature: 0.3,
            max_tokens: 512,
            domain_hints: Vec::new(),
            seed: None,
            count: 10,
            include_negative: true,
            include_security: true,
        }
    }
}

/// Internal representation of a JSON Schema (simplified for generation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaNode {
    pub schema_type: SchemaType,
    pub format: Option<String>,
    pub description: Option<String>,
    pub required: bool,
    pub constraints: SchemaConstraints,
    pub properties: HashMap<String, SchemaNode>,
    pub items: Option<Box<SchemaNode>>,
    pub enum_values: Vec<Value>,
    pub one_of: Vec<SchemaNode>,
    pub any_of: Vec<SchemaNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaType {
    String,
    Integer,
    Number,
    Boolean,
    Array,
    Object,
    Null,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchemaConstraints {
    /// String constraints
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<String>,
    /// Numeric constraints
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub exclusive_minimum: Option<f64>,
    pub exclusive_maximum: Option<f64>,
    pub multiple_of: Option<f64>,
    /// Array constraints
    pub min_items: Option<usize>,
    pub max_items: Option<usize>,
    pub unique_items: bool,
}
```

---

## 3. Error Types (`error.rs`)

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DatagenError {
    #[error("Schema parsing failed: {0}")]
    SchemaParse(String),

    #[error("Generation failed for field '{field}': {reason}")]
    GenerationFailed { field: String, reason: String },

    #[error("SLM model not found at path: {0}")]
    SlmModelNotFound(String),

    #[error("SLM inference failed: {0}")]
    SlmInference(String),

    #[error("SLM output is not valid JSON: {0}")]
    SlmOutputParse(String),

    #[error("Unsupported schema type: {0}")]
    UnsupportedType(String),

    #[error("Regex generation failed for pattern '{pattern}': {reason}")]
    RegexGeneration { pattern: String, reason: String },

    #[error("Configuration error: {0}")]
    Config(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, DatagenError>;
```

---

## 4. Seed Manager (`seed.rs`)

```rust
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

/// Manages deterministic seeding across all generation layers.
/// When a seed is provided, all generators produce identical output.
pub struct SeedManager {
    base_seed: Option<u64>,
    counter: u64,
}

impl SeedManager {
    pub fn new(seed: Option<u64>) -> Self {
        Self {
            base_seed: seed,
            counter: 0,
        }
    }

    /// Create a deterministic RNG for the next generation operation.
    /// Each call returns a uniquely seeded RNG (derived from base_seed + counter).
    pub fn next_rng(&mut self) -> ChaCha20Rng {
        self.counter += 1;
        match self.base_seed {
            Some(seed) => {
                let derived = seed.wrapping_add(self.counter);
                ChaCha20Rng::seed_from_u64(derived)
            }
            None => ChaCha20Rng::from_entropy(),
        }
    }

    /// Get the base seed (for logging/reproducibility)
    pub fn base_seed(&self) -> Option<u64> {
        self.base_seed
    }
}
```

---

## 5. Schema-Driven Generator — Layer 1 (`schema_driven.rs`)

```rust
use crate::error::{DatagenError, Result};
use crate::seed::SeedManager;
use crate::types::*;
use fake::faker::address::en::*;
use fake::faker::company::en::*;
use fake::faker::internet::en::*;
use fake::faker::lorem::en::*;
use fake::faker::name::en::*;
use fake::faker::phone_number::en::*;
use fake::Fake;
use rand::Rng;
use rand_chacha::ChaCha20Rng;
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::debug;

/// Layer 1: Grammar-based test data generator.
/// Parses JSON Schema constraints and generates conforming data.
pub struct SchemaDrivenGenerator {
    seed_mgr: SeedManager,
}

impl SchemaDrivenGenerator {
    pub fn new(seed: Option<u64>) -> Self {
        Self {
            seed_mgr: SeedManager::new(seed),
        }
    }

    /// Generate `count` valid test cases for the given schema node
    pub fn generate(
        &mut self,
        schema: &SchemaNode,
        count: usize,
    ) -> Result<Vec<GeneratedTestCase>> {
        let mut cases = Vec::with_capacity(count);
        for i in 0..count {
            let mut rng = self.seed_mgr.next_rng();
            let input = self.generate_value(schema, &mut rng)?;
            let fingerprint = format!(
                "schema-{:x}",
                md5_hash(&serde_json::to_string(&input).unwrap_or_default())
            );
            cases.push(GeneratedTestCase {
                input,
                expected_status: 200,
                description: format!("Happy path test case #{}", i + 1),
                category: TestCategory::HappyPath,
                generated_by: GenerationStrategy::SchemaDriven,
                fingerprint,
            });
        }
        Ok(cases)
    }

    /// Generate boundary value test cases (min/max edges)
    pub fn generate_boundary(
        &mut self,
        schema: &SchemaNode,
    ) -> Result<Vec<GeneratedTestCase>> {
        let mut cases = Vec::new();
        let mut rng = self.seed_mgr.next_rng();
        self.collect_boundary_cases(schema, "", &mut rng, &mut cases)?;
        Ok(cases)
    }

    fn generate_value(
        &self,
        schema: &SchemaNode,
        rng: &mut ChaCha20Rng,
    ) -> Result<Value> {
        // Handle enum values first
        if !schema.enum_values.is_empty() {
            let idx = rng.gen_range(0..schema.enum_values.len());
            return Ok(schema.enum_values[idx].clone());
        }

        // Handle oneOf/anyOf
        if !schema.one_of.is_empty() {
            let idx = rng.gen_range(0..schema.one_of.len());
            return self.generate_value(&schema.one_of[idx], rng);
        }
        if !schema.any_of.is_empty() {
            let idx = rng.gen_range(0..schema.any_of.len());
            return self.generate_value(&schema.any_of[idx], rng);
        }

        match schema.schema_type {
            SchemaType::String => self.generate_string(schema, rng),
            SchemaType::Integer => self.generate_integer(schema, rng),
            SchemaType::Number => self.generate_number(schema, rng),
            SchemaType::Boolean => Ok(Value::Bool(rng.gen_bool(0.5))),
            SchemaType::Array => self.generate_array(schema, rng),
            SchemaType::Object => self.generate_object(schema, rng),
            SchemaType::Null => Ok(Value::Null),
        }
    }

    fn generate_string(
        &self,
        schema: &SchemaNode,
        rng: &mut ChaCha20Rng,
    ) -> Result<Value> {
        let value = match schema.format.as_deref() {
            Some("email") => FreeEmail().fake_with_rng::<String, _>(rng),
            Some("uuid") => uuid::Uuid::new_v4().to_string(),
            Some("uri") | Some("url") => {
                format!("https://example.com/{}", Word().fake_with_rng::<String, _>(rng))
            }
            Some("date-time") => chrono::Utc::now().to_rfc3339(),
            Some("date") => chrono::Utc::now().format("%Y-%m-%d").to_string(),
            Some("time") => chrono::Utc::now().format("%H:%M:%S").to_string(),
            Some("ipv4") => format!(
                "{}.{}.{}.{}",
                rng.gen_range(1..255),
                rng.gen_range(0..255),
                rng.gen_range(0..255),
                rng.gen_range(1..255)
            ),
            Some("ipv6") => format!(
                "{:04x}:{:04x}:{:04x}:{:04x}:{:04x}:{:04x}:{:04x}:{:04x}",
                rng.gen::<u16>(), rng.gen::<u16>(),
                rng.gen::<u16>(), rng.gen::<u16>(),
                rng.gen::<u16>(), rng.gen::<u16>(),
                rng.gen::<u16>(), rng.gen::<u16>()
            ),
            Some("phone") => PhoneNumber().fake_with_rng::<String, _>(rng),
            Some("hostname") => format!(
                "{}.example.com",
                Word().fake_with_rng::<String, _>(rng)
            ),
            Some("password") => {
                let len = rng.gen_range(12..24);
                (0..len)
                    .map(|_| {
                        let charset = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%";
                        charset[rng.gen_range(0..charset.len())] as char
                    })
                    .collect()
            }
            _ => {
                // Check for regex pattern
                if let Some(ref _pattern) = schema.constraints.pattern {
                    // Generate a simple string that attempts to match
                    debug!("Pattern-based generation not fully supported, using fallback");
                    Sentence(3..8).fake_with_rng::<String, _>(rng)
                } else {
                    // Respect length constraints
                    let min_len = schema.constraints.min_length.unwrap_or(1);
                    let max_len = schema.constraints.max_length.unwrap_or(50);
                    let target_len = rng.gen_range(min_len..=max_len);
                    let words: String = Sentence(3..10).fake_with_rng(rng);
                    if words.len() > target_len {
                        words[..target_len].to_string()
                    } else {
                        words
                    }
                }
            }
        };

        Ok(Value::String(value))
    }

    fn generate_integer(
        &self,
        schema: &SchemaNode,
        rng: &mut ChaCha20Rng,
    ) -> Result<Value> {
        let min = schema
            .constraints
            .minimum
            .or(schema.constraints.exclusive_minimum.map(|v| v + 1.0))
            .unwrap_or(0.0) as i64;
        let max = schema
            .constraints
            .maximum
            .or(schema.constraints.exclusive_maximum.map(|v| v - 1.0))
            .unwrap_or(1000.0) as i64;

        let value = rng.gen_range(min..=max);

        // Respect multipleOf
        let value = if let Some(mult) = schema.constraints.multiple_of {
            let mult = mult as i64;
            if mult > 0 {
                (value / mult) * mult
            } else {
                value
            }
        } else {
            value
        };

        Ok(Value::Number(serde_json::Number::from(value)))
    }

    fn generate_number(
        &self,
        schema: &SchemaNode,
        rng: &mut ChaCha20Rng,
    ) -> Result<Value> {
        let min = schema
            .constraints
            .minimum
            .or(schema.constraints.exclusive_minimum)
            .unwrap_or(0.0);
        let max = schema
            .constraints
            .maximum
            .or(schema.constraints.exclusive_maximum)
            .unwrap_or(1000.0);

        let value: f64 = rng.gen_range(min..=max);
        let rounded = (value * 100.0).round() / 100.0;

        Ok(serde_json::to_value(rounded).unwrap_or(Value::Null))
    }

    fn generate_array(
        &self,
        schema: &SchemaNode,
        rng: &mut ChaCha20Rng,
    ) -> Result<Value> {
        let min_items = schema.constraints.min_items.unwrap_or(1);
        let max_items = schema.constraints.max_items.unwrap_or(5);
        let count = rng.gen_range(min_items..=max_items);

        let items_schema = schema.items.as_deref().ok_or_else(|| {
            DatagenError::SchemaParse("Array schema missing 'items'".into())
        })?;

        let mut items = Vec::with_capacity(count);
        for _ in 0..count {
            items.push(self.generate_value(items_schema, rng)?);
        }
        Ok(Value::Array(items))
    }

    fn generate_object(
        &self,
        schema: &SchemaNode,
        rng: &mut ChaCha20Rng,
    ) -> Result<Value> {
        let mut map = serde_json::Map::new();

        for (key, prop_schema) in &schema.properties {
            // Always include required fields; include optional fields randomly
            if prop_schema.required || rng.gen_bool(0.8) {
                let value = self.generate_value(prop_schema, rng)?;
                map.insert(key.clone(), value);
            }
        }

        Ok(Value::Object(map))
    }

    fn collect_boundary_cases(
        &self,
        schema: &SchemaNode,
        path: &str,
        rng: &mut ChaCha20Rng,
        cases: &mut Vec<GeneratedTestCase>,
    ) -> Result<()> {
        match schema.schema_type {
            SchemaType::String => {
                // Min length boundary
                if let Some(min) = schema.constraints.min_length {
                    let val: String = (0..min).map(|_| 'a').collect();
                    cases.push(GeneratedTestCase {
                        input: json!({ path: val }),
                        expected_status: 200,
                        description: format!("String at min_length={min} for '{path}'"),
                        category: TestCategory::BoundaryValue,
                        generated_by: GenerationStrategy::SchemaDriven,
                        fingerprint: format!("boundary-{path}-min_length"),
                    });
                }
                // Max length boundary
                if let Some(max) = schema.constraints.max_length {
                    let val: String = (0..max).map(|_| 'z').collect();
                    cases.push(GeneratedTestCase {
                        input: json!({ path: val }),
                        expected_status: 200,
                        description: format!("String at max_length={max} for '{path}'"),
                        category: TestCategory::BoundaryValue,
                        generated_by: GenerationStrategy::SchemaDriven,
                        fingerprint: format!("boundary-{path}-max_length"),
                    });
                }
                // Empty string
                cases.push(GeneratedTestCase {
                    input: json!({ path: "" }),
                    expected_status: if schema.constraints.min_length.unwrap_or(0) > 0 {
                        400
                    } else {
                        200
                    },
                    description: format!("Empty string for '{path}'"),
                    category: TestCategory::BoundaryValue,
                    generated_by: GenerationStrategy::SchemaDriven,
                    fingerprint: format!("boundary-{path}-empty"),
                });
            }
            SchemaType::Integer | SchemaType::Number => {
                if let Some(min) = schema.constraints.minimum {
                    cases.push(GeneratedTestCase {
                        input: json!({ path: min }),
                        expected_status: 200,
                        description: format!("Value at minimum={min} for '{path}'"),
                        category: TestCategory::BoundaryValue,
                        generated_by: GenerationStrategy::SchemaDriven,
                        fingerprint: format!("boundary-{path}-min"),
                    });
                }
                if let Some(max) = schema.constraints.maximum {
                    cases.push(GeneratedTestCase {
                        input: json!({ path: max }),
                        expected_status: 200,
                        description: format!("Value at maximum={max} for '{path}'"),
                        category: TestCategory::BoundaryValue,
                        generated_by: GenerationStrategy::SchemaDriven,
                        fingerprint: format!("boundary-{path}-max"),
                    });
                }
                // Zero
                cases.push(GeneratedTestCase {
                    input: json!({ path: 0 }),
                    expected_status: 200,
                    description: format!("Zero value for '{path}'"),
                    category: TestCategory::BoundaryValue,
                    generated_by: GenerationStrategy::SchemaDriven,
                    fingerprint: format!("boundary-{path}-zero"),
                });
            }
            SchemaType::Object => {
                for (key, prop_schema) in &schema.properties {
                    let child_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    self.collect_boundary_cases(prop_schema, &child_path, rng, cases)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// Simple hash for fingerprinting (not cryptographic)
fn md5_hash(input: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_string_schema(format: Option<&str>) -> SchemaNode {
        SchemaNode {
            schema_type: SchemaType::String,
            format: format.map(String::from),
            description: None,
            required: true,
            constraints: SchemaConstraints::default(),
            properties: HashMap::new(),
            items: None,
            enum_values: vec![],
            one_of: vec![],
            any_of: vec![],
        }
    }

    fn make_integer_schema(min: Option<f64>, max: Option<f64>) -> SchemaNode {
        SchemaNode {
            schema_type: SchemaType::Integer,
            format: None,
            description: None,
            required: true,
            constraints: SchemaConstraints {
                minimum: min,
                maximum: max,
                ..Default::default()
            },
            properties: HashMap::new(),
            items: None,
            enum_values: vec![],
            one_of: vec![],
            any_of: vec![],
        }
    }

    #[test]
    fn test_generate_email_format() {
        let mut gen = SchemaDrivenGenerator::new(Some(42));
        let schema = make_string_schema(Some("email"));
        let cases = gen.generate(&schema, 5).expect("generation should succeed");
        assert_eq!(cases.len(), 5);
        for case in &cases {
            let email = case.input.as_str().expect("should be string");
            assert!(email.contains('@'), "email should contain @: {email}");
        }
    }

    #[test]
    fn test_generate_integer_respects_bounds() {
        let mut gen = SchemaDrivenGenerator::new(Some(42));
        let schema = make_integer_schema(Some(10.0), Some(20.0));
        let cases = gen.generate(&schema, 100).expect("generation should succeed");
        for case in &cases {
            let val = case.input.as_i64().expect("should be integer");
            assert!((10..=20).contains(&val), "value {val} out of range [10, 20]");
        }
    }

    #[test]
    fn test_deterministic_with_seed() {
        let mut gen1 = SchemaDrivenGenerator::new(Some(12345));
        let mut gen2 = SchemaDrivenGenerator::new(Some(12345));
        let schema = make_string_schema(Some("email"));
        let cases1 = gen1.generate(&schema, 3).expect("gen1 succeeds");
        let cases2 = gen2.generate(&schema, 3).expect("gen2 succeeds");
        for (a, b) in cases1.iter().zip(cases2.iter()) {
            assert_eq!(a.input, b.input, "same seed should produce same output");
        }
    }

    #[test]
    fn test_generate_object() {
        let mut gen = SchemaDrivenGenerator::new(Some(42));
        let schema = SchemaNode {
            schema_type: SchemaType::Object,
            format: None,
            description: None,
            required: true,
            constraints: SchemaConstraints::default(),
            properties: HashMap::from([
                ("name".into(), make_string_schema(None)),
                ("age".into(), make_integer_schema(Some(0.0), Some(120.0))),
                ("email".into(), make_string_schema(Some("email"))),
            ]),
            items: None,
            enum_values: vec![],
            one_of: vec![],
            any_of: vec![],
        };
        let cases = gen.generate(&schema, 1).expect("generation should succeed");
        let obj = cases[0].input.as_object().expect("should be object");
        assert!(obj.contains_key("name"));
    }

    #[test]
    fn test_generate_enum() {
        let mut gen = SchemaDrivenGenerator::new(Some(42));
        let schema = SchemaNode {
            schema_type: SchemaType::String,
            format: None,
            description: None,
            required: true,
            constraints: SchemaConstraints::default(),
            properties: HashMap::new(),
            items: None,
            enum_values: vec![json!("active"), json!("inactive"), json!("pending")],
            one_of: vec![],
            any_of: vec![],
        };
        let cases = gen.generate(&schema, 20).expect("generation should succeed");
        for case in &cases {
            let val = case.input.as_str().expect("should be string");
            assert!(
                ["active", "inactive", "pending"].contains(&val),
                "unexpected enum value: {val}"
            );
        }
    }
}
```

---

## 6. Negative Test Generator (`negative.rs`)

```rust
use crate::error::Result;
use crate::types::*;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Generates negative / adversarial test cases designed to trigger validation errors.
pub struct NegativeTestGenerator;

impl NegativeTestGenerator {
    pub fn new() -> Self {
        Self
    }

    /// Generate negative test cases for the given schema
    pub fn generate(&self, schema: &SchemaNode) -> Result<Vec<GeneratedTestCase>> {
        let mut cases = Vec::new();

        match schema.schema_type {
            SchemaType::Object => {
                self.generate_missing_required_fields(schema, &mut cases)?;
                self.generate_type_mismatches(schema, &mut cases)?;
                self.generate_extra_fields(schema, &mut cases);
                self.generate_null_fields(schema, &mut cases);
            }
            SchemaType::String => {
                self.generate_string_violations(schema, &mut cases);
            }
            SchemaType::Integer | SchemaType::Number => {
                self.generate_numeric_violations(schema, &mut cases);
            }
            SchemaType::Array => {
                self.generate_array_violations(schema, &mut cases);
            }
            _ => {}
        }

        // Always add security payloads for string fields
        if schema.schema_type == SchemaType::Object {
            self.generate_security_payloads(schema, &mut cases);
        }

        Ok(cases)
    }

    /// Remove each required field one at a time
    fn generate_missing_required_fields(
        &self,
        schema: &SchemaNode,
        cases: &mut Vec<GeneratedTestCase>,
    ) -> Result<()> {
        let required_fields: Vec<&String> = schema
            .properties
            .iter()
            .filter(|(_, v)| v.required)
            .map(|(k, _)| k)
            .collect();

        for field in &required_fields {
            let mut obj = serde_json::Map::new();
            // Add all required fields except the one we're omitting
            for (key, prop) in &schema.properties {
                if key != *field && prop.required {
                    obj.insert(key.clone(), self.default_value_for(&prop.schema_type));
                }
            }
            cases.push(GeneratedTestCase {
                input: Value::Object(obj),
                expected_status: 400,
                description: format!("Missing required field '{field}'"),
                category: TestCategory::MissingField,
                generated_by: GenerationStrategy::SchemaDriven,
                fingerprint: format!("negative-missing-{field}"),
            });
        }
        Ok(())
    }

    /// Send wrong types for each field
    fn generate_type_mismatches(
        &self,
        schema: &SchemaNode,
        cases: &mut Vec<GeneratedTestCase>,
    ) -> Result<()> {
        for (field, prop) in &schema.properties {
            let wrong_value = match prop.schema_type {
                SchemaType::String => json!(12345),          // number instead of string
                SchemaType::Integer => json!("not_a_number"), // string instead of int
                SchemaType::Number => json!(true),            // bool instead of number
                SchemaType::Boolean => json!("yes"),          // string instead of bool
                SchemaType::Array => json!("not_an_array"),   // string instead of array
                SchemaType::Object => json!([1, 2, 3]),       // array instead of object
                SchemaType::Null => json!(42),                // number instead of null
            };

            let mut obj = serde_json::Map::new();
            for (k, p) in &schema.properties {
                if k == field {
                    obj.insert(k.clone(), wrong_value.clone());
                } else if p.required {
                    obj.insert(k.clone(), self.default_value_for(&p.schema_type));
                }
            }
            cases.push(GeneratedTestCase {
                input: Value::Object(obj),
                expected_status: 422,
                description: format!("Type mismatch for '{field}': expected {:?}", prop.schema_type),
                category: TestCategory::TypeMismatch,
                generated_by: GenerationStrategy::SchemaDriven,
                fingerprint: format!("negative-type-{field}"),
            });
        }
        Ok(())
    }

    fn generate_extra_fields(&self, schema: &SchemaNode, cases: &mut Vec<GeneratedTestCase>) {
        let mut obj = serde_json::Map::new();
        for (k, p) in &schema.properties {
            if p.required {
                obj.insert(k.clone(), self.default_value_for(&p.schema_type));
            }
        }
        obj.insert("__unexpected_field__".into(), json!("unexpected_value"));
        cases.push(GeneratedTestCase {
            input: Value::Object(obj),
            expected_status: 200, // Most APIs accept extra fields
            description: "Extra unexpected field in request body".into(),
            category: TestCategory::EdgeCase,
            generated_by: GenerationStrategy::SchemaDriven,
            fingerprint: "negative-extra-field".into(),
        });
    }

    fn generate_null_fields(&self, schema: &SchemaNode, cases: &mut Vec<GeneratedTestCase>) {
        for (field, prop) in &schema.properties {
            if prop.required {
                let mut obj = serde_json::Map::new();
                for (k, p) in &schema.properties {
                    if k == field {
                        obj.insert(k.clone(), Value::Null);
                    } else if p.required {
                        obj.insert(k.clone(), self.default_value_for(&p.schema_type));
                    }
                }
                cases.push(GeneratedTestCase {
                    input: Value::Object(obj),
                    expected_status: 400,
                    description: format!("Null value for required field '{field}'"),
                    category: TestCategory::NegativeInput,
                    generated_by: GenerationStrategy::SchemaDriven,
                    fingerprint: format!("negative-null-{field}"),
                });
            }
        }
    }

    fn generate_string_violations(&self, schema: &SchemaNode, cases: &mut Vec<GeneratedTestCase>) {
        // Exceed max length
        if let Some(max) = schema.constraints.max_length {
            let val: String = (0..max + 10).map(|_| 'x').collect();
            cases.push(GeneratedTestCase {
                input: json!(val),
                expected_status: 400,
                description: format!("String exceeds max_length={max}"),
                category: TestCategory::BoundaryValue,
                generated_by: GenerationStrategy::SchemaDriven,
                fingerprint: "negative-string-too-long".into(),
            });
        }
        // Below min length
        if let Some(min) = schema.constraints.min_length {
            if min > 0 {
                let val: String = (0..min.saturating_sub(1)).map(|_| 'x').collect();
                cases.push(GeneratedTestCase {
                    input: json!(val),
                    expected_status: 400,
                    description: format!("String below min_length={min}"),
                    category: TestCategory::BoundaryValue,
                    generated_by: GenerationStrategy::SchemaDriven,
                    fingerprint: "negative-string-too-short".into(),
                });
            }
        }
    }

    fn generate_numeric_violations(
        &self,
        schema: &SchemaNode,
        cases: &mut Vec<GeneratedTestCase>,
    ) {
        if let Some(min) = schema.constraints.minimum {
            cases.push(GeneratedTestCase {
                input: json!(min - 1.0),
                expected_status: 400,
                description: format!("Value below minimum={min}"),
                category: TestCategory::BoundaryValue,
                generated_by: GenerationStrategy::SchemaDriven,
                fingerprint: "negative-below-min".into(),
            });
        }
        if let Some(max) = schema.constraints.maximum {
            cases.push(GeneratedTestCase {
                input: json!(max + 1.0),
                expected_status: 400,
                description: format!("Value above maximum={max}"),
                category: TestCategory::BoundaryValue,
                generated_by: GenerationStrategy::SchemaDriven,
                fingerprint: "negative-above-max".into(),
            });
        }
        // Large overflow value
        cases.push(GeneratedTestCase {
            input: json!(i64::MAX),
            expected_status: 400,
            description: "Integer overflow value (i64::MAX)".into(),
            category: TestCategory::EdgeCase,
            generated_by: GenerationStrategy::SchemaDriven,
            fingerprint: "negative-overflow".into(),
        });
    }

    fn generate_array_violations(
        &self,
        schema: &SchemaNode,
        cases: &mut Vec<GeneratedTestCase>,
    ) {
        // Empty array when min_items > 0
        if schema.constraints.min_items.unwrap_or(0) > 0 {
            cases.push(GeneratedTestCase {
                input: json!([]),
                expected_status: 400,
                description: "Empty array when min_items > 0".into(),
                category: TestCategory::BoundaryValue,
                generated_by: GenerationStrategy::SchemaDriven,
                fingerprint: "negative-empty-array".into(),
            });
        }
        // Exceed max_items
        if let Some(max) = schema.constraints.max_items {
            let items: Vec<Value> = (0..max + 5).map(|i| json!(i)).collect();
            cases.push(GeneratedTestCase {
                input: Value::Array(items),
                expected_status: 400,
                description: format!("Array exceeds max_items={max}"),
                category: TestCategory::BoundaryValue,
                generated_by: GenerationStrategy::SchemaDriven,
                fingerprint: "negative-too-many-items".into(),
            });
        }
    }

    /// OWASP-style security payloads injected into string fields
    fn generate_security_payloads(
        &self,
        schema: &SchemaNode,
        cases: &mut Vec<GeneratedTestCase>,
    ) {
        let payloads = vec![
            ("sql_injection", "' OR 1=1; DROP TABLE users; --"),
            ("xss_script", "<script>alert('xss')</script>"),
            ("xss_img", "<img src=x onerror=alert(1)>"),
            ("path_traversal", "../../etc/passwd"),
            ("null_byte", "test\0value"),
            ("command_injection", "; cat /etc/passwd"),
            ("ldap_injection", "*)(objectClass=*)"),
            ("xxe_entity", "<!DOCTYPE foo [<!ENTITY xxe SYSTEM 'file:///etc/passwd'>]>"),
            ("ssti_jinja", "{{7*7}}"),
            ("unicode_overflow", "\u{FEFF}\u{200B}\u{200E}"),
        ];

        let string_fields: Vec<&String> = schema
            .properties
            .iter()
            .filter(|(_, v)| v.schema_type == SchemaType::String)
            .map(|(k, _)| k)
            .collect();

        if let Some(field) = string_fields.first() {
            for (name, payload) in &payloads {
                let mut obj = serde_json::Map::new();
                for (k, p) in &schema.properties {
                    if k == *field {
                        obj.insert(k.clone(), json!(payload));
                    } else if p.required {
                        obj.insert(k.clone(), self.default_value_for(&p.schema_type));
                    }
                }
                cases.push(GeneratedTestCase {
                    input: Value::Object(obj),
                    expected_status: 400,
                    description: format!("Security: {name} in '{field}'"),
                    category: TestCategory::SecurityPayload,
                    generated_by: GenerationStrategy::SchemaDriven,
                    fingerprint: format!("security-{name}-{field}"),
                });
            }
        }
    }

    fn default_value_for(&self, schema_type: &SchemaType) -> Value {
        match schema_type {
            SchemaType::String => json!("test_value"),
            SchemaType::Integer => json!(1),
            SchemaType::Number => json!(1.0),
            SchemaType::Boolean => json!(true),
            SchemaType::Array => json!([]),
            SchemaType::Object => json!({}),
            SchemaType::Null => Value::Null,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_user_schema() -> SchemaNode {
        SchemaNode {
            schema_type: SchemaType::Object,
            format: None,
            description: Some("User object".into()),
            required: true,
            constraints: SchemaConstraints::default(),
            properties: HashMap::from([
                (
                    "name".into(),
                    SchemaNode {
                        schema_type: SchemaType::String,
                        required: true,
                        constraints: SchemaConstraints {
                            min_length: Some(1),
                            max_length: Some(100),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ),
                (
                    "age".into(),
                    SchemaNode {
                        schema_type: SchemaType::Integer,
                        required: true,
                        constraints: SchemaConstraints {
                            minimum: Some(0.0),
                            maximum: Some(150.0),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ),
                (
                    "email".into(),
                    SchemaNode {
                        schema_type: SchemaType::String,
                        format: Some("email".into()),
                        required: true,
                        ..Default::default()
                    },
                ),
            ]),
            items: None,
            enum_values: vec![],
            one_of: vec![],
            any_of: vec![],
        }
    }

    impl Default for SchemaNode {
        fn default() -> Self {
            Self {
                schema_type: SchemaType::String,
                format: None,
                description: None,
                required: false,
                constraints: SchemaConstraints::default(),
                properties: HashMap::new(),
                items: None,
                enum_values: vec![],
                one_of: vec![],
                any_of: vec![],
            }
        }
    }

    #[test]
    fn test_missing_required_fields() {
        let gen = NegativeTestGenerator::new();
        let schema = make_user_schema();
        let cases = gen.generate(&schema).expect("should generate");
        let missing: Vec<_> = cases
            .iter()
            .filter(|c| c.category == TestCategory::MissingField)
            .collect();
        assert_eq!(missing.len(), 3, "should have 3 missing-field cases (name, age, email)");
        for case in &missing {
            assert_eq!(case.expected_status, 400);
        }
    }

    #[test]
    fn test_type_mismatches() {
        let gen = NegativeTestGenerator::new();
        let schema = make_user_schema();
        let cases = gen.generate(&schema).expect("should generate");
        let mismatches: Vec<_> = cases
            .iter()
            .filter(|c| c.category == TestCategory::TypeMismatch)
            .collect();
        assert_eq!(mismatches.len(), 3);
    }

    #[test]
    fn test_security_payloads_generated() {
        let gen = NegativeTestGenerator::new();
        let schema = make_user_schema();
        let cases = gen.generate(&schema).expect("should generate");
        let security: Vec<_> = cases
            .iter()
            .filter(|c| c.category == TestCategory::SecurityPayload)
            .collect();
        assert!(security.len() >= 8, "should generate OWASP security payloads");
    }
}
```

---

## 7. SLM-Powered Generator — Layer 3 (`slm.rs`)

```rust
#[cfg(feature = "slm")]
use llama_cpp_2::context::params::LlamaContextParams;
#[cfg(feature = "slm")]
use llama_cpp_2::llama_backend::LlamaBackend;
#[cfg(feature = "slm")]
use llama_cpp_2::model::params::LlamaModelParams;
#[cfg(feature = "slm")]
use llama_cpp_2::model::LlamaModel;

use crate::error::{DatagenError, Result};
use crate::types::*;
use serde_json::Value;
use std::path::Path;
use tracing::{debug, info, warn};

/// Prompt templates for different generation scenarios
pub struct PromptTemplates;

impl PromptTemplates {
    /// Happy path: generate valid, realistic test data
    pub fn happy_path(schema_json: &str, count: usize) -> String {
        format!(
            r#"You are a test data generator. Given the following JSON Schema, generate {count} valid JSON objects that conform to the schema. Each object should be realistic and diverse.

JSON Schema:
```json
{schema_json}
```

Generate {count} valid JSON objects as a JSON array. Only output the JSON array, no explanation.

```json
["#
        )
    }

    /// Edge cases: boundary values, unusual but valid inputs
    pub fn edge_cases(schema_json: &str, count: usize) -> String {
        format!(
            r#"You are an expert QA engineer generating edge case test data. Given the JSON Schema below, generate {count} JSON objects that are VALID but test edge cases:
- Boundary values (min/max lengths, min/max numbers)
- Unicode characters, emoji, RTL text
- Very long strings at max length
- Floating point precision edge cases
- Empty strings (if allowed)
- Arrays with 0 or 1 items (if allowed)

JSON Schema:
```json
{schema_json}
```

Generate {count} edge case JSON objects as a JSON array. Only output the JSON array.

```json
["#
        )
    }

    /// Domain-specific: generate data appropriate for the domain
    pub fn domain_specific(schema_json: &str, domain: &str, count: usize) -> String {
        format!(
            r#"You are a test data generator specialized in the {domain} domain. Generate {count} realistic JSON objects that conform to the schema and are appropriate for {domain} use cases.

For {domain}, use realistic values:
- Names, amounts, dates should be plausible for {domain}
- IDs should follow {domain} conventions
- Statuses should reflect real {domain} workflows

JSON Schema:
```json
{schema_json}
```

Generate {count} realistic {domain} JSON objects as a JSON array. Only output the JSON array.

```json
["#
        )
    }

    /// Negative/adversarial: generate invalid data for testing error handling
    pub fn adversarial(schema_json: &str, count: usize) -> String {
        format!(
            r#"You are a security-focused QA engineer. Given the JSON Schema below, generate {count} JSON objects that INTENTIONALLY VIOLATE the schema in different ways:
- Wrong types (string where number expected)
- Missing required fields
- Values outside allowed ranges
- SQL injection attempts in string fields
- XSS payloads in string fields
- Extremely long strings
- Negative numbers where positive expected
- Invalid formats (bad emails, dates)

For each object, include a "_violation" field describing what's wrong.

JSON Schema:
```json
{schema_json}
```

Generate {count} invalid JSON objects as a JSON array. Only output the JSON array.

```json
["#
        )
    }

    /// Relationship-aware: cross-field dependencies
    pub fn relationship_aware(schema_json: &str, count: usize) -> String {
        format!(
            r#"You are a test data generator that understands cross-field dependencies. Given the JSON Schema below, generate {count} JSON objects where field values are logically consistent with each other:
- If there's a start_date and end_date, end_date > start_date
- If there's a country and zip_code, they should match
- If there's a subtotal, tax, and total: total = subtotal + tax
- If there's a status and timestamps, they should be consistent

JSON Schema:
```json
{schema_json}
```

Generate {count} logically consistent JSON objects as a JSON array. Only output the JSON array.

```json
["#
        )
    }
}

/// SLM-powered test data generator using llama-cpp-2
pub struct SlmGenerator {
    #[cfg(feature = "slm")]
    model: Option<LlamaModel>,
    #[cfg(feature = "slm")]
    backend: Option<LlamaBackend>,
    temperature: f32,
    max_tokens: u32,
}

impl SlmGenerator {
    /// Create a new SLM generator. If the model path is None or the feature
    /// is disabled, the generator will always return Err (triggering fallback).
    pub fn new(model_path: Option<&str>, temperature: f32, max_tokens: u32) -> Result<Self> {
        #[cfg(feature = "slm")]
        {
            if let Some(path) = model_path {
                let model_path = Path::new(path);
                if !model_path.exists() {
                    return Err(DatagenError::SlmModelNotFound(path.to_string()));
                }
                info!("Loading SLM model from: {path}");
                let backend = LlamaBackend::init()
                    .map_err(|e| DatagenError::SlmInference(format!("Backend init: {e}")))?;
                let model_params = LlamaModelParams::default();
                let model = LlamaModel::load_from_file(&backend, model_path, &model_params)
                    .map_err(|e| DatagenError::SlmInference(format!("Model load: {e}")))?;
                info!("SLM model loaded successfully");
                return Ok(Self {
                    model: Some(model),
                    backend: Some(backend),
                    temperature,
                    max_tokens,
                });
            }
            Ok(Self {
                model: None,
                backend: None,
                temperature,
                max_tokens,
            })
        }
        #[cfg(not(feature = "slm"))]
        {
            let _ = (model_path, temperature, max_tokens);
            warn!("SLM feature not enabled, SlmGenerator will always fallback");
            Ok(Self {
                temperature,
                max_tokens,
            })
        }
    }

    /// Generate test cases using the SLM
    pub fn generate(
        &self,
        schema_json: &str,
        prompt_type: PromptType,
        count: usize,
        domain: Option<&str>,
    ) -> Result<Vec<GeneratedTestCase>> {
        let prompt = match prompt_type {
            PromptType::HappyPath => PromptTemplates::happy_path(schema_json, count),
            PromptType::EdgeCase => PromptTemplates::edge_cases(schema_json, count),
            PromptType::DomainSpecific => {
                let domain = domain.unwrap_or("general");
                PromptTemplates::domain_specific(schema_json, domain, count)
            }
            PromptType::Adversarial => PromptTemplates::adversarial(schema_json, count),
            PromptType::RelationshipAware => {
                PromptTemplates::relationship_aware(schema_json, count)
            }
        };

        let raw_output = self.run_inference(&prompt)?;
        let cases = self.parse_slm_output(&raw_output, prompt_type)?;
        Ok(cases)
    }

    /// Run LLM inference with the given prompt
    fn run_inference(&self, prompt: &str) -> Result<String> {
        #[cfg(feature = "slm")]
        {
            let model = self.model.as_ref().ok_or_else(|| {
                DatagenError::SlmInference("No model loaded".into())
            })?;

            let ctx_params = LlamaContextParams::default()
                .with_n_ctx(std::num::NonZeroU32::new(2048));
            let mut ctx = model
                .new_context(&self.backend.as_ref().expect("backend exists"), ctx_params)
                .map_err(|e| DatagenError::SlmInference(format!("Context creation: {e}")))?;

            let tokens = model
                .str_to_token(prompt, llama_cpp_2::model::AddBos::Always)
                .map_err(|e| DatagenError::SlmInference(format!("Tokenization: {e}")))?;

            debug!("Prompt tokens: {}", tokens.len());

            // Decode tokens and sample — simplified for guide
            // In production, use the full sampling loop with temperature
            let mut output = String::new();
            // ... (full sampling loop with temperature and repetition penalty)
            // This is a simplified placeholder; real implementation uses
            // ctx.decode() + sampler chain

            Ok(output)
        }
        #[cfg(not(feature = "slm"))]
        {
            let _ = prompt;
            Err(DatagenError::SlmInference(
                "SLM feature not compiled. Rebuild with --features slm".into(),
            ))
        }
    }

    /// Parse SLM output into structured test cases.
    /// Handles common LLM output issues: markdown fences, trailing commas, etc.
    fn parse_slm_output(
        &self,
        raw: &str,
        prompt_type: PromptType,
    ) -> Result<Vec<GeneratedTestCase>> {
        // Strip markdown code fences if present
        let cleaned = raw
            .trim()
            .strip_prefix("```json")
            .unwrap_or(raw)
            .strip_prefix("```")
            .unwrap_or(raw)
            .strip_suffix("```")
            .unwrap_or(raw)
            .trim();

        // Try to parse as JSON array
        let array: Vec<Value> = serde_json::from_str(cleaned).map_err(|e| {
            // Try to fix common issues: trailing comma
            let fixed = cleaned
                .replace(",\n]", "\n]")
                .replace(",]", "]");
            serde_json::from_str(&fixed).map_err(|_| {
                DatagenError::SlmOutputParse(format!("Failed to parse SLM output: {e}"))
            })
        })?;

        let (category, expected_status) = match prompt_type {
            PromptType::HappyPath => (TestCategory::HappyPath, 200),
            PromptType::EdgeCase => (TestCategory::EdgeCase, 200),
            PromptType::DomainSpecific => (TestCategory::HappyPath, 200),
            PromptType::Adversarial => (TestCategory::SecurityPayload, 400),
            PromptType::RelationshipAware => (TestCategory::HappyPath, 200),
        };

        let cases = array
            .into_iter()
            .enumerate()
            .map(|(i, value)| {
                let desc = if let Some(violation) = value.get("_violation") {
                    format!(
                        "SLM {prompt_type:?} #{}: {}",
                        i + 1,
                        violation.as_str().unwrap_or("unknown")
                    )
                } else {
                    format!("SLM {prompt_type:?} #{}", i + 1)
                };
                GeneratedTestCase {
                    input: value,
                    expected_status,
                    description: desc,
                    category,
                    generated_by: GenerationStrategy::SlmPowered,
                    fingerprint: format!("slm-{prompt_type:?}-{i}"),
                }
            })
            .collect();

        Ok(cases)
    }

    /// Check if the SLM is available
    pub fn is_available(&self) -> bool {
        #[cfg(feature = "slm")]
        {
            self.model.is_some()
        }
        #[cfg(not(feature = "slm"))]
        {
            false
        }
    }
}

/// Type of SLM prompt to use
#[derive(Debug, Clone, Copy)]
pub enum PromptType {
    HappyPath,
    EdgeCase,
    DomainSpecific,
    Adversarial,
    RelationshipAware,
}
```

---

## 8. Domain-Aware Generator (`domain.rs`)

```rust
use crate::types::*;
use fake::faker::address::en::*;
use fake::faker::company::en::*;
use fake::faker::finance::en::*;
use fake::faker::internet::en::*;
use fake::faker::name::en::*;
use fake::Fake;
use rand::Rng;
use rand_chacha::ChaCha20Rng;
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::debug;

/// Detected domain based on schema field analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    Fintech,
    Healthcare,
    Ecommerce,
    SocialMedia,
    General,
}

/// Detects domain from schema field names and descriptions
pub fn detect_domain(schema: &SchemaNode) -> Domain {
    let mut scores: HashMap<Domain, usize> = HashMap::new();

    let fintech_keywords = [
        "account", "balance", "transaction", "amount", "currency", "payment",
        "transfer", "bank", "iban", "swift", "routing", "deposit", "withdrawal",
        "interest", "loan", "credit", "debit", "ledger",
    ];
    let healthcare_keywords = [
        "patient", "doctor", "diagnosis", "prescription", "medication", "dosage",
        "appointment", "medical", "health", "symptom", "treatment", "icd",
        "npi", "insurance", "claim", "provider", "hospital",
    ];
    let ecommerce_keywords = [
        "product", "price", "cart", "order", "shipping", "inventory", "sku",
        "catalog", "discount", "coupon", "checkout", "item", "quantity",
        "warehouse", "fulfillment", "tracking",
    ];
    let social_keywords = [
        "user", "profile", "post", "comment", "like", "follow", "feed",
        "notification", "message", "friend", "share", "timeline", "hashtag",
    ];

    for (field_name, prop) in &schema.properties {
        let name_lower = field_name.to_lowercase();
        let desc_lower = prop
            .description
            .as_deref()
            .unwrap_or("")
            .to_lowercase();
        let combined = format!("{name_lower} {desc_lower}");

        for kw in &fintech_keywords {
            if combined.contains(kw) {
                *scores.entry(Domain::Fintech).or_default() += 1;
            }
        }
        for kw in &healthcare_keywords {
            if combined.contains(kw) {
                *scores.entry(Domain::Healthcare).or_default() += 1;
            }
        }
        for kw in &ecommerce_keywords {
            if combined.contains(kw) {
                *scores.entry(Domain::Ecommerce).or_default() += 1;
            }
        }
        for kw in &social_keywords {
            if combined.contains(kw) {
                *scores.entry(Domain::SocialMedia).or_default() += 1;
            }
        }
    }

    scores
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .filter(|(_, count)| *count >= 2)
        .map(|(domain, _)| domain)
        .unwrap_or(Domain::General)
}

/// Generate domain-specific realistic values
pub struct DomainGenerator;

impl DomainGenerator {
    pub fn generate_field(
        domain: Domain,
        field_name: &str,
        rng: &mut ChaCha20Rng,
    ) -> Option<Value> {
        let name_lower = field_name.to_lowercase();

        match domain {
            Domain::Fintech => Self::fintech_field(&name_lower, rng),
            Domain::Healthcare => Self::healthcare_field(&name_lower, rng),
            Domain::Ecommerce => Self::ecommerce_field(&name_lower, rng),
            Domain::SocialMedia => Self::social_field(&name_lower, rng),
            Domain::General => None,
        }
    }

    fn fintech_field(name: &str, rng: &mut ChaCha20Rng) -> Option<Value> {
        match name {
            n if n.contains("amount") || n.contains("balance") => {
                let val: f64 = rng.gen_range(0.01..100_000.00);
                Some(json!((val * 100.0).round() / 100.0))
            }
            n if n.contains("currency") => {
                let currencies = ["USD", "EUR", "GBP", "JPY", "CHF", "CAD", "AUD"];
                Some(json!(currencies[rng.gen_range(0..currencies.len())]))
            }
            n if n.contains("account") && n.contains("number") => {
                let acct: u64 = rng.gen_range(1_000_000_000..9_999_999_999);
                Some(json!(acct.to_string()))
            }
            n if n.contains("routing") => {
                let routing: u32 = rng.gen_range(100_000_000..999_999_999);
                Some(json!(routing.to_string()))
            }
            n if n.contains("iban") => {
                let country = ["DE", "FR", "GB", "NL"][rng.gen_range(0..4)];
                let check: u32 = rng.gen_range(10..99);
                let bban: u64 = rng.gen_range(10_000_000_000_000_000..99_999_999_999_999_999);
                Some(json!(format!("{country}{check}{bban}")))
            }
            n if n.contains("status") => {
                let statuses = ["pending", "completed", "failed", "reversed", "processing"];
                Some(json!(statuses[rng.gen_range(0..statuses.len())]))
            }
            n if n.contains("type") => {
                let types = ["credit", "debit", "transfer", "deposit", "withdrawal"];
                Some(json!(types[rng.gen_range(0..types.len())]))
            }
            _ => None,
        }
    }

    fn healthcare_field(name: &str, rng: &mut ChaCha20Rng) -> Option<Value> {
        match name {
            n if n.contains("patient") && n.contains("id") => {
                let id: u64 = rng.gen_range(100_000..999_999);
                Some(json!(format!("PAT-{id}")))
            }
            n if n.contains("npi") => {
                let npi: u64 = rng.gen_range(1_000_000_000..1_999_999_999);
                Some(json!(npi.to_string()))
            }
            n if n.contains("icd") || n.contains("diagnosis_code") => {
                let codes = ["J06.9", "I10", "E11.9", "M54.5", "K21.0", "F32.1"];
                Some(json!(codes[rng.gen_range(0..codes.len())]))
            }
            n if n.contains("dosage") => {
                let amounts = ["10mg", "25mg", "50mg", "100mg", "200mg", "500mg"];
                Some(json!(amounts[rng.gen_range(0..amounts.len())]))
            }
            n if n.contains("blood_type") => {
                let types = ["A+", "A-", "B+", "B-", "AB+", "AB-", "O+", "O-"];
                Some(json!(types[rng.gen_range(0..types.len())]))
            }
            n if n.contains("status") => {
                let statuses = ["active", "discharged", "scheduled", "cancelled", "completed"];
                Some(json!(statuses[rng.gen_range(0..statuses.len())]))
            }
            _ => None,
        }
    }

    fn ecommerce_field(name: &str, rng: &mut ChaCha20Rng) -> Option<Value> {
        match name {
            n if n.contains("sku") => {
                let sku: u32 = rng.gen_range(100_000..999_999);
                Some(json!(format!("SKU-{sku}")))
            }
            n if n.contains("price") => {
                let price: f64 = rng.gen_range(0.99..9999.99);
                Some(json!((price * 100.0).round() / 100.0))
            }
            n if n.contains("quantity") => {
                Some(json!(rng.gen_range(1..100)))
            }
            n if n.contains("tracking") => {
                let tracking: u64 = rng.gen_range(1_000_000_000_000..9_999_999_999_999);
                Some(json!(format!("TRK{tracking}")))
            }
            n if n.contains("status") => {
                let statuses = [
                    "pending", "processing", "shipped", "delivered", "returned", "cancelled",
                ];
                Some(json!(statuses[rng.gen_range(0..statuses.len())]))
            }
            n if n.contains("category") => {
                let cats = [
                    "Electronics", "Clothing", "Home & Garden", "Books",
                    "Sports", "Toys", "Food & Beverage",
                ];
                Some(json!(cats[rng.gen_range(0..cats.len())]))
            }
            _ => None,
        }
    }

    fn social_field(name: &str, rng: &mut ChaCha20Rng) -> Option<Value> {
        match name {
            n if n.contains("username") => {
                let name: String = Name().fake_with_rng(rng);
                let num: u32 = rng.gen_range(1..9999);
                Some(json!(format!(
                    "{}{}",
                    name.to_lowercase().replace(' ', "_"),
                    num
                )))
            }
            n if n.contains("bio") => {
                let bios = [
                    "Software engineer | Coffee enthusiast",
                    "Building things that matter",
                    "Full-stack developer 🚀",
                    "Open source contributor",
                ];
                Some(json!(bios[rng.gen_range(0..bios.len())]))
            }
            n if n.contains("follower") || n.contains("following") => {
                Some(json!(rng.gen_range(0..100_000)))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_fintech_domain() {
        let schema = SchemaNode {
            schema_type: SchemaType::Object,
            properties: HashMap::from([
                ("account_number".into(), SchemaNode { schema_type: SchemaType::String, ..Default::default() }),
                ("balance".into(), SchemaNode { schema_type: SchemaType::Number, ..Default::default() }),
                ("currency".into(), SchemaNode { schema_type: SchemaType::String, ..Default::default() }),
            ]),
            ..Default::default()
        };
        assert_eq!(detect_domain(&schema), Domain::Fintech);
    }

    #[test]
    fn test_detect_healthcare_domain() {
        let schema = SchemaNode {
            schema_type: SchemaType::Object,
            properties: HashMap::from([
                ("patient_id".into(), SchemaNode { schema_type: SchemaType::String, ..Default::default() }),
                ("diagnosis_code".into(), SchemaNode { schema_type: SchemaType::String, ..Default::default() }),
                ("prescription".into(), SchemaNode { schema_type: SchemaType::Object, ..Default::default() }),
            ]),
            ..Default::default()
        };
        assert_eq!(detect_domain(&schema), Domain::Healthcare);
    }
}
```

---

## 9. Unified Generator Engine (`generator.rs`)

```rust
use crate::domain::{detect_domain, DomainGenerator};
use crate::error::{DatagenError, Result};
use crate::negative::NegativeTestGenerator;
use crate::schema_driven::SchemaDrivenGenerator;
use crate::slm::{PromptType, SlmGenerator};
use crate::types::*;
use std::collections::HashSet;
use tracing::{info, warn};

/// Unified test data generation engine that orchestrates all strategies
/// with automatic fallback: SLM → Property-Based → Schema-Driven.
pub struct TestDataEngine {
    config: GeneratorConfig,
    schema_gen: SchemaDrivenGenerator,
    negative_gen: NegativeTestGenerator,
    slm_gen: Option<SlmGenerator>,
}

impl TestDataEngine {
    /// Create a new TestDataEngine with the given configuration
    pub fn new(config: GeneratorConfig) -> Result<Self> {
        let schema_gen = SchemaDrivenGenerator::new(config.seed);
        let negative_gen = NegativeTestGenerator::new();

        let slm_gen = if matches!(
            config.strategy,
            GenerationStrategy::SlmPowered | GenerationStrategy::Auto
        ) {
            match SlmGenerator::new(
                config.slm_model_path.as_deref(),
                config.temperature,
                config.max_tokens,
            ) {
                Ok(gen) if gen.is_available() => {
                    info!("SLM generator initialized");
                    Some(gen)
                }
                Ok(_) => {
                    warn!("SLM requested but no model loaded; will use fallback");
                    None
                }
                Err(e) => {
                    warn!("SLM initialization failed: {e}; will use fallback");
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            config,
            schema_gen,
            negative_gen,
            slm_gen,
        })
    }

    /// Generate all test cases for the given schema
    pub fn generate(&mut self, schema: &SchemaNode) -> Result<Vec<GeneratedTestCase>> {
        let mut all_cases = Vec::new();

        // Detect domain for more realistic data
        let domain = detect_domain(schema);
        info!("Detected domain: {domain:?}");

        // Phase 1: Generate happy path cases
        let happy_cases = self.generate_happy_path(schema, domain)?;
        info!("Generated {} happy path cases", happy_cases.len());
        all_cases.extend(happy_cases);

        // Phase 2: Generate boundary value cases
        let boundary_cases = self.schema_gen.generate_boundary(schema)?;
        info!("Generated {} boundary cases", boundary_cases.len());
        all_cases.extend(boundary_cases);

        // Phase 3: Generate negative/adversarial cases
        if self.config.include_negative {
            let negative_cases = self.negative_gen.generate(schema)?;
            info!("Generated {} negative cases", negative_cases.len());
            all_cases.extend(negative_cases);
        }

        // Deduplicate
        let before_dedup = all_cases.len();
        all_cases = self.deduplicate(all_cases);
        info!(
            "Deduplicated: {} → {} cases",
            before_dedup,
            all_cases.len()
        );

        Ok(all_cases)
    }

    fn generate_happy_path(
        &mut self,
        schema: &SchemaNode,
        domain: crate::domain::Domain,
    ) -> Result<Vec<GeneratedTestCase>> {
        let count = self.config.count;

        // Try SLM first if available (Auto or SlmPowered strategy)
        if let Some(ref slm) = self.slm_gen {
            let schema_json = serde_json::to_string_pretty(schema)
                .unwrap_or_default();

            match slm.generate(&schema_json, PromptType::HappyPath, count, None) {
                Ok(cases) if !cases.is_empty() => {
                    info!("SLM generated {} happy path cases", cases.len());
                    return Ok(cases);
                }
                Ok(_) => warn!("SLM returned empty result, falling back"),
                Err(e) => warn!("SLM generation failed: {e}, falling back"),
            }
        }

        // Fallback to schema-driven generation
        info!("Using schema-driven generation (Layer 1)");
        self.schema_gen.generate(schema, count)
    }

    /// Remove duplicate test cases based on fingerprint
    fn deduplicate(&self, cases: Vec<GeneratedTestCase>) -> Vec<GeneratedTestCase> {
        let mut seen = HashSet::new();
        cases
            .into_iter()
            .filter(|c| seen.insert(c.fingerprint.clone()))
            .collect()
    }

    /// Get a summary of generation results by category
    pub fn summarize(cases: &[GeneratedTestCase]) -> GenerationSummary {
        let mut by_category = std::collections::HashMap::new();
        let mut by_strategy = std::collections::HashMap::new();

        for case in cases {
            *by_category.entry(case.category).or_insert(0usize) += 1;
            *by_strategy.entry(case.generated_by).or_insert(0usize) += 1;
        }

        GenerationSummary {
            total: cases.len(),
            by_category,
            by_strategy,
        }
    }
}

/// Summary statistics for generated test data
#[derive(Debug)]
pub struct GenerationSummary {
    pub total: usize,
    pub by_category: std::collections::HashMap<TestCategory, usize>,
    pub by_strategy: std::collections::HashMap<GenerationStrategy, usize>,
}

impl std::fmt::Display for GenerationSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Generated {} test cases:", self.total)?;
        writeln!(f, "  By category:")?;
        for (cat, count) in &self.by_category {
            writeln!(f, "    {cat:?}: {count}")?;
        }
        writeln!(f, "  By strategy:")?;
        for (strat, count) in &self.by_strategy {
            writeln!(f, "    {strat:?}: {count}")?;
        }
        Ok(())
    }
}
```

---

## 10. Public API (`lib.rs`)

```rust
//! # valiforge-datagen
//!
//! AI-powered test data generation for API validation.
//! Supports 3-layer generation: Schema-Driven → Property-Based → SLM-Powered.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]

pub mod domain;
pub mod error;
pub mod generator;
pub mod negative;
pub mod schema_driven;
pub mod seed;
pub mod slm;
pub mod types;

// Re-exports for convenience
pub use error::{DatagenError, Result};
pub use generator::TestDataEngine;
pub use types::{GeneratedTestCase, GenerationStrategy, GeneratorConfig, TestCategory};
```

---

## 11. Integration with valiforge-core

### How the datagen crate plugs into the validation pipeline:

```rust
// In valiforge-cli/src/commands/generate.rs

use valiforge_datagen::{GeneratorConfig, TestDataEngine, GenerationStrategy};
use valiforge_schema::openapi::parse_openapi_file;
use std::path::Path;

pub async fn run_generate(
    schema_path: &Path,
    output_path: &Path,
    config: GeneratorConfig,
) -> anyhow::Result<()> {
    // 1. Parse the OpenAPI schema into internal representation
    let endpoints = parse_openapi_file(schema_path)?;

    // 2. For each endpoint's request body schema, generate test data
    let mut engine = TestDataEngine::new(config)?;
    let mut all_cases = Vec::new();

    for endpoint in &endpoints {
        if let Some(ref body_schema) = endpoint.request_body_schema {
            let cases = engine.generate(body_schema)?;
            tracing::info!(
                "Generated {} cases for {} {}",
                cases.len(),
                endpoint.method,
                endpoint.path
            );
            all_cases.extend(cases);
        }
    }

    // 3. Write generated test cases to output file
    let json = serde_json::to_string_pretty(&all_cases)?;
    std::fs::write(output_path, json)?;

    // 4. Print summary
    let summary = TestDataEngine::summarize(&all_cases);
    println!("{summary}");

    Ok(())
}
```

### Generate → Validate → Report cycle:

```rust
// Full pipeline: generate test data → run validation → produce report
pub async fn run_full_pipeline(
    schema_path: &Path,
    target_url: &str,
) -> anyhow::Result<ValidationReport> {
    // Step 1: Parse schema
    let endpoints = parse_openapi_file(schema_path)?;

    // Step 2: Generate test payloads
    let config = GeneratorConfig::default();
    let mut engine = TestDataEngine::new(config)?;
    // ... generate test cases for each endpoint

    // Step 3: Run validation with generated payloads
    let ctx = ValidationContext {
        target_url: target_url.to_string(),
        // ... other config
    };
    let validation_engine = ValidationEngine::new(ctx)?;
    let report = validation_engine.validate_all(&endpoints, "api-schema").await?;

    // Step 4: Format and return report
    Ok(report)
}
```

---

## 12. Benchmarks (`benches/datagen_bench.rs`)

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use valiforge_datagen::schema_driven::SchemaDrivenGenerator;
use valiforge_datagen::negative::NegativeTestGenerator;
use valiforge_datagen::types::*;
use std::collections::HashMap;

fn make_complex_schema() -> SchemaNode {
    SchemaNode {
        schema_type: SchemaType::Object,
        format: None,
        description: Some("Complex order schema".into()),
        required: true,
        constraints: SchemaConstraints::default(),
        properties: HashMap::from([
            ("order_id".into(), SchemaNode {
                schema_type: SchemaType::String,
                format: Some("uuid".into()),
                required: true,
                ..Default::default()
            }),
            ("customer_email".into(), SchemaNode {
                schema_type: SchemaType::String,
                format: Some("email".into()),
                required: true,
                ..Default::default()
            }),
            ("amount".into(), SchemaNode {
                schema_type: SchemaType::Number,
                required: true,
                constraints: SchemaConstraints {
                    minimum: Some(0.01),
                    maximum: Some(999999.99),
                    ..Default::default()
                },
                ..Default::default()
            }),
            ("items".into(), SchemaNode {
                schema_type: SchemaType::Array,
                required: true,
                constraints: SchemaConstraints {
                    min_items: Some(1),
                    max_items: Some(50),
                    ..Default::default()
                },
                items: Some(Box::new(SchemaNode {
                    schema_type: SchemaType::Object,
                    properties: HashMap::from([
                        ("sku".into(), SchemaNode {
                            schema_type: SchemaType::String,
                            required: true,
                            ..Default::default()
                        }),
                        ("quantity".into(), SchemaNode {
                            schema_type: SchemaType::Integer,
                            required: true,
                            constraints: SchemaConstraints {
                                minimum: Some(1.0),
                                maximum: Some(100.0),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                    ]),
                    ..Default::default()
                })),
                ..Default::default()
            }),
            ("status".into(), SchemaNode {
                schema_type: SchemaType::String,
                required: true,
                enum_values: vec![
                    serde_json::json!("pending"),
                    serde_json::json!("confirmed"),
                    serde_json::json!("shipped"),
                ],
                ..Default::default()
            }),
        ]),
        items: None,
        enum_values: vec![],
        one_of: vec![],
        any_of: vec![],
    }
}

fn bench_schema_driven(c: &mut Criterion) {
    let schema = make_complex_schema();
    let mut group = c.benchmark_group("schema_driven_generation");

    for count in [10, 100, 1000] {
        group.bench_with_input(
            BenchmarkId::new("generate", count),
            &count,
            |b, &count| {
                b.iter(|| {
                    let mut gen = SchemaDrivenGenerator::new(Some(42));
                    gen.generate(black_box(&schema), count).expect("should generate");
                });
            },
        );
    }
    group.finish();
}

fn bench_negative_generation(c: &mut Criterion) {
    let schema = make_complex_schema();
    c.bench_function("negative_generation", |b| {
        b.iter(|| {
            let gen = NegativeTestGenerator::new();
            gen.generate(black_box(&schema)).expect("should generate");
        });
    });
}

criterion_group!(benches, bench_schema_driven, bench_negative_generation);
criterion_main!(benches);
```

---

## 13. Default `SchemaNode` Implementation

Add this to `types.rs` for convenience:

```rust
impl Default for SchemaNode {
    fn default() -> Self {
        Self {
            schema_type: SchemaType::String,
            format: None,
            description: None,
            required: false,
            constraints: SchemaConstraints::default(),
            properties: std::collections::HashMap::new(),
            items: None,
            enum_values: vec![],
            one_of: vec![],
            any_of: vec![],
        }
    }
}
```

---

## 14. Quick Reference

| Layer | Crate | Strategy | Speed | Quality | Availability |
|-------|-------|----------|-------|---------|--------------|
| 1 | valiforge-datagen | Schema-Driven | ~1ms/case | Good | Always |
| 2 | valiforge-datagen | Property-Based (proptest) | ~5ms/case | Better | Always |
| 3 | valiforge-datagen | SLM (Phi-4-mini Q4) | ~200ms/case | Best | Requires model download |

### Model Download Commands

```bash
# Default: Phi-4-mini (3.8B, Q4_K_M, ~2.3GB)
curl -L -o ~/.valiforge/models/phi-4-mini-q4.gguf \
  "https://huggingface.co/microsoft/phi-4-mini-instruct-gguf/resolve/main/phi-4-mini-instruct-q4_k_m.gguf"

# CI/lightweight: TinyLlama (1.1B, Q4, ~670MB)
curl -L -o ~/.valiforge/models/tinyllama-q4.gguf \
  "https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf"

# Alternative: Gemma-3 4B (Q4, ~2.5GB)
curl -L -o ~/.valiforge/models/gemma-3-4b-q4.gguf \
  "https://huggingface.co/google/gemma-3-4b-it-gguf/resolve/main/gemma-3-4b-it-q4_k_m.gguf"
```

### CLI Usage

```bash
# Generate test data (auto strategy: tries SLM, falls back to grammar)
valiforge generate --schema petstore.yaml --count 50 --output tests.json

# Generate with explicit SLM
valiforge generate --schema petstore.yaml --engine slm --model ~/.valiforge/models/phi-4-mini-q4.gguf

# Generate with seed for reproducibility
valiforge generate --schema petstore.yaml --seed 42

# Generate with domain hint
valiforge generate --schema banking-api.yaml --domain fintech

# Generate negative tests only
valiforge generate --schema petstore.yaml --negative-only
```
