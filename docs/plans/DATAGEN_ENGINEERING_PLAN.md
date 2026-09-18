# ValiForge: AI-Powered Test Data Generation — Engineering Plan

**Author:** AI/ML Engineering Team
**Date:** 2026-03-13
**Version:** 1.0
**Status:** Draft

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Schema-Driven Generation Tasks](#2-schema-driven-generation-tasks)
3. [Negative Test Case Generation Tasks](#3-negative-test-case-generation-tasks)
4. [SLM Integration Tasks](#4-slm-integration-tasks)
5. [Domain-Aware Generation Tasks](#5-domain-aware-generation-tasks)
6. [Performance & Resource Optimization Tasks](#6-performance--resource-optimization-tasks)
7. [Testing Strategy Tasks](#7-testing-strategy-tasks)
8. [Summary: Effort, Critical Path, Sprint Plan](#8-summary)
9. [Risks & Mitigations](#9-risks--mitigations)
10. [SLM Prompt Templates](#10-slm-prompt-templates)

---

## 1. Architecture Overview

### Layered Generation Strategy

```
┌─────────────────────────────────────────────────────────┐
│                   ValiForge CLI / API                    │
│               --engine slm|grammar|auto                 │
├─────────────────────────────────────────────────────────┤
│  Layer 3: SLM-Powered (Premium)                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │ Phi-4-mini / TinyLlama / Gemma-3 4B             │    │
│  │ Domain-aware, creative edge cases, realistic     │    │
│  │ Fallback: retry once → Layer 2                   │    │
│  └─────────────────────────────────────────────────┘    │
│                         ▲ fallback                       │
│  Layer 2: Property-Based (Enhanced)                     │
│  ┌─────────────────────────────────────────────────┐    │
│  │ proptest strategies, shrinking, invariant checks │    │
│  │ Deterministic with seed, composable generators   │    │
│  └─────────────────────────────────────────────────┘    │
│                         ▲ fallback                       │
│  Layer 1: Schema-Driven (Base — Always Available)       │
│  ┌─────────────────────────────────────────────────┐    │
│  │ JSON Schema → typed generators, boundary values  │    │
│  │ Format-aware (email, UUID, date-time, etc.)      │    │
│  │ fake crate for realistic base data               │    │
│  └─────────────────────────────────────────────────┘    │
├─────────────────────────────────────────────────────────┤
│  Cross-Cutting: Diversity Scorer, Seed Manager,         │
│  Negative Injector, Domain Detector, Output Validator   │
└─────────────────────────────────────────────────────────┘
```

### Automatic Fallback Logic

```
fn generate(schema, engine, config) -> Vec<Payload> {
    match engine {
        Engine::Auto => {
            if slm_available() && config.allow_slm {
                match try_slm_generate(schema, config) {
                    Ok(payloads) if validate_all(&payloads, schema) => payloads,
                    _ => {
                        warn!("SLM generation failed, falling back to property-based");
                        proptest_generate(schema, config)
                    }
                }
            } else {
                proptest_generate(schema, config)
            }
        }
        Engine::Slm => try_slm_generate(schema, config)
            .unwrap_or_else(|_| grammar_generate(schema, config)),
        Engine::Grammar => grammar_generate(schema, config),
    }
}
```

### Deterministic Seeding Model

Every generator accepts an optional `u64` seed. When provided, the same seed + schema + engine combination produces identical output. Seeds propagate through all layers:

- **Layer 1:** `StdRng::seed_from_u64(seed)` drives all `fake` and custom generators.
- **Layer 2:** `proptest::test_runner::TestRunner::new_with_rng(seed)`.
- **Layer 3:** SLM temperature set to 0.0 with seed passed to sampler. Note: SLM reproducibility is best-effort due to floating-point nondeterminism across hardware.

---

## 2. Schema-Driven Generation Tasks

---

### DATAGEN-001: OpenAPI Schema Parser

**Title:** Parse OpenAPI 3.x schemas into internal type representation

**Description:**
Build a parser that ingests OpenAPI 3.0/3.1 JSON and YAML specs and extracts all schema objects into ValiForge's internal `SchemaIR` (intermediate representation). This IR must capture field types, constraints (min/max/pattern/enum), required fields, nested objects, `$ref` resolution, and composition keywords (allOf/oneOf/anyOf). This is the foundation every other task depends on.

**Estimated Effort:** 5 days

**Dependencies:** None (foundational)

**Acceptance Criteria:**
- Parses OpenAPI 3.0 and 3.1 specs (JSON and YAML).
- Resolves all `$ref` pointers, including circular references (detected and depth-limited).
- Extracts: type, format, enum, pattern, minimum, maximum, minLength, maxLength, minItems, maxItems, required, nullable, default, example, description.
- Handles allOf (merge), oneOf (variant selection), anyOf (flexible merge).
- Handles discriminator mappings.
- Produces a `SchemaIR` enum: `Object { fields }`, `Array { items, min, max }`, `String { format, pattern, min_len, max_len, enum_values }`, `Number { min, max, multiple_of, exclusive_min, exclusive_max }`, `Integer { ... }`, `Boolean`, `Null`, `OneOf(Vec<SchemaIR>)`, `AnyOf(Vec<SchemaIR>)`.
- Unit tests for at least 20 distinct schema patterns.
- Error on unparseable specs with actionable messages.

**Best Practices & Pitfalls:**
- Use `openapiv3` crate for parsing — it handles most OAS 3.0 correctly. For 3.1, you may need `oas3` or custom handling since 3.1 aligns with JSON Schema 2020-12.
- Circular `$ref` is common in real specs (e.g., tree structures). Use a `HashSet<String>` of visited refs and cap recursion at depth 10.
- Do NOT flatten allOf eagerly — preserve the composition structure so generators can produce variants.
- `nullable: true` in OAS 3.0 vs. `type: ["string", "null"]` in OAS 3.1 — handle both.
- Store the original field `description` — it feeds domain detection later.

**Recommended Crates:**
- `openapiv3` (OAS 3.0 parsing)
- `serde_json`, `serde_yaml` (deserialization)
- `url` (for `$ref` URI resolution)

---

### DATAGEN-002: Core Type Generators

**Title:** Implement base generators for each JSON Schema primitive type

**Description:**
Create generator functions for each primitive type that respect schema constraints. Each generator takes a `SchemaIR` node and an `Rng` and produces a `serde_json::Value`. This forms the deterministic backbone of Layer 1 generation. Generators must be composable — an object generator calls field generators, which may call nested object generators.

**Estimated Effort:** 4 days

**Dependencies:** DATAGEN-001

**Acceptance Criteria:**
- `StringGenerator`: respects minLength, maxLength, pattern (via `regex_generate` or `rand_regex`), enum (uniform selection).
- `NumberGenerator` / `IntegerGenerator`: respects minimum, maximum, exclusiveMinimum, exclusiveMaximum, multipleOf.
- `BooleanGenerator`: 50/50 distribution by default.
- `ArrayGenerator`: respects minItems, maxItems, uniqueItems; generates items recursively.
- `ObjectGenerator`: generates all required fields, randomly includes optional fields (configurable probability, default 50%).
- `NullGenerator`: produces `null`.
- `OneOfGenerator`: selects one variant uniformly at random.
- `AnyOfGenerator`: selects one or merges compatible variants.
- All generators accept `&mut StdRng` for deterministic output.
- 100% of generated values pass validation against their source schema (tested with `jsonschema` crate).

**Best Practices & Pitfalls:**
- Use the builder pattern: `StringGenerator::new(schema_node).with_rng(rng).generate()`.
- For `pattern` fields, use `rand_regex::Regex` to generate matching strings. Beware: some regex patterns (e.g., with lookaheads) are not supported — fall back to a fixed example string.
- `multipleOf` with floating-point: generate the multiplier as an integer then multiply to avoid precision issues.
- `uniqueItems` arrays: generate into a `HashSet` then convert; cap retries at 1000 to avoid infinite loops on constrained schemas.
- Always bound string generation even if no maxLength is specified — use a configurable default (e.g., 256 chars) to prevent runaway generation.

**Recommended Crates:**
- `rand` (0.8+), `rand_regex`, `serde_json`
- `jsonschema` (validation in tests)

---

### DATAGEN-003: Format-Aware Generators

**Title:** Implement generators for standard JSON Schema string formats

**Description:**
JSON Schema defines standard formats (email, uri, uuid, date-time, date, time, ipv4, ipv6, hostname, etc.) that carry semantic meaning beyond raw string constraints. Implement dedicated generators for each format that produce realistic, valid values. These plug into `StringGenerator` when a `format` field is present.

**Estimated Effort:** 3 days

**Dependencies:** DATAGEN-002

**Acceptance Criteria:**
- Supported formats: `email`, `uri`, `uri-reference`, `uuid`, `date-time` (RFC 3339), `date`, `time`, `ipv4`, `ipv6`, `hostname`, `idn-email`, `idn-hostname`, `iri`, `iri-reference`, `json-pointer`, `relative-json-pointer`, `regex`, `uri-template`.
- Also support common non-standard formats: `phone`, `credit-card`, `ssn`, `zip-code`, `currency`.
- Each format generator uses the `fake` crate where available, custom logic otherwise.
- Generated emails are syntactically valid (RFC 5322).
- Generated UUIDs are valid v4.
- Generated date-times are valid RFC 3339 with timezone.
- Generated IPv4/IPv6 are syntactically valid.
- Phone numbers follow E.164 format.
- Unknown formats fall back to basic string generation with a warning log.

**Best Practices & Pitfalls:**
- Use `fake::faker::internet::en::SafeEmail` for emails — `FreeEmail` can produce duplicates.
- For `date-time`, randomize across a wide range (1970–2030) to test boundary parsing in consumers.
- IPv6 generation should include both full and compressed forms.
- Do NOT generate real-looking SSNs or credit card numbers that pass Luhn checks in default mode — provide a `--realistic-pii` flag that must be explicitly opted into, with a CLI warning.
- Register format generators in a `HashMap<String, Box<dyn FormatGenerator>>` for extensibility.

**Recommended Crates:**
- `fake` (0.9+) — primary faker library
- `uuid` — for guaranteed-valid UUIDs
- `chrono` — for date-time generation
- `phonenumber` — for E.164 validation

---

### DATAGEN-004: Boundary Value Generator

**Title:** Systematic boundary value generation for numeric and string constraints

**Description:**
For every constrained field, generate a set of boundary values that exercise the edges of the valid range. This is critical for finding off-by-one errors in validation logic. The boundary generator produces both valid boundary values (min, max, min+1, max-1) and optionally invalid boundary values (min-1, max+1) tagged for negative testing.

**Estimated Effort:** 2 days

**Dependencies:** DATAGEN-001, DATAGEN-002

**Acceptance Criteria:**
- For integer fields with min/max: generates `[min-1, min, min+1, max-1, max, max+1]` — first and last tagged as `BoundaryKind::Invalid`.
- For number fields: same logic with epsilon offsets (configurable, default 0.001).
- For string fields with minLength/maxLength: generates strings of length `[minLen-1, minLen, minLen+1, maxLen-1, maxLen, maxLen+1]`.
- For array fields with minItems/maxItems: generates arrays of length `[minItems-1, minItems, maxItems, maxItems+1]`.
- For exclusive bounds: adjusts accordingly (exclusiveMinimum means min itself is invalid).
- Returns `Vec<BoundaryTestCase>` where each case is tagged `{ value, kind: Valid|Invalid, boundary_type: Min|Max|MinPlusOne|... }`.
- When no bounds are specified, generates: `[0, 1, -1, i64::MAX, i64::MIN]` for integers, `[0.0, -0.0, f64::EPSILON, f64::MAX, f64::MIN, f64::INFINITY, f64::NEG_INFINITY, f64::NAN]` for numbers.

**Best Practices & Pitfalls:**
- Integer overflow: when max is `i64::MAX`, `max+1` wraps. Use `i64::checked_add` and skip if it overflows.
- `f64::NAN != f64::NAN` — ensure your test harness handles NaN comparison correctly.
- Exclusive bounds in JSON Schema: `exclusiveMinimum: 0` means 0 is invalid, 0.001 is the first valid value. Make sure the boundary tagged as Invalid is correct.
- For string length boundaries, use a configurable fill character (default: `'a'`) but also offer unicode fill (`'é'`, `'中'`) for multibyte testing.

**Recommended Crates:**
- `num-traits` (for generic numeric boundary computation)

---

### DATAGEN-005: Unicode & Special Character Injection

**Title:** Generate strings with Unicode edge cases and special characters

**Description:**
Many APIs break on unexpected Unicode input. Build a special character injector that augments generated strings with problematic character sequences: zero-width characters, RTL markers, emoji, surrogate pairs, null bytes, extremely long grapheme clusters, and mixed scripts. This runs as an optional enrichment pass on any string field.

**Estimated Effort:** 2 days

**Dependencies:** DATAGEN-002

**Acceptance Criteria:**
- Injection categories, each selectable:
  - `null_bytes`: `\0` embedded in strings
  - `zero_width`: ZWJ (`\u{200D}`), ZWNJ (`\u{200C}`), ZWSP (`\u{200B}`), BOM (`\uFEFF`)
  - `rtl_override`: RTL override (`\u{202E}`), LTR override (`\u{202D}`)
  - `emoji`: single emoji, multi-codepoint emoji (family, flag), emoji with skin tone modifiers
  - `surrogate`: unpaired surrogates (invalid UTF-8 — for binary protocol testing only)
  - `homoglyph`: Cyrillic `а` vs Latin `a`, Greek `ο` vs Latin `o`
  - `long_grapheme`: combining character stacks (e.g., `a` + 100 combining diacriticals)
  - `mixed_script`: Latin + CJK + Arabic in same string
  - `control_chars`: ASCII control characters 0x01–0x1F
  - `whitespace_variants`: tab, vertical tab, form feed, NBSP, ideographic space, en/em space
- Each injection can be applied at: start, middle, end, or replacing the entire string.
- Configurable injection probability (default: 10% of generated strings).
- All injected strings are valid UTF-8 unless explicitly requesting binary mode.

**Best Practices & Pitfalls:**
- Rust strings are always valid UTF-8. For surrogate pair testing, use `Vec<u8>` payloads, not `String`.
- Combining character stacks can crash renderers — cap at 100 combiners per base character.
- Some of these values will break JSON serialization (`\0` in JSON strings is `\u0000`). Ensure output is valid JSON.
- Test your injector against `serde_json::to_string` — some characters need escaping.

**Recommended Crates:**
- `unicode-segmentation` (grapheme cluster handling)
- `unicode-normalization` (NFC/NFD testing)

---

### DATAGEN-006: Proptest Strategy Builders

**Title:** Build proptest strategies from SchemaIR for property-based testing

**Description:**
Create a function `schema_to_strategy(schema: &SchemaIR) -> BoxedStrategy<serde_json::Value>` that converts any SchemaIR node into a composable proptest strategy. This enables Layer 2 generation: proptest's shrinking and counterexample minimization applied to API payloads. Users can run `valiforge fuzz --schema api.yaml --iterations 10000` to find edge cases through property-based testing.

**Estimated Effort:** 4 days

**Dependencies:** DATAGEN-001, DATAGEN-002

**Acceptance Criteria:**
- Every `SchemaIR` variant maps to a `BoxedStrategy<Value>`.
- String strategies respect minLength, maxLength, pattern, format, enum.
- Numeric strategies use `prop::num::i64::ANY.prop_filter()` with constraint filters, or `min..=max` ranges.
- Object strategies compose field strategies with `prop::collection::hash_map` or tuple mapping.
- Array strategies use `prop::collection::vec(item_strategy, min..=max)`.
- OneOf uses `prop_oneof!` across variant strategies.
- Shrinking works correctly: a shrunk counterexample still violates the property being tested.
- Can generate 10,000 payloads/second for simple schemas on a modern CPU.
- Deterministic when given a seed via `TestRunner::new_with_rng`.

**Best Practices & Pitfalls:**
- `prop_filter` with high rejection rates is slow. Prefer `prop_flat_map` to construct values that inherently satisfy constraints rather than filtering.
- For regex patterns, use `proptest::string::string_regex(pattern)` — it is much faster than generate-and-filter.
- Nested strategies can blow up in size. Set a `max_depth` parameter (default: 5) and generate leaf values (null, simple strings) beyond that depth.
- `BoxedStrategy` has allocation overhead. For hot paths, consider `SBoxedStrategy` (stack-boxed) if the strategy is small.
- proptest's default config generates 256 cases. For API fuzzing, increase to 10,000+ and use `PROPTEST_CASES` env var.

**Recommended Crates:**
- `proptest` (1.x)
- `proptest-derive` (for internal struct strategies)

---

### DATAGEN-007: Diversity Scorer

**Title:** Score and maximize diversity of generated test data sets

**Description:**
When generating N test payloads, ensure they cover diverse regions of the input space rather than clustering around defaults. Implement a diversity scorer that measures how well a set of payloads covers the schema's value space. Use this score to guide generation: if diversity is low, adjust generation parameters (wider ranges, more enum variants, more optional field combinations).

**Estimated Effort:** 3 days

**Dependencies:** DATAGEN-002, DATAGEN-006

**Acceptance Criteria:**
- Diversity dimensions measured:
  - **Type coverage**: percentage of oneOf/anyOf variants exercised.
  - **Enum coverage**: percentage of enum values used per field.
  - **Optional field coverage**: percentage of optional field presence/absence combinations exercised.
  - **Numeric distribution**: standard deviation of numeric values (higher = more diverse).
  - **String length distribution**: variety of lengths generated.
  - **Null coverage**: if nullable, both null and non-null present.
  - **Boundary coverage**: percentage of boundary values included.
- Overall diversity score: 0.0 (all identical) to 1.0 (maximum coverage).
- `DiversityConfig` with minimum thresholds per dimension (default: 0.7 overall).
- When `--engine auto`, if diversity < threshold after initial generation, trigger supplemental generation targeting uncovered regions.
- Report: `valiforge generate --report-diversity` outputs a per-field coverage table.

**Best Practices & Pitfalls:**
- Do not aim for 1.0 diversity on every run — it would require exponential payloads for schemas with many optional fields. Use probabilistic coverage.
- For optional field combinations with K optional fields, full coverage is 2^K. Instead, use pairwise coverage (all pairs of present/absent) which is O(K^2).
- Weight security-relevant fields (passwords, tokens, auth headers) higher in diversity scoring.
- Cache diversity scores incrementally — don't rescore the entire set when adding one payload.

**Recommended Crates:**
- `statrs` (statistical functions for distribution measurement)
- `itertools` (combinations for pairwise coverage)

---

## 3. Negative Test Case Generation Tasks

---

### DATAGEN-008: Invalid Type Injector

**Title:** Generate payloads with deliberately wrong types for each field

**Description:**
For every field in a schema, generate payloads where that field's value has the wrong type. If a field expects a string, inject an integer, boolean, null, array, and object. This systematically tests type validation in the target API. Each generated payload should differ from the valid payload in exactly one field (single-fault injection) to pinpoint which field causes rejection.

**Estimated Effort:** 2 days

**Dependencies:** DATAGEN-001, DATAGEN-002

**Acceptance Criteria:**
- For each field, generates payloads with: wrong primitive type (all 5 alternatives), null (if not nullable), nested object (if expects primitive), array (if expects scalar).
- Single-fault: only one field is invalid per payload; all others are valid.
- Multi-fault mode available: `--multi-fault` injects 2+ errors per payload.
- Each negative payload tagged with metadata: `{ field: "age", expected_type: "integer", injected_type: "string", injected_value: "not_a_number" }`.
- Generated payloads are valid JSON (the structure is valid, just the types are wrong).
- For arrays: injects wrong item types, mixed-type arrays when `items` specifies a single type.

**Best Practices & Pitfalls:**
- Do NOT inject `undefined` — it doesn't exist in JSON. Use `null` or omit the field instead.
- When injecting a string where a number is expected, use tricky strings: `"123"` (looks like a number), `"12.34.56"` (almost a number), `""` (empty), `"NaN"`, `"Infinity"`.
- For boolean fields, inject `"true"` and `"false"` (strings that look like booleans) and `0`/`1` (integers that some languages coerce to boolean).
- Ensure the rest of the payload is fully valid so the API's error response clearly points to the injected field.

**Recommended Crates:**
- `serde_json` (Value manipulation)

---

### DATAGEN-009: Required Field Omission Generator

**Title:** Generate payloads with systematic required field omissions

**Description:**
For schemas with N required fields, generate payloads missing each required field individually, missing all combinations of 2 fields (pairwise), and missing all required fields. This tests whether the API correctly identifies and reports missing fields. For schemas with many required fields, use pairwise combinatorial coverage rather than exhaustive 2^N combinations.

**Estimated Effort:** 2 days

**Dependencies:** DATAGEN-001, DATAGEN-002

**Acceptance Criteria:**
- Single omission: N payloads, each missing exactly one required field.
- Pairwise omission: C(N,2) payloads, each missing exactly two required fields.
- Total omission: 1 payload with all required fields missing (empty object or only optional fields).
- For schemas with N > 20 required fields, use covering arrays instead of exhaustive pairwise (keep payload count < 200).
- Each payload tagged: `{ missing_fields: ["name", "email"], category: "required_field_omission" }`.
- Optional fields remain present with valid values to isolate the error to missing required fields.

**Best Practices & Pitfalls:**
- Some APIs treat `null` and missing differently. Generate both variants: field omitted entirely vs. field present with `null` value.
- Nested required fields: if `address` is required and has its own required sub-fields, test both `address` missing entirely and `address` present but missing sub-fields.
- Use the `covering_array` algorithm (IPOG or similar) for efficient pairwise coverage when N is large.

**Recommended Crates:**
- `itertools` (combinations)

---

### DATAGEN-010: Constraint Violation Generator

**Title:** Generate payloads that violate individual constraints

**Description:**
For every constraint in the schema (minLength, maxLength, minimum, maximum, pattern, minItems, maxItems, uniqueItems, multipleOf, enum), generate a payload that violates exactly that constraint while satisfying all others. This tests granular constraint validation.

**Estimated Effort:** 3 days

**Dependencies:** DATAGEN-001, DATAGEN-004

**Acceptance Criteria:**
- For each numeric constraint: value just outside the boundary (min-1, max+1, non-multiple-of).
- For each string constraint: string too short, too long, non-matching pattern.
- For enum: value not in the enum list but of the correct type.
- For array constraints: too few items, too many items, non-unique items when `uniqueItems: true`.
- For string format: syntactically invalid email, malformed UUID, invalid date, etc.
- Single-constraint violation: only one constraint violated per payload.
- Tagged: `{ field: "email", constraint: "format", violation: "missing_@_sign", value: "userexample.com" }`.
- Includes `additionalProperties: false` testing — inject extra fields that aren't in the schema.

**Best Practices & Pitfalls:**
- Pattern violations: do not just generate random strings — generate strings that *almost* match the pattern (e.g., for `^\d{3}-\d{4}$`, generate `"12-3456"` and `"123-456"` and `"abc-defg"`).
- For enum violations, generate values that are close to enum values (typos, different casing) to test fuzzy matching.
- `multipleOf` violations: generate value = valid_multiple + 1, not random values, so it's clearly a multipleOf issue.

**Recommended Crates:**
- `regex-syntax` (for analyzing and mutating regex patterns)

---

### DATAGEN-011: Security Probe Generator

**Title:** Generate payloads containing common injection and attack patterns

**Description:**
Build a library of security probe payloads that inject common attack strings into every string field of a schema. Categories include SQL injection, XSS, command injection, path traversal, LDAP injection, and template injection. These payloads test the API's input sanitization. Each payload should be tagged with the attack category and severity.

**Estimated Effort:** 3 days

**Dependencies:** DATAGEN-001, DATAGEN-002

**Acceptance Criteria:**
- SQL injection: at least 30 patterns including `' OR 1=1--`, `'; DROP TABLE users;--`, union-based, blind boolean, time-based, stacked queries, second-order patterns.
- XSS: at least 25 patterns including `<script>alert(1)</script>`, event handlers (`onerror`, `onload`), SVG-based, polyglot XSS, DOM-based patterns, encoded variants (`&#x3C;script&#x3E;`).
- Command injection: backticks, `$(cmd)`, `; cmd`, `| cmd`, `&& cmd`, newline injection.
- Path traversal: `../../../etc/passwd`, `....//....//`, `%2e%2e%2f`, null byte injection `%00`.
- LDAP injection: `*`, `)(`, `\00`.
- Template injection (SSTI): `{{7*7}}`, `${7*7}`, `<%= 7*7 %>`.
- NoSQL injection: `{"$gt": ""}`, `{"$ne": null}`.
- Header injection: CRLF injection `\r\nX-Injected: true`.
- Each probe tagged: `{ category: "sql_injection", subcategory: "union_based", severity: "high", payload: "' UNION SELECT..." }`.
- Probes injected one-at-a-time into each string field (single-fault).
- Configurable probe categories via `--security-probes sql,xss,cmdi`.

**Best Practices & Pitfalls:**
- Do NOT include actual exploit payloads that would cause real damage if accidentally run against a production system. Use detection-only payloads (e.g., `SLEEP(5)` for time-based SQL injection detection).
- Store probe patterns in a separate data file (`probes.toml` or `probes.json`), not hardcoded, so users can extend.
- URL-encode and double-encode variants — many WAFs only check one encoding layer.
- Include both raw and JSON-escaped versions of each probe.
- Add a `--safe-mode` flag (default: on) that excludes the most dangerous probes (e.g., `DROP TABLE`).

**Recommended Crates:**
- `toml` or `serde_json` (for probe pattern storage)
- Consider bundling payloads from SecLists (MIT-licensed subsets).

---

### DATAGEN-012: Malformed Input Generator

**Title:** Generate structurally malformed payloads for robustness testing

**Description:**
Generate payloads that are structurally invalid: malformed JSON, extremely deep nesting, extremely large payloads, duplicate keys, trailing commas, single-quoted strings, and other common parser-breaking patterns. These test the API's JSON parser and request handler robustness.

**Estimated Effort:** 2 days

**Dependencies:** DATAGEN-001

**Acceptance Criteria:**
- Malformed JSON categories:
  - Truncated JSON (cut off mid-value, mid-key, mid-array).
  - Duplicate keys: `{"name": "a", "name": "b"}`.
  - Trailing commas: `{"a": 1,}`.
  - Single-quoted strings: `{'key': 'value'}`.
  - Unquoted keys: `{key: "value"}`.
  - Comments in JSON: `{"a": 1 /* comment */}`.
  - BOM prefix: `\uFEFF{"a": 1}`.
  - Wrong content-type body: XML body, form-encoded body, plain text, binary.
- Depth bombs: 1000-level deep nesting `{"a":{"a":{"a":...}}}`.
- Width bombs: object with 10,000 keys.
- Size bombs: 10MB string value, 100MB payload.
- Extra fields not in schema (for `additionalProperties: false` testing).
- Empty body, empty object `{}`, empty array `[]`, `null` as body.
- Output as raw bytes (`Vec<u8>`) since these are not valid JSON and cannot be represented as `serde_json::Value`.

**Best Practices & Pitfalls:**
- Size bombs should be configurable. Default max: 10MB. CI mode: 1MB.
- Depth bombs can crash the test runner itself. Generate the JSON as a string, don't try to parse it.
- Duplicate keys: JSON spec says behavior is undefined. Test that the API handles it deterministically.
- These payloads should be sent as raw bytes to the API, not through a JSON serializer that would reject them.

**Recommended Crates:**
- `bytes` (for raw byte payload construction)

---

### DATAGEN-013: Negative Test Case Categorizer

**Title:** Unified categorization and tagging system for all negative test cases

**Description:**
Create a taxonomy and tagging system for all negative test cases. Every generated negative payload gets a category, subcategory, severity, expected HTTP status code, and human-readable description. This enables filtering, reporting, and integration with test frameworks.

**Estimated Effort:** 1.5 days

**Dependencies:** DATAGEN-008, DATAGEN-009, DATAGEN-010, DATAGEN-011, DATAGEN-012

**Acceptance Criteria:**
- Top-level categories: `type_error`, `constraint_violation`, `required_field_missing`, `security_probe`, `malformed_input`, `boundary_value`.
- Each category has subcategories (e.g., `security_probe::sql_injection`, `constraint_violation::pattern_mismatch`).
- Severity levels: `info`, `low`, `medium`, `high`, `critical`.
- Expected HTTP status: `400` (validation error), `422` (unprocessable), `413` (payload too large), `500` (server error — indicates a bug).
- Output format: `NegativeTestCase { payload: Value|Bytes, metadata: TestCaseMetadata }`.
- Filter API: `cases.filter(|c| c.category == SecurityProbe && c.severity >= High)`.
- Serialization to JSON lines format for integration with test runners.
- Report summary: count by category, severity distribution, coverage matrix.

**Best Practices & Pitfalls:**
- Severity assignment should be from the *consumer's* perspective: an unhandled 500 error from a security probe is critical; a 400 error from a missing field is info.
- Make severity configurable — different teams have different risk tolerances.
- Use Rust enums (not strings) for categories and severities to get compile-time exhaustiveness checking.

**Recommended Crates:**
- `serde` (serialization)
- `tabled` or `comfy-table` (for CLI report tables)

---

## 4. SLM Integration Tasks

---

### DATAGEN-014: Inference Runtime Abstraction Layer

**Title:** Build a trait-based abstraction over multiple inference backends

**Description:**
Create an `InferenceBackend` trait that abstracts over llama-cpp-2 (GGUF models), ort/ONNX Runtime, and candle (pure Rust). This allows ValiForge to use the best available backend on each platform without changing generation logic. The abstraction handles model loading, tokenization, prompt formatting, and text generation.

**Estimated Effort:** 4 days

**Dependencies:** None (parallel with schema work)

**Acceptance Criteria:**
- Trait definition:
  ```rust
  trait InferenceBackend: Send + Sync {
      fn load_model(&mut self, path: &Path, config: &ModelConfig) -> Result<()>;
      fn generate(&self, prompt: &str, params: &SamplingParams) -> Result<String>;
      fn generate_batch(&self, prompts: &[String], params: &SamplingParams) -> Result<Vec<String>>;
      fn model_info(&self) -> ModelInfo; // name, param count, quantization, memory usage
      fn unload(&mut self) -> Result<()>;
  }
  ```
- `SamplingParams`: temperature, top_p, top_k, max_tokens, seed, stop_sequences, repeat_penalty.
- `LlamaCppBackend`: wraps `llama-cpp-2` crate, supports GGUF format, GPU offloading.
- `OrtBackend`: wraps `ort` crate, supports ONNX format.
- `CandleBackend`: pure Rust, no C/C++ dependencies, supports safetensors format.
- Backend auto-detection: try llama-cpp-2 first (best performance), fall back to ort, then candle.
- Feature flags: `slm-llamacpp`, `slm-ort`, `slm-candle` — at least one must be enabled for SLM features.
- Thread-safe: backend can be shared across threads with `Arc<Mutex<dyn InferenceBackend>>`.

**Best Practices & Pitfalls:**
- `llama-cpp-2` requires linking to C++ llama.cpp. This can be painful on Windows. Make it optional via feature flag.
- `ort` requires ONNX Runtime shared libraries. Bundle them or provide clear download instructions.
- `candle` is pure Rust but slower (no BLAS by default). Enable `candle-core/cuda` or `candle-core/metal` feature flags for GPU.
- Model loading is slow (2-10 seconds). Load once, reuse across generations.
- Set `max_tokens` conservatively (default: 2048) to prevent runaway generation.
- Use `stop_sequences: ["```", "\n\n\n"]` to prevent models from generating beyond the JSON payload.

**Recommended Crates:**
- `llama-cpp-2` (0.1.x) — best GGUF support
- `ort` (2.x) — ONNX Runtime bindings
- `candle-core`, `candle-transformers` — pure Rust inference
- `hf-hub` — model downloading from HuggingFace

---

### DATAGEN-015: Model Management System

**Title:** Download, cache, verify, and manage SLM model files

**Description:**
Build a model management system that downloads quantized GGUF models from HuggingFace Hub, caches them in `~/.valiforge/models/`, verifies integrity via SHA-256, and manages disk space. Support multiple models concurrently and allow users to specify custom model paths.

**Estimated Effort:** 3 days

**Dependencies:** DATAGEN-014

**Acceptance Criteria:**
- Default model registry (hardcoded with override):
  - `phi-4-mini-q4`: Phi-4-mini Q4_K_M (~2.2GB), default for local dev
  - `tinyllama-q4`: TinyLlama 1.1B Q4_K_M (~670MB), default for CI
  - `gemma3-4b-q4`: Gemma-3 4B Q4_K_M (~2.5GB), alternative
- Download with progress bar, resume support, and retry (3 attempts).
- SHA-256 verification after download; re-download on mismatch.
- Cache location: `~/.valiforge/models/` (configurable via `VALIFORGE_MODEL_DIR`).
- `valiforge model list` — show available and downloaded models.
- `valiforge model download <name>` — download a specific model.
- `valiforge model remove <name>` — delete a cached model.
- `valiforge model use <path>` — use a custom model file.
- Auto-download on first use with user confirmation prompt (skip in CI with `--yes`).
- Disk space check before download; warn if < 1GB remaining after download.
- Lock file to prevent concurrent downloads of the same model.

**Best Practices & Pitfalls:**
- Use `hf-hub` crate for HuggingFace downloads — it handles auth tokens, CDN redirects, and LFS files.
- GGUF files are large. Use streaming download with `reqwest` and write to a temp file, then rename atomically.
- SHA-256 verification is critical — corrupted models produce garbage silently.
- In CI environments, pre-download models in a Docker layer or cache step. Document this.
- Model registry should be a TOML file that users can extend, not just hardcoded.
- Rate-limit HuggingFace API calls to avoid 429 errors.

**Recommended Crates:**
- `hf-hub` (HuggingFace Hub client)
- `sha2` (SHA-256 verification)
- `indicatif` (progress bars)
- `fs2` (file locking)
- `dirs` (home directory detection)

---

### DATAGEN-016: Prompt Engineering System

**Title:** Design and implement prompt templates for structured test data generation

**Description:**
Create a prompt engineering system with templates optimized for each supported SLM. Prompts must instruct the model to output valid JSON payloads conforming to a given schema. Use few-shot examples dynamically constructed from the schema. Include chain-of-thought suppression (direct JSON output) and schema constraint reinforcement in the prompt.

**Estimated Effort:** 3 days

**Dependencies:** DATAGEN-001, DATAGEN-014

**Acceptance Criteria:**
- Prompt template structure:
  1. System instruction: role, output format, constraints.
  2. Schema description: JSON Schema in compact form.
  3. Few-shot examples: 2-3 valid examples auto-generated from Layer 1.
  4. Generation instruction: "Generate {N} diverse, realistic test payloads."
  5. Output format enforcement: "Respond with ONLY a JSON array. No explanations."
- Per-model prompt variants (different models respond differently to formatting):
  - Phi-4-mini: ChatML format `<|system|>...<|user|>...<|assistant|>`
  - TinyLlama: `<|system|>...<|user|>...<|assistant|>`
  - Gemma-3: `<start_of_turn>user\n...<end_of_turn>\n<start_of_turn>model\n`
- Temperature presets:
  - `conservative`: temp=0.3, top_p=0.9 (high validity, lower diversity)
  - `balanced`: temp=0.7, top_p=0.95 (default)
  - `creative`: temp=1.0, top_p=0.98 (max diversity, lower validity)
- Prompt size management: if schema is too large for context window, summarize or split.
- Template registry with `Handlebars` or `Tera` for maintainability.
- Prompt versioning: templates stored as files, versioned, with A/B testing support.

**Best Practices & Pitfalls:**
- Small models are bad at following complex instructions. Keep prompts under 500 tokens for TinyLlama.
- Always include the expected JSON structure in the prompt — models anchor on examples.
- Do NOT ask the model to "think step by step" — this wastes tokens and small models produce worse JSON after chain-of-thought.
- Include explicit constraints: "The email field must contain an @ sign", "The age field must be between 0 and 150".
- Use `stop_sequences` to cut generation at the closing `]` of the JSON array.
- Test prompts against all target models — what works for Phi-4-mini may fail on TinyLlama.

**Recommended Crates:**
- `tera` (template engine) or `handlebars`
- `tiktoken-rs` or `tokenizers` (for token counting)

---

### DATAGEN-017: SLM Output Parser & Validator

**Title:** Parse, validate, and repair SLM-generated JSON output

**Description:**
SLMs frequently produce malformed or non-conforming JSON. Build a robust output parser that extracts JSON from model output (which may include markdown fences, explanatory text, or truncated output), validates it against the schema, and attempts repair for common issues. This is the critical reliability layer between SLM output and ValiForge's test data pipeline.

**Estimated Effort:** 3 days

**Dependencies:** DATAGEN-001, DATAGEN-016

**Acceptance Criteria:**
- JSON extraction: find JSON array or object in model output, ignoring:
  - Markdown code fences (` ```json ... ``` `)
  - Explanatory text before/after JSON
  - Trailing commas
  - Single-line comments
- JSON repair for common SLM mistakes:
  - Missing closing brackets/braces (auto-close).
  - Trailing commas in objects and arrays.
  - Single-quoted strings → double-quoted.
  - Unquoted keys → quoted.
  - `NaN`, `Infinity` → `null`.
  - Truncated strings → close quote and finish structure.
  - JavaScript-style comments → remove.
- Schema validation: each extracted payload validated against source schema.
- Per-payload verdict: `Valid`, `Repaired(repairs: Vec<RepairAction>)`, `Invalid(errors: Vec<ValidationError>)`.
- Retry logic: if >50% of payloads are invalid after repair, regenerate with lower temperature.
- Fallback: if retry also fails, log warning and fall back to grammar-based generation for failed payloads.
- Metrics: track SLM success rate, repair rate, fallback rate per schema.

**Best Practices & Pitfalls:**
- Use a lenient JSON parser first (`json5` or `serde_json` with preprocessing), then validate with strict `jsonschema`.
- Truncated output is the #1 SLM failure mode. Always set `max_tokens` high enough for the expected output size. Estimate: `(average_payload_size_tokens * N_payloads) * 1.5`.
- Do NOT blindly trust repaired JSON — validate after repair.
- Log all repairs for debugging. Over time, common repairs indicate prompt improvements needed.
- Track metrics in a local SQLite database for prompt optimization feedback.

**Recommended Crates:**
- `jsonschema` (JSON Schema validation)
- `json5` (lenient JSON parsing)
- `serde_json` (strict parsing)

---

### DATAGEN-018: Resource Manager & GPU Detection

**Title:** Manage system resources for SLM inference (CPU, GPU, memory)

**Description:**
Build a resource manager that detects available hardware (CPU cores, RAM, GPU type and VRAM), configures inference appropriately (thread count, GPU layer offloading, memory mapping), and enforces resource limits. Prevent OOM kills and excessive CPU usage in shared environments (CI runners, developer laptops).

**Estimated Effort:** 2.5 days

**Dependencies:** DATAGEN-014

**Acceptance Criteria:**
- Hardware detection:
  - CPU: core count, architecture (x86_64/aarch64), SIMD support (AVX2, AVX-512, NEON).
  - RAM: total and available memory.
  - GPU: CUDA devices (name, VRAM, compute capability), Metal support (macOS), Vulkan support.
- Resource configuration:
  - `--threads N`: CPU threads for inference (default: physical cores / 2 to leave room for other work).
  - `--gpu-layers N`: number of model layers to offload to GPU (default: auto based on VRAM).
  - `--max-memory <bytes>`: hard limit on model memory usage (default: 50% of available RAM).
  - `--no-gpu`: force CPU-only inference.
- Memory-mapped model loading: use `mmap` for fast subsequent loads (model stays in OS page cache).
- Pre-flight check: before loading model, verify sufficient resources. Fail fast with helpful message:
  `"Model requires ~2.5GB RAM but only 1.8GB available. Use --model tinyllama-q4 (670MB) or free memory."`
- Graceful degradation: if GPU detection fails, fall back to CPU silently.
- Resource reporting: `valiforge system-info` prints detected hardware and recommended settings.

**Best Practices & Pitfalls:**
- macOS with Apple Silicon: use Metal for GPU acceleration. `llama-cpp-2` supports this natively.
- Linux CI runners often have no GPU. Default to CPU-optimized settings with lower thread count.
- `mmap` on macOS: works well but can increase memory pressure warnings. Honor `--max-memory`.
- CUDA version mismatches are common. Detect CUDA version and warn if incompatible with the bundled llama.cpp.
- Thread count: more threads != faster. For small models, 4-8 threads is often optimal. Benchmark and auto-tune.
- Docker containers: detect cgroup memory limits, not host memory.

**Recommended Crates:**
- `sysinfo` (CPU, RAM, GPU detection)
- `nvml-wrapper` (NVIDIA GPU details)
- `memmap2` (memory-mapped files)
- `num_cpus` (core count)

---

### DATAGEN-019: Batch Generation Pipeline

**Title:** Generate N test payloads efficiently in batch mode

**Description:**
Implement a batch generation pipeline that generates N payloads efficiently. For grammar-based generation, this is trivially parallel. For SLM generation, batch multiple payloads per inference call (ask the model to generate an array of N payloads) and parallelize across multiple inference calls for large N. Implement a streaming output mode for large batch sizes.

**Estimated Effort:** 2.5 days

**Dependencies:** DATAGEN-014, DATAGEN-016, DATAGEN-017

**Acceptance Criteria:**
- CLI: `valiforge generate --schema api.yaml --count 1000 --engine auto`.
- Grammar-based: uses rayon for parallel generation across CPU cores.
- SLM-based: generates in batches of `--batch-size` (default: 10 payloads per inference call).
- For count > batch_size * 10: parallel inference with `--parallel-inferences 2` (default: 1 to limit memory).
- Streaming output: `--output-format jsonl` writes one payload per line as generated (don't buffer all in memory).
- Progress bar: shows generated/total, payloads/sec, estimated time remaining.
- Deduplication: optional `--unique` flag that deduplicates payloads (exact JSON match after key sorting).
- Output formats: JSON array, JSON Lines, CSV (flattened), YAML.
- Configurable output file: `--output payloads.jsonl`.

**Best Practices & Pitfalls:**
- Do NOT load the model once per payload — load once, generate many.
- For SLM batch: asking for 50 payloads in one prompt often degrades quality. Cap at 10-15 per inference call.
- Streaming to JSONL is important for large batches — a 100K payload JSON array can OOM.
- Deduplication with exact match misses near-duplicates. Consider optional Jaccard similarity dedup (expensive but thorough).
- Progress bar should update per-batch, not per-payload, to avoid overhead.
- CSV flattening: nested objects use dot notation (`address.street`). Document this clearly.

**Recommended Crates:**
- `rayon` (parallel grammar generation)
- `indicatif` (progress bars)
- `csv` (CSV output)
- `tokio` (async streaming output)

---

### DATAGEN-020: Engine Selection & Feature Flag System

**Title:** Implement --engine flag and automatic engine selection logic

**Description:**
Build the CLI flag and configuration system for engine selection. `--engine auto` (default) uses the best available engine, `--engine slm` forces SLM (fails if unavailable), `--engine grammar` uses only grammar-based generation. Auto mode checks SLM availability, model presence, and system resources before deciding. Provide clear user feedback about which engine was selected and why.

**Estimated Effort:** 1.5 days

**Dependencies:** DATAGEN-014, DATAGEN-015, DATAGEN-018

**Acceptance Criteria:**
- CLI flag: `--engine slm|grammar|auto` (default: `auto`).
- Config file: `~/.valiforge/config.toml` with `default_engine = "auto"`.
- Auto selection logic:
  1. If SLM feature compiled in AND model downloaded AND sufficient resources → SLM.
  2. Else → grammar-based.
  3. Log reason: `"Using grammar engine: SLM model not found. Run 'valiforge model download' to enable SLM."`.
- `--engine slm` with no model: error with download instructions.
- `--engine slm` with insufficient resources: error with resource requirements.
- Environment variable override: `VALIFORGE_ENGINE=grammar`.
- Per-schema override in config: certain schemas always use grammar (e.g., simple schemas where SLM adds no value).
- Telemetry (opt-in): track engine selection distribution for product decisions.

**Best Practices & Pitfalls:**
- Auto mode should be fast — don't check GPU or model on every invocation. Cache system info in a `~/.valiforge/system_info.json` that refreshes daily.
- Clear error messages are critical. A user who sets `--engine slm` and gets a cryptic error will abandon the tool.
- Feature flags at compile time (Cargo features) vs. runtime flags (CLI): use compile time for backend availability, runtime for selection.

**Recommended Crates:**
- `clap` (CLI parsing)
- `toml` (config file)
- `tracing` (structured logging)

---

## 5. Domain-Aware Generation Tasks

---

### DATAGEN-021: Field Name Domain Detector

**Title:** Detect semantic domain from schema field names and descriptions

**Description:**
Build a classifier that infers the semantic domain of each field from its name, description, and context (parent object, sibling fields). For example, a field named `email` in an object with fields `first_name`, `last_name`, `phone` is clearly a user/contact domain. This detection feeds into domain-specific data generation, producing more realistic test payloads.

**Estimated Effort:** 3 days

**Dependencies:** DATAGEN-001

**Acceptance Criteria:**
- Detection methods (in priority order):
  1. **Exact match**: field name matches known pattern (e.g., `email`, `phone_number`, `zip_code`).
  2. **Substring/prefix match**: `user_email`, `shipping_address_line1`.
  3. **Description parsing**: field description contains "email address", "phone number", etc.
  4. **Contextual inference**: if sibling fields suggest a domain (e.g., `sku` + `price` + `quantity` → e-commerce), apply to ambiguous fields.
  5. **SLM-assisted** (optional): ask the SLM to classify ambiguous fields.
- Detected domains:
  - `person`: name, email, phone, DOB, gender, SSN
  - `address`: street, city, state, zip, country, coordinates
  - `ecommerce`: product name, SKU, price, quantity, category
  - `fintech`: account number, routing number, IBAN, currency, transaction amount
  - `healthcare`: patient ID, diagnosis code (ICD-10), medication, dosage
  - `auth`: username, password, token, API key, session ID
  - `temporal`: created_at, updated_at, expires_at, scheduled_for
  - `content`: title, body, description, tags, slug, URL
- Confidence score: 0.0–1.0 per field detection.
- Threshold: only apply domain-specific generation if confidence > 0.7 (configurable).
- Results cached per schema (detection is done once, not per payload).

**Best Practices & Pitfalls:**
- Field names are often abbreviated or domain-specific. Build a synonym table: `addr` = `address`, `qty` = `quantity`, `amt` = `amount`, `dob` = `date_of_birth`.
- Context matters: `amount` in a financial schema vs. `amount` in a recipe schema mean different things. Use sibling fields to disambiguate.
- False positives are worse than false negatives. A wrong domain produces unrealistic data that may confuse tests. Default to generic generation on low confidence.
- Make the domain detection rules extensible via a user-provided `domains.toml` file.

**Recommended Crates:**
- `aho-corasick` (fast multi-pattern matching for field name lookup)
- `strsim` (fuzzy string matching for abbreviations)

---

### DATAGEN-022: Domain-Specific Data Generators

**Title:** Implement realistic data generators for each detected domain

**Description:**
For each domain detected by DATAGEN-021, implement a specialized generator that produces realistic, internally consistent data. A person generator produces names that match the locale, emails derived from the name, and phones with correct country codes. An e-commerce generator produces products with realistic prices, SKUs, and categories. Domain generators override the base generators when domain confidence is high.

**Estimated Effort:** 5 days

**Dependencies:** DATAGEN-003, DATAGEN-021

**Acceptance Criteria:**
- **Person generator:**
  - Name: locale-aware (US: "John Smith", JP: "田中太郎", IN: "Priya Sharma").
  - Email: derived from name (j.smith@example.com) or random.
  - Phone: valid format for detected country.
  - DOB: realistic distribution (18-80 for adults, configurable).
  - Internal consistency: same person's name/email/phone are coherent within a payload.
- **Address generator:**
  - Locale-aware: US (123 Main St, Anytown, CA 90210), UK (10 Downing Street, London, SW1A 2AA), etc.
  - Coordinates match the city (approximate).
  - State/province matches country.
- **E-commerce generator:**
  - Product names from category (Electronics: "Wireless Bluetooth Headphones").
  - Prices: realistic ranges per category ($10-$50 for books, $200-$2000 for electronics).
  - SKUs: format matches common patterns (ABC-12345).
- **Fintech generator:**
  - Account numbers: correct length per type.
  - IBAN: valid check digits for specified country.
  - Currency amounts: 2 decimal places, realistic ranges.
  - Transaction types: realistic distributions.
- **Healthcare generator:**
  - ICD-10 codes: valid format (A00-Z99.9).
  - Drug names from a sample list (not real prescriptions).
  - Dosages: realistic units and ranges.
- **Auth generator:**
  - Passwords meeting common requirements (length, complexity).
  - JWT-like tokens (not valid, but structurally correct).
  - API keys: hex/base64 strings of appropriate length.
- All domain generators accept locale parameter.
- Fallback: if domain generation fails, fall back to format-aware generic generation.

**Best Practices & Pitfalls:**
- Internal consistency is the key differentiator. Random email + random name is obviously fake. Email derived from name is convincing.
- Do NOT use real data. Even "realistic" SSNs, credit cards, or IBANs should fail checksum validation by default.
- Healthcare data is sensitive territory. Document clearly that generated data is synthetic and not derived from real patient data.
- Locale data is large. Use the `fake` crate's locale support where available. For unsupported locales, fall back to `en_US`.
- Price generation: use log-normal distribution, not uniform, for realistic price distributions.

**Recommended Crates:**
- `fake` with locale features
- `iban_validate` (for generating valid IBAN structures)
- `rand_distr` (log-normal, normal distributions for realistic values)

---

### DATAGEN-023: Locale-Aware Generation System

**Title:** Support locale-specific data generation across all domains

**Description:**
Build a locale system that configures all generators with region-specific data patterns. Support major locales (en_US, en_GB, de_DE, fr_FR, ja_JP, zh_CN, ko_KR, hi_IN, pt_BR, es_MX, ar_SA) with appropriate name pools, address formats, phone formats, date formats, and currency conventions. Allow locale specification at the CLI level and per-field override.

**Estimated Effort:** 3 days

**Dependencies:** DATAGEN-022

**Acceptance Criteria:**
- CLI flag: `--locale en_US` (default), `--locale ja_JP`, `--locale auto` (detect from schema).
- Locale affects: names, addresses, phones, dates, currencies, number formatting.
- At least 12 locales supported with realistic data pools.
- `auto` locale detection: if schema description mentions "Japan" or fields use Japanese patterns, select `ja_JP`.
- Multi-locale mode: `--locale en_US,ja_JP,de_DE` generates payloads distributed across locales.
- Locale data stored in resource files (not hardcoded) for easy community contribution.
- Date format follows locale convention: US (MM/DD/YYYY), EU (DD/MM/YYYY), ISO (YYYY-MM-DD).
- Currency: locale determines default currency symbol and formatting.

**Best Practices & Pitfalls:**
- Locale data files can be large. Use lazy loading — only load the requested locale.
- CJK names have different structures (family name first). Ensure name generators respect this.
- Arabic and Hebrew are RTL — include RTL markers in generated strings to test UI rendering.
- Phone number formats vary wildly. Use `libphonenumber` patterns as reference.
- Do not conflate language and locale. `fr_CA` (French Canadian) has different addresses than `fr_FR` (French France).
- Some locales need Unicode normalization (NFC vs. NFD). Generate both forms.

**Recommended Crates:**
- `fake` (locale support)
- `chrono` with locale formatting
- `rust_icu` or `icu4x` (advanced internationalization — optional)

---

## 6. Performance & Resource Optimization Tasks

---

### DATAGEN-024: Model Quantization & Format Optimization

**Title:** Benchmark and select optimal quantization formats for each model

**Description:**
Evaluate quantization formats (Q4_K_M, Q4_K_S, Q5_K_M, Q5_K_S, Q6_K, Q8_0, F16) for each supported model across quality (JSON validity rate, data diversity) and performance (tokens/sec, memory usage, load time) metrics. Document recommended formats for each use case (local dev, CI, high-quality). Provide conversion scripts for users with custom models.

**Estimated Effort:** 3 days

**Dependencies:** DATAGEN-014, DATAGEN-015

**Acceptance Criteria:**
- Benchmark matrix: 3 models x 7 quantizations x 3 hardware profiles (Apple M2, x86 16GB, CI runner 8GB).
- Metrics per configuration:
  - Tokens per second (generation speed).
  - JSON validity rate (% of outputs that parse as valid JSON).
  - Schema conformance rate (% of valid JSON that matches the schema).
  - Memory usage (RSS peak).
  - Model load time.
  - Diversity score (from DATAGEN-007).
- Recommended defaults:
  - Local dev: Q4_K_M (best speed/quality tradeoff).
  - CI: Q4_K_S (smallest for fast download) or TinyLlama Q4_K_M.
  - High-quality: Q5_K_M (better output quality, moderate speed impact).
- Document: quantization comparison table in `docs/model-benchmarks.md`.
- Conversion tool: `valiforge model convert --input model.safetensors --output model.gguf --quant Q4_K_M`.

**Best Practices & Pitfalls:**
- Q4_K_M is the sweet spot for most use cases. Q4_K_S saves ~10% memory but noticeably degrades quality on complex schemas.
- Q8_0 and F16 are only useful for debugging quantization issues — too large for practical use.
- Benchmark on the actual target hardware — numbers from an A100 don't apply to a MacBook Air.
- Model load time matters for CI. A 5-second load time on a 10-second test run is 33% overhead.
- Use `criterion` for rigorous benchmarking with statistical analysis.

**Recommended Crates:**
- `criterion` (benchmarking)
- `llama-cpp-2` (conversion via `llama-quantize` bindings if available)

---

### DATAGEN-025: Memory-Mapped & Lazy Model Loading

**Title:** Implement memory-mapped model loading with lazy initialization

**Description:**
Models should be memory-mapped to leverage OS page caching — after the first load, subsequent loads are nearly instant. Additionally, implement lazy initialization: the SLM model is NOT loaded until the first SLM generation request. This keeps `valiforge` fast for grammar-only usage while making SLM a zero-cost option when not used.

**Estimated Effort:** 2 days

**Dependencies:** DATAGEN-014, DATAGEN-018

**Acceptance Criteria:**
- `mmap`-based model loading via llama-cpp-2's built-in mmap support (enabled by default).
- First load: 2-5 seconds (depending on model size and disk speed).
- Subsequent loads (warm cache): < 500ms.
- Lazy initialization: model loaded on first `generate()` call, not on startup.
- `OnceCell<Arc<dyn InferenceBackend>>` pattern for thread-safe lazy init.
- Pre-warm option: `--pre-warm` flag to load model during startup (useful for batch jobs).
- Unload on idle: if no generation request in `--idle-timeout 300` seconds, unload model to free memory.
- Memory reporting: `valiforge status` shows model memory usage (mapped vs. resident).

**Best Practices & Pitfalls:**
- `mmap` works great on Linux and macOS. On Windows, use `CreateFileMapping` (handled by `memmap2` crate).
- `mmap` increases virtual memory usage but not physical memory. Don't alarm users with high VIRT numbers.
- Lazy init adds latency to the first request. Document this clearly.
- `OnceCell` (or `once_cell::sync::Lazy`) is the correct pattern. Do NOT use `Mutex<Option<Backend>>` with manual init checks — it's error-prone.
- Unload-on-idle is important for long-running processes (language server mode). But don't unload if generation is expected soon — use a configurable threshold.

**Recommended Crates:**
- `memmap2`
- `once_cell`

---

### DATAGEN-026: CPU Inference Optimization

**Title:** Optimize CPU inference with SIMD and threading configuration

**Description:**
Configure llama-cpp-2 for optimal CPU inference: detect SIMD capabilities (AVX2, AVX-512, NEON for ARM), set optimal thread count, configure batch sizes for prompt processing vs. token generation, and enable memory-efficient attention. Target: 20+ tokens/sec for Phi-4-mini Q4_K_M on Apple M2, 10+ tokens/sec on x86-64 CI runner.

**Estimated Effort:** 2 days

**Dependencies:** DATAGEN-014, DATAGEN-018

**Acceptance Criteria:**
- SIMD detection: auto-detect AVX2, AVX-512, NEON. Compile llama.cpp with appropriate flags.
- Thread tuning: benchmark and auto-select optimal thread count.
  - General rule: `min(physical_cores / 2, 8)` for interactive, `physical_cores` for batch.
- Batch size tuning: larger batches for prompt processing (512 tokens), smaller for generation (1 token).
- Performance targets:
  - Apple M2 (8 cores): 25+ tokens/sec with Phi-4-mini Q4_K_M.
  - x86-64 16GB (8 cores, AVX2): 15+ tokens/sec.
  - CI runner (4 vCPUs, no AVX-512): 8+ tokens/sec with TinyLlama Q4_K_M.
- `valiforge benchmark` command: generates 100 tokens and reports tokens/sec, latency P50/P95/P99.
- Config auto-save: optimal settings saved to `~/.valiforge/tuning.toml` after first benchmark.

**Best Practices & Pitfalls:**
- More threads does NOT always mean faster inference. Profile and find the knee of the curve.
- NUMA-aware thread placement matters on multi-socket servers. Use `hwloc` if available.
- AVX-512 can cause frequency throttling on some Intel CPUs, making it slower than AVX2 in practice. Benchmark both.
- CI runners often have limited memory bandwidth — memory-bound operations won't benefit from more threads.
- `llama.cpp` thread count is set per-context. If running parallel inferences, divide threads equally.

**Recommended Crates:**
- `raw-cpuid` (SIMD detection)
- `num_cpus` (core count)

---

### DATAGEN-027: GPU Acceleration Support

**Title:** Enable GPU-accelerated inference with CUDA, Metal, and Vulkan backends

**Description:**
Implement GPU detection and layer offloading for accelerated SLM inference. Support CUDA (NVIDIA), Metal (Apple Silicon), and Vulkan (cross-platform fallback). Auto-detect available GPU and VRAM, then calculate optimal layer offloading (partial offload if VRAM is insufficient for full model).

**Estimated Effort:** 3 days

**Dependencies:** DATAGEN-014, DATAGEN-018

**Acceptance Criteria:**
- GPU detection:
  - CUDA: detect via `nvml`, report device name, VRAM, compute capability.
  - Metal: detect via system info on macOS, report unified memory.
  - Vulkan: detect via `ash` or `vulkano`, report device and VRAM.
- Layer offloading:
  - Auto mode: calculate layers that fit in VRAM = `(available_vram - 500MB buffer) / per_layer_size`.
  - Full offload: all layers on GPU (fast, but requires sufficient VRAM).
  - Partial offload: some layers on GPU, rest on CPU (flexible).
  - No offload: `--no-gpu` flag.
- Performance targets:
  - Apple M2 (Metal, full offload): 40+ tokens/sec with Phi-4-mini Q4_K_M.
  - RTX 3060 12GB (CUDA, full offload): 60+ tokens/sec.
  - Partial offload: linear speedup proportional to offloaded layers.
- Compile-time feature flags: `gpu-cuda`, `gpu-metal`, `gpu-vulkan`.
- Graceful fallback: if GPU init fails (driver issue, OOM), fall back to CPU with warning.

**Best Practices & Pitfalls:**
- Metal on macOS uses unified memory. "VRAM" = system RAM. Be conservative with offloading to avoid starving the OS.
- CUDA requires matching driver version. Detect and warn: "CUDA 12.x detected, but llama.cpp compiled for CUDA 11.x".
- Vulkan support in llama.cpp is newer and less optimized. Use as last resort.
- Multi-GPU: for simplicity, use only GPU 0. Multi-GPU inference is complex and rarely needed for 3-4B models.
- GPU inference has high fixed overhead (kernel launch, memory transfer). For tiny models (TinyLlama 1.1B), CPU may actually be faster.

**Recommended Crates:**
- `nvml-wrapper` (CUDA detection)
- `metal` (Apple Metal — via llama.cpp's Metal backend)
- `criterion` (benchmarks)

---

### DATAGEN-028: Performance Benchmark Suite

**Title:** Comprehensive benchmark suite for all generation strategies

**Description:**
Build a benchmark suite using criterion that measures generation throughput, latency, memory usage, and output quality for all engines across multiple schemas of varying complexity. Results are tracked over time to detect performance regressions. This suite runs in CI on every merge to main.

**Estimated Effort:** 2 days

**Dependencies:** DATAGEN-002, DATAGEN-006, DATAGEN-019

**Acceptance Criteria:**
- Benchmark schemas (bundled):
  - `simple`: 5 fields, no nesting. (e.g., Petstore Pet).
  - `medium`: 20 fields, 2 levels nesting, enums, arrays. (e.g., Stripe Charge).
  - `complex`: 50+ fields, 4 levels nesting, oneOf/allOf, recursive. (e.g., GitHub Event).
- Benchmarks:
  - `grammar_throughput`: payloads/sec for each schema complexity.
  - `proptest_throughput`: payloads/sec for property-based generation.
  - `slm_throughput`: tokens/sec and payloads/sec (CPU only, for CI reproducibility).
  - `memory_usage`: peak RSS during generation of 1000 payloads.
  - `schema_parse_time`: time to parse each benchmark schema.
  - `diversity_score`: diversity score of 100 payloads per engine.
  - `validity_rate`: percentage of generated payloads that pass schema validation.
- Results stored in `target/criterion/` with HTML reports.
- CI integration: fail if throughput regresses by >10% compared to baseline.
- `valiforge bench` command for users to run locally.

**Best Practices & Pitfalls:**
- SLM benchmarks are inherently noisy due to model inference variability. Use larger sample sizes and wider confidence intervals.
- Pin benchmark schemas in version control — changing them invalidates historical data.
- Benchmark on a dedicated CI runner (not a shared runner) for stable results.
- Memory benchmarks: use `jemalloc` with stats enabled, not just RSS.
- Include a "cold start" benchmark (first generation including model load) and "warm" benchmark (subsequent generations).

**Recommended Crates:**
- `criterion` (2.x)
- `jemallocator` with `stats` feature
- `peak_alloc` (simpler peak memory tracking)

---

## 7. Testing Strategy Tasks

---

### DATAGEN-029: Unit Tests for Core Generators

**Title:** Comprehensive unit tests for every generator component

**Description:**
Write unit tests for each generator (type generators, format generators, boundary value generator, domain generators). Tests verify that generated data conforms to constraints, covers expected ranges, and is deterministic when seeded. Target: 90%+ code coverage for the generation module.

**Estimated Effort:** 4 days

**Dependencies:** DATAGEN-002, DATAGEN-003, DATAGEN-004, DATAGEN-005

**Acceptance Criteria:**
- Every generator has tests for:
  - Basic generation: produces valid output.
  - Constraint adherence: respects all schema constraints (min/max/pattern/enum).
  - Determinism: same seed → same output (assert exact equality).
  - Edge cases: empty constraints, maximum constraints, conflicting constraints.
- Format generator tests:
  - Each format tested with regex validation (e.g., email matches RFC 5322 pattern).
  - At least 100 samples per format to catch intermittent failures.
- Boundary value tests:
  - Correct boundary values for each constraint type.
  - Overflow handling verified.
- Unicode injection tests:
  - Injected strings are valid UTF-8 (or valid raw bytes in binary mode).
  - JSON serialization round-trips correctly.
- Code coverage: 90%+ for `src/generators/` module (measured by `cargo-tarpaulin`).
- Test execution time: < 30 seconds for all unit tests.

**Best Practices & Pitfalls:**
- Use `#[test]` for deterministic tests, `proptest` for property-based tests that verify invariants across many inputs.
- Seeded tests should assert exact values, not just validity. This catches subtle behavior changes.
- Run tests with `RUST_BACKTRACE=1` in CI for debugging failures.
- Avoid flaky tests: never use `SystemTime::now()` as a seed in tests. Use hardcoded seeds.
- Test error paths: what happens when a schema has `minLength: 10, maxLength: 5`? Should error, not panic.

**Recommended Crates:**
- `cargo-tarpaulin` (code coverage)
- `pretty_assertions` (readable test diffs)
- `proptest` (property-based meta-tests)

---

### DATAGEN-030: Property-Based Meta-Tests

**Title:** Verify that all generated data conforms to its source schema

**Description:**
The most critical invariant of ValiForge: any payload generated from a schema MUST validate against that schema. Write property-based tests that generate random schemas, generate payloads from those schemas, and then validate the payloads. This catches generator bugs that unit tests with fixed schemas might miss.

**Estimated Effort:** 3 days

**Dependencies:** DATAGEN-001, DATAGEN-002, DATAGEN-006

**Acceptance Criteria:**
- Schema generator: a proptest strategy that generates random SchemaIR trees (bounded depth, bounded field count).
- Meta-property: `for all schema S, for all payload P generated from S: validate(P, S) == Ok`.
- Run 10,000 random schemas per test run.
- Negative meta-property: `for all schema S, for all negative payload N generated from S: validate(N, S) == Err`.
- Shrinking: when a counterexample is found, proptest shrinks both the schema and the payload to the minimal failing case.
- Known schema corpus: also test against 50 real-world schemas from the OpenAPI directory.
- Test both Layer 1 (grammar) and Layer 2 (proptest) generation.
- Layer 3 (SLM) tested separately with larger tolerance (SLM output may need repair, which is acceptable).

**Best Practices & Pitfalls:**
- Random schema generation must produce *valid* schemas. A schema with `minimum > maximum` is invalid and should not be generated.
- Limit schema depth to 5 and field count to 20 to keep tests fast. Deep schemas tested separately.
- The `jsonschema` crate used for validation must match the JSON Schema draft used by ValiForge. Pin to Draft 2020-12.
- If a meta-test fails, the shrunk counterexample is gold — save it as a regression test.
- `allOf`/`oneOf`/`anyOf` compositions are the most likely to cause failures. Weight their generation probability higher.

**Recommended Crates:**
- `proptest` (test framework)
- `jsonschema` (validation)

---

### DATAGEN-031: Snapshot Tests for Deterministic Generators

**Title:** Snapshot tests ensuring seeded generation is stable across versions

**Description:**
For deterministic generators (grammar-based with a fixed seed), create snapshot tests that capture the exact output and fail if it changes. This ensures that ValiForge updates don't silently change generated data, which would break users' reproducible test suites.

**Estimated Effort:** 1.5 days

**Dependencies:** DATAGEN-002, DATAGEN-003

**Acceptance Criteria:**
- Snapshot tests for:
  - Each primitive type generator with seed 42.
  - Each format generator with seed 42.
  - Boundary value generator for a representative schema.
  - A full complex schema (20+ fields) with seed 42.
- Snapshots stored in `tests/snapshots/` using `insta` crate.
- Snapshots reviewed and committed. Changes require explicit `cargo insta review`.
- If a generator algorithm changes intentionally, update snapshots with justification in commit message.
- Version flag: `valiforge --generator-version` prints the generator algorithm version (e.g., `gen-v1`). Major changes bump this version.

**Best Practices & Pitfalls:**
- Snapshot tests are brittle by design — that's the point. Do NOT make them "fuzzy".
- Use `insta::assert_json_snapshot!` for JSON output — it normalizes formatting.
- Snapshot files should be in version control. Configure `.gitattributes` to treat `.snap` files as text.
- When upgrading the `rand` crate, output for the same seed may change. Pin `rand` version carefully.
- Document the seed contract: "ValiForge guarantees that the same seed + schema + generator-version produces identical output."

**Recommended Crates:**
- `insta` (snapshot testing)

---

### DATAGEN-032: Integration Tests with Real-World OpenAPI Specs

**Title:** Test generation against real-world API schemas

**Description:**
Collect a corpus of real-world OpenAPI specs (Stripe, GitHub, Twilio, Petstore, Kubernetes, Slack) and write integration tests that parse each spec, generate payloads for every endpoint, and validate them against the schema. This ensures ValiForge handles the full complexity of production API schemas.

**Estimated Effort:** 3 days

**Dependencies:** DATAGEN-001, DATAGEN-002, DATAGEN-006

**Acceptance Criteria:**
- Test corpus (bundled in `tests/fixtures/openapi/`):
  - Petstore (simple, canonical example).
  - Stripe API (deeply nested, many oneOf/anyOf, discriminators).
  - GitHub API (recursive types, large enums).
  - Kubernetes API (extremely complex, allOf composition, many required fields).
  - Twilio API (moderate complexity, good format usage).
  - Slack API (webhook payloads, event types).
- For each spec:
  - Parse succeeds without errors.
  - Generate 10 payloads per request body schema.
  - 100% of grammar-generated payloads validate against the schema.
  - 90%+ of proptest-generated payloads validate.
  - Negative payloads generate without panics.
- Tests tagged `#[ignore]` for slow ones (Kubernetes takes time). Run in CI with `cargo test -- --include-ignored`.
- Regression: if a real-world spec fails, add it as a permanent test case.

**Best Practices & Pitfalls:**
- Real-world specs are often subtly invalid themselves (missing refs, incorrect types). Document known spec issues and skip those specific schemas.
- Stripe's API spec is ~2MB of YAML. Parsing takes time — use a shared, lazy-loaded fixture.
- Kubernetes API has recursive types. Ensure your parser's cycle detection works.
- Pin specific versions of each API spec (e.g., `stripe-api-2024-12-18.yaml`). APIs change over time.
- Some specs use vendor extensions (`x-stripe-*`). Ensure the parser ignores unknown extensions gracefully.

**Recommended Crates:**
- `jsonschema` (validation)
- `serde_yaml` (YAML parsing)

---

### DATAGEN-033: Criterion Benchmarks for Generation Speed

**Title:** Criterion-based benchmarks for all generation paths

**Description:**
Implement criterion benchmark groups measuring throughput, latency, and scaling characteristics of all generation engines. Benchmarks run automatically in CI to catch performance regressions.

**Estimated Effort:** 1.5 days

**Dependencies:** DATAGEN-028 (shares benchmark infrastructure)

**Acceptance Criteria:**
- Benchmark groups:
  - `parse_schema`: time to parse schemas of varying sizes.
  - `generate_single_grammar`: time to generate one payload per schema complexity.
  - `generate_batch_grammar`: time to generate 100/1000/10000 payloads.
  - `generate_single_proptest`: single payload generation time.
  - `generate_negative`: time to generate all negative cases for a schema.
  - `diversity_scoring`: time to score diversity of N payloads.
  - `domain_detection`: time to detect domains for a schema.
- Comparison benchmarks: grammar vs. proptest vs. SLM for the same schema.
- Scaling benchmarks: generation time vs. schema complexity (field count, nesting depth).
- Results in `target/criterion/` with HTML reports and comparison graphs.
- CI: `cargo bench --no-run` compiles benchmarks. `cargo bench` runs on schedule (not every PR — too slow).

**Best Practices & Pitfalls:**
- `criterion` runs each benchmark for ~5 seconds by default. For fast generators, increase iterations.
- Benchmark noise: close Chrome and Slack before running locally. Use `--warm-up-time 3` for stable results.
- Plot generation time vs. field count to verify O(n) scaling, not O(n^2).
- Separate IO benchmarks (file parsing) from CPU benchmarks (generation) to avoid disk speed interference.

**Recommended Crates:**
- `criterion` (2.x with `html_reports` feature)

---

### DATAGEN-034: Adversarial Schema Tests

**Title:** Test generation against edge-case and pathological schemas

**Description:**
Create a suite of hand-crafted adversarial schemas that exercise edge cases: circular references, deeply nested allOf/oneOf/anyOf, conflicting constraints, empty schemas, enormous enums, and other pathological patterns. These schemas are designed to break generators and ensure graceful handling.

**Estimated Effort:** 2 days

**Dependencies:** DATAGEN-001, DATAGEN-002

**Acceptance Criteria:**
- Adversarial schema categories:
  - **Circular refs**: A → B → A (direct cycle), A → B → C → A (indirect cycle).
  - **Deep nesting**: 50 levels of nested objects.
  - **Wide objects**: 1000 fields.
  - **Large enums**: enum with 10,000 values.
  - **Conflicting constraints**: `minLength: 10, maxLength: 5` (impossible — should error).
  - **Empty schema**: `{}` (matches anything — should generate diverse types).
  - **Boolean schemas**: `true` (matches anything), `false` (matches nothing).
  - **Complex composition**: `allOf` with conflicting `oneOf` branches.
  - **Recursive arrays**: array of self-referencing items.
  - **Discriminator edge cases**: missing discriminator value, ambiguous mapping.
  - **Format with conflicting pattern**: `format: email` + `pattern: ^\d+$`.
  - **additionalProperties with schema**: `additionalProperties: { type: string }`.
  - **readOnly/writeOnly**: fields that should/shouldn't appear in requests vs. responses.
- For each adversarial schema:
  - Generator does not panic (critical).
  - Generator produces valid output OR returns a clear error for impossible schemas.
  - Generation completes within 10 seconds (no infinite loops).
- All adversarial schemas stored in `tests/fixtures/adversarial/` with documentation.

**Best Practices & Pitfalls:**
- Infinite loops are the biggest risk. Always have depth counters and iteration limits.
- Conflicting constraints: detect and report as warnings, don't silently generate nothing.
- Empty schema `{}` should generate values of different types, not just empty objects.
- Boolean schema `false` should return an error "no valid value exists for this schema".
- Test with timeouts: `#[test] fn test_deep_nesting() { std::thread::Builder::new().spawn(|| { ... }).join_timeout(Duration::from_secs(10)) }`.

**Recommended Crates:**
- (No additional crates — uses existing test infrastructure)

---

## 8. Summary

### Total Estimated Effort

| Section | Tasks | Total Days |
|---------|-------|------------|
| Schema-Driven Generation | DATAGEN-001 to 007 | 24 days |
| Negative Test Generation | DATAGEN-008 to 013 | 13.5 days |
| SLM Integration | DATAGEN-014 to 020 | 19.5 days |
| Domain-Aware Generation | DATAGEN-021 to 023 | 11 days |
| Performance Optimization | DATAGEN-024 to 028 | 12 days |
| Testing Strategy | DATAGEN-029 to 034 | 15 days |
| **TOTAL** | **34 tasks** | **95 days** |

With a team of 2 engineers: ~12 weeks (3 months).
With a team of 3 engineers: ~8 weeks (2 months).

### Critical Path

The longest dependency chain determines minimum project duration:

```
DATAGEN-001 (5d) → DATAGEN-002 (4d) → DATAGEN-006 (4d) → DATAGEN-007 (3d)
     ↓                    ↓                                       ↓
DATAGEN-003 (3d)    DATAGEN-004 (2d)                     DATAGEN-030 (3d)
     ↓
DATAGEN-022 (5d) → DATAGEN-023 (3d)

Critical path: 001→002→006→007→030 = 19 days (minimum calendar time with infinite parallelism)
```

SLM path (can run in parallel):
```
DATAGEN-014 (4d) → DATAGEN-015 (3d) → DATAGEN-016 (3d) → DATAGEN-017 (3d) → DATAGEN-019 (2.5d)
     ↓
DATAGEN-018 (2.5d) → DATAGEN-025 (2d) → DATAGEN-026 (2d) → DATAGEN-027 (3d)
```

SLM critical path: 014→015→016→017→019 = 15.5 days

### Recommended Sprint Plan (2-week sprints, 2 engineers)

#### Sprint 1 (Weeks 1-2): Foundation
| Engineer A | Engineer B |
|-----------|-----------|
| DATAGEN-001: Schema Parser (5d) | DATAGEN-014: Inference Abstraction (4d) |
| DATAGEN-002: Core Type Generators (4d, starts mid-week 1) | DATAGEN-015: Model Management (3d) |
| | DATAGEN-018: Resource Manager (2.5d) |

**Sprint Goal:** Parse OpenAPI specs and generate basic typed payloads. Inference backend compiles and loads a model.

#### Sprint 2 (Weeks 3-4): Generation Depth
| Engineer A | Engineer B |
|-----------|-----------|
| DATAGEN-003: Format Generators (3d) | DATAGEN-016: Prompt Engineering (3d) |
| DATAGEN-004: Boundary Values (2d) | DATAGEN-017: Output Parser (3d) |
| DATAGEN-005: Unicode Injection (2d) | DATAGEN-020: Engine Selection (1.5d) |
| DATAGEN-006: Proptest Strategies (4d, starts late) | DATAGEN-025: Lazy Loading (2d) |

**Sprint Goal:** All Layer 1 generators working. SLM generates JSON payloads end-to-end.

#### Sprint 3 (Weeks 5-6): Negative Testing & Domain
| Engineer A | Engineer B |
|-----------|-----------|
| DATAGEN-008: Invalid Type Injector (2d) | DATAGEN-021: Domain Detector (3d) |
| DATAGEN-009: Required Field Omission (2d) | DATAGEN-022: Domain Generators (5d) |
| DATAGEN-010: Constraint Violation (3d) | |
| DATAGEN-011: Security Probes (3d) | |

**Sprint Goal:** Full negative test generation. Domain detection working for 5+ domains.

#### Sprint 4 (Weeks 7-8): Quality & Performance
| Engineer A | Engineer B |
|-----------|-----------|
| DATAGEN-012: Malformed Input (2d) | DATAGEN-023: Locale System (3d) |
| DATAGEN-013: Categorizer (1.5d) | DATAGEN-024: Quantization Benchmarks (3d) |
| DATAGEN-007: Diversity Scorer (3d) | DATAGEN-026: CPU Optimization (2d) |
| DATAGEN-019: Batch Pipeline (2.5d) | DATAGEN-027: GPU Support (3d) |

**Sprint Goal:** Complete generation pipeline with batch support. Performance optimized.

#### Sprint 5 (Weeks 9-10): Testing & Polish
| Engineer A | Engineer B |
|-----------|-----------|
| DATAGEN-029: Unit Tests (4d) | DATAGEN-030: Meta-Tests (3d) |
| DATAGEN-031: Snapshot Tests (1.5d) | DATAGEN-032: Integration Tests (3d) |
| DATAGEN-034: Adversarial Tests (2d) | DATAGEN-033: Criterion Benchmarks (1.5d) |
| | DATAGEN-028: Benchmark Suite (2d) |

**Sprint Goal:** 90%+ code coverage. All tests passing. Performance baselines established.

#### Sprint 6 (Weeks 11-12): Buffer & Hardening
- Bug fixes from testing.
- Documentation.
- CI/CD pipeline setup.
- Performance regression tracking.
- User acceptance testing.

---

## 9. Risks & Mitigations

### Risk 1: SLM Output Quality Too Low
**Probability:** Medium
**Impact:** High
**Description:** Small language models may produce JSON that frequently fails schema validation, making the SLM layer unreliable.
**Mitigation:**
- Invest heavily in prompt engineering (DATAGEN-016). Test 50+ prompt variants per model.
- Robust repair layer (DATAGEN-017) fixes common SLM mistakes.
- Automatic fallback to grammar-based generation ensures reliability.
- Track SLM validity rate metrics. If < 70% after repair, demote the model.
- Consider constrained decoding (force valid JSON tokens) via llama.cpp's grammar sampling.

### Risk 2: Model Size vs. CI Runner Resources
**Probability:** High
**Impact:** Medium
**Description:** CI runners typically have 4-8GB RAM. Even quantized models (2-3GB) consume significant resources alongside the test suite.
**Mitigation:**
- TinyLlama (670MB Q4) as CI default — fits easily.
- Grammar-only mode for resource-constrained environments.
- Pre-download models in Docker image layers, not at test time.
- `--engine grammar` flag for CI environments that can't handle SLM.

### Risk 3: Cross-Platform Compilation Complexity
**Probability:** High
**Impact:** Medium
**Description:** `llama-cpp-2` links to C++ code. Building on Windows, Linux, and macOS with different toolchains is complex.
**Mitigation:**
- Feature flags: `slm-llamacpp` is optional. ValiForge works with grammar-only by default.
- Provide pre-compiled binaries for major platforms.
- `candle` pure Rust backend as fallback (no C/C++ dependency).
- Comprehensive CI matrix: test on Linux x86_64, macOS ARM64, Windows x86_64.

### Risk 4: OpenAPI Spec Diversity
**Probability:** Medium
**Impact:** Medium
**Description:** Real-world OpenAPI specs use every corner of the specification, including vendor extensions, deprecated features, and ambiguous patterns.
**Mitigation:**
- Test against 50+ real-world specs (DATAGEN-032).
- Graceful degradation: unknown extensions are ignored, not errors.
- Community-driven issue reporting: users submit specs that fail.
- Regular updates to the OAS parser as the specification evolves.

### Risk 5: Determinism Guarantees Across Platforms
**Probability:** Medium
**Impact:** Low-Medium
**Description:** Floating-point operations may differ across CPU architectures, causing seeded generation to produce slightly different output on x86 vs. ARM.
**Mitigation:**
- Use integer operations where possible in generators.
- Document: "Determinism is guaranteed within the same platform (OS + architecture). Cross-platform determinism is best-effort."
- Snapshot tests run on the same architecture as the reference snapshots.
- Use `f64::to_bits()` for exact float comparison where needed.

### Risk 6: Security Probe Liability
**Probability:** Low
**Impact:** High
**Description:** Generated security probes could be misused to attack real systems. ValiForge could face liability or reputational issues.
**Mitigation:**
- `--safe-mode` (default: on) excludes destructive payloads.
- Clear documentation: "These payloads are for testing YOUR OWN APIs only."
- Probes are detection-focused (e.g., `SLEEP(5)` for timing), not exploitation-focused.
- Terms of use requiring responsible use.
- No real exploit development capabilities — just common OWASP pattern strings.

---

## 10. SLM Prompt Templates

### Template 1: Basic Payload Generation

```
<|system|>
You are a test data generator for REST APIs. You generate realistic, diverse JSON payloads that conform to the provided JSON Schema. Output ONLY valid JSON — no explanations, no markdown, no comments.
<|end|>

<|user|>
Generate {{count}} diverse test payloads for this JSON Schema:

```json
{{schema_json}}
```

Requirements:
- Each payload MUST conform to the schema above.
- Use realistic values (real-looking names, valid emails, reasonable numbers).
- Vary the data: different names, different values, edge cases.
- Include at least one payload with minimum valid values and one with maximum valid values.

Respond with ONLY a JSON array of {{count}} objects. No other text.
<|end|>

<|assistant|>
[
```

**Notes:**
- The `[` at the end of the assistant turn primes the model to continue with JSON array content.
- `stop_sequences: ["\n```", "\n\n\n"]` prevents generation beyond the JSON.
- `{{schema_json}}` is a compacted version of the JSON Schema (no descriptions to save tokens).
- `{{count}}` should be 5-10 for best quality. Over 15, quality degrades.

---

### Template 2: Domain-Aware Payload Generation

```
<|system|>
You are an expert test data generator specializing in {{domain}} applications. Generate realistic {{domain}} data that would appear in a production system. Output ONLY valid JSON.
<|end|>

<|user|>
Schema:
```json
{{schema_json}}
```

Domain context: This is a {{domain_description}} API endpoint.
{{#if field_hints}}
Field-specific guidance:
{{#each field_hints}}
- {{this.field}}: {{this.hint}}
{{/each}}
{{/if}}

Generate {{count}} diverse, realistic payloads. Include:
- Common cases (80% of payloads)
- Edge cases (20% of payloads): empty optional fields, boundary values, unusual but valid data

Respond with ONLY a JSON array.
<|end|>

<|assistant|>
[
```

**Notes:**
- `{{domain}}` is detected by DATAGEN-021 (e.g., "e-commerce", "fintech").
- `{{domain_description}}` is a one-line context (e.g., "product catalog management").
- `{{field_hints}}` are domain-specific hints per field (e.g., "price: realistic product price in USD, $0.99 to $9999.99").

---

### Template 3: Negative Test Case Generation via SLM

```
<|system|>
You are a security testing expert. Generate invalid and adversarial test payloads that should be REJECTED by a well-implemented API. Each payload should break exactly one validation rule. Output ONLY valid JSON.
<|end|>

<|user|>
Target schema:
```json
{{schema_json}}
```

Generate {{count}} INVALID payloads. For each payload, include a "_meta" field explaining the violation:

Example format:
[
  {
    "name": 12345,
    "_meta": {"violation": "type_error", "field": "name", "expected": "string", "got": "number"}
  }
]

Categories to cover:
1. Wrong types (string where number expected, etc.)
2. Missing required fields
3. Constraint violations (too short, too long, out of range)
4. Invalid formats (malformed email, bad UUID)
5. Boundary values (just outside valid range)

Respond with ONLY a JSON array. Include "_meta" in each object.
<|end|>

<|assistant|>
[
```

**Notes:**
- The `_meta` field is stripped after generation — it's a structured explanation for logging.
- SLM-generated negative cases complement the deterministic generators (DATAGEN-008 to 012) by producing creative, unusual violations that rule-based generators miss.
- Temperature should be higher (0.8-1.0) for negative cases to encourage creativity.

---

### Template 4: Minimal Prompt for TinyLlama (CI Mode)

```
<|system|>
Generate JSON test data matching this schema. Output only JSON array.
<|end|>

<|user|>
Schema: {{schema_json_compact}}
Examples:
{{#each few_shot_examples}}
{{this}}
{{/each}}

Generate {{count}} more:
<|end|>

<|assistant|>
[
```

**Notes:**
- TinyLlama has a 2048 token context. Keep the prompt under 500 tokens.
- `{{schema_json_compact}}` removes all descriptions and examples from the schema.
- `{{few_shot_examples}}` are 2-3 valid payloads generated by Layer 1, each on one line.
- Few-shot examples are critical for TinyLlama — it follows patterns better than instructions.
- Generate only 3-5 payloads per call for acceptable quality.

---

### Template 5: Constrained Generation with Grammar Hint

```
<|system|>
You generate test data. Follow the JSON structure EXACTLY. Every field must match the specified type and constraints.
<|end|>

<|user|>
Generate {{count}} test objects. Each MUST have:
{{#each fields}}
- "{{this.name}}": {{this.type}}{{#if this.constraints}} ({{this.constraints}}){{/if}}
{{/each}}

Rules:
{{#each rules}}
- {{this}}
{{/each}}

JSON array only:
<|end|>

<|assistant|>
[
```

**Notes:**
- This template avoids sending the raw JSON Schema (which wastes tokens). Instead, it lists fields and constraints in natural language.
- `{{fields}}` example: `[{name: "age", type: "integer", constraints: "18 to 120"}, ...]`.
- `{{rules}}` example: `["email must contain @", "phone must start with +"]`.
- Works well when the schema is too large for the context window — summarize instead of include.
- Use this as a fallback when Template 1 produces poor results due to schema complexity.

---

## Appendix A: Cargo.toml Dependencies (Recommended)

```toml
[package]
name = "valiforge"
version = "0.1.0"
edition = "2021"

[features]
default = ["grammar"]
grammar = []
slm = ["slm-llamacpp"]
slm-llamacpp = ["dep:llama-cpp-2"]
slm-ort = ["dep:ort"]
slm-candle = ["dep:candle-core", "dep:candle-transformers"]
gpu-cuda = []
gpu-metal = []
gpu-vulkan = []

[dependencies]
# Schema parsing
openapiv3 = "2.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"

# Generation
rand = "0.8"
rand_regex = "0.17"
fake = { version = "2.9", features = ["derive"] }
uuid = { version = "1.0", features = ["v4"] }
chrono = "0.4"

# Property-based testing
proptest = "1.4"

# Validation
jsonschema = "0.18"

# SLM backends (optional)
llama-cpp-2 = { version = "0.1", optional = true }
ort = { version = "2.0", optional = true }
candle-core = { version = "0.6", optional = true }
candle-transformers = { version = "0.6", optional = true }

# Model management
hf-hub = "0.3"
sha2 = "0.10"
indicatif = "0.17"

# System
sysinfo = "0.31"
memmap2 = "0.9"
num_cpus = "1.16"
once_cell = "1.19"
dirs = "5.0"

# CLI
clap = { version = "4.5", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = "0.3"

# Utilities
rayon = "1.10"
itertools = "0.13"
aho-corasick = "1.1"
toml = "0.8"
tera = "1.19"

# Data quality
statrs = "0.17"

# Output
comfy-table = "7.1"
csv = "1.3"
tokio = { version = "1.37", features = ["full"] }

[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
insta = { version = "1.38", features = ["json"] }
pretty_assertions = "1.4"
cargo-tarpaulin = "0.27"
json5 = "0.4"

[[bench]]
name = "generation_benchmarks"
harness = false
```

---

## Appendix B: Module Structure

```
src/
├── lib.rs
├── main.rs                          # CLI entry point
├── cli/
│   ├── mod.rs
│   ├── generate.rs                  # generate subcommand
│   ├── model.rs                     # model management subcommand
│   ├── benchmark.rs                 # bench subcommand
│   └── system_info.rs               # system-info subcommand
├── schema/
│   ├── mod.rs
│   ├── parser.rs                    # DATAGEN-001: OpenAPI parser
│   ├── ir.rs                        # SchemaIR types
│   └── resolver.rs                  # $ref resolution
├── generators/
│   ├── mod.rs
│   ├── traits.rs                    # Generator trait
│   ├── types/
│   │   ├── mod.rs
│   │   ├── string.rs                # DATAGEN-002: string gen
│   │   ├── number.rs                # DATAGEN-002: number gen
│   │   ├── integer.rs               # DATAGEN-002: integer gen
│   │   ├── boolean.rs               # DATAGEN-002: boolean gen
│   │   ├── array.rs                 # DATAGEN-002: array gen
│   │   ├── object.rs                # DATAGEN-002: object gen
│   │   └── null.rs                  # DATAGEN-002: null gen
│   ├── formats/
│   │   ├── mod.rs                   # DATAGEN-003: format registry
│   │   ├── email.rs
│   │   ├── uuid.rs
│   │   ├── datetime.rs
│   │   ├── uri.rs
│   │   ├── ip.rs
│   │   └── phone.rs
│   ├── boundary.rs                  # DATAGEN-004
│   ├── unicode.rs                   # DATAGEN-005
│   ├── proptest_strategies.rs       # DATAGEN-006
│   └── diversity.rs                 # DATAGEN-007
├── negative/
│   ├── mod.rs
│   ├── type_injector.rs             # DATAGEN-008
│   ├── required_omission.rs         # DATAGEN-009
│   ├── constraint_violation.rs      # DATAGEN-010
│   ├── security_probes.rs           # DATAGEN-011
│   ├── malformed.rs                 # DATAGEN-012
│   └── categorizer.rs              # DATAGEN-013
├── slm/
│   ├── mod.rs
│   ├── backend/
│   │   ├── mod.rs                   # DATAGEN-014: trait
│   │   ├── llamacpp.rs              # llama-cpp-2 backend
│   │   ├── ort.rs                   # ONNX backend
│   │   └── candle.rs                # candle backend
│   ├── models.rs                    # DATAGEN-015: model management
│   ├── prompts/
│   │   ├── mod.rs                   # DATAGEN-016: prompt system
│   │   ├── templates/               # .tera template files
│   │   └── few_shot.rs              # few-shot example generation
│   ├── parser.rs                    # DATAGEN-017: output parser
│   ├── resources.rs                 # DATAGEN-018: resource manager
│   └── batch.rs                     # DATAGEN-019: batch pipeline
├── domain/
│   ├── mod.rs
│   ├── detector.rs                  # DATAGEN-021
│   ├── generators/
│   │   ├── mod.rs                   # DATAGEN-022
│   │   ├── person.rs
│   │   ├── address.rs
│   │   ├── ecommerce.rs
│   │   ├── fintech.rs
│   │   ├── healthcare.rs
│   │   └── auth.rs
│   └── locale.rs                    # DATAGEN-023
├── engine.rs                        # DATAGEN-020: engine selection
├── config.rs                        # configuration management
└── output/
    ├── mod.rs
    ├── json.rs
    ├── jsonl.rs
    ├── csv.rs
    └── yaml.rs

tests/
├── fixtures/
│   ├── openapi/                     # DATAGEN-032: real-world specs
│   │   ├── petstore.yaml
│   │   ├── stripe.yaml
│   │   ├── github.yaml
│   │   └── kubernetes.yaml
│   ├── adversarial/                 # DATAGEN-034
│   │   ├── circular_ref.json
│   │   ├── deep_nesting.json
│   │   └── conflicting_constraints.json
│   └── probes/                      # DATAGEN-011
│       └── security_probes.toml
├── snapshots/                       # DATAGEN-031: insta snapshots
├── unit/                            # DATAGEN-029
├── property/                        # DATAGEN-030
├── integration/                     # DATAGEN-032
└── adversarial/                     # DATAGEN-034

benches/
├── generation_benchmarks.rs         # DATAGEN-033
└── schemas/                         # benchmark schemas
    ├── simple.json
    ├── medium.json
    └── complex.json
```

---

## Appendix C: Quick Reference — Task Dependency Graph

```
                    DATAGEN-001 (Schema Parser)
                   /      |       |        \
                  /       |       |         \
          002 (Types) 008(TypeInj) 009(Reqd) 021(Domain)
         / | \        010(Constr)  012(Malf)      \
        /  |  \       011(SecProb)                022(DomGen)
       /   |   \           \                        \
  003  004  005  006        013(Categorizer)       023(Locale)
  (Fmt)(Bnd)(Uni)(Prop)
   \              /
    \            /
     007(Diversity)
          |
     030(MetaTests)

  DATAGEN-014 (Inference) ──────────────────────────────
       |          \            \            \            \
   015(Models) 018(Resources) 025(Lazy)  026(CPU)   027(GPU)
       |          \                         |
   016(Prompts)  020(Engine)           028(Benchmarks)
       |
   017(Parser)
       |
   019(Batch)

  Testing (mostly depends on generators being complete):
  029(UnitTests) ← 002,003,004,005
  030(MetaTests) ← 001,002,006
  031(Snapshots) ← 002,003
  032(Integration) ← 001,002,006
  033(Criterion) ← 028
  034(Adversarial) ← 001,002
```

---

*This document is the authoritative engineering plan for ValiForge's test data generation system. All task estimates assume a senior Rust engineer familiar with the codebase. Adjust estimates +30% for onboarding new engineers.*
