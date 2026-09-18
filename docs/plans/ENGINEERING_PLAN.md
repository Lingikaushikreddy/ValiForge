# ValiForge — Complete Engineering Plan

> **Headless, API-first validation engine in Rust**
> Validates OpenAPI, gRPC, and GraphQL contracts against live endpoints.
> Detects breaking changes, generates test data, runs in CI/CD.

---

## Table of Contents

1. [Workspace Architecture](#1-workspace-architecture)
2. [Schema Parser Module](#2-schema-parser-module-valiforge-schema)
3. [Validation Engine](#3-validation-engine-valiforge-core)
4. [Breaking Change Detection](#4-breaking-change-detection-valiforge-diff)
5. [CLI Module](#5-cli-module-valiforge-cli)
6. [Report Generation](#6-report-generation-valiforge-report)
7. [Test Data Generation](#7-test-data-generation-valiforge-datagen)
8. [WASM Plugin System](#8-wasm-plugin-system-valiforge-plugin)
9. [SDK](#9-sdk-valiforge-sdk)
10. [Cross-Platform Distribution](#10-cross-platform-binary-distribution)
11. [Summary & Sprint Plan](#11-summary--sprint-plan)

---

## 1. Workspace Architecture

### Target Directory Layout

```
valiforge/
├── Cargo.toml                  # Workspace root
├── Cargo.lock
├── rust-toolchain.toml         # Pin MSRV
├── deny.toml                   # cargo-deny config
├── clippy.toml                 # Clippy lint config
├── .cargo/
│   └── config.toml             # Cross-compilation profiles
├── crates/
│   ├── valiforge-cli/          # Binary crate — clap-based CLI
│   ├── valiforge-core/         # Validation engine runtime
│   ├── valiforge-schema/       # Schema parsing (OpenAPI, Proto, GraphQL)
│   ├── valiforge-diff/         # Breaking change detection
│   ├── valiforge-report/       # Output formatters (JUnit, SARIF, JSON, Markdown, TAP)
│   ├── valiforge-datagen/      # Property-based test data generation
│   ├── valiforge-plugin/       # WASM plugin host (wasmtime)
│   └── valiforge-sdk/          # Public Rust SDK for embedding
├── tests/
│   ├── integration/            # Cross-crate integration tests
│   └── fixtures/               # Real-world schema files
├── xtask/                      # Build automation (cargo xtask)
├── .github/
│   └── workflows/
│       ├── ci.yml
│       ├── release.yml
│       └── audit.yml
└── docs/
    ├── architecture.md
    └── plugin-guide.md
```

### Dependency Graph (crate → crate)

```
valiforge-cli
  ├── valiforge-core
  ├── valiforge-schema
  ├── valiforge-diff
  ├── valiforge-report
  ├── valiforge-datagen
  └── valiforge-plugin

valiforge-core
  ├── valiforge-schema
  └── valiforge-report

valiforge-diff
  └── valiforge-schema

valiforge-datagen
  └── valiforge-schema

valiforge-sdk
  ├── valiforge-core
  ├── valiforge-schema
  ├── valiforge-diff
  └── valiforge-report
```

---

### SETUP-001: Initialize Cargo Workspace

**Description:** Create the root `Cargo.toml` with workspace members, shared dependency versions via `[workspace.dependencies]`, and shared metadata (edition, version, license, repository). Pin all shared dependencies at the workspace level so crate-level `Cargo.toml` files use `dep.workspace = true`.

**Estimated Effort:** 1 day

**Dependencies:** None

**Acceptance Criteria:**
- `cargo check --workspace` succeeds with all crates as empty libs/bins.
- All shared dependencies (serde, tokio, thiserror, etc.) declared once at workspace level.
- `cargo tree` shows no duplicate versions of any shared dependency.
- Workspace `resolver = "2"` is set.

**Best Practices:**
- Use `[workspace.dependencies]` (Rust 2021+ / Cargo 1.64+) to centralize versions.
- Set `edition = "2021"` at the workspace level via `[workspace.package]`.
- Keep the binary crate (`valiforge-cli`) thin — it should only wire together library crates.
- Never pin exact versions (`=1.2.3`) for library crates; use semver ranges (`1.2`).

**Pitfalls:**
- Forgetting `resolver = "2"` causes surprising feature unification across dev/build/normal deps.
- Do not put integration test binaries inside library crates; use the top-level `tests/` directory.

**Recommended Crates/Patterns:**
- `[workspace.package]` for shared metadata
- `[workspace.lints]` for shared clippy/rustc lint configuration (Rust 1.74+)

---

### SETUP-002: Rust Toolchain & MSRV Policy

**Description:** Create `rust-toolchain.toml` pinning the stable channel (e.g., 1.78.0). Establish MSRV at N-2 (two releases behind current stable). Add a CI job that tests against MSRV to prevent accidental breakage.

**Estimated Effort:** 0.5 days

**Dependencies:** SETUP-001

**Acceptance Criteria:**
- `rust-toolchain.toml` present with `channel = "1.78.0"` and `components = ["rustfmt", "clippy"]`.
- CI matrix includes a job with the MSRV toolchain.
- `Cargo.toml` has `rust-version = "1.76.0"` (or chosen MSRV).

**Best Practices:**
- Use `rust-version` field in `Cargo.toml` so `cargo` itself will refuse to build on too-old toolchains.
- Pin `rust-toolchain.toml` to a specific version, not `stable`, so builds are reproducible.
- Update MSRV deliberately as a semver-minor bump, never accidentally.

**Pitfalls:**
- Nightly-only features creeping in via dependencies — audit regularly.
- `rust-toolchain.toml` overrides whatever the developer has installed; document this for contributors.

---

### SETUP-003: Feature Flag Strategy

**Description:** Define feature flags for optional protocol support (`grpc`, `graphql`), optional heavyweight subsystems (`wasm-plugins`, `datagen`), and optional output formats (`sarif`, `junit`). The default feature set should be minimal: OpenAPI validation + JSON report. All features must compile independently.

**Estimated Effort:** 1 day

**Dependencies:** SETUP-001

**Acceptance Criteria:**
- `cargo check --workspace --no-default-features` compiles (core OpenAPI only).
- `cargo check --workspace --all-features` compiles.
- Each feature flag is tested in CI independently: `--features grpc`, `--features graphql`, `--features wasm-plugins`.
- Feature flags are documented in the root README with a feature matrix table.

**Best Practices:**
- Features must be **additive** — enabling a feature must never remove functionality.
- Use `cfg(feature = "...")` sparingly at module boundaries, not scattered through functions.
- Create internal `feature-check` module that emits compile_error! on incompatible combinations.
- Feature flags in library crates should bubble up via `valiforge-cli/Cargo.toml` re-exports.

**Pitfalls:**
- Feature unification in workspace builds can mask missing `#[cfg]` guards. Always test `--no-default-features` per-crate.
- Avoid features that change public API signatures; use trait-based extension instead.

**Feature Matrix:**

| Feature | Crate(s) Affected | Default | Key Deps Added |
|---|---|---|---|
| `grpc` | schema, core, diff | No | prost, tonic |
| `graphql` | schema, core, diff | No | apollo-rs |
| `wasm-plugins` | plugin | No | wasmtime, wit-bindgen |
| `datagen` | datagen | No | proptest |
| `sarif` | report | No | serde_json (already) |
| `junit` | report | No | quick-xml |

---

### SETUP-004: CI Pipeline (GitHub Actions)

**Description:** Create GitHub Actions workflows for continuous integration: lint (clippy + rustfmt), test (unit + integration + doc-tests), security (cargo-audit + cargo-deny), and coverage (llvm-cov). Use matrix strategy for MSRV and feature combinations.

**Estimated Effort:** 1.5 days

**Dependencies:** SETUP-001, SETUP-002, SETUP-003

**Acceptance Criteria:**
- PR checks enforce: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --workspace`, `cargo deny check`, `cargo audit`.
- Matrix tests: `{stable, MSRV} x {default, all-features, no-default-features}`.
- Coverage report uploaded to Codecov or similar.
- CI completes in under 15 minutes for a clean build with caching.
- Dependabot or Renovate configured for automated dependency updates.

**Best Practices:**
- Cache `~/.cargo/registry`, `~/.cargo/git`, and `target/` via `actions/cache` or `Swatinem/rust-cache`.
- Run clippy with `--all-targets` to catch issues in tests and benches too.
- Use `cargo deny` for license compliance and duplicate dependency detection.
- Separate jobs for fast-fail (fmt check runs first, tests only if fmt passes).

**Pitfalls:**
- `actions/cache` with naive key can restore stale caches and cause link errors. Key on `Cargo.lock` hash.
- macOS and Windows runners are expensive; only run them on release branches or weekly cron.
- `cargo audit` can block on advisory DB fetch — set a timeout.

**Recommended Tools:**
- `Swatinem/rust-cache@v2` for build caching
- `cargo-deny` for license and advisory checking
- `cargo-llvm-cov` for coverage
- `cargo-nextest` for faster test execution (parallel, better output)

---

### SETUP-005: Shared Error & Logging Infrastructure

**Description:** Establish the cross-crate error handling and logging approach. Use `thiserror` for library crates (structured, typed errors) and `anyhow` only in the CLI binary. Set up `tracing` for structured logging with configurable verbosity.

**Estimated Effort:** 1 day

**Dependencies:** SETUP-001

**Acceptance Criteria:**
- Each library crate defines its own error enum using `thiserror::Error`.
- Errors compose via `#[from]` across crate boundaries (e.g., `CoreError::Schema(SchemaError)`).
- `tracing` subscriber initialized in CLI with `-v` / `-vv` / `-vvv` verbosity levels.
- No library crate depends on `anyhow`.
- Errors include enough context to debug without a stack trace (file paths, line numbers, schema paths).

**Best Practices:**
- Use `#[error(transparent)]` for wrapping inner errors without losing source chains.
- Implement `From<X>` only for errors that make semantic sense — don't auto-convert everything.
- Use `tracing::instrument` on async functions for automatic span creation.
- Error enums should be `#[non_exhaustive]` so adding variants is non-breaking.

**Pitfalls:**
- `anyhow` in libraries prevents callers from matching on error variants — never use it in libs.
- Over-wrapping errors hides the root cause. Keep error chains shallow (max 3 levels).
- `tracing` without a subscriber silently drops events — always install a default subscriber early.

**Recommended Crates:**
- `thiserror` 2.x for derive macros on error enums
- `anyhow` 1.x only in `valiforge-cli`
- `tracing` + `tracing-subscriber` with `EnvFilter` for runtime log level control
- `miette` as optional alternative for rich CLI diagnostics (source-annotated errors)

---

### SETUP-006: xtask Build Automation

**Description:** Create an `xtask/` crate (binary) for custom build/dev commands: generating shell completions, running benchmarks, updating fixtures, building release binaries. This replaces Makefiles with a cross-platform Rust solution.

**Estimated Effort:** 0.5 days

**Dependencies:** SETUP-001

**Acceptance Criteria:**
- `cargo xtask codegen` generates shell completions to `target/completions/`.
- `cargo xtask dist` builds release binaries for the current platform.
- `cargo xtask fixtures` downloads/updates test fixture schemas.
- xtask is excluded from workspace default-members so `cargo build` doesn't build it.

**Best Practices:**
- Use the `xtask` pattern from `matklad/cargo-xtask`.
- Keep xtask dependencies minimal (no tokio — keep it sync and fast).
- Define an alias in `.cargo/config.toml`: `[alias] xtask = "run --package xtask --"`.

---

## 2. Schema Parser Module (valiforge-schema)

### SCHEMA-001: Core Schema Abstraction Layer

**Description:** Define protocol-agnostic schema types that represent the common concepts across OpenAPI, gRPC, and GraphQL: endpoints/operations, request/response shapes, field types, constraints, and metadata. This is the unified internal representation (IR) that all downstream crates consume.

**Estimated Effort:** 3 days

**Dependencies:** SETUP-001, SETUP-005

**Acceptance Criteria:**
- `UnifiedSchema` enum with variants for each protocol (OpenAPI, Protobuf, GraphQL).
- Common types: `Endpoint`, `Field`, `TypeDef`, `Constraint`, `SchemaMetadata`.
- All types implement `Serialize`, `Deserialize`, `Debug`, `Clone`, `PartialEq`.
- JSON round-trip tests pass for all types.
- The IR is sufficient to power both validation and diff operations.

**Best Practices:**
- Use newtypes for identifiers: `EndpointId(String)`, `FieldPath(Vec<String>)`.
- Prefer enums with data over stringly-typed fields.
- Make the IR version-aware — include schema format version in metadata.
- Keep the IR `#[non_exhaustive]` to allow future protocol additions.

**Pitfalls:**
- Over-abstracting kills protocol-specific semantics. Keep protocol-specific data accessible via downcast or variant.
- Don't try to unify everything — some concepts (gRPC streaming, GraphQL subscriptions) are protocol-specific.

**Recommended Patterns:**
- Visitor pattern for schema traversal
- `serde` for serialization with `#[serde(tag = "protocol")]` for tagged enums
- `PartialEq` + `Eq` on the IR for diff operations

---

### SCHEMA-002: OpenAPI 3.0/3.1 Parser

**Description:** Build a parser that reads OpenAPI 3.0 and 3.1 specifications (YAML and JSON) and converts them to the internal schema representation. Handle `$ref` resolution (local, remote, and circular), `allOf`/`oneOf`/`anyOf` composition, and discriminator mappings.

**Estimated Effort:** 5 days

**Dependencies:** SCHEMA-001

**Acceptance Criteria:**
- Parses OpenAPI 3.0.x and 3.1.0 specs (both YAML and JSON input).
- Resolves all `$ref` references including circular references (with cycle detection).
- Handles `allOf`, `oneOf`, `anyOf` composition correctly.
- Extracts all endpoints with methods, parameters, request bodies, and response schemas.
- Validates the spec itself against the OpenAPI meta-schema.
- Passes against the OpenAPI Initiative's official test suite fixtures (50+ specs).
- Error messages include the JSON Pointer path to the problematic element.

**Best Practices:**
- Use `openapiv3` crate for initial parsing, then convert to internal IR.
- Resolve `$ref` lazily — build a reference map first, resolve on access.
- Handle the OpenAPI 3.0 vs 3.1 JSON Schema dialect difference explicitly (3.1 uses standard JSON Schema 2020-12).
- Cache resolved references to avoid re-resolution in large specs.

**Pitfalls:**
- Circular `$ref` chains will cause infinite loops if you recurse naively. Use a visited-set.
- `openapiv3` crate may lag behind spec changes — have an escape hatch for raw `serde_json::Value` access.
- Remote `$ref` resolution requires HTTP calls — make it async and optional (default to local-only).
- YAML parsing with `serde_yaml` can silently coerce types (the Norway problem: `NO` → `false`).

**Recommended Crates:**
- `openapiv3` 2.x for spec deserialization
- `serde_yaml` for YAML input
- `serde_json` for JSON input and `$ref` path resolution
- `jsonschema` for meta-schema validation
- `url` for remote reference resolution

---

### SCHEMA-003: Protobuf/gRPC Schema Parser

**Description:** Parse `.proto` files and compiled `FileDescriptorSet` binaries to extract service definitions, message types, field rules, and streaming configurations. Convert to the unified schema IR. Behind the `grpc` feature flag.

**Estimated Effort:** 4 days

**Dependencies:** SCHEMA-001, SETUP-003

**Acceptance Criteria:**
- Parses `.proto` files (proto3 syntax) including imports.
- Parses compiled `FileDescriptorSet` (protobuf binary format).
- Extracts services, methods (unary, server-streaming, client-streaming, bidi-streaming).
- Extracts message definitions with field types, numbers, and rules.
- Handles `oneof`, `map`, `repeated`, nested messages, enums.
- Gated behind `#[cfg(feature = "grpc")]` — workspace compiles without it.
- Test fixtures include at least 10 real-world proto files.

**Best Practices:**
- Use `prost-types` `FileDescriptorSet` for pre-compiled descriptors (most reliable).
- For raw `.proto` parsing, use `protox` or `protobuf-parse` — don't write a parser from scratch.
- Track field numbers explicitly — they're critical for breaking change detection.
- Store the wire type alongside the Rust type for accurate compatibility checks.

**Pitfalls:**
- Proto import resolution requires knowing the include paths — accept them as configuration.
- `prost` generates Rust code; you need `prost-types` for reflection/descriptor access at runtime.
- Proto2 vs Proto3 have different default value and required-field semantics — handle both.
- Proto file parsing is surprisingly complex (options, extensions, custom options) — start with a subset.

**Recommended Crates:**
- `prost` + `prost-types` for descriptor handling
- `protox` for `.proto` file parsing without `protoc`
- `bytes` for binary descriptor set reading

---

### SCHEMA-004: GraphQL SDL Parser

**Description:** Parse GraphQL Schema Definition Language (SDL) files to extract types, queries, mutations, subscriptions, directives, and input types. Convert to the unified schema IR. Behind the `graphql` feature flag.

**Estimated Effort:** 3 days

**Dependencies:** SCHEMA-001, SETUP-003

**Acceptance Criteria:**
- Parses GraphQL SDL (schema-first) files.
- Extracts: Object types, Input types, Enums, Unions, Interfaces, Scalars.
- Extracts: Query/Mutation/Subscription root types with field arguments.
- Handles directives (`@deprecated`, custom directives).
- Handles schema extensions (`extend type`).
- Gated behind `#[cfg(feature = "graphql")]`.
- Test fixtures include GitHub's public GraphQL schema and at least 5 others.

**Best Practices:**
- Use `apollo-rs` (`apollo-compiler`) — it provides full SDL parsing, validation, and a typed AST.
- `apollo-compiler` also validates the schema against the GraphQL spec, which saves work.
- Map GraphQL nullability (`String!` vs `String`) to the IR's constraint system explicitly.
- Preserve directive metadata — custom directives often carry validation rules.

**Pitfalls:**
- GraphQL introspection JSON and SDL are different formats — support both.
- `apollo-rs` is evolving rapidly; pin a specific version and test on update.
- GraphQL schema extensions can appear across multiple files — support multi-file input.
- Subscription types have different runtime semantics — flag them in the IR.

**Recommended Crates:**
- `apollo-compiler` 1.x for parsing and validation
- `graphql-parser` as a lighter-weight fallback if `apollo-rs` is too heavy

---

### SCHEMA-005: JSON Schema Validation Engine

**Description:** Integrate a JSON Schema validation engine that validates JSON instances against JSON Schema drafts 4, 6, 7, 2019-09, and 2020-12. This is used for validating API responses against their declared schemas and for validating configuration files.

**Estimated Effort:** 2 days

**Dependencies:** SCHEMA-001

**Acceptance Criteria:**
- Validates JSON instances against JSON Schema (drafts 4, 7, 2020-12 at minimum).
- Returns structured validation errors with JSON Pointer paths to failing elements.
- Supports custom format validators (e.g., `date-time`, `email`, `uri`).
- Handles `$ref` in JSON Schemas.
- Benchmark: validates a 1MB JSON document against a 500-node schema in under 100ms.

**Best Practices:**
- Use the `jsonschema` crate — it supports all major drafts and is well-maintained.
- Pre-compile schemas (call `jsonschema::validator_for()` once, reuse) for hot-path validation.
- Map `jsonschema` errors to ValiForge's own error types — don't leak the dependency.
- Support schema caching for repeated validation against the same schema.

**Pitfalls:**
- JSON Schema 2020-12 (used by OpenAPI 3.1) has different semantics than Draft 7 (used by OpenAPI 3.0). Be explicit about which draft you're validating against.
- `jsonschema` crate's error types are not `Send + Sync` in all cases — check before using in async contexts.
- Custom format validators must not panic — wrap in catch_unwind if accepting user-provided ones.

**Recommended Crates:**
- `jsonschema` 0.22+ for validation
- `serde_json` for JSON manipulation
- `referencing` (if needed for advanced `$ref` resolution)

---

### SCHEMA-006: Schema Parser Test Infrastructure

**Description:** Build the test fixture system: curated real-world schemas, property-based schema generators, and a test harness that validates parser correctness against known-good outputs. This infrastructure is shared across all parser tests.

**Estimated Effort:** 2 days

**Dependencies:** SCHEMA-002, SCHEMA-003, SCHEMA-004

**Acceptance Criteria:**
- `tests/fixtures/` directory with at least 20 OpenAPI specs, 10 proto files, 10 GraphQL schemas.
- Fixtures sourced from public APIs (Stripe, GitHub, Twilio, Kubernetes, etc.).
- Property-based tests using `proptest` generate random valid schemas and verify round-trip.
- Snapshot tests using `insta` for parser output stability.
- CI job runs the full fixture suite.

**Best Practices:**
- Use `insta` for snapshot testing — it makes reviewing parser output changes trivial.
- Store fixtures as checked-in files, not downloaded at test time (reproducible offline builds).
- Organize fixtures by protocol and complexity: `fixtures/openapi/simple/`, `fixtures/openapi/complex/`.
- Include pathological cases: huge schemas, deeply nested `$ref` chains, empty schemas.

**Pitfalls:**
- Don't use copyrighted schemas without checking licenses.
- Snapshot files can bloat the repository — use `insta`'s `--delete-unreferenced-snapshots` in CI.
- Property-based tests can be slow — set `PROPTEST_CASES=10` in CI, `256` locally.

**Recommended Crates:**
- `insta` 1.x for snapshot testing
- `proptest` for property-based testing
- `test-case` for parameterized tests

---

## 3. Validation Engine (valiforge-core)

### CORE-001: Core Trait Definitions

**Description:** Define the foundational traits that all protocol validators implement: `ProtocolValidator`, `ValidationContext`, `ValidationResult`. These traits are the public API surface of the validation engine and must be designed for extensibility (WASM plugins will implement them too).

**Estimated Effort:** 2 days

**Dependencies:** SCHEMA-001, SETUP-005

**Acceptance Criteria:**
- `ProtocolValidator` trait with async methods: `validate_endpoint`, `validate_schema`, `validate_response`.
- `ValidationContext` struct carrying: config, schema, auth credentials, timeout, retry policy.
- `ValidationResult` type with: pass/fail/skip status, error details, timing, metadata.
- All traits are `Send + Sync + 'static` (required for tokio tasks).
- Traits are object-safe (can be used as `dyn ProtocolValidator`).
- Documentation on each trait method with examples.

**Best Practices:**
- Use `async_trait` crate or native async trait (Rust 1.75+) — prefer native if MSRV allows.
- Make `ValidationContext` cheaply cloneable (`Arc` internal fields) for concurrent use.
- Use builder pattern for `ValidationContext` — many optional fields.
- Return `ValidationResult` (not `Result<(), Error>`) — validation failures are data, not errors.

**Pitfalls:**
- Object safety: no generic methods, no `Self` in return types, no associated types with complex bounds.
- Don't make the traits too fine-grained — one trait with 3-5 methods beats five traits with 1 method each.
- Async traits in Rust are still evolving — if using `async_trait`, be aware it boxes futures (minor perf cost).

**Recommended Patterns:**
```rust
#[async_trait::async_trait]
pub trait ProtocolValidator: Send + Sync {
    async fn validate(
        &self,
        ctx: &ValidationContext,
        endpoint: &Endpoint,
    ) -> ValidationResult;

    fn protocol(&self) -> Protocol;
    fn name(&self) -> &str;
}
```

---

### CORE-002: HTTP Request Builder & Executor

**Description:** Build the HTTP client layer that constructs requests from endpoint definitions (method, URL, headers, query params, body) and executes them against live endpoints. Supports authentication (API key, Bearer token, Basic auth, OAuth2), TLS configuration, and proxy support.

**Estimated Effort:** 3 days

**Dependencies:** CORE-001

**Acceptance Criteria:**
- Builds HTTP requests from `Endpoint` definitions (all HTTP methods).
- Supports authentication: API key (header/query), Bearer token, Basic auth.
- Supports custom headers and query parameters from config.
- Configurable timeouts (connect, read, total).
- Configurable TLS (custom CA certs, client certs, insecure skip-verify).
- Proxy support (HTTP, SOCKS5, environment variable auto-detection).
- Request/response logging via `tracing` at debug level.
- Response captured as `CapturedResponse` with status, headers, body bytes, timing.

**Best Practices:**
- Create a single `reqwest::Client` and reuse it (connection pooling).
- Use `reqwest::ClientBuilder` — never construct raw hyper clients.
- Make the client configurable via the `ValidationContext`, not hardcoded.
- Record request/response timing with `std::time::Instant`, not wall-clock time.
- Implement `Display` on `CapturedResponse` for debugging.

**Pitfalls:**
- `reqwest` default timeout is none — always set explicit timeouts.
- Large response bodies can OOM — set a max body size (default 10MB, configurable).
- Don't log sensitive headers (Authorization, Cookie) — redact them.
- Connection pool exhaustion under high concurrency — set max idle connections per host.

**Recommended Crates:**
- `reqwest` 0.12+ with `rustls-tls` feature (no OpenSSL dependency)
- `http` for header/method types
- `url` for URL construction
- `secrecy` for wrapping auth tokens (prevents accidental logging)

---

### CORE-003: Response Validation Pipeline

**Description:** Build the response validation pipeline that checks HTTP responses against their declared schemas: status code validation, content-type negotiation, header validation, body schema validation (JSON, XML), and response time assertions.

**Estimated Effort:** 4 days

**Dependencies:** CORE-002, SCHEMA-005

**Acceptance Criteria:**
- Validates response status code against declared responses (200, 201, 4xx, 5xx, default).
- Validates `Content-Type` header against declared media types.
- Validates response headers against declared header schemas.
- Validates JSON response body against JSON Schema.
- Reports all validation failures (not just the first one).
- Supports response time thresholds (configurable per-endpoint).
- Handles edge cases: empty body, binary body, chunked encoding, compressed responses.

**Best Practices:**
- Collect ALL validation errors per response, not short-circuit on the first.
- Use a `ValidationIssue` struct: `{ path: JsonPointer, message: String, severity: Severity }`.
- Validate against the specific response schema (by status code), falling back to `default`.
- Content-type matching should handle parameters: `application/json; charset=utf-8` matches `application/json`.

**Pitfalls:**
- OpenAPI allows multiple response schemas per status code (via content negotiation). Match on `Accept` header.
- Empty response bodies (`204 No Content`) should not trigger "body does not match schema" errors.
- XML validation is hard — defer to a later sprint if not critical.
- Compressed responses (`Content-Encoding: gzip`) must be decompressed before validation. `reqwest` handles this by default, but verify.

---

### CORE-004: Concurrent Endpoint Validation

**Description:** Implement parallel validation of multiple endpoints with configurable concurrency limits. Use `tokio::sync::Semaphore` to control the number of simultaneous HTTP requests. Support per-host rate limiting to avoid overwhelming target servers.

**Estimated Effort:** 3 days

**Dependencies:** CORE-002, CORE-003

**Acceptance Criteria:**
- Validates all endpoints in a schema concurrently (default: 10 parallel).
- Concurrency limit configurable via CLI flag `--concurrency` and config file.
- Per-host rate limiting: configurable requests per second per host.
- Individual endpoint timeout does not block other validations.
- Failed validations are collected, not panicked.
- Progress reporting: emits events for "started / completed / failed" per endpoint.
- Graceful shutdown on Ctrl+C (tokio signal handling).

**Best Practices:**
- Use `tokio::sync::Semaphore` for concurrency limiting — it's the idiomatic Rust approach.
- Use `futures::stream::FuturesUnordered` or `tokio::JoinSet` for managing concurrent tasks.
- Wrap each validation in `tokio::time::timeout` to enforce per-endpoint timeouts.
- Use channels (`tokio::sync::mpsc`) to stream results to the reporter as they complete.

**Pitfalls:**
- `JoinSet` panics if a task panics — wrap task bodies in `std::panic::catch_unwind` (via `tokio::spawn`).
- Semaphore permits must be released even on error — use RAII guard (`SemaphorePermit`).
- Rate limiting with a simple semaphore is not rate limiting — use `governor` crate for token bucket.
- DNS resolution can be a bottleneck under high concurrency. `reqwest` uses `trust-dns` if enabled.

**Recommended Crates:**
- `tokio` with `rt-multi-thread`, `sync`, `signal`, `time` features
- `governor` for token-bucket rate limiting
- `tokio::sync::Semaphore` for concurrency limiting
- `tokio::signal` for Ctrl+C handling

---

### CORE-005: Retry Logic & Error Classification

**Description:** Implement configurable retry logic for transient failures (network errors, 429 Too Many Requests, 503 Service Unavailable). Classify errors into categories: transient (retryable), permanent (not retryable), validation failure (expected behavior, not an error). Support exponential backoff with jitter.

**Estimated Effort:** 2 days

**Dependencies:** CORE-002

**Acceptance Criteria:**
- Retries on: connection refused, timeout, DNS failure, HTTP 429, HTTP 502/503/504.
- Does NOT retry on: HTTP 4xx (except 429), HTTP 5xx (except 502/503/504), validation failures.
- Configurable: max retries (default 3), initial backoff (default 1s), max backoff (default 30s), backoff multiplier (default 2).
- Exponential backoff with full jitter.
- Respects `Retry-After` header on 429 and 503.
- Each retry is logged at warn level with attempt number and reason.
- Total request time (including retries) is tracked.

**Best Practices:**
- Implement as middleware/wrapper around the HTTP executor, not inline.
- Use full jitter: `sleep(random(0, min(cap, base * 2^attempt)))` — reduces thundering herd.
- Make the retry policy a trait so users can provide custom logic.
- Record retry count in the `ValidationResult` for debugging.

**Pitfalls:**
- Retrying POST/PUT/PATCH requests can cause duplicate side effects — only retry idempotent methods by default.
- `Retry-After` can be a date string or seconds integer — parse both.
- Don't retry forever — enforce a total timeout across all attempts.
- Jitter must use a proper RNG, not `rand::random()` in a tight loop (can be slow).

**Recommended Crates:**
- `backoff` crate for exponential backoff implementation
- `rand` for jitter
- `tokio::time::sleep` for async delay

---

### CORE-006: State Mutation Detection

**Description:** Implement detection of API endpoints that mutate state (POST, PUT, PATCH, DELETE) and provide safeguards: dry-run mode (validate schema without calling), confirmation prompts, idempotency detection, and rollback suggestions.

**Estimated Effort:** 2 days

**Dependencies:** CORE-001, CORE-002

**Acceptance Criteria:**
- Detects mutating endpoints by HTTP method (POST, PUT, PATCH, DELETE).
- Default mode: skip mutating endpoints with a warning.
- `--include-mutations` flag enables validation of mutating endpoints.
- `--dry-run` flag validates the schema only (no HTTP calls at all).
- Mutating endpoints are clearly marked in the validation report.
- Configuration supports per-endpoint override (allow/deny list).

**Best Practices:**
- Default-safe: never call mutating endpoints unless explicitly opted in.
- Log mutating endpoint skips at INFO level so users know what was skipped.
- Support `x-valiforge-safe: true` vendor extension in OpenAPI to mark endpoints as safe despite being POST.
- Track which endpoints were skipped and why in the report.

**Pitfalls:**
- Some GET endpoints have side effects (e.g., tracking pixels) — allow user overrides.
- GraphQL mutations are always POST — need to detect mutation operations in the query, not just HTTP method.
- Some APIs use POST for everything (gRPC-web, JSON-RPC) — method alone is insufficient.

---

### CORE-007: Performance Baseline Tracking

**Description:** Implement response time tracking and performance baseline comparison. Record P50, P95, P99 latencies per endpoint. Compare against stored baselines and flag regressions.

**Estimated Effort:** 2 days

**Dependencies:** CORE-003, CORE-004

**Acceptance Criteria:**
- Records response time for every endpoint call.
- Computes P50, P95, P99 percentiles per endpoint (requires multiple runs with `--iterations N`).
- Stores baselines to disk (JSON file in `.valiforge/baselines.json`).
- `--update-baseline` flag saves current run as the new baseline.
- Flags regressions: > 20% slower than baseline (threshold configurable).
- Performance summary included in validation report.

**Best Practices:**
- Use `hdrhistogram` crate for accurate percentile computation with low memory overhead.
- Baseline file should be check-in-able to version control.
- Warm-up requests (configurable count) before measuring.
- Report both absolute times and deltas from baseline.

**Pitfalls:**
- Network variability makes single-run baselines unreliable. Require `--iterations >= 5` for baseline creation.
- Clock resolution on some OS/container environments can be poor — use `Instant::now()`, not system time.
- Don't include retry time in the latency metric — track separately.

**Recommended Crates:**
- `hdrhistogram` for percentile computation
- `serde_json` for baseline file storage

---

## 4. Breaking Change Detection (valiforge-diff)

### DIFF-001: Schema Diff Core Algorithm

**Description:** Implement the core diffing algorithm that compares two versions of a schema (old vs. new) and produces a structured list of changes. Each change has a type, path, description, and severity. This is the foundation for all protocol-specific diff implementations.

**Estimated Effort:** 3 days

**Dependencies:** SCHEMA-001

**Acceptance Criteria:**
- `SchemaDiff` struct with ordered list of `Change` entries.
- Each `Change`: `{ path: SchemaPath, change_type: ChangeType, severity: Severity, description: String, old_value: Option<Value>, new_value: Option<Value> }`.
- `ChangeType` enum: Added, Removed, Modified, Deprecated.
- `Severity` enum: Breaking, Warning, Info.
- Diff is deterministic (same inputs always produce same output).
- Diff output is serializable to JSON.

**Best Practices:**
- Use a path-based approach (like JSON Patch RFC 6902) for identifying change locations.
- Make severity classification configurable — what's "breaking" depends on context.
- Diff should be computed without network calls — it's a pure data comparison.
- Include `migration_hint: Option<String>` for actionable guidance on each breaking change.

**Pitfalls:**
- Schema refactoring (moving a type to a shared component) looks like remove+add unless you detect renames.
- Don't try to diff at the raw text level — diff the parsed IR.
- Ordering of changes matters for human readability — sort by severity (breaking first), then path.

---

### DIFF-002: OpenAPI Breaking Change Rules

**Description:** Implement the OpenAPI-specific breaking change detection rules. This is the most complex diff component because OpenAPI has many subtle compatibility rules around parameter changes, response changes, and schema evolution.

**Estimated Effort:** 4 days

**Dependencies:** DIFF-001, SCHEMA-002

**Acceptance Criteria:**
- Detects endpoint removal (Breaking).
- Detects HTTP method removal on existing path (Breaking).
- Detects new required request parameter (Breaking).
- Detects request parameter type change (Breaking).
- Detects required field added to request body (Breaking).
- Detects response field removal (Breaking for clients).
- Detects response type change (Breaking).
- Detects enum value removal in request parameter (Breaking).
- Detects adding enum value to response (Warning — clients may not handle it).
- Detects new optional request parameter (Info).
- Detects new endpoint (Info).
- Detects new response field (Info).
- At least 30 test cases covering all rules.

**Best Practices:**
- Follow the Postel's Law principle: be liberal in what you accept (responses), conservative in what you send (requests).
- Request changes are evaluated from the **server's** perspective (new requirements break clients).
- Response changes are evaluated from the **client's** perspective (removed fields break clients).
- Create a rule registry so rules can be enabled/disabled individually.

**Pitfalls:**
- `allOf`/`oneOf` changes cascade through compositions — must resolve before diffing.
- A field changing from required to optional is NOT breaking (it's relaxing a constraint).
- A field changing from optional to required IS breaking (it's tightening a constraint).
- Numeric constraint changes (min/max) need careful direction analysis.

---

### DIFF-003: Protobuf Breaking Change Rules

**Description:** Implement Protobuf-specific breaking change detection following the official `buf` breaking change rules. Protobuf has well-defined wire compatibility rules based on field numbers, types, and wire types.

**Estimated Effort:** 3 days

**Dependencies:** DIFF-001, SCHEMA-003

**Acceptance Criteria:**
- Detects field number reuse (Breaking — wire incompatibility).
- Detects field type change (Breaking — wire type mismatch).
- Detects field removal without `reserved` (Breaking).
- Detects enum value number reuse (Breaking).
- Detects service method removal (Breaking).
- Detects service method type change (unary → streaming, etc.) (Breaking).
- Detects message removal (Breaking if referenced by services).
- Detects renaming without `json_name` preservation (Warning).
- Detects new required field in proto2 (Breaking).
- Gated behind `#[cfg(feature = "grpc")]`.
- At least 20 test cases.

**Best Practices:**
- Follow `buf breaking` rules as the industry standard reference.
- Track wire types (varint, fixed32, fixed64, length-delimited) — type changes within the same wire type are sometimes safe.
- `reserved` field numbers and names are important — check they're maintained.
- Support both FILE-level and PACKAGE-level breaking change detection.

**Pitfalls:**
- Field number 0 is invalid — validate as part of schema parsing, not diffing.
- `oneof` member additions/removals have non-obvious compatibility implications.
- Renaming a field is NOT a breaking change in protobuf (wire format uses numbers, not names), but it IS breaking for JSON encoding.
- Maps are syntactic sugar for repeated messages — diff the desugared form.

---

### DIFF-004: GraphQL Schema Diff

**Description:** Implement GraphQL-specific breaking change detection. GraphQL has its own compatibility model based on the type system: removing types, removing fields, changing nullability, and modifying arguments all have specific breakage implications.

**Estimated Effort:** 3 days

**Dependencies:** DIFF-001, SCHEMA-004

**Acceptance Criteria:**
- Detects type removal (Breaking).
- Detects field removal from object types (Breaking).
- Detects argument removal from fields (Breaking).
- Detects non-null → nullable return type change (Warning — widening).
- Detects nullable → non-null argument change (Breaking — tightening).
- Detects enum value removal (Breaking).
- Detects union member removal (Breaking).
- Detects interface implementation removal (Breaking).
- Detects input field addition (Breaking if required/non-null, Info if optional/nullable).
- Detects `@deprecated` additions (Info).
- Gated behind `#[cfg(feature = "graphql")]`.
- At least 20 test cases.

**Best Practices:**
- Follow the `graphql-inspector` breaking change categorization as a reference.
- Nullability is key in GraphQL — changing `String!` to `String` in a return type is safe (widening), but changing `String` to `String!` in an argument is breaking (tightening).
- Consider default values: adding a required argument is not breaking if it has a default value.
- Interface changes cascade to all implementing types — track transitively.

**Pitfalls:**
- Schema extensions can add fields to existing types — treat extend additions as non-breaking.
- Directive changes (`@deprecated`, `@auth`) may be semantic but not structural — classify as Warning.
- Custom scalars have no structural information — can't diff their semantics, only their existence.

---

### DIFF-005: Diff Output Formatting

**Description:** Implement multiple output formats for diff results: human-readable colored terminal output (with context), machine-readable JSON, and Markdown (for PR comments). Support filtering by severity and path.

**Estimated Effort:** 2 days

**Dependencies:** DIFF-001, DIFF-002

**Acceptance Criteria:**
- Terminal output: colored, grouped by severity, with before/after context.
- JSON output: complete structured diff, parseable by downstream tools.
- Markdown output: formatted table suitable for GitHub PR comments.
- `--severity-threshold` flag: only report changes at or above the given severity.
- `--path-filter` flag: only report changes matching a glob pattern (e.g., `/users/*`).
- Exit code: 0 if no breaking changes, 1 if breaking changes detected, 2 if error.

**Best Practices:**
- Use the same severity coloring consistently: red for breaking, yellow for warning, blue for info.
- Markdown output should include a summary line at the top: "3 breaking, 5 warnings, 12 info".
- JSON output should include schema metadata (versions, filenames) for traceability.
- Support `--format=json | markdown | terminal` flag.

---

## 5. CLI Module (valiforge-cli)

### CLI-001: Command Structure & Argument Parsing

**Description:** Implement the `clap` derive-based CLI structure with subcommands: `init`, `validate`, `diff`, `generate`, `report`, `plugin`. Each subcommand has its own argument set. Support global flags for output format, verbosity, and configuration file path.

**Estimated Effort:** 2 days

**Dependencies:** SETUP-001, SETUP-005

**Acceptance Criteria:**
- `valiforge init` — scaffolds a `.valiforge.toml` configuration file.
- `valiforge validate <schema-file>` — validates schema against live endpoints.
- `valiforge diff <old-schema> <new-schema>` — detects breaking changes.
- `valiforge generate <schema-file>` — generates test data.
- `valiforge report <results-file>` — converts results to different output formats.
- `valiforge plugin <subcommand>` — manages WASM plugins.
- Global flags: `--config <path>`, `--format <json|terminal|markdown>`, `-v/-vv/-vvv`, `--no-color`.
- `--help` output is clear and includes examples.
- Shell completions generated for bash, zsh, fish, PowerShell.

**Best Practices:**
- Use `clap` derive API (`#[derive(Parser)]`) — more maintainable than builder API.
- Group related arguments with `#[command(flatten)]` for reuse across subcommands.
- Use `clap`'s `value_parser` for custom types (URL, file path, duration).
- Set `#[command(version, about, long_about = None)]` for auto-generated help.
- Use `clap_complete` for shell completion generation.

**Pitfalls:**
- Don't put business logic in the CLI crate — it should only parse args and delegate to library crates.
- `clap` derive API generates a lot of code — enable `wrap_help` and `derive` features only.
- Argument names should use kebab-case (`--no-color`), not snake_case (`--no_color`).
- Test CLI parsing with unit tests (`Command::try_parse_from`) — don't rely only on integration tests.

**Recommended Crates:**
- `clap` 4.x with `derive` feature
- `clap_complete` for shell completions
- `clap-verbosity-flag` for `-v/-vv/-vvv` handling

---

### CLI-002: Configuration File Loading

**Description:** Implement hierarchical configuration loading: defaults → config file (`.valiforge.toml` or `.valiforge.yaml`) → environment variables → CLI arguments. Support TOML and YAML formats. Configuration includes: target URLs, auth, timeouts, retry policy, concurrency, format preferences, and feature toggles.

**Estimated Effort:** 2 days

**Dependencies:** CLI-001

**Acceptance Criteria:**
- Loads `.valiforge.toml` from current directory, walking up to repository root.
- Also supports `.valiforge.yaml` (auto-detected by extension).
- Environment variables override config file: `VALIFORGE_TIMEOUT=30s`.
- CLI arguments override everything.
- `valiforge init` generates a documented template config file.
- Invalid configuration produces a clear error message with the problematic field highlighted.
- Configuration is validated at load time (not at use time).

**Best Practices:**
- Use `config` crate (or `figment`) for layered configuration merging.
- Define a `Config` struct with `serde::Deserialize` — single source of truth.
- Use `#[serde(default)]` for optional fields with sensible defaults.
- Support profile-based configs: `[profile.ci]` section with overrides for CI environments.
- Document every config field with a comment in the generated template.

**Pitfalls:**
- TOML and YAML have different type coercion rules — test both formats.
- Environment variable names with dots are awkward. Use `VALIFORGE_` prefix with underscores for nesting.
- Don't deserialize config lazily — validate everything upfront for fail-fast behavior.
- Watch out for relative file paths in config — resolve them relative to the config file's directory, not CWD.

**Recommended Crates:**
- `figment` for layered configuration (toml + yaml + env + cli)
- `toml` for TOML parsing
- `serde_yaml` for YAML parsing
- `directories` for platform-specific config paths (if supporting global config)

---

### CLI-003: Terminal Output & UX

**Description:** Implement rich terminal output: colored status messages, progress bars for long-running validation, summary tables, and interactive prompts. Support `--no-color` and `NO_COLOR` environment variable. Detect TTY vs piped output.

**Estimated Effort:** 1.5 days

**Dependencies:** CLI-001

**Acceptance Criteria:**
- Colored output: green for pass, red for fail, yellow for warning, blue for info.
- Progress bar shows: current endpoint / total endpoints, elapsed time, ETA.
- Summary table at end: total endpoints, passed, failed, skipped, duration.
- `--no-color` flag and `NO_COLOR` env var disable colors.
- When output is piped (not a TTY), automatically disables colors and progress bars.
- Supports `--quiet` flag (only errors and final summary).

**Best Practices:**
- Use `owo-colors` (faster, fewer dependencies) or `colored` for terminal colors.
- Use `indicatif` for progress bars — it handles TTY detection automatically.
- Check `atty::is(Stream::Stdout)` (or `std::io::IsTerminal` on Rust 1.70+) for TTY detection.
- Follow the `NO_COLOR` standard (https://no-color.org/).

**Pitfalls:**
- Windows terminal color support varies — `colored` handles this, but test on Windows.
- Progress bars can interfere with structured output (JSON) — disable when `--format=json`.
- Don't use ANSI codes directly — always go through a color library.

**Recommended Crates:**
- `owo-colors` or `colored` for colors
- `indicatif` for progress bars
- `console` for terminal size detection and TTY checks
- `comfy-table` for summary tables

---

### CLI-004: Exit Code Strategy & Error Reporting

**Description:** Implement a consistent exit code strategy and user-facing error reporting. Errors should be actionable, with context about what went wrong and how to fix it.

**Estimated Effort:** 1 day

**Dependencies:** CLI-001, SETUP-005

**Acceptance Criteria:**
- Exit code 0: all validations passed, no breaking changes.
- Exit code 1: validation failures or breaking changes detected.
- Exit code 2: configuration error (bad config file, missing schema file).
- Exit code 3: network error (cannot reach target).
- Exit code 4: internal error (bug).
- User-facing errors include: what happened, why, and how to fix it.
- `--debug` flag enables full error chain and backtrace output.
- Panics are caught and converted to exit code 4 with a bug report suggestion.

**Best Practices:**
- Use `std::process::ExitCode` (Rust 1.61+) for type-safe exit codes.
- Implement a `main() -> ExitCode` pattern, not `main() -> Result<()>`.
- Use `miette` for rich error display in the terminal (source annotations, help text).
- Set `RUST_BACKTRACE=1` equivalent via the `--debug` flag, not by default.

**Pitfalls:**
- `std::process::exit()` skips destructors — use `ExitCode` return from main instead.
- Don't print stack traces by default — they scare users.
- Panic messages should suggest filing a bug report with the command that was run.

---

## 6. Report Generation (valiforge-report)

### REPORT-001: Report Core Data Model

**Description:** Define the report data model that all formatters consume. This is the canonical representation of validation results, independent of output format.

**Estimated Effort:** 1 day

**Dependencies:** CORE-001

**Acceptance Criteria:**
- `ValidationReport` struct: metadata, summary, endpoint results, timing, environment info.
- `EndpointResult`: endpoint info, request/response details, validation issues, timing.
- `ValidationIssue`: path, message, severity, expected vs actual values.
- `ReportSummary`: counts by severity, total duration, pass rate percentage.
- All types implement `Serialize` and `Deserialize`.
- Report can be merged (combine results from multiple runs).

---

### REPORT-002: JUnit XML Output

**Description:** Generate JUnit XML reports for CI/CD integration. JUnit is the universal test result format supported by Jenkins, GitLab CI, GitHub Actions, Azure DevOps, and CircleCI.

**Estimated Effort:** 1.5 days

**Dependencies:** REPORT-001

**Acceptance Criteria:**
- Generates valid JUnit XML according to the JUnit 5 schema.
- Each endpoint maps to a test case.
- Validation failures map to `<failure>` elements with message and details.
- Network errors map to `<error>` elements.
- Skipped endpoints map to `<skipped>` elements.
- Timing information included on test suites and test cases.
- Output validated against JUnit XSD.
- Tested with GitHub Actions test result rendering.

**Best Practices:**
- Use `quick-xml` for XML generation — it's fast and low-dependency.
- Escape XML special characters in messages (`&`, `<`, `>`, `"`, `'`).
- Use the `classname` attribute for grouping (e.g., by path prefix or tag).

**Recommended Crates:**
- `quick-xml` for XML writing

---

### REPORT-003: SARIF Output

**Description:** Generate SARIF (Static Analysis Results Interchange Format) output for GitHub Code Scanning integration. This allows validation results to appear as annotations on pull requests.

**Estimated Effort:** 2 days

**Dependencies:** REPORT-001

**Acceptance Criteria:**
- Generates valid SARIF v2.1.0 JSON.
- Each validation rule is a SARIF `reportingDescriptor`.
- Each validation failure is a SARIF `result` with location information.
- Severity maps to SARIF levels: error, warning, note.
- Schema file line numbers included when available (for diff results).
- Output validates against the SARIF schema.
- Tested with GitHub Code Scanning upload (`gh api` or `codeql`).

**Best Practices:**
- SARIF is verbose — use `serde_json` with `serde_json::to_writer` (streaming) for large reports.
- Include the tool version and rule descriptions for GitHub to display.
- Map file paths to relative paths (from repository root) for GitHub integration.

**Recommended Crates:**
- `serde_json` for JSON generation
- `serde` with `#[serde(rename_all = "camelCase")]` for SARIF field naming

---

### REPORT-004: JSON, Markdown, and TAP Output

**Description:** Implement the remaining output formatters: structured JSON (for machine consumption), Markdown (for PR comments), colored terminal output (for human consumption), and TAP (Test Anything Protocol) for compatibility with TAP consumers.

**Estimated Effort:** 2 days

**Dependencies:** REPORT-001

**Acceptance Criteria:**
- JSON: complete report as pretty-printed or compact JSON.
- Markdown: summary table + details, suitable for GitHub PR comment body.
- Terminal: colored, with summary table, grouped by severity.
- TAP: version 13, one line per endpoint, YAML diagnostics for failures.
- All formats include the same information (no format-specific data loss).
- `--format` flag selects the output format.

**Best Practices:**
- Markdown output should be under GitHub's 65,536 character comment limit — truncate with "... N more results" if needed.
- TAP output should work with `prove` and other TAP consumers.
- Terminal output should use the same color scheme as CLI-003.
- JSON output should have a stable schema — document it.

---

## 7. Test Data Generation (valiforge-datagen)

### DATAGEN-001: Schema-Driven Data Generation

**Description:** Generate valid and invalid test data from API schemas using property-based testing techniques. For each endpoint, generate: valid request bodies, boundary-value bodies, and intentionally invalid bodies for negative testing.

**Estimated Effort:** 4 days

**Dependencies:** SCHEMA-001, SCHEMA-002

**Acceptance Criteria:**
- Generates valid JSON instances from JSON Schema (string, number, boolean, array, object, enum).
- Respects constraints: `minLength`, `maxLength`, `minimum`, `maximum`, `pattern`, `format`, `required`.
- Generates boundary values: empty strings, zero, max int, min int, empty arrays, null.
- Generates invalid instances: wrong types, missing required fields, out-of-range values, extra fields.
- Generates request bodies for OpenAPI endpoints (method, URL, headers, body).
- Output as JSON, optionally as `curl` commands or HTTP files.
- `valiforge generate <schema> --count 10` generates 10 samples per endpoint.

**Best Practices:**
- Use `proptest` for the generation engine — it provides shrinking for free.
- Map JSON Schema types to `proptest` strategies compositionally.
- For `format: "email"`, generate realistic-looking emails, not random strings.
- Cache schemas to avoid re-parsing for each generated sample.

**Pitfalls:**
- Recursive schemas (`$ref` cycles) can cause infinite generation — set a depth limit (default 5).
- `oneOf`/`anyOf` require choosing a variant — cycle through variants evenly.
- Pattern (`regex`) constraints need a regex-to-string generator — use `proptest`'s `string_regex`.
- Very large schemas can generate huge outputs — limit by default, let users increase.

**Recommended Crates:**
- `proptest` for generation and shrinking
- `fake` for realistic data (names, emails, addresses)
- `rand` for RNG seeding (deterministic generation with `--seed`)

---

## 8. WASM Plugin System (valiforge-plugin)

### PLUGIN-001: WASM Runtime Integration

**Description:** Integrate `wasmtime` as the WASM runtime for loading and executing user-provided validation plugins. Support WASI Preview 2 for file system and network access (sandboxed). Plugins extend ValiForge with custom validation rules, data generators, or output formatters.

**Estimated Effort:** 4 days

**Dependencies:** CORE-001, SETUP-003

**Acceptance Criteria:**
- Loads `.wasm` files from a configured plugin directory.
- Executes plugins in a sandboxed `wasmtime` instance.
- Plugins can access: the schema IR (read-only), request/response data (read-only), a result collector (write).
- Resource limits: max memory (default 64MB), max execution time (default 5s), no network access by default.
- Plugin errors are caught and reported without crashing the host.
- Gated behind `#[cfg(feature = "wasm-plugins")]`.
- Example plugin in Rust that compiles to WASM.

**Best Practices:**
- Use `wit-bindgen` for the plugin interface definition — it generates bindings for Rust, Go, JS, Python guest languages.
- Define the WIT (WebAssembly Interface Type) file in `crates/valiforge-plugin/wit/`.
- Use `wasmtime`'s fuel mechanism for CPU limiting.
- Pre-compile WASM modules (AOT) and cache the compiled artifacts.

**Pitfalls:**
- WASI Preview 2 is still stabilizing — pin `wasmtime` version carefully.
- WASM module compilation is expensive (100ms+) — do it once at startup, not per invocation.
- Passing complex data (schemas, responses) across the WASM boundary requires serialization — use JSON or Protocol Buffers, not raw memory.
- Plugin panics should not crash the host — catch `wasmtime::Trap`.

**Recommended Crates:**
- `wasmtime` 20+ with `component-model` feature
- `wit-bindgen` for guest/host bindings
- `wasmtime-wasi` for WASI support

---

### PLUGIN-002: Plugin Discovery & Management

**Description:** Implement plugin discovery (local directory, registry URL), plugin manifest format, dependency resolution, and CLI commands for managing plugins (`valiforge plugin install/list/remove`).

**Estimated Effort:** 3 days

**Dependencies:** PLUGIN-001

**Acceptance Criteria:**
- Plugins discovered from: `~/.valiforge/plugins/`, `./.valiforge/plugins/`, config file entries.
- Plugin manifest (`plugin.toml`): name, version, author, description, capabilities, min ValiForge version.
- `valiforge plugin list` shows installed plugins with status.
- `valiforge plugin install <path-or-url>` installs a plugin.
- `valiforge plugin remove <name>` removes a plugin.
- Plugins loaded in deterministic order (alphabetical, then by priority in config).
- Invalid plugins produce clear error messages and don't prevent other plugins from loading.

---

## 9. SDK (valiforge-sdk)

### SDK-001: Public Rust SDK

**Description:** Create `valiforge-sdk` as the stable public API for embedding ValiForge in other Rust applications. This crate re-exports selected types from internal crates and provides a high-level `ValiForge` builder struct for common operations.

**Estimated Effort:** 3 days

**Dependencies:** CORE-001, CORE-003, DIFF-001, REPORT-001

**Acceptance Criteria:**
- `ValiForge::builder().schema("openapi.yaml").build()` — builder pattern for configuration.
- `valiforge.validate().await` — returns `ValidationReport`.
- `valiforge.diff(old, new)` — returns `SchemaDiff`.
- `valiforge.generate(schema, count)` — returns generated data.
- Public types are documented with doc-tests.
- Crate has `#![deny(missing_docs)]`.
- Semantic versioning: public API changes require a minor version bump.
- Integration tests demonstrate embedding in a hypothetical CI tool.

**Best Practices:**
- Re-export only what users need — don't expose internal types.
- Use the facade pattern: `valiforge-sdk` depends on internal crates but presents a curated API.
- Every public type must implement `Debug`, `Clone`, and `Send + Sync`.
- Provide both async and blocking APIs (via `tokio::runtime::Runtime::block_on` wrapper).

---

## 10. Cross-Platform Binary Distribution

### DIST-001: Cross-Compilation Matrix

**Description:** Set up GitHub Actions workflows for building release binaries across all target platforms. Use `cross` for cross-compilation and musl for static Linux binaries.

**Estimated Effort:** 2 days

**Dependencies:** SETUP-004

**Acceptance Criteria:**
- Builds for: `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`.
- Linux binaries are statically linked (verify with `ldd` / `file`).
- macOS universal binary created with `lipo`.
- All binaries stripped (debug info removed) for size.
- SHA256 checksums generated for all artifacts.
- Release triggered by git tag (`v*`).
- Binaries uploaded to GitHub Releases.

**Best Practices:**
- Use `cross` for Linux cross-compilation (handles musl toolchains).
- macOS builds must run on macOS runners (code signing).
- Use `cargo-dist` if it supports the matrix — otherwise manual workflow.
- Compress binaries with `tar.gz` (Linux/macOS) and `.zip` (Windows).
- Sign macOS binaries with `codesign` if distributing outside Homebrew.

**Recommended Crates/Tools:**
- `cross` for cross-compilation
- `cargo-dist` for release automation
- `cargo-binstall` support via `[package.metadata.binstall]`

---

### DIST-002: Package Manager Distribution

**Description:** Create distribution packages for Homebrew (macOS), cargo install, cargo-binstall, and Docker.

**Estimated Effort:** 2 days

**Dependencies:** DIST-001

**Acceptance Criteria:**
- `cargo install valiforge` works from crates.io.
- `cargo binstall valiforge` downloads pre-built binary.
- Homebrew formula in a tap repository: `brew install valiforge/tap/valiforge`.
- Docker image: `docker run ghcr.io/org/valiforge validate schema.yaml` (FROM scratch + static binary).
- Docker image size under 20MB.
- All distribution channels tested in CI.

**Best Practices:**
- `cargo install` should have minimal compile-time dependencies — feature-gate heavy deps.
- Docker `FROM scratch` requires a fully static binary — use musl.
- Homebrew formula should use the GitHub Release URL for the bottle.
- Include man pages in the distribution (generate from clap with `clap_mangen`).

---

## 11. Summary & Sprint Plan

### Total Effort Estimate

| Section | Tasks | Total Days |
|---|---|---|
| **Workspace Setup** | SETUP-001 through SETUP-006 | **5.5** |
| **Schema Parsers** | SCHEMA-001 through SCHEMA-006 | **19** |
| **Validation Engine** | CORE-001 through CORE-007 | **18** |
| **Breaking Change Detection** | DIFF-001 through DIFF-005 | **15** |
| **CLI** | CLI-001 through CLI-004 | **6.5** |
| **Report Generation** | REPORT-001 through REPORT-004 | **6.5** |
| **Data Generation** | DATAGEN-001 | **4** |
| **WASM Plugins** | PLUGIN-001 through PLUGIN-002 | **7** |
| **SDK** | SDK-001 | **3** |
| **Distribution** | DIST-001 through DIST-002 | **4** |
| **TOTAL** | **35 tasks** | **88.5 engineering days** |

With a single engineer: approximately 18 weeks (4.5 months).
With two engineers: approximately 10 weeks (2.5 months) accounting for coordination overhead.

---

### Critical Path

The critical path determines the minimum project duration assuming unlimited parallelism:

```
SETUP-001 (1d)
  → SETUP-005 (1d)
    → SCHEMA-001 (3d)
      → SCHEMA-002 (5d)
        → CORE-001 (2d)
          → CORE-002 (3d)
            → CORE-003 (4d)
              → CORE-004 (3d)
                → CORE-007 (2d)
                  → SDK-001 (3d)
                    → DIST-001 (2d)

Critical Path Length: 29 engineering days (~6 weeks)
```

The diff pipeline is a parallel track:
```
SCHEMA-001 → DIFF-001 (3d) → DIFF-002 (4d) → DIFF-005 (2d)
```

The plugin system is independent after CORE-001:
```
CORE-001 → PLUGIN-001 (4d) → PLUGIN-002 (3d)
```

---

### Recommended Sprint Plan (2-Week Sprints)

#### Sprint 1 (Weeks 1-2): Foundation

| Task | Engineer | Days |
|---|---|---|
| SETUP-001: Workspace init | E1 | 1 |
| SETUP-002: Toolchain & MSRV | E1 | 0.5 |
| SETUP-003: Feature flags | E1 | 1 |
| SETUP-004: CI pipeline | E1 | 1.5 |
| SETUP-005: Error & logging infra | E2 | 1 |
| SETUP-006: xtask automation | E2 | 0.5 |
| SCHEMA-001: Core schema IR | E2 | 3 |
| SCHEMA-002: OpenAPI parser (start) | E1 | 3 |

**Sprint Goal:** Workspace compiles, CI is green, schema IR defined, OpenAPI parsing in progress.

#### Sprint 2 (Weeks 3-4): Schema Parsers

| Task | Engineer | Days |
|---|---|---|
| SCHEMA-002: OpenAPI parser (finish) | E1 | 2 |
| SCHEMA-005: JSON Schema validation | E1 | 2 |
| SCHEMA-006: Test infrastructure | E1 | 2 |
| SCHEMA-003: Protobuf parser | E2 | 4 |
| SCHEMA-004: GraphQL parser | E2 | 3 |
| CLI-001: Command structure (start) | E1 | 1 |

**Sprint Goal:** All three protocol parsers functional with test fixtures.

#### Sprint 3 (Weeks 5-6): Validation Engine

| Task | Engineer | Days |
|---|---|---|
| CORE-001: Core traits | E1 | 2 |
| CORE-002: HTTP executor | E1 | 3 |
| CORE-003: Response validation | E1 | 4 |
| DIFF-001: Diff core algorithm | E2 | 3 |
| DIFF-002: OpenAPI breaking rules | E2 | 4 |
| CLI-001: Command structure (finish) | E2 | 1 |

**Sprint Goal:** Can validate OpenAPI endpoints against live servers. Diff pipeline started.

#### Sprint 4 (Weeks 7-8): Concurrency, Diff & CLI

| Task | Engineer | Days |
|---|---|---|
| CORE-004: Concurrent validation | E1 | 3 |
| CORE-005: Retry logic | E1 | 2 |
| CORE-006: State mutation detection | E1 | 2 |
| DIFF-003: Protobuf breaking rules | E2 | 3 |
| DIFF-004: GraphQL diff | E2 | 3 |
| DIFF-005: Diff output formatting | E2 | 2 |
| CLI-002: Config loading | E1 | 1 |

**Sprint Goal:** Full validation pipeline with concurrency. All protocol diffs working.

#### Sprint 5 (Weeks 9-10): Reports, CLI Polish & Data Gen

| Task | Engineer | Days |
|---|---|---|
| REPORT-001: Report data model | E1 | 1 |
| REPORT-002: JUnit XML | E1 | 1.5 |
| REPORT-003: SARIF output | E1 | 2 |
| REPORT-004: JSON/Markdown/TAP | E1 | 2 |
| CLI-002: Config loading (finish) | E2 | 1 |
| CLI-003: Terminal UX | E2 | 1.5 |
| CLI-004: Exit codes & errors | E2 | 1 |
| DATAGEN-001: Data generation | E2 | 4 |

**Sprint Goal:** All output formats working. CLI is polished and user-friendly.

#### Sprint 6 (Weeks 11-12): Plugins, SDK & Distribution

| Task | Engineer | Days |
|---|---|---|
| PLUGIN-001: WASM runtime | E1 | 4 |
| PLUGIN-002: Plugin management | E1 | 3 |
| CORE-007: Performance baselines | E2 | 2 |
| SDK-001: Public SDK | E2 | 3 |
| DIST-001: Cross-compilation | E2 | 2 |
| DIST-002: Package distribution | E2 | 2 |

**Sprint Goal:** WASM plugin system operational. SDK published. Binaries shipping.

#### Sprint 7 (Weeks 13-14): Hardening & Launch Prep

- End-to-end integration testing against real-world APIs
- Performance benchmarking and optimization
- Documentation review and completion
- Security audit (dependency audit, fuzzing with `cargo-fuzz`)
- Beta release and feedback collection

---

### Key Technical Risks & Mitigations

| # | Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| 1 | **OpenAPI 3.1 JSON Schema 2020-12 compatibility gaps** — `openapiv3` crate may not fully support 3.1 | Medium | High | Maintain a fallback path using raw `serde_json::Value` for 3.1-specific features. Contribute upstream patches. |
| 2 | **WASM plugin interface instability** — WASI Preview 2 and Component Model are evolving | High | Medium | Pin `wasmtime` version. Keep the plugin interface minimal (JSON in/out). Feature-gate the entire subsystem. |
| 3 | **Cross-platform build failures** — musl + static linking can break with certain dependencies | Medium | Medium | Test cross-compilation in CI from Sprint 1. Avoid C dependencies (use `rustls` not `openssl`, `ring` not system crypto). |
| 4 | **Performance under large schemas** — A 10,000+ endpoint OpenAPI spec could be slow to parse/validate | Low | High | Benchmark with Kubernetes' OpenAPI spec (3,000+ endpoints) from Sprint 3. Optimize hot paths with profiling (`cargo flamegraph`). |
| 5 | **Schema parser correctness** — Edge cases in OpenAPI/Proto/GraphQL specs can cause silent failures | High | High | Use the official test suites as fixtures. Fuzzing with `cargo-fuzz` and `arbitrary`. Property-based testing with `proptest`. |
| 6 | **Dependency supply chain** — A compromised or abandoned dependency | Low | Critical | `cargo-deny` for advisory checking. `cargo-vet` for audit trail. Minimize dependency count. Prefer well-maintained crates with multiple maintainers. |
| 7 | **Feature flag combinatorial explosion** — Untested feature combinations | Medium | Medium | CI matrix tests `no-default-features`, `default-features`, `all-features`, and each feature individually. |
| 8 | **Async ecosystem complexity** — Mixing sync and async code, executor mismatches | Medium | Medium | Standardize on `tokio` everywhere. No `block_on` calls inside async contexts. Use `tokio::test` for all async tests. |

---

### Dependency Summary (Key External Crates)

| Crate | Version | Purpose | Feature-Gated? |
|---|---|---|---|
| `tokio` | 1.x | Async runtime | No (core) |
| `reqwest` | 0.12+ | HTTP client | No (core) |
| `clap` | 4.x | CLI argument parsing | No (cli) |
| `serde` + `serde_json` | 1.x | Serialization | No (core) |
| `thiserror` | 2.x | Error derives | No (core) |
| `anyhow` | 1.x | Error handling (CLI only) | No (cli) |
| `tracing` | 0.1 | Structured logging | No (core) |
| `openapiv3` | 2.x | OpenAPI parsing | No (core) |
| `jsonschema` | 0.22+ | JSON Schema validation | No (core) |
| `prost` + `prost-types` | 0.13+ | Protobuf handling | Yes (`grpc`) |
| `tonic` | 0.12+ | gRPC client | Yes (`grpc`) |
| `apollo-compiler` | 1.x | GraphQL parsing | Yes (`graphql`) |
| `wasmtime` | 20+ | WASM runtime | Yes (`wasm-plugins`) |
| `wit-bindgen` | 0.25+ | WASM interface types | Yes (`wasm-plugins`) |
| `proptest` | 1.x | Property-based testing | Yes (`datagen`) |
| `indicatif` | 0.17+ | Progress bars | No (cli) |
| `owo-colors` | 4.x | Terminal colors | No (cli) |
| `quick-xml` | 0.36+ | XML generation (JUnit) | Yes (`junit`) |
| `governor` | 0.6+ | Rate limiting | No (core) |
| `hdrhistogram` | 7.x | Percentile computation | No (core) |
| `insta` | 1.x | Snapshot testing (dev) | Dev only |
| `cargo-deny` | 0.14+ | License/advisory audit | CI only |

---

### Task Cross-Reference Index

| Task ID | Title | Days | Depends On |
|---|---|---|---|
| SETUP-001 | Initialize Cargo Workspace | 1 | — |
| SETUP-002 | Rust Toolchain & MSRV Policy | 0.5 | SETUP-001 |
| SETUP-003 | Feature Flag Strategy | 1 | SETUP-001 |
| SETUP-004 | CI Pipeline | 1.5 | SETUP-001, SETUP-002, SETUP-003 |
| SETUP-005 | Error & Logging Infrastructure | 1 | SETUP-001 |
| SETUP-006 | xtask Build Automation | 0.5 | SETUP-001 |
| SCHEMA-001 | Core Schema Abstraction Layer | 3 | SETUP-001, SETUP-005 |
| SCHEMA-002 | OpenAPI 3.0/3.1 Parser | 5 | SCHEMA-001 |
| SCHEMA-003 | Protobuf/gRPC Schema Parser | 4 | SCHEMA-001, SETUP-003 |
| SCHEMA-004 | GraphQL SDL Parser | 3 | SCHEMA-001, SETUP-003 |
| SCHEMA-005 | JSON Schema Validation Engine | 2 | SCHEMA-001 |
| SCHEMA-006 | Schema Parser Test Infrastructure | 2 | SCHEMA-002, SCHEMA-003, SCHEMA-004 |
| CORE-001 | Core Trait Definitions | 2 | SCHEMA-001, SETUP-005 |
| CORE-002 | HTTP Request Builder & Executor | 3 | CORE-001 |
| CORE-003 | Response Validation Pipeline | 4 | CORE-002, SCHEMA-005 |
| CORE-004 | Concurrent Endpoint Validation | 3 | CORE-002, CORE-003 |
| CORE-005 | Retry Logic & Error Classification | 2 | CORE-002 |
| CORE-006 | State Mutation Detection | 2 | CORE-001, CORE-002 |
| CORE-007 | Performance Baseline Tracking | 2 | CORE-003, CORE-004 |
| DIFF-001 | Schema Diff Core Algorithm | 3 | SCHEMA-001 |
| DIFF-002 | OpenAPI Breaking Change Rules | 4 | DIFF-001, SCHEMA-002 |
| DIFF-003 | Protobuf Breaking Change Rules | 3 | DIFF-001, SCHEMA-003 |
| DIFF-004 | GraphQL Schema Diff | 3 | DIFF-001, SCHEMA-004 |
| DIFF-005 | Diff Output Formatting | 2 | DIFF-001, DIFF-002 |
| CLI-001 | Command Structure & Argument Parsing | 2 | SETUP-001, SETUP-005 |
| CLI-002 | Configuration File Loading | 2 | CLI-001 |
| CLI-003 | Terminal Output & UX | 1.5 | CLI-001 |
| CLI-004 | Exit Code Strategy & Error Reporting | 1 | CLI-001, SETUP-005 |
| REPORT-001 | Report Core Data Model | 1 | CORE-001 |
| REPORT-002 | JUnit XML Output | 1.5 | REPORT-001 |
| REPORT-003 | SARIF Output | 2 | REPORT-001 |
| REPORT-004 | JSON, Markdown, and TAP Output | 2 | REPORT-001 |
| DATAGEN-001 | Schema-Driven Data Generation | 4 | SCHEMA-001, SCHEMA-002 |
| PLUGIN-001 | WASM Runtime Integration | 4 | CORE-001, SETUP-003 |
| PLUGIN-002 | Plugin Discovery & Management | 3 | PLUGIN-001 |
| SDK-001 | Public Rust SDK | 3 | CORE-001, CORE-003, DIFF-001, REPORT-001 |
| DIST-001 | Cross-Compilation Matrix | 2 | SETUP-004 |
| DIST-002 | Package Manager Distribution | 2 | DIST-001 |

---

*Document version: 1.0.0*
*Generated: 2026-03-13*
*Author: ValiForge Lead Rust Core Engine Engineer*
