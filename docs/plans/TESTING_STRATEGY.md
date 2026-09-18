# ValiForge Testing Strategy & Quality Assurance Plan

### Comprehensive QA Playbook for a Rust-Based API Validation Engine

**Version:** 1.0
**Date:** March 13, 2026
**Author:** QA Lead / Testing Strategist
**Status:** Ready for Engineering Review

---

## Table of Contents

1. [Testing Pyramid](#1-testing-pyramid-for-valiforge)
2. [Test Infrastructure](#2-test-infrastructure)
3. [Quality Gates](#3-quality-gates-must-pass-before-merge)
4. [Performance Benchmarking Framework](#4-performance-benchmarking-framework)
5. [Compatibility Testing](#5-compatibility-testing)
6. [Error Handling & Resilience Testing](#6-error-handling--resilience-testing)
7. [Security Testing](#7-security-testing)
8. [Release Process & Quality Checklist](#8-release-process--quality-checklist)
9. [Dogfooding Strategy](#9-dogfooding-strategy)
10. [Task Registry](#10-task-registry)
11. [Sprint Plan & Critical Path](#11-sprint-plan--critical-path)
12. [Quality Metrics Dashboard](#12-quality-metrics-dashboard-design)
13. [Key Risks](#13-key-risks)

---

## 1. Testing Pyramid for ValiForge

ValiForge validates APIs -- so our own test suite must be the most rigorous in the ecosystem. We follow an inverted testing philosophy: because ValiForge is a testing tool, we invest disproportionately at every pyramid layer. A bug in a testing tool erodes trust non-linearly.

```
                    /\
                   /  \          E2E (CLI invocations)
                  /    \         ~50 tests, 5% of suite
                 /------\
                / Integr \       Integration (real specs, wiremock)
               /   ation  \     ~200 tests, 15% of suite
              /------------\
             / Property-Based\   Property + Snapshot + Fuzz
            /   & Snapshot    \  ~300 tests, 20% of suite
           /------------------\
          /     Unit Tests      \  Per-crate unit tests
         /    (Foundation)       \ ~1500+ tests, 60% of suite
        /________________________\
```

### 1.1 Unit Tests

**Philosophy:** Every public function has at least one positive and one negative test. Every match arm is exercised. Error paths are tested as thoroughly as happy paths.

**Coverage Targets Per Crate:**

| Crate | Line Coverage Target | Branch Coverage Target | Rationale |
|-------|---------------------|----------------------|-----------|
| `valiforge-core` | >= 90% | >= 85% | Central orchestration logic; bugs here cascade everywhere |
| `valiforge-schema` | >= 95% | >= 90% | Parser correctness is existential; a misparse is a silent failure |
| `valiforge-datagen` | >= 85% | >= 80% | Complex SLM/grammar paths; SLM tests use mocked inference |
| `valiforge-diff` | >= 90% | >= 85% | Breaking change detection must be exhaustive; false negatives are unacceptable |
| `valiforge-report` | >= 85% | >= 80% | Output formatting; snapshot tests cover more than line coverage |
| `valiforge-cli` | >= 80% | >= 75% | Thin layer over core; E2E tests cover most CLI logic |
| `valiforge-plugin` | >= 85% | >= 80% | WASM sandbox boundary is a security surface |
| `valiforge-sdk` | >= 85% | >= 80% | Public API; every exported function must be tested |
| **Workspace aggregate** | **>= 85%** | **>= 80%** | Hard gate in CI |

**Mocking Strategy:**

| Dependency | Mock Crate | Strategy |
|-----------|-----------|----------|
| HTTP client (reqwest) | `wiremock-rs` | Intercept all outbound HTTP in tests; never hit real APIs in unit tests |
| SLM inference (candle/llama-cpp-2/ort) | `mockall` | Trait-based abstraction `trait InferenceEngine`; mock returns predetermined JSON payloads |
| File system I/O | `tempfile` + `mockall` | Use `tempfile::TempDir` for real FS tests; mock for error-path testing (permission denied, disk full) |
| Database (state mutation) | `mockall` + `testcontainers` | Mock for unit tests; real PostgreSQL via testcontainers for integration |
| WASM runtime (wasmtime) | `mockall` | Mock the plugin host interface; real wasmtime only in integration |
| System clock | `mock_instant` or custom trait | Deterministic timestamps in reports and baselines |

**Key Conventions:**
- All test modules use `#[cfg(test)] mod tests { ... }` inside each source file.
- Shared test utilities live in a private `valiforge-test-utils` crate (not published).
- Test data builders use the builder pattern (e.g., `SchemaBuilder::new().with_endpoint("/users").with_method(GET).build()`).
- No `#[ignore]` without a tracking issue number in the ignore message.
- Tests must not depend on execution order. Use `cargo nextest` which runs tests in separate processes.

---

### 1.2 Integration Tests

**Philosophy:** Integration tests validate that crates compose correctly and that real-world OpenAPI/Protobuf/GraphQL specs are parsed and validated without error. They use wiremock-rs to simulate target APIs, never hitting the internet.

**Test Spec Corpus:**

| Spec Name | Complexity | Endpoints | Key Edge Cases |
|-----------|-----------|-----------|----------------|
| Petstore (OpenAPI 3.0.3) | Simple | 13 | Basic CRUD, path params, query params |
| Petstore Extended (OpenAPI 3.1.0) | Medium | 20 | Webhooks, JSON Schema 2020-12 |
| Stripe (OpenAPI 3.0.0) | Complex | 300+ | Deep nested objects, polymorphism, `anyOf` everywhere |
| GitHub API (OpenAPI 3.0.3) | Complex | 800+ | Pagination, conditional headers, nested `$ref` chains |
| Kubernetes (OpenAPI 3.0.0) | Extreme | 1000+ | Circular `$ref`, deeply nested objects, CRD patterns |
| Twilio (OpenAPI 3.0.1) | Medium | 150+ | Auth schemes, nested accounts, date formats |
| Slack (OpenAPI 3.0.0) | Medium | 100+ | Token types, event payloads, rate limiting headers |
| Custom: circular-refs.yaml | Edge case | 5 | Self-referencing schemas, mutual references |
| Custom: allof-oneof-anyof.yaml | Edge case | 10 | Every composition keyword, discriminators |
| Custom: callbacks-webhooks.yaml | Edge case | 3 | Callback expressions, webhook payloads |
| Custom: malformed-partial.yaml | Error case | 8 | Missing required fields, invalid types, YAML anchors |

**Integration Test Categories:**

```
tests/
  integration/
    schema_parsing/
      test_openapi_30x.rs       # All 3.0.x sub-versions
      test_openapi_31.rs        # 3.1.0 specifics (JSON Schema 2020-12)
      test_protobuf.rs          # proto2, proto3, well-known types
      test_graphql.rs           # SDL, introspection, federation
      test_circular_refs.rs     # Circular reference resolution
      test_composition.rs       # allOf, oneOf, anyOf, discriminators
    validation/
      test_contract_validation.rs    # Schema-vs-response matching
      test_breaking_changes.rs       # Diff detection accuracy
      test_state_mutation.rs         # Pre/post condition assertions
      test_performance_baseline.rs   # Latency regression detection
    datagen/
      test_slm_generation.rs         # Mocked SLM output processing
      test_grammar_generation.rs     # Grammar-based fallback
      test_negative_generation.rs    # Invalid/edge-case payloads
    reporting/
      test_json_output.rs
      test_junit_xml.rs
      test_markdown_output.rs
      test_terminal_output.rs
    plugin/
      test_wasm_lifecycle.rs         # Load, execute, unload
      test_wasm_sandbox.rs           # Resource limits, security
```

---

### 1.3 End-to-End Tests

**Philosophy:** E2E tests invoke the actual compiled binary through `assert_cmd` and verify stdout/stderr/exit codes using `predicates`. They are the final line of defense before release.

**Crate Dependencies:**
```toml
[dev-dependencies]
assert_cmd = "2"
predicates = "3"
assert_fs = "1"       # Temporary file fixtures
escargot = "0.5"      # Cargo binary compilation
```

**E2E Scenarios:**

| Test ID | Scenario | Assertions |
|---------|---------|------------|
| E2E-001 | `valiforge init` in empty directory | Creates `valiforge.toml`, exit code 0, helpful output |
| E2E-002 | `valiforge init` in project with `openapi.yaml` | Auto-detects schema, configures path in TOML |
| E2E-003 | `valiforge validate --schema petstore.yaml --target <wiremock>` | All endpoints pass, exit code 0, JSON report correct |
| E2E-004 | `valiforge validate` with schema violations | Exit code 1, violation details in stderr, JUnit XML accurate |
| E2E-005 | `valiforge diff old.yaml new.yaml` (no breaking changes) | Exit code 0, "no breaking changes" message |
| E2E-006 | `valiforge diff old.yaml new.yaml` (breaking changes) | Exit code 1, lists each breaking change with severity |
| E2E-007 | `valiforge generate --schema api.yaml --count 10` | Outputs 10 valid payloads, valid JSON |
| E2E-008 | `valiforge generate --include-negative` | Negative payloads are clearly labeled |
| E2E-009 | `valiforge report --format junit` | Valid JUnit XML output |
| E2E-010 | `valiforge validate` with invalid YAML file | Exit code 2, actionable error message |
| E2E-011 | `valiforge validate` with no arguments and no config | Exit code 2, suggests running `valiforge init` |
| E2E-012 | `valiforge --version` | Prints version, exit code 0 |
| E2E-013 | `valiforge validate` with `--parallel 4` flag | Completes faster than serial, same results |
| E2E-014 | `valiforge validate` with timeout (target unreachable) | Graceful error, no panic, exit code 1 |
| E2E-015 | `valiforge validate` piped to file | No ANSI escape codes in output |

---

### 1.4 Property-Based Tests

**Philosophy:** Property-based testing is ValiForge's secret weapon. Since ValiForge generates test data from schemas, the data generation logic itself must be fuzz-tested with `proptest`. We define invariants that must hold for ALL inputs, not just the ones we think of.

**Crate:** `proptest = "1"`

**Properties to Test:**

| Property ID | Component | Invariant |
|-------------|-----------|-----------|
| PROP-001 | Schema parser | Any valid OpenAPI 3.x YAML that passes the official validator also parses without error in ValiForge |
| PROP-002 | Schema parser | Parsing then serializing then re-parsing produces an equivalent AST (round-trip) |
| PROP-003 | Data generator | Generated "valid" payloads always pass ValiForge's own schema validation |
| PROP-004 | Data generator | Generated "negative" payloads always FAIL ValiForge's own schema validation |
| PROP-005 | Diff engine | `diff(A, A)` always returns zero breaking changes for any schema A |
| PROP-006 | Diff engine | `diff(A, B)` where B is A with a removed required field always detects a breaking change |
| PROP-007 | Report formatter | JSON report output is always valid JSON |
| PROP-008 | Report formatter | JUnit XML output is always valid XML |
| PROP-009 | Config parser | Any valid TOML config round-trips through serialize/deserialize without data loss |
| PROP-010 | Validation engine | Adding an optional field to a schema never causes previously-passing endpoints to fail |
| PROP-011 | Validation engine | Validation result count equals endpoint count (no silent skips) |
| PROP-012 | Data generator (seed) | Same schema + same seed = identical output (determinism) |

**Strategy for proptest:**
```rust
// Example: PROP-003
proptest! {
    #[test]
    fn generated_valid_payloads_pass_validation(
        schema in arb_openapi_schema(),  // Custom strategy
        seed in 0u64..10000,
    ) {
        let payloads = datagen::generate_valid(&schema, seed, 10);
        for payload in &payloads {
            let result = validator::validate_payload(payload, &schema);
            prop_assert!(result.is_ok(), "Valid payload failed validation: {:?}", result);
        }
    }
}
```

**Custom Strategies (in `valiforge-test-utils`):**
- `arb_openapi_schema()` -- generates random but structurally valid OpenAPI schemas
- `arb_endpoint()` -- generates random endpoint definitions
- `arb_json_schema_type()` -- generates random JSON Schema types with constraints
- `arb_protobuf_message()` -- generates random protobuf message definitions
- `arb_graphql_type()` -- generates random GraphQL type definitions

---

### 1.5 Snapshot Tests

**Philosophy:** CLI output, report formats, and error messages are user-facing contracts. Any unintentional change to these is a regression. We use `insta` to snapshot-test all output.

**Crate:** `insta = "1"` with `yaml` and `json` features

**Snapshot Categories:**

| Category | Files | Review Process |
|----------|-------|---------------|
| CLI help text | `valiforge --help`, `valiforge validate --help`, etc. | Review every change; help text is documentation |
| Terminal output (colored) | Validation pass, fail, partial | Strip ANSI codes before snapshot |
| JSON reports | Full validation report for Petstore spec | Exact match |
| JUnit XML reports | Full report for Petstore spec | Exact match |
| Markdown reports | Full report for Petstore spec | Exact match |
| Error messages | Every known error code (VF-E001 through VF-E099) | Every error must include a fix suggestion |
| Diff output | Breaking change report for schema pairs | Exact match |
| Config generation | `valiforge init` output for various project types | Exact match |

**Workflow:**
```bash
# Develop with snapshot review
cargo insta test               # Run all snapshot tests
cargo insta review             # Interactive review of changed snapshots
cargo insta accept             # Accept all pending changes

# CI rejects any pending snapshots
cargo insta test --check       # Fails if any snapshots are pending review
```

---

### 1.6 Fuzzing

**Philosophy:** ValiForge's attack surface is user-provided schemas. A malformed OpenAPI spec, protobuf file, or GraphQL SDL must never cause a panic, segfault, or infinite loop. Fuzzing is our first line of defense against untrusted input.

**Tool:** `cargo-fuzz` (libFuzzer via `libfuzzer-sys`)

**Fuzz Targets:**

| Target ID | Input | Function Under Test | Goal |
|-----------|-------|-------------------|------|
| FUZZ-001 | Random bytes | `schema::parse_openapi_yaml()` | No panic on any input |
| FUZZ-002 | Random bytes | `schema::parse_openapi_json()` | No panic on any input |
| FUZZ-003 | Random bytes | `schema::parse_protobuf()` | No panic on any input |
| FUZZ-004 | Random bytes | `schema::parse_graphql_sdl()` | No panic on any input |
| FUZZ-005 | Random bytes | `config::parse_toml()` | No panic on any input |
| FUZZ-006 | Structured (AST mutation) | `diff::compute_breaking_changes()` | No panic; result is always valid |
| FUZZ-007 | Structured (AST mutation) | `datagen::generate_from_schema()` | No panic; output is always valid JSON |
| FUZZ-008 | Random bytes | `report::parse_junit_xml()` | No panic on re-ingestion of JUnit output |
| FUZZ-009 | Large structured | `validator::validate_all()` with 10K endpoints | No OOM, completes within 60s |
| FUZZ-010 | Random bytes | `plugin::load_wasm_module()` | No panic; malformed WASM rejected gracefully |

**Fuzz Corpus:** Seed with the real-world spec collection (Section 1.2) plus hand-crafted edge cases. Store corpus in `fuzz/corpus/<target>/`.

**CI Integration:**
- Nightly fuzz runs: 10 minutes per target, 7 targets = 70 minutes total.
- On fuzz failure: auto-create GitHub issue with reproducer input.
- Regression corpus: every discovered crash input is added to the corpus permanently.

---

### 1.7 Benchmarks

**Philosophy:** Performance is a product feature. We benchmark on every PR and block merges that regress beyond 10%. Benchmarks are published with every release.

**Tool:** `criterion = "0.5"` with `html_reports` feature

**Benchmark Suite:**

| Benchmark ID | What | Input | Baseline Target |
|-------------|------|-------|----------------|
| BENCH-001 | Cold start time | Binary startup to first output | < 100ms |
| BENCH-002 | Schema parse (small) | Petstore (13 endpoints) | < 5ms |
| BENCH-003 | Schema parse (medium) | Twilio (~150 endpoints) | < 50ms |
| BENCH-004 | Schema parse (large) | GitHub API (~800 endpoints) | < 200ms |
| BENCH-005 | Schema parse (extreme) | Kubernetes (~1000 endpoints) | < 500ms |
| BENCH-006 | Validation per endpoint | Single endpoint against wiremock | < 50ms |
| BENCH-007 | Validation suite (100 ep) | 100-endpoint spec, full validation | < 5s |
| BENCH-008 | Diff (small) | Two versions of Petstore | < 10ms |
| BENCH-009 | Diff (large) | Two versions of Stripe API | < 500ms |
| BENCH-010 | Data gen (grammar, 50) | 50 payloads, grammar engine | < 200ms |
| BENCH-011 | Data gen (grammar, 500) | 500 payloads, grammar engine | < 2s |
| BENCH-012 | Report gen (JSON) | 100-endpoint result to JSON | < 50ms |
| BENCH-013 | Report gen (JUnit XML) | 100-endpoint result to XML | < 50ms |
| BENCH-014 | Report gen (Markdown) | 100-endpoint result to MD | < 50ms |
| BENCH-015 | Memory: peak RSS | 100-endpoint validation | < 50MB |
| BENCH-016 | WASM plugin load | Load + initialize a sample plugin | < 100ms |

**Regression Detection:**
- Criterion stores baselines in `target/criterion/`.
- CI compares PR benchmarks against the `main` branch baseline.
- If any benchmark regresses >10%, the PR is flagged (not blocked on first offense; blocked on second consecutive regression for the same benchmark).
- Benchmark results are published as PR comments via GitHub Actions.

---

## 2. Test Infrastructure

### Task QA-001: Test Fixture Collection

**Task ID:** QA-001
**Title:** Curate and Maintain Real-World OpenAPI Spec Corpus
**Description:** Assemble a curated collection of real-world API specs that represent the full spectrum of complexity ValiForge must handle. These fixtures are the foundation of integration tests, property-based tests, benchmarks, and compatibility testing.

**Estimated Effort:** 3 engineering days

**Dependencies:** None (foundational task)

**Acceptance Criteria:**
- Directory `schemas/fixtures/` contains at least 15 real-world specs.
- Each spec is annotated with metadata: version, endpoint count, known edge cases.
- Specs are validated against the official OpenAPI validator before inclusion.
- A `schemas/fixtures/README.md` catalogs every fixture with its purpose.
- Custom edge-case specs are hand-authored for: circular refs, deep nesting (20+ levels), allOf/oneOf/anyOf compositions, discriminators, callbacks, webhooks, and mixed YAML anchors.
- All fixture files are committed to the repo (not downloaded at test time).

**Best Practices:**
- Pin exact spec versions (e.g., Stripe 2026-02-15, not "latest").
- Include both YAML and JSON variants for OpenAPI specs.
- Do NOT include proprietary or copyrighted specs; use only publicly available APIs.

**Pitfalls to Avoid:**
- Do not rely on downloading specs from the internet during CI. Network failures will flake your tests.
- Do not use specs that require authentication to be structurally valid.

---

### Task QA-002: Mock API Server Infrastructure

**Task ID:** QA-002
**Title:** Build wiremock-rs Based Mock API Server for Integration Tests
**Description:** Create a reusable mock API server framework that dynamically configures itself from OpenAPI specs. Given a spec, the server registers stubs for every endpoint that return schema-valid responses.

**Estimated Effort:** 5 engineering days

**Dependencies:** QA-001 (needs fixture specs)

**Acceptance Criteria:**
- A function `MockApiServer::from_spec(spec: &OpenApiSpec) -> MockApiServer` that auto-generates stubs.
- All stubs return responses that match the schema (status code, content type, body structure).
- Stubs can be overridden per-test for negative testing (e.g., return 500, return malformed JSON).
- Server supports concurrent test execution (each test gets its own server instance on a random port).
- Latency injection: stubs can be configured to delay responses (for timeout testing).
- TLS support: optional HTTPS stubs for TLS error testing.
- Request recording: tests can assert on received requests (method, headers, body).

**Best Practices:**
- Use `wiremock::MockServer::start()` per test function; do not share servers between tests.
- Register stubs in setup, not inline in assertions, to separate concerns.

**Pitfalls to Avoid:**
- Do not use a global static mock server; it creates test coupling and race conditions.
- Do not hardcode ports; always use the port assigned by wiremock-rs.

---

### Task QA-003: Docker-Based PostgreSQL for State Mutation Testing

**Task ID:** QA-003
**Title:** Testcontainers Setup for State Mutation Verification
**Description:** ValiForge's state mutation testing feature validates database changes after API calls. Integration tests need a real PostgreSQL instance to verify this feature, provided by `testcontainers-rs`.

**Estimated Effort:** 3 engineering days

**Dependencies:** QA-002 (needs mock API server for driving API calls)

**Acceptance Criteria:**
- `testcontainers` spins up PostgreSQL 16 for each test suite (not each test).
- Schema migrations are applied automatically before tests run.
- Tests verify pre/post state: "Before POST /users, `users` table has N rows; after, N+1 rows with expected data."
- Teardown is automatic (container destroyed on drop).
- Tests are skipped gracefully on systems without Docker (with `#[cfg_attr(not(feature = "docker-tests"), ignore)]`).

**Best Practices:**
- Use transactions and rollback between tests to avoid cross-test contamination.
- Keep the migration SQL in `schemas/fixtures/migrations/` versioned alongside the fixtures.

**Pitfalls to Avoid:**
- Do not assume Docker is available in all CI environments. The `docker-tests` feature flag allows selective execution.
- Do not leave orphan containers. Always use RAII (Drop trait) for cleanup.

---

### Task QA-004: CI Test Matrix Configuration

**Task ID:** QA-004
**Title:** Cross-Platform CI Test Matrix (GitHub Actions)
**Description:** Configure GitHub Actions workflows to test ValiForge on all supported platforms with parallel execution.

**Estimated Effort:** 4 engineering days

**Dependencies:** QA-001, QA-002

**Acceptance Criteria:**
- Matrix covers:
  - `ubuntu-22.04` (x86_64) -- primary CI target
  - `ubuntu-24.04` (x86_64) -- forward compatibility
  - `macos-14` (ARM64, Apple Silicon) -- developer laptop target
  - `windows-2022` (x86_64) -- Windows support
- All matrix entries run: fmt check, clippy, unit tests, integration tests, snapshot tests.
- E2E tests run on Linux only (primary) with manual trigger for macOS/Windows.
- Fuzz tests run nightly on Linux only.
- Benchmarks run on a dedicated Linux runner (to minimize noise from shared CI runners).
- Total CI time target: < 15 minutes for the PR check suite.
- Caching: `~/.cargo/registry`, `target/` directory, and `sccache` for compilation.

**Best Practices:**
- Use `cargo nextest` instead of `cargo test` for parallel test execution and better output.
- Cache aggressively: Rust compilation is slow; cache the `target/` directory keyed on `Cargo.lock` hash.
- Use `sccache` for distributed compilation caching across CI runs.

**Pitfalls to Avoid:**
- Do not run benchmarks on shared CI runners (noisy neighbor problem). Use a dedicated self-hosted runner or a large GitHub-hosted runner.
- Do not use `ubuntu-latest` -- pin exact versions to avoid surprise OS upgrades breaking tests.

---

### Task QA-005: Test Parallelism with nextest

**Task ID:** QA-005
**Title:** Configure cargo-nextest for Parallel Test Execution
**Description:** Replace the default `cargo test` harness with `cargo-nextest` for 2-4x faster test execution through process-per-test isolation and parallel scheduling.

**Estimated Effort:** 2 engineering days

**Dependencies:** None

**Acceptance Criteria:**
- `.config/nextest.toml` configured with:
  - Default test threads = number of CPU cores
  - Slow-test threshold = 30s (warn), 120s (terminate)
  - Retry count = 2 for flaky test detection
  - JUnit XML output for CI integration
- All existing tests pass under nextest without modification.
- Tests that require serial execution are annotated with `#[serial_test::serial]`.
- CI workflow uses `cargo nextest run` instead of `cargo test`.

**Best Practices:**
- Use nextest's built-in flaky test detection: any test that fails then passes on retry is flagged.
- Configure nextest to output test timing data for identifying slow tests.

**Pitfalls to Avoid:**
- Some tests may fail under nextest due to implicit shared state (e.g., environment variables, temp directories). Fix the tests, don't disable nextest.

---

### Task QA-006: Coverage Reporting Infrastructure

**Task ID:** QA-006
**Title:** Configure cargo-llvm-cov for Coverage Reporting with Thresholds
**Description:** Set up code coverage reporting using `cargo-llvm-cov` (preferred over tarpaulin for accuracy on complex Rust code) with enforced thresholds in CI.

**Estimated Effort:** 2 engineering days

**Dependencies:** QA-004 (needs CI pipeline)

**Acceptance Criteria:**
- `cargo llvm-cov` runs in CI on every PR.
- Coverage report generated in: lcov (for codecov.io), HTML (for local review), JSON (for threshold enforcement).
- Per-crate coverage thresholds enforced (see Section 1.1 table).
- Workspace aggregate threshold: >= 85% line coverage.
- Coverage report uploaded to Codecov (or Coveralls) with PR comment showing delta.
- Coverage badge in README.md.

**Best Practices:**
- Exclude test code itself, build scripts, and generated code from coverage metrics.
- Use `#[cfg(not(tarpaulin_include))]` sparingly and only for truly untestable code (e.g., process exit).

**Pitfalls to Avoid:**
- Do not count integration test coverage toward unit test coverage targets. Track them separately.
- Coverage numbers are meaningless without branch coverage. Track both line and branch coverage.

---

## 3. Quality Gates (Must Pass Before Merge)

Every PR must pass the following gates before the merge button is enabled. No exceptions. No "I'll fix it in the next PR."

### Gate Definitions

| Gate | Command | Failure Behavior | Rationale |
|------|---------|-----------------|-----------|
| **G1: Format** | `cargo fmt --all --check` | Block merge | Consistent formatting eliminates trivial review comments |
| **G2: Lint** | `cargo clippy --workspace --all-targets -- -D warnings` | Block merge | Clippy catches real bugs (unused results, logic errors, perf issues) |
| **G3: Unit Tests** | `cargo nextest run --workspace` | Block merge | Correctness is non-negotiable |
| **G4: Integration Tests** | `cargo nextest run --workspace --features integration` | Block merge | Cross-crate correctness |
| **G5: Snapshot Tests** | `cargo insta test --check` | Block merge | Output regressions caught immediately |
| **G6: Security Audit** | `cargo audit` | Block merge | Known vulnerabilities must be addressed before merge |
| **G7: License Compliance** | `cargo deny check licenses` | Block merge | Apache-2.0 compatibility; no GPL contamination |
| **G8: Duplicate Deps** | `cargo deny check bans` | Warn (do not block) | Alert on duplicate transitive dependencies |
| **G9: Coverage** | `cargo llvm-cov --fail-under-lines 85` | Block merge | Coverage cannot decrease below threshold |
| **G10: Benchmarks** | `cargo bench` (compare to baseline) | Warn if >10% regression; block if >20% | Performance regressions are bugs |
| **G11: Binary Size** | Custom script comparing release binary size | Warn if >10% growth; block if >20% | Prevent binary bloat; <15MB target |
| **G12: Doc Tests** | `cargo test --doc` | Block merge | Documentation examples must compile and run |
| **G13: No Unsafe** | `cargo geiger` | Block merge if new unsafe added without audit comment | Unsafe code requires explicit justification in PR description |

### Task QA-007: Quality Gate CI Pipeline

**Task ID:** QA-007
**Title:** Implement All Quality Gates as GitHub Actions CI Pipeline
**Description:** Create a GitHub Actions workflow that runs all 13 quality gates, reports results as PR status checks, and blocks merge on any hard failure.

**Estimated Effort:** 5 engineering days

**Dependencies:** QA-004, QA-005, QA-006

**Acceptance Criteria:**
- All 13 gates run as separate jobs (parallel where possible).
- Each gate reports as a separate GitHub status check (not one monolithic check).
- Branch protection rules require all "Block merge" gates to pass.
- Gate results are summarized in a PR comment with pass/fail for each gate.
- Total gate execution time < 15 minutes.
- Gates are defined in a reusable workflow (`.github/workflows/quality-gates.yml`).
- `cargo deny` is configured via `deny.toml` with allowed/denied license lists.

**Best Practices:**
- Run format and lint checks first (they are fast and catch the most common issues).
- Run security audit in parallel with tests (they are independent).
- Cache cargo registry and target directory aggressively.

**Pitfalls to Avoid:**
- Do not use `continue-on-error: true` on blocking gates. The whole point is that they block.
- Do not run all gates sequentially. Parallel execution reduces CI time by 3-4x.

---

### Task QA-008: cargo-deny Configuration

**Task ID:** QA-008
**Title:** Configure cargo-deny for License and Dependency Policy
**Description:** Create `deny.toml` that enforces ValiForge's dependency policy: no GPL-family licenses (ValiForge is Apache-2.0), no known-vulnerable dependencies, controlled duplicate detection.

**Estimated Effort:** 1 engineering day

**Dependencies:** None

**Acceptance Criteria:**
- `deny.toml` specifies:
  - Allowed licenses: `Apache-2.0`, `MIT`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Zlib`, `Unicode-DFS-2016`
  - Denied licenses: `GPL-2.0`, `GPL-3.0`, `AGPL-3.0`, `LGPL-2.1`, `LGPL-3.0` (unless dynamically linked)
  - Advisory database: deny on unmaintained, unsound, or vulnerable crates
  - Bans: deny specific problematic crates (e.g., `openssl` in favor of `rustls`)
- `cargo deny check` passes on current dependency tree.
- Policy documented in `DEPENDENCY_POLICY.md`.

**Best Practices:**
- Prefer `rustls` over `openssl` to avoid system dependency and simplify cross-compilation.
- Audit each `MPL-2.0` dependency individually (MPL-2.0 is file-level copyleft, usually fine).

**Pitfalls to Avoid:**
- Do not blanket-allow `MPL-2.0` without reviewing which crates use it.

---

## 4. Performance Benchmarking Framework

### Task QA-009: Criterion Benchmark Suite

**Task ID:** QA-009
**Title:** Implement Comprehensive Criterion Benchmark Suite
**Description:** Create a full benchmark suite using `criterion` that covers every performance-sensitive code path. Benchmarks are the source of truth for ValiForge's performance claims.

**Estimated Effort:** 8 engineering days

**Dependencies:** QA-001 (needs fixture specs), QA-002 (needs mock API server)

**Acceptance Criteria:**
- All 16 benchmarks from Section 1.7 are implemented.
- Each benchmark has a descriptive name and group (e.g., `schema_parse/petstore`, `schema_parse/github`).
- Benchmarks use `criterion::BenchmarkGroup` with parameterized inputs (e.g., endpoint counts).
- Baseline files checked into the repo under `benches/baselines/`.
- `cargo bench` produces HTML reports in `target/criterion/report/`.
- A `benches/compare.sh` script compares current results against baseline and outputs a summary.

**Best Practices:**
- Use `criterion::black_box()` to prevent compiler optimization of benchmark inputs.
- Warm up the benchmark (criterion does this automatically, but verify for IO-bound benchmarks).
- Run benchmarks on a dedicated machine (or at minimum, close all other applications).

**Pitfalls to Avoid:**
- Do not benchmark with debug builds. Always use `cargo bench` (which compiles with optimizations).
- Do not benchmark code that hits the network. Use wiremock-rs.
- Beware of thermal throttling on CI runners during long benchmark suites.

---

### Task QA-010: Memory Profiling Benchmark

**Task ID:** QA-010
**Title:** Peak RSS Memory Benchmarks Using DHAT or jemalloc
**Description:** Measure peak memory usage during validation to enforce the <50MB target. Use `dhat-rs` (Valgrind's DHAT as a Rust crate) or `jemalloc` with profiling enabled.

**Estimated Effort:** 3 engineering days

**Dependencies:** QA-009

**Acceptance Criteria:**
- Peak RSS measured for: schema parsing, validation execution, data generation, report generation.
- Memory benchmark runs as part of the nightly benchmark suite.
- Alert if peak RSS exceeds 50MB for a 100-endpoint validation (without SLM).
- Alert if peak RSS exceeds 2GB for SLM-powered data generation.
- Memory flamegraphs generated and stored as CI artifacts.

**Best Practices:**
- Use `#[global_allocator]` with `dhat::Alloc` only in benchmark binaries, not the main binary.
- Measure allocation count (number of allocations), not just total bytes.

**Pitfalls to Avoid:**
- DHAT adds overhead. Do not compare DHAT-instrumented benchmarks against non-instrumented baselines.

---

### Task QA-011: Benchmark CI Integration and Regression Detection

**Task ID:** QA-011
**Title:** Automated Benchmark Regression Detection in CI
**Description:** Run benchmarks on every PR and post a comment with performance comparison against the `main` branch baseline. Flag regressions above 10%.

**Estimated Effort:** 4 engineering days

**Dependencies:** QA-009, QA-004

**Acceptance Criteria:**
- Benchmarks run on a consistent CI environment (dedicated runner or pinned instance type).
- PR comment includes a table: benchmark name, baseline, current, delta percentage, status (pass/warn/fail).
- Thresholds: <5% = pass (green), 5-10% = warn (yellow), >10% = fail (red).
- Benchmark results archived per commit SHA for historical tracking.
- A `criterion-compare` GitHub Action or equivalent automates this.

**Best Practices:**
- Run each benchmark 3 times and take the median to reduce noise.
- Pin the CI runner instance type to minimize hardware variance.

**Pitfalls to Avoid:**
- Shared CI runners have noisy neighbors. If you cannot use a dedicated runner, increase the noise tolerance threshold to 15%.
- Do not block PRs on benchmark regressions caused by CI noise. Use the "two consecutive regressions" rule.

---

### Task QA-012: Competitive Benchmark Suite

**Task ID:** QA-012
**Title:** ValiForge vs. Competitors Performance Comparison
**Description:** Create a reproducible benchmark comparing ValiForge against Schemathesis, Postman CLI, Karate DSL, and stepci. Published with each major release.

**Estimated Effort:** 5 engineering days

**Dependencies:** QA-009, QA-001

**Acceptance Criteria:**
- Benchmark script in `benches/competitive/` sets up each tool and runs against the same Petstore spec.
- Metrics measured: cold start time, 50-endpoint validation time, peak memory, binary/install size.
- Results published as a Markdown table and a chart (SVG generated by a script).
- Benchmark is reproducible: pinned versions of each tool, Docker-based execution.
- Disclaimer: "Run on [machine spec]. Your results may vary." included in all published results.
- Results refreshed with every major or minor release.

**Best Practices:**
- Use Docker containers for each competitor to ensure clean, reproducible environments.
- Measure wall-clock time, not CPU time (users experience wall-clock).
- Run each competitor 5 times, report median.

**Pitfalls to Avoid:**
- Do not cherry-pick benchmark scenarios that favor ValiForge. Test on the same spec, same endpoints, same machine.
- Do not use competitors' debug modes. Use their recommended production/CI configurations.

---

## 5. Compatibility Testing

### Task QA-013: OpenAPI Specification Compatibility Matrix

**Task ID:** QA-013
**Title:** OpenAPI Version Compatibility Test Suite
**Description:** Verify that ValiForge correctly handles all OpenAPI 3.x sub-versions, including version-specific features that differ between 3.0.x and 3.1.0.

**Estimated Effort:** 6 engineering days

**Dependencies:** QA-001

**Acceptance Criteria:**
- Test matrix:

| Feature | 3.0.0 | 3.0.1 | 3.0.2 | 3.0.3 | 3.1.0 |
|---------|-------|-------|-------|-------|-------|
| Basic schema parsing | Yes | Yes | Yes | Yes | Yes |
| `nullable` keyword | Yes | Yes | Yes | Yes | N/A (uses JSON Schema `type: [string, null]`) |
| `exclusiveMinimum` as boolean | Yes | Yes | Yes | Yes | N/A (is a number in 3.1) |
| JSON Schema 2020-12 | N/A | N/A | N/A | N/A | Yes |
| Webhooks | N/A | N/A | N/A | N/A | Yes |
| `$ref` alongside other keywords | N/A | N/A | N/A | N/A | Yes |
| `discriminator` | Yes | Yes | Yes | Yes | Yes |
| `callbacks` | Yes | Yes | Yes | Yes | Yes |

- Every cell in the matrix has at least one test.
- Edge case specs from Section 1.2 are included.
- Tests verify both successful parsing AND correct semantic interpretation.

**Best Practices:**
- Test both machine-generated and human-authored specs (they differ in quality significantly).
- Include specs with `$ref` to external files (resolved at parse time).

**Pitfalls to Avoid:**
- Do not assume 3.1.0 is a superset of 3.0.x. The `nullable` handling is fundamentally different.
- Do not ignore `3.0.0` -- many older specs still use it.

---

### Task QA-014: Protobuf and GraphQL Compatibility

**Task ID:** QA-014
**Title:** Protobuf and GraphQL Compatibility Test Suite
**Description:** Test ValiForge's support for Protobuf (proto2, proto3) and GraphQL (SDL, introspection, federation).

**Estimated Effort:** 5 engineering days

**Dependencies:** QA-001

**Acceptance Criteria:**
- **Protobuf:**
  - proto2 syntax (required/optional/repeated fields, groups, extensions)
  - proto3 syntax (scalar defaults, `oneof`, maps, `Any`, `Timestamp`, `Duration`)
  - Well-known types: `google.protobuf.Timestamp`, `google.protobuf.Duration`, `google.protobuf.Struct`, `google.protobuf.Value`
  - Nested messages, enums, packages, imports
  - Service definitions with unary, server-streaming, client-streaming, bidirectional RPCs
- **GraphQL:**
  - SDL parsing: types, interfaces, unions, enums, input types, scalars, directives
  - Introspection query handling
  - Federation: `@key`, `@extends`, `@external`, `_entities` query
  - Schema with 100+ types (complexity test)
  - Circular type references in GraphQL
- Test fixtures stored in `schemas/fixtures/protobuf/` and `schemas/fixtures/graphql/`.

**Best Practices:**
- Use the official proto3 test fixtures from the protobuf repo.
- For GraphQL federation, test with a real federation subgraph set.

**Pitfalls to Avoid:**
- proto2 and proto3 have different default value semantics. Test both explicitly.
- GraphQL introspection schema has its own edge cases (deprecated fields, custom scalars).

---

### Task QA-015: CI Platform Compatibility Testing

**Task ID:** QA-015
**Title:** Verify ValiForge Works in Major CI Environments
**Description:** Test that ValiForge's CI integration (exit codes, output formats, env variable handling) works correctly in GitHub Actions, GitLab CI, Jenkins, CircleCI, and Buildkite.

**Estimated Effort:** 5 engineering days

**Dependencies:** QA-007

**Acceptance Criteria:**
- Example CI configs in `examples/ci/`:
  - `.github/workflows/valiforge.yml` (GitHub Actions)
  - `.gitlab-ci.yml` (GitLab CI)
  - `Jenkinsfile` (Jenkins)
  - `.circleci/config.yml` (CircleCI)
  - `.buildkite/pipeline.yml` (Buildkite)
- Each config is tested manually (at minimum) or via integration test (preferred).
- JUnit XML output is parseable by each CI platform's test result viewer.
- Exit codes are correctly interpreted by each CI platform.
- Environment variable injection (`VALIFORGE_TOKEN`, etc.) works in each platform.
- Documentation for each platform in `docs/ci/`.

**Best Practices:**
- Use the most basic configuration first, then add advanced features.
- Test with both installed binary and Docker-based approaches.

**Pitfalls to Avoid:**
- Jenkins has unique shell quoting rules. Test actual Jenkins execution, not just syntax.
- GitLab CI uses a different YAML parser than GitHub Actions. Validate YAML for each platform.

---

### Task QA-016: OS Compatibility Testing

**Task ID:** QA-016
**Title:** Cross-OS Compatibility Verification
**Description:** Ensure the ValiForge binary works correctly on all target operating systems, testing file path handling, signal handling, and platform-specific behavior.

**Estimated Effort:** 4 engineering days

**Dependencies:** QA-004

**Acceptance Criteria:**
- CI matrix covers:
  - Ubuntu 22.04, 24.04 (x86_64)
  - Debian 12 (via Docker)
  - Alpine 3.18+ (musl libc, via Docker)
  - macOS 14 (ARM64), macOS 15 (when available)
  - Windows 11 (x86_64)
- Platform-specific tests:
  - File paths with spaces and Unicode characters (Windows is strictest)
  - Signal handling: SIGINT/SIGTERM graceful shutdown (Unix), Ctrl+C (Windows)
  - Line endings: CRLF on Windows, LF on Unix
  - Colored output: ANSI support detection
  - Temp directory: `TMPDIR` (Unix) vs `%TEMP%` (Windows)
- Alpine/musl target: static binary works without glibc.

**Best Practices:**
- Use `cfg(target_os)` for platform-specific code and test each branch.
- Test the actual release binary (not just `cargo test`). Download the release artifact and run it.

**Pitfalls to Avoid:**
- Alpine Linux uses musl libc. If you link against glibc, the binary will not work on Alpine. Test the musl build.
- Windows path separators (`\` vs `/`). Use `std::path::PathBuf` everywhere, never string concatenation.

---

## 6. Error Handling & Resilience Testing

### Task QA-017: Network Failure Resilience Tests

**Task ID:** QA-017
**Title:** Network Failure Scenario Testing
**Description:** Verify ValiForge handles every type of network failure gracefully -- no panics, helpful error messages, correct exit codes.

**Estimated Effort:** 4 engineering days

**Dependencies:** QA-002

**Acceptance Criteria:**
- Scenarios tested via wiremock-rs and system-level mocking:

| Scenario | Expected Behavior |
|----------|------------------|
| Connection refused (target not running) | Error: "Connection refused to http://localhost:3000. Is the target server running?" |
| DNS resolution failure | Error: "Could not resolve hostname 'api.example.com'. Check your network or use an IP address." |
| Connection timeout (30s) | Error: "Connection timed out after 30s. Consider increasing --timeout or check network connectivity." |
| Read timeout (response too slow) | Error: "Response timeout after 10s for GET /users. The server may be overloaded." |
| TLS certificate error (self-signed) | Error: "TLS certificate verification failed. Use --insecure to skip verification (not recommended)." |
| TLS handshake failure | Error: "TLS handshake failed. The server may not support TLS or may use an unsupported protocol version." |
| Connection reset mid-response | Error: "Connection reset while reading response for POST /orders. Retrying (attempt 2/3)..." |
| HTTP 429 (rate limited) | Automatic retry with exponential backoff; "Rate limited by server. Waiting 2s before retry." |
| HTTP 503 (server unavailable) | Retry with backoff; after max retries: "Server returned 503 after 3 retries. Target may be down." |

- Every error message includes: what happened, why it might have happened, and a suggested fix.
- Exit code is 1 for network errors (not 2, which is config errors).
- `--retry` flag controls retry count (default: 3).
- `--timeout` flag controls per-request timeout (default: 30s).

**Best Practices:**
- Use reqwest's built-in timeout configuration, not manual `tokio::time::timeout`.
- Log the full error chain at `--verbose` level but show a user-friendly message by default.

**Pitfalls to Avoid:**
- Do not retry on 4xx errors (client errors). Only retry on 429, 503, and network-level failures.
- Do not swallow errors. Every error must surface to the user.

---

### Task QA-018: Malformed Input Handling Tests

**Task ID:** QA-018
**Title:** Malformed Input Resilience Testing
**Description:** Verify ValiForge handles every type of malformed input gracefully, with actionable error messages.

**Estimated Effort:** 4 engineering days

**Dependencies:** QA-001

**Acceptance Criteria:**
- Scenarios:

| Input Type | Test Case | Expected Error |
|-----------|-----------|---------------|
| Invalid YAML | Missing colon in key-value | "Parse error at line 15: expected ':' after key. Check YAML syntax." |
| Invalid JSON | Trailing comma | "JSON parse error at line 42: trailing comma. Remove the comma before '}'." |
| Valid YAML, invalid OpenAPI | Missing `paths` key | "Invalid OpenAPI spec: missing required field 'paths'. See https://spec.openapis.org/oas/v3.1.0" |
| Truncated file | File ends mid-object | "Unexpected end of file. The schema file appears to be truncated." |
| Binary file | User passes a .exe or .png | "File 'spec.png' does not appear to be YAML or JSON. Expected an OpenAPI/Protobuf/GraphQL schema." |
| Empty file | Zero bytes | "Schema file 'api.yaml' is empty." |
| File not found | Bad path | "File not found: '/path/to/api.yaml'. Check the path and permissions." |
| Permission denied | Unreadable file | "Permission denied: cannot read '/path/to/api.yaml'. Check file permissions." |
| Extremely large file | 100MB YAML | "Schema file exceeds maximum size (50MB). Consider splitting the schema." |
| Recursive YAML anchors | Self-referencing anchor | "YAML anchor recursion detected. This is likely an error in the schema." |
| Mixed tabs and spaces | YAML indentation error | "YAML indentation error at line 23. Do not mix tabs and spaces." |
| UTF-8 BOM | File starts with BOM | Silently stripped; parsed correctly |
| Non-UTF-8 encoding | Latin-1 encoded file | "File is not valid UTF-8. ValiForge requires UTF-8 encoded schemas." |

- Every error message is tested via snapshot tests (insta).
- Every error message includes the file path, line number (where applicable), and a fix suggestion.
- No error message contains a Rust stack trace or internal panic message.

**Best Practices:**
- Use Rust's `miette` or `ariadne` crate for beautiful, IDE-like error messages with line annotations.
- Include a "did you mean?" suggestion when the error is close to a valid input (e.g., "Did you mean 'paths'?" when user writes 'path').

**Pitfalls to Avoid:**
- Do not use `unwrap()` or `expect()` anywhere in production code. All errors must be handled.
- Do not expose internal error types in user-facing messages. Map them to ValiForge error codes.

---

### Task QA-019: Resource Exhaustion Testing

**Task ID:** QA-019
**Title:** Resource Exhaustion and Pathological Input Testing
**Description:** Verify ValiForge handles pathologically large or deeply nested inputs without OOM, infinite loops, or CPU exhaustion.

**Estimated Effort:** 4 engineering days

**Dependencies:** QA-001, QA-002

**Acceptance Criteria:**
- Scenarios:

| Scenario | Input | Expected Behavior |
|----------|-------|------------------|
| Huge schema | 1000+ endpoints | Completes within 30s; peak RSS <200MB |
| Deep nesting | 100 levels of nested objects | Detects and limits at 50 levels; warns user |
| Massive response body | 10MB JSON response from target | Streams response; does not load entirely into memory |
| Many query parameters | 500 query params on one endpoint | Parses correctly; does not explode combinatorially in datagen |
| Circular `$ref` (infinite) | `A -> B -> A` references | Detected and reported as error, not infinite loop |
| Very long field names | 10KB field name | Truncates in reports; no buffer overflow |
| Many validation failures | All 1000 endpoints fail | Reports all failures; does not OOM from storing results |
| Concurrent validation | 100 parallel HTTP requests | Configurable concurrency limit (default: 20); backpressure |

- Configurable limits in `valiforge.toml`:
  - `max_depth = 50`
  - `max_endpoints = 5000`
  - `max_response_size = "10MB"`
  - `max_concurrency = 20`
- Exceeding limits produces a clear warning, not a crash.

**Best Practices:**
- Use streaming JSON parsing (`serde_json::StreamDeserializer`) for large responses.
- Implement depth tracking in recursive schema resolution to detect infinite recursion.

**Pitfalls to Avoid:**
- Do not allocate `Vec<u8>` for the full response body. Use streaming.
- Do not use recursion without a depth counter. Stack overflow on deep schemas is a real risk.

---

### Task QA-020: Graceful Degradation Testing

**Task ID:** QA-020
**Title:** Graceful Degradation When Optional Components Fail
**Description:** Verify that ValiForge continues to function when optional components (SLM, plugins, database adapters) are unavailable.

**Estimated Effort:** 3 engineering days

**Dependencies:** QA-002

**Acceptance Criteria:**
- Degradation matrix:

| Component | Unavailable Scenario | Expected Behavior |
|-----------|---------------------|------------------|
| SLM model | Model file missing | Falls back to grammar-based generation; warns: "SLM model not found. Using grammar-based generation." |
| SLM model | Model corrupted | Falls back to grammar; warns: "SLM model failed to load: invalid format. Using grammar-based generation." |
| SLM inference | CUDA/Metal unavailable | Falls back to CPU inference (slower) or grammar; warns user |
| WASM plugin | Plugin crashes (panic/trap) | Catches WASM trap; reports plugin error; continues validation without plugin |
| WASM plugin | Plugin exceeds resource limit | Terminates plugin after 10s; reports timeout; continues without plugin |
| Database adapter | PostgreSQL not reachable | Skips state mutation tests; warns: "Database unreachable. State mutation tests skipped." |
| Config file | `valiforge.toml` malformed | Falls back to CLI flags + defaults; warns about config parse error |
| Network | Entire network down | Reports which validations require network; runs offline-capable validations |

- All degradation paths are tested with explicit assertions on warning messages and fallback behavior.
- Exit code is 0 if core validation passes, even if optional components degrade.

**Best Practices:**
- Use the Result type aggressively. Every fallible operation returns Result.
- Log degradation events at WARN level so users can see them with `--verbose`.

**Pitfalls to Avoid:**
- Do not silently degrade. Users must know when a component is unavailable.
- Do not make the SLM a hard dependency. Grammar-based generation is the baseline.

---

## 7. Security Testing

### Task QA-021: Fuzz All Input Parsers

**Task ID:** QA-021
**Title:** Comprehensive Fuzzing of All Input Parsers
**Description:** Systematically fuzz every input parser in ValiForge (OpenAPI YAML/JSON, Protobuf, GraphQL SDL, TOML config, WASM modules). This is the primary defense against untrusted input.

**Estimated Effort:** 6 engineering days

**Dependencies:** QA-001

**Acceptance Criteria:**
- 10 fuzz targets (from Section 1.6) implemented in `fuzz/fuzz_targets/`.
- Each target has an initial corpus seeded with real-world fixtures.
- Fuzz targets run for minimum 10 minutes each in nightly CI.
- Zero panics discovered in fuzz testing (or all fixed before merge).
- Coverage-guided fuzzing (libFuzzer) for maximum code path exploration.
- Regression test: every crash-inducing input becomes a permanent test case.
- Structured fuzzing (using `arbitrary` crate) for fuzz targets that benefit from semi-valid input (FUZZ-006, FUZZ-007).

**Best Practices:**
- Start with unstructured fuzzing (random bytes) for parser robustness.
- Graduate to structured fuzzing (AST-level mutations) for deeper logic bugs.
- Use `cargo-fuzz` `--sanitizer address` to detect memory bugs (use-after-free, buffer overflow, though these are rare in safe Rust, they can occur in unsafe blocks or FFI).

**Pitfalls to Avoid:**
- Do not fuzz with sanitizers enabled and compare performance against unsanitized benchmarks.
- Do not run fuzz tests in the same CI job as unit tests. They need dedicated time and resources.

---

### Task QA-022: Secret Detection in ValiForge Output

**Task ID:** QA-022
**Title:** Ensure ValiForge Never Leaks API Keys or Tokens
**Description:** ValiForge sends HTTP requests to target APIs using user-provided authentication headers (Bearer tokens, API keys). These secrets must NEVER appear in logs, reports, error messages, or crash dumps.

**Estimated Effort:** 3 engineering days

**Dependencies:** QA-002

**Acceptance Criteria:**
- All HTTP headers with names matching `Authorization`, `X-Api-Key`, `Cookie`, `X-Auth-Token` (case-insensitive) are redacted in:
  - Terminal output
  - JSON/JUnit/Markdown reports
  - Error messages
  - Debug/verbose logs
  - Crash reports (opt-in telemetry)
- Redaction format: `Authorization: Bearer ***REDACTED***`
- Request bodies are NOT redacted (they may contain test data needed for debugging).
- Redaction is tested with integration tests: send a request with `Authorization: Bearer sk-secret-key-12345`, verify no output contains `sk-secret-key-12345`.
- Regex-based detection for common secret patterns: `sk-*`, `ghp_*`, `glpat-*`, `xoxb-*`, `AKIA*`.
- A dedicated `SecretRedactor` module with 100% test coverage.

**Best Practices:**
- Implement redaction at the output layer, not the HTTP layer. The HTTP client needs the real secret to authenticate.
- Use a trait `Redactable` that all output types implement.
- Redact BEFORE writing to any output stream (including logs), not after.

**Pitfalls to Avoid:**
- Do not redact only known patterns. Also redact any header whose name contains "auth", "token", "key", "secret", "password" (case-insensitive).
- Do not log full HTTP responses if the response body might contain tokens (e.g., OAuth token endpoints).

---

### Task QA-023: Unsafe Code Audit Policy

**Task ID:** QA-023
**Title:** Establish and Enforce Zero-Unsafe Policy
**Description:** ValiForge uses safe Rust as its security foundation. Any `unsafe` block requires explicit justification, audit, and ongoing review.

**Estimated Effort:** 2 engineering days

**Dependencies:** None

**Acceptance Criteria:**
- `cargo geiger` runs in CI and reports unsafe usage.
- Any new `unsafe` block in a PR requires:
  - A comment explaining why safe Rust cannot achieve the same result.
  - A `// SAFETY: <explanation>` comment on the unsafe block.
  - Miri (`cargo +nightly miri test`) run on the containing module.
  - Explicit approval from a designated code owner.
- Transitive unsafe (from dependencies) is tracked via `cargo geiger --output-format=json`.
- A report `UNSAFE_AUDIT.md` lists all known unsafe usage (direct and transitive) with justification.
- Goal: zero direct unsafe blocks. Indirect unsafe (from reqwest, tokio, etc.) is acceptable but tracked.

**Best Practices:**
- Use `#![forbid(unsafe_code)]` at the crate root for crates that should never contain unsafe (valiforge-cli, valiforge-report, valiforge-diff, valiforge-sdk).
- Allow unsafe only in `valiforge-core` (for FFI with SLM runtimes) and `valiforge-plugin` (for WASM runtime).

**Pitfalls to Avoid:**
- Do not assume dependency unsafe is harmless. Use `cargo crev` to check community audits.
- Do not use `unsafe` for performance optimization without benchmarks proving it matters.

---

### Task QA-024: OWASP Test Data Labeling

**Task ID:** QA-024
**Title:** Ensure Generated Injection Payloads Are Properly Labeled
**Description:** ValiForge generates negative test data including SQL injection, XSS, command injection, and path traversal payloads. These must be clearly labeled so users understand their purpose and do not mistake them for real attacks.

**Estimated Effort:** 2 engineering days

**Dependencies:** None

**Acceptance Criteria:**
- Every generated payload that contains an injection pattern has metadata:
  ```json
  {
    "payload": "' OR 1=1 --",
    "category": "negative",
    "attack_type": "sql_injection",
    "severity": "high",
    "description": "Tests whether the endpoint is vulnerable to SQL injection via string termination",
    "owasp_category": "A03:2021-Injection"
  }
  ```
- Categories: `sql_injection`, `xss`, `command_injection`, `path_traversal`, `ldap_injection`, `xml_injection`, `nosql_injection`.
- OWASP Top 10 (2021) mapping for each attack type.
- Generated payloads are well-known test patterns from OWASP Testing Guide, not novel attack vectors.
- A `--no-injection` flag disables injection payload generation entirely.

**Best Practices:**
- Use established injection test patterns from OWASP ZAP or Burp Suite's payload lists.
- Label payloads at generation time, not post-hoc.

**Pitfalls to Avoid:**
- Do not generate novel attack payloads. Stick to well-known test patterns.
- Do not send injection payloads to production APIs by default. Require explicit opt-in via `--include-injection` flag.

---

### Task QA-025: Dependency Vulnerability Scanning

**Task ID:** QA-025
**Title:** Continuous Dependency Vulnerability Monitoring
**Description:** Automated scanning for known vulnerabilities in ValiForge's dependency tree, integrated into CI and release process.

**Estimated Effort:** 2 engineering days

**Dependencies:** QA-007 (CI pipeline)

**Acceptance Criteria:**
- `cargo audit` runs on every PR (Gate G6).
- `cargo audit` runs nightly against `main` branch.
- GitHub Dependabot or Renovate configured for automated dependency update PRs.
- `cargo audit fix` applied to lockfile when safe updates are available.
- A weekly report of dependency health posted to the team channel.
- Severity policy:
  - Critical/High: Block PR; fix within 24 hours.
  - Medium: Block PR; fix within 1 week.
  - Low: Warn; fix within 1 month.
  - Informational: Track; fix at convenience.
- RustSec advisory database is the primary data source.

**Best Practices:**
- Use `cargo audit --deny warnings` to catch advisories that are not yet classified as vulnerabilities.
- Review each advisory individually before upgrading. Sometimes the vulnerability does not affect your usage of the crate.

**Pitfalls to Avoid:**
- Do not auto-merge dependency update PRs without running the full test suite.
- Do not ignore advisories for transitive dependencies. They are still in your binary.

---

## 8. Release Process & Quality Checklist

### Task QA-026: Semantic Versioning Strategy

**Task ID:** QA-026
**Title:** Define and Enforce Semantic Versioning Policy
**Description:** Establish clear rules for version bumping, compatible with Cargo's semver enforcement and the expectations of library consumers.

**Estimated Effort:** 1 engineering day

**Dependencies:** None

**Acceptance Criteria:**
- Version format: `MAJOR.MINOR.PATCH` (e.g., `0.1.0`, `1.0.0`, `1.2.3`).
- Rules:
  - **MAJOR** (1.0.0 -> 2.0.0): Removed public API, changed behavior of existing public API, removed CLI flags, changed exit code semantics.
  - **MINOR** (1.0.0 -> 1.1.0): New features, new CLI commands/flags, new output formats, new protocol support, new config options.
  - **PATCH** (1.0.0 -> 1.0.1): Bug fixes, performance improvements, dependency updates, documentation fixes.
- Pre-1.0: MINOR bumps may include breaking changes (Rust ecosystem convention).
- All public APIs marked with `#[doc(hidden)]` are not considered part of the versioned API surface.
- `cargo-semver-checks` runs in CI to detect accidental breaking changes in library crates.

**Best Practices:**
- Use conventional commits (`feat:`, `fix:`, `chore:`, `docs:`, `perf:`, `refactor:`, `test:`, `ci:`) to auto-determine version bumps.
- Tag releases with `v` prefix: `v0.1.0`, `v1.0.0`.

**Pitfalls to Avoid:**
- Do not bump MAJOR version for internal-only changes (changes to non-`pub` items).
- Do not treat performance regressions as semver-breaking (they are bugs, not API changes).

---

### Task QA-027: Release Candidate Process

**Task ID:** QA-027
**Title:** Release Candidate (RC) Workflow
**Description:** Define the process for cutting release candidates, testing them, and promoting to stable releases.

**Estimated Effort:** 3 engineering days

**Dependencies:** QA-007, QA-009, QA-026

**Acceptance Criteria:**
- RC workflow:
  1. Branch `release/vX.Y.Z` from `main`.
  2. Bump version to `X.Y.Z-rc.1` in all `Cargo.toml` files.
  3. CI runs the FULL quality gate suite (all 13 gates + benchmarks + competitive benchmarks + compatibility matrix).
  4. RC binary published to GitHub Releases as a pre-release.
  5. Core team members test RC binary on their local machines (macOS, Linux, Windows).
  6. Community beta testers (opt-in list) are notified via Discord.
  7. RC soak period: minimum 72 hours for minor releases, 7 days for major releases.
  8. If bugs found: fix on release branch, bump to `rc.2`, repeat soak.
  9. If no blocking bugs after soak: promote to `X.Y.Z` (remove `-rc.N`), merge release branch to `main`, tag.
- Maximum 3 RC iterations. If RC.3 still has blocking bugs, abort the release and reassess.

**Best Practices:**
- Use a release checklist issue template (GitHub issue template) to track each step.
- Automate as much as possible: version bumping, changelog generation, binary building.

**Pitfalls to Avoid:**
- Do not skip the soak period under deadline pressure. The 72-hour minimum exists for a reason.
- Do not merge feature PRs into the release branch. Only bug fixes.

---

### Task QA-028: Changelog Generation

**Task ID:** QA-028
**Title:** Automated Changelog Generation with git-cliff
**Description:** Use `git-cliff` with conventional commits to auto-generate changelogs for each release.

**Estimated Effort:** 2 engineering days

**Dependencies:** QA-026

**Acceptance Criteria:**
- `cliff.toml` configured with:
  - Commit groups: Features, Bug Fixes, Performance, Breaking Changes, Documentation.
  - Conventional commit parsing (from `feat:`, `fix:`, etc.).
  - Breaking changes highlighted prominently (from `feat!:` or `BREAKING CHANGE:` footer).
  - GitHub PR links included.
  - Author attribution.
- `CHANGELOG.md` auto-generated and committed as part of the release process.
- Each release entry includes: version, date, full list of changes, contributor list.
- `git-cliff` runs in the release CI workflow.

**Best Practices:**
- Enforce conventional commits via a pre-commit hook or CI check.
- Write commit messages for the audience (users reading the changelog), not for the team.

**Pitfalls to Avoid:**
- Do not include internal refactoring or CI changes in user-facing changelogs unless they affect behavior.
- Do not auto-generate changelogs without human review. Always review before publishing.

---

### Task QA-029: Pre-Release Quality Checklist

**Task ID:** QA-029
**Title:** Comprehensive Pre-Release Quality Checklist
**Description:** A mandatory checklist that must be completed before any release is published. Enforced via a GitHub issue template.

**Estimated Effort:** 2 engineering days

**Dependencies:** QA-027

**Acceptance Criteria:**
- Checklist items:

```markdown
## Pre-Release Checklist for vX.Y.Z

### CI & Testing
- [ ] All CI quality gates green on release branch
- [ ] Full benchmark suite run; no regressions >10%
- [ ] Competitive benchmarks updated (if applicable)
- [ ] Compatibility matrix: all OpenAPI versions pass
- [ ] Compatibility matrix: all target OS binaries tested
- [ ] Fuzz suite: minimum 10 minutes per target, zero crashes
- [ ] Coverage: >= 85% workspace aggregate

### Documentation
- [ ] CHANGELOG.md updated (via git-cliff, then human-reviewed)
- [ ] README.md version references updated
- [ ] API docs (`cargo doc --no-deps`) generated and reviewed
- [ ] Migration guide written (if breaking changes)
- [ ] Blog post drafted (for minor/major releases)

### Artifacts
- [ ] Binary built for: linux-x86_64, linux-aarch64, macos-x86_64, macos-aarch64, windows-x86_64
- [ ] Binary sizes within threshold (<15MB each)
- [ ] SHA256 checksums generated for all binaries
- [ ] Homebrew formula updated
- [ ] Cargo publish dry-run successful (`cargo publish --dry-run`)
- [ ] GitHub Action marketplace listing updated

### Manual Testing
- [ ] `valiforge init` works on fresh project
- [ ] `valiforge validate` works against local Petstore mock
- [ ] `valiforge diff` works with sample schema pair
- [ ] `valiforge generate` produces valid output
- [ ] Error messages are helpful (spot-check 5 error scenarios)

### Security
- [ ] `cargo audit` shows zero critical/high vulnerabilities
- [ ] `cargo deny check` passes
- [ ] No new `unsafe` code without audit

### Sign-Off
- [ ] QA lead sign-off
- [ ] Engineering lead sign-off
- [ ] Release notes approved by product owner
```

**Best Practices:**
- Automate every checkable item. The checklist is the human verification layer on top of automated checks.
- Block the release if any item is unchecked. No "we'll fix it in a patch release."

**Pitfalls to Avoid:**
- Do not treat the checklist as a formality. Every checkbox represents a real risk if skipped.

---

### Task QA-030: Post-Release Monitoring

**Task ID:** QA-030
**Title:** Post-Release Monitoring and Response Playbook
**Description:** Define monitoring, alerting, and response procedures for the 72 hours after each release.

**Estimated Effort:** 2 engineering days

**Dependencies:** QA-029

**Acceptance Criteria:**
- Monitoring channels:
  - **GitHub Issues:** Set up a saved search for issues created within 72 hours of a release. Alert if >5 issues mention the new version.
  - **Discord #help:** Designate a rotating "release monitor" who watches for user complaints in the 72 hours post-release.
  - **Binary downloads:** Track download counts per release on GitHub Releases. Alert if downloads plateau significantly below previous release (indicates distribution issue).
  - **Crash reports (opt-in):** If telemetry is enabled, monitor crash rate. Target: <0.01%. Alert if >0.05%.
  - **Twitter/X mentions:** Monitor `valiforge` mentions for user complaints.
- Response playbook:
  - **P0 (data loss, crash, security):** Yank release within 1 hour. Publish hotfix within 24 hours.
  - **P1 (major feature broken):** Publish patch within 48 hours.
  - **P2 (minor bug):** Track in backlog; fix in next patch.
- Rollback procedure: document how to yank a crate version (`cargo yank --version X.Y.Z`) and remove GitHub release.

**Best Practices:**
- The first 72 hours are the highest-risk period. Have the release engineer on-call.
- Proactively post release notes to Discord and Twitter when publishing.

**Pitfalls to Avoid:**
- Do not release on Fridays. Release Monday-Wednesday to have weekdays for monitoring.
- Do not yank a release without publishing a replacement. Users on the yanked version lose the ability to build.

---

## 9. Dogfooding Strategy

### Task QA-031: ValiForge Validates Its Own Cloud API

**Task ID:** QA-031
**Title:** Use ValiForge to Validate ValiForge Cloud's API
**Description:** ValiForge Cloud (the SaaS product) exposes REST APIs. ValiForge (the CLI) must be used in ValiForge Cloud's CI pipeline to validate these APIs.

**Estimated Effort:** 3 engineering days

**Dependencies:** ValiForge Cloud exists (Phase 2 prerequisite)

**Acceptance Criteria:**
- ValiForge Cloud's CI pipeline includes a `valiforge validate` step that:
  - Validates the Cloud API against its OpenAPI spec.
  - Runs breaking change detection on schema changes.
  - Generates test data and sends it to a staging environment.
- Any ValiForge CLI bug discovered through dogfooding is prioritized as P1.
- Dogfooding results are shared publicly (blog post: "How We Test ValiForge With ValiForge").

**Best Practices:**
- Use the latest released version of ValiForge in Cloud's CI, not the development build. This tests the actual user experience.
- Track dogfooding-discovered bugs separately to measure dogfooding effectiveness.

**Pitfalls to Avoid:**
- Do not use a development build for dogfooding. Use the same binary your users download.
- Do not exempt ValiForge Cloud from ValiForge's own quality standards.

---

### Task QA-032: ValiForge in ValiForge's Own CI

**Task ID:** QA-032
**Title:** Use ValiForge to Validate ValiForge's Test Data Generation
**Description:** ValiForge's data generation engine produces JSON payloads from schemas. Use ValiForge's own schema validation to verify these payloads are correct.

**Estimated Effort:** 2 engineering days

**Dependencies:** QA-001

**Acceptance Criteria:**
- A CI step runs `valiforge validate` against ValiForge's own generated test fixtures.
- If ValiForge generates a "valid" payload that fails ValiForge's own validation, the CI fails.
- This is the ultimate self-consistency check: generator and validator must agree.

**Best Practices:**
- This is essentially a continuous version of PROP-003 and PROP-004 from the property-based testing section.
- Run this on every PR, not just releases.

**Pitfalls to Avoid:**
- Do not skip this check because "we already have property-based tests." The CI dogfooding catches integration-level inconsistencies that property tests miss.

---

### Task QA-033: Engineer Local Workflow Integration

**Task ID:** QA-033
**Title:** Every ValiForge Engineer Uses ValiForge Locally
**Description:** Mandate that every ValiForge engineer uses ValiForge as part of their local development workflow (pre-commit hook, pre-push check).

**Estimated Effort:** 1 engineering day

**Dependencies:** ValiForge MVP functional

**Acceptance Criteria:**
- `.pre-commit-config.yaml` includes a ValiForge pre-push hook that validates any schema changes.
- Onboarding documentation includes "Install ValiForge and run `valiforge init` in this repo."
- Monthly "dogfooding retrospective" where engineers share pain points discovered through personal use.
- Bug reports from engineers using ValiForge on their own projects are tracked with the `dogfooding` label.

**Best Practices:**
- Lead by example. The CEO and CTO should be the most active dogfooders.
- Create a `#dogfooding` Discord channel for informal feedback.

**Pitfalls to Avoid:**
- Do not make local ValiForge usage optional. It must be part of the standard workflow.
- Do not dismiss pain points as "edge cases." If an engineer hits it, users will too.

---

## 10. Task Registry

### Summary Table

| Task ID | Title | Effort (days) | Dependencies | Sprint |
|---------|-------|--------------|-------------|--------|
| QA-001 | Test Fixture Collection | 3 | None | S1 |
| QA-002 | Mock API Server (wiremock-rs) | 5 | QA-001 | S1 |
| QA-003 | Docker PostgreSQL (testcontainers) | 3 | QA-002 | S2 |
| QA-004 | CI Test Matrix | 4 | QA-001, QA-002 | S2 |
| QA-005 | nextest Configuration | 2 | None | S1 |
| QA-006 | Coverage Reporting | 2 | QA-004 | S2 |
| QA-007 | Quality Gate CI Pipeline | 5 | QA-004, QA-005, QA-006 | S2 |
| QA-008 | cargo-deny Configuration | 1 | None | S1 |
| QA-009 | Criterion Benchmark Suite | 8 | QA-001, QA-002 | S2-S3 |
| QA-010 | Memory Profiling Benchmark | 3 | QA-009 | S3 |
| QA-011 | Benchmark CI Integration | 4 | QA-009, QA-004 | S3 |
| QA-012 | Competitive Benchmark Suite | 5 | QA-009, QA-001 | S4 |
| QA-013 | OpenAPI Compatibility Matrix | 6 | QA-001 | S2 |
| QA-014 | Protobuf & GraphQL Compatibility | 5 | QA-001 | S3 |
| QA-015 | CI Platform Compatibility | 5 | QA-007 | S4 |
| QA-016 | OS Compatibility Testing | 4 | QA-004 | S3 |
| QA-017 | Network Failure Resilience | 4 | QA-002 | S2 |
| QA-018 | Malformed Input Handling | 4 | QA-001 | S2 |
| QA-019 | Resource Exhaustion Testing | 4 | QA-001, QA-002 | S3 |
| QA-020 | Graceful Degradation Testing | 3 | QA-002 | S3 |
| QA-021 | Fuzz All Input Parsers | 6 | QA-001 | S2-S3 |
| QA-022 | Secret Detection/Redaction | 3 | QA-002 | S2 |
| QA-023 | Unsafe Code Audit Policy | 2 | None | S1 |
| QA-024 | OWASP Test Data Labeling | 2 | None | S3 |
| QA-025 | Dependency Vulnerability Scanning | 2 | QA-007 | S2 |
| QA-026 | Semantic Versioning Strategy | 1 | None | S1 |
| QA-027 | Release Candidate Process | 3 | QA-007, QA-009, QA-026 | S4 |
| QA-028 | Changelog Generation | 2 | QA-026 | S3 |
| QA-029 | Pre-Release Quality Checklist | 2 | QA-027 | S4 |
| QA-030 | Post-Release Monitoring | 2 | QA-029 | S4 |
| QA-031 | Dogfooding: Cloud API | 3 | Cloud API exists | S5+ |
| QA-032 | Dogfooding: Own CI | 2 | QA-001 | S3 |
| QA-033 | Dogfooding: Engineer Workflow | 1 | MVP exists | S3 |

### Total Effort

| Category | Tasks | Total Days |
|----------|-------|-----------|
| Test Infrastructure | QA-001 through QA-006 | 19 days |
| Quality Gates | QA-007, QA-008 | 6 days |
| Benchmarking | QA-009 through QA-012 | 20 days |
| Compatibility | QA-013 through QA-016 | 20 days |
| Error Handling & Resilience | QA-017 through QA-020 | 15 days |
| Security | QA-021 through QA-025 | 15 days |
| Release Process | QA-026 through QA-030 | 10 days |
| Dogfooding | QA-031 through QA-033 | 6 days |
| **TOTAL** | **33 tasks** | **111 engineering days** |

At 1 QA engineer: ~22 weeks (~5.5 months).
At 2 engineers (1 QA + 1 infra): ~12 weeks (~3 months).
Recommended: 2 engineers dedicated to QA infrastructure for the first 3 months, with ongoing maintenance at 20% capacity thereafter.

---

## 11. Sprint Plan & Critical Path

### Sprint Structure (2-week sprints)

```
Sprint 1 (Weeks 1-2): Foundation
├── QA-001: Test Fixture Collection (3d)
├── QA-005: nextest Configuration (2d)
├── QA-008: cargo-deny Configuration (1d)
├── QA-023: Unsafe Code Audit Policy (2d)
├── QA-026: Semantic Versioning Strategy (1d)
└── Buffer: 1d
    Total: 10 engineering days (1 engineer x 10 days)

Sprint 2 (Weeks 3-4): Core Infrastructure
├── QA-002: Mock API Server (5d)
├── QA-004: CI Test Matrix (4d) [depends on QA-001, QA-002]
├── QA-006: Coverage Reporting (2d) [depends on QA-004, start late]
├── QA-017: Network Failure Resilience (4d) [depends on QA-002]
├── QA-018: Malformed Input Handling (4d) [depends on QA-001]
├── QA-022: Secret Detection (3d) [depends on QA-002]
└── Buffer: 2d
    Total: 20 engineering days (2 engineers x 10 days)

Sprint 3 (Weeks 5-6): Quality Gates & Compatibility
├── QA-007: Quality Gate CI Pipeline (5d) [depends on QA-004, QA-005, QA-006]
├── QA-013: OpenAPI Compatibility Matrix (6d) [depends on QA-001]
├── QA-025: Dependency Vulnerability Scanning (2d) [depends on QA-007]
├── QA-021: Fuzz All Input Parsers (begins, 3d of 6d) [depends on QA-001]
└── Buffer: 2d
    Total: 20 engineering days (2 engineers x 10 days)

Sprint 4 (Weeks 7-8): Benchmarks & Advanced Testing
├── QA-009: Criterion Benchmark Suite (begins, 5d of 8d) [depends on QA-001, QA-002]
├── QA-021: Fuzz All Input Parsers (completes, 3d) [continued]
├── QA-003: Docker PostgreSQL (3d) [depends on QA-002]
├── QA-016: OS Compatibility (4d) [depends on QA-004]
├── QA-019: Resource Exhaustion (4d) [depends on QA-001, QA-002]
└── Buffer: 1d
    Total: 20 engineering days (2 engineers x 10 days)

Sprint 5 (Weeks 9-10): Benchmarks & Degradation
├── QA-009: Criterion Benchmark Suite (completes, 3d) [continued]
├── QA-010: Memory Profiling (3d) [depends on QA-009]
├── QA-011: Benchmark CI Integration (4d) [depends on QA-009, QA-004]
├── QA-014: Protobuf & GraphQL Compatibility (5d) [depends on QA-001]
├── QA-020: Graceful Degradation (3d) [depends on QA-002]
├── QA-024: OWASP Test Data Labeling (2d)
└── Buffer: 0d
    Total: 20 engineering days (2 engineers x 10 days)

Sprint 6 (Weeks 11-12): Release Readiness
├── QA-012: Competitive Benchmark Suite (5d) [depends on QA-009, QA-001]
├── QA-015: CI Platform Compatibility (5d) [depends on QA-007]
├── QA-027: Release Candidate Process (3d) [depends on QA-007, QA-009, QA-026]
├── QA-028: Changelog Generation (2d) [depends on QA-026]
├── QA-029: Pre-Release Quality Checklist (2d) [depends on QA-027]
├── QA-030: Post-Release Monitoring (2d) [depends on QA-029]
├── QA-032: Dogfooding Own CI (2d) [depends on QA-001]
├── QA-033: Dogfooding Engineer Workflow (1d)
└── Buffer: 0d
    Total: 20 engineering days (2 engineers x 10 days, tight)

Post-Launch (Ongoing):
├── QA-031: Dogfooding Cloud API (3d, when Cloud is ready)
├── Maintenance: 20% of engineering time on QA infra
└── Continuous improvement based on dogfooding and user feedback
```

### Critical Path

The critical path determines the minimum time to reach release readiness:

```
QA-001 (3d) → QA-002 (5d) → QA-004 (4d) → QA-007 (5d) → QA-027 (3d) → QA-029 (2d)
   Day 0        Day 3         Day 8         Day 12         Day 17        Day 20

Total critical path: 22 engineering days (~4.5 weeks with 1 engineer)
With 2 engineers and parallelism: ~3 weeks to release readiness (quality gates operational)
Full suite (all 33 tasks): ~12 weeks with 2 engineers
```

**Critical path bottleneck:** QA-002 (Mock API Server). This blocks most integration testing work. Prioritize this task above all others.

---

## 12. Quality Metrics Dashboard Design

### Dashboard Layout

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    VALIFORGE QUALITY DASHBOARD                               │
│                    Last updated: 2026-03-13 14:30 UTC                        │
├─────────────────────┬───────────────────────┬───────────────────────────────┤
│  BUILD HEALTH       │  COVERAGE             │  PERFORMANCE                   │
│                     │                       │                                │
│  CI Status: ✓ PASS  │  Workspace: 87.2%     │  100-ep validation: 3.8s      │
│  Last failure: 4d   │  ┌──────────────┐     │  Schema parse (med): 42ms     │
│  Flaky tests: 0     │  │ ████████▒▒   │ 87% │  Cold start: 78ms             │
│  Test count: 2,147  │  └──────────────┘     │  Peak RSS: 38MB               │
│  Pass rate: 100%    │                       │  Binary size: 11.2MB          │
│                     │  Per-Crate:           │                                │
│  ┌───────────────┐  │  schema:  96%  ██████ │  ┌──────────────────────────┐ │
│  │ 30d trend: ─  │  │  core:    91%  █████▒ │  │ Benchmark trend (30d)    │ │
│  │ ──────────    │  │  diff:    92%  █████▒ │  │ ┌─────────────────────┐  │ │
│  │  (flat=good)  │  │  datagen: 87%  ████▒▒ │  │ │                     │  │ │
│  └───────────────┘  │  report:  88%  ████▒▒ │  │ │  ──────────────     │  │ │
│                     │  cli:     82%  ████▒  │  │ │  (flat or down=good)│  │ │
│                     │  plugin:  86%  ████▒▒ │  │ └─────────────────────┘  │ │
│                     │  sdk:     85%  ████▒  │  └──────────────────────────┘ │
├─────────────────────┼───────────────────────┼───────────────────────────────┤
│  SECURITY           │  COMPATIBILITY        │  RELEASE STATUS                │
│                     │                       │                                │
│  Vulnerabilities: 0 │  OpenAPI 3.0.x: PASS  │  Current: v0.4.2              │
│  Unsafe blocks: 0   │  OpenAPI 3.1.0: PASS  │  RC: v0.5.0-rc.1             │
│  Audit status: OK   │  Protobuf: PASS       │  Soak period: 48h / 72h      │
│  License: OK        │  GraphQL: PASS        │                                │
│  Last fuzz: 2d ago  │                       │  Checklist: 18/22 complete    │
│  Fuzz crashes: 0    │  OS Matrix:           │  ████████████████▒▒▒▒ 82%    │
│                     │  Linux x86: PASS      │                                │
│  Secret redaction:  │  macOS ARM: PASS      │  Download counts (v0.4.2):    │
│  Tests: 12/12 PASS  │  Windows: PASS        │  Linux:  2,341                │
│                     │  Alpine: PASS         │  macOS:  1,892                │
│                     │                       │  Windows: 734                  │
├─────────────────────┴───────────────────────┴───────────────────────────────┤
│  RECENT QUALITY EVENTS                                                       │
│                                                                              │
│  [2026-03-12] Benchmark regression detected: schema_parse/github +8.2%      │
│  [2026-03-11] Coverage improved: valiforge-datagen 85% → 87%                │
│  [2026-03-10] Fuzz run complete: 10 targets x 10 min, 0 crashes             │
│  [2026-03-09] Release v0.4.2 published. 72h soak: no issues reported.       │
│  [2026-03-08] Dependency update: reqwest 0.12.4 → 0.12.5 (security fix)    │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Metrics Definitions

| Metric | Source | Update Frequency | Alert Threshold |
|--------|--------|-----------------|-----------------|
| CI pass rate | GitHub Actions API | Per-commit | < 95% over 7 days |
| Flaky test count | nextest retry data | Per-CI-run | > 0 |
| Workspace coverage | cargo-llvm-cov JSON | Per-PR | < 85% |
| Per-crate coverage | cargo-llvm-cov JSON | Per-PR | Below crate threshold (Section 1.1) |
| 100-endpoint validation time | criterion JSON | Per-PR | > 5.5s (10% above 5s target) |
| Cold start time | criterion JSON | Per-PR | > 110ms |
| Peak RSS | DHAT JSON | Nightly | > 55MB |
| Binary size | `ls -la` in CI | Per-PR | > 16.5MB (10% above 15MB target) |
| Vulnerability count | cargo audit JSON | Per-PR + nightly | > 0 (any severity) |
| Unsafe block count | cargo geiger JSON | Per-PR | Any increase |
| Fuzz crash count | cargo-fuzz output | Nightly | > 0 |
| Test count | nextest output | Per-PR | Decrease (tests should only increase) |
| Release soak issues | GitHub Issues API | Post-release | > 5 issues in 72h |

### Implementation

- Dashboard data collected by CI and pushed to a JSON file in a `gh-pages` branch (or S3 bucket).
- Static HTML dashboard generated by a script in `tools/dashboard/`.
- Hosted at `quality.valiforge.dev` (GitHub Pages or Cloudflare Pages).
- Team Slack/Discord webhook for alert notifications.

---

## 13. Key Risks

### Risk Registry

| Risk ID | Risk | Probability | Impact | Mitigation | Owner |
|---------|------|-------------|--------|------------|-------|
| R-001 | **CI time exceeds 15 minutes** as test suite grows | High | Medium | Aggressive caching, nextest parallelism, split slow tests into nightly job | QA Lead |
| R-002 | **Flaky tests** from network-dependent integration tests | Medium | High | Zero network dependency in tests (wiremock-rs only); nextest flaky detection | QA Lead |
| R-003 | **Benchmark noise** on shared CI runners causes false regressions | High | Medium | Dedicated benchmark runner; 3-run median; "two consecutive regressions" rule | Infra |
| R-004 | **Coverage targets unreachable** for SLM-dependent code paths | Medium | Low | Mock SLM inference; measure coverage excluding SLM integration (separate report) | QA Lead |
| R-005 | **Fuzz testing discovers deep parser bugs** requiring architectural changes | Medium | High | Invest in parser robustness early; use `nom` or `winnow` for parser combinators (structured error handling) | Tech Lead |
| R-006 | **Cross-platform CI matrix is expensive** (GitHub Actions minutes) | High | Medium | Run full matrix only on `main` merges; PR checks run Linux-only; use self-hosted ARM runner for macOS | Infra |
| R-007 | **Test fixture specs become stale** as real APIs evolve | Medium | Low | Pin spec versions; update fixtures quarterly; do not download at test time | QA Lead |
| R-008 | **Engineers resist quality gates** ("too slow", "too strict") | Medium | High | Make gates fast (<15 min total); provide good error messages on failure; demonstrate value (bugs caught) | QA Lead + Eng Manager |
| R-009 | **Snapshot tests become a maintenance burden** with frequent output changes | Medium | Medium | Keep snapshots focused (error messages, reports); avoid snapshotting internal data structures | QA Lead |
| R-10 | **Competitive benchmarks become adversarial** if competitors notice | Low | Medium | Be factual, reproducible, and fair. Publish methodology. Invite competitors to verify. | CEO |

### Risk Mitigation Priority

1. **R-002 (Flaky tests):** This is the single greatest threat to developer trust in the test suite. Address first.
2. **R-001 (CI time):** Fast CI is essential for developer productivity. Optimize continuously.
3. **R-008 (Engineer resistance):** Cultural. Lead by example; demonstrate value.
4. **R-005 (Parser bugs from fuzzing):** Invest early to avoid late-stage rewrites.
5. All others: manage through ongoing monitoring.

---

## Appendix A: Tool Versions (Pinned)

| Tool | Version | Purpose |
|------|---------|---------|
| `cargo-nextest` | >= 0.9.68 | Parallel test runner |
| `cargo-llvm-cov` | >= 0.6.0 | Coverage reporting |
| `cargo-audit` | >= 0.20.0 | Vulnerability scanning |
| `cargo-deny` | >= 0.14.0 | License/dependency policy |
| `cargo-fuzz` | >= 0.12.0 | Fuzz testing |
| `cargo-geiger` | >= 0.11.0 | Unsafe code detection |
| `cargo-semver-checks` | >= 0.31.0 | Semver compliance |
| `criterion` | 0.5.x | Benchmarking |
| `proptest` | 1.x | Property-based testing |
| `insta` | 1.x | Snapshot testing |
| `wiremock` | 0.6.x | Mock HTTP server |
| `testcontainers` | 0.17.x | Docker test containers |
| `assert_cmd` | 2.x | CLI testing |
| `predicates` | 3.x | CLI assertion matchers |
| `mockall` | 0.12.x | Mock generation |
| `git-cliff` | >= 2.0 | Changelog generation |
| `dhat-rs` | 0.3.x | Heap profiling |

---

## Appendix B: Directory Structure for Test Infrastructure

```
valiforge/
├── Cargo.toml                          # Workspace root
├── deny.toml                           # cargo-deny configuration
├── cliff.toml                          # git-cliff configuration
├── .config/
│   └── nextest.toml                    # nextest configuration
├── .github/
│   └── workflows/
│       ├── quality-gates.yml           # PR quality gates (13 checks)
│       ├── benchmarks.yml              # Benchmark regression detection
│       ├── nightly.yml                 # Nightly: fuzz, full compat matrix, memory profiling
│       └── release.yml                 # Release workflow (RC process)
├── benches/
│   ├── schema_parse.rs                 # BENCH-002 through BENCH-005
│   ├── validation.rs                   # BENCH-006, BENCH-007
│   ├── diff.rs                         # BENCH-008, BENCH-009
│   ├── datagen.rs                      # BENCH-010, BENCH-011
│   ├── reporting.rs                    # BENCH-012 through BENCH-014
│   ├── memory.rs                       # BENCH-015
│   ├── cold_start.rs                   # BENCH-001
│   ├── baselines/                      # Stored criterion baselines
│   ├── competitive/                    # Competitive benchmark scripts
│   │   ├── Dockerfile.schemathesis
│   │   ├── Dockerfile.karate
│   │   ├── Dockerfile.postman
│   │   ├── run_comparison.sh
│   │   └── results/
│   └── compare.sh                      # Baseline comparison script
├── fuzz/
│   ├── Cargo.toml
│   ├── fuzz_targets/
│   │   ├── fuzz_openapi_yaml.rs        # FUZZ-001
│   │   ├── fuzz_openapi_json.rs        # FUZZ-002
│   │   ├── fuzz_protobuf.rs            # FUZZ-003
│   │   ├── fuzz_graphql_sdl.rs         # FUZZ-004
│   │   ├── fuzz_toml_config.rs         # FUZZ-005
│   │   ├── fuzz_diff_engine.rs         # FUZZ-006
│   │   ├── fuzz_datagen.rs             # FUZZ-007
│   │   ├── fuzz_junit_parser.rs        # FUZZ-008
│   │   ├── fuzz_large_schema.rs        # FUZZ-009
│   │   └── fuzz_wasm_loader.rs         # FUZZ-010
│   └── corpus/                         # Seed corpus per target
│       ├── fuzz_openapi_yaml/
│       ├── fuzz_openapi_json/
│       └── ...
├── schemas/
│   └── fixtures/
│       ├── README.md                   # Fixture catalog
│       ├── openapi/
│       │   ├── petstore-3.0.3.yaml
│       │   ├── petstore-3.1.0.yaml
│       │   ├── stripe-2026-02-15.yaml
│       │   ├── github-api-3.0.3.json
│       │   ├── kubernetes-1.29.yaml
│       │   ├── twilio-3.0.1.yaml
│       │   ├── slack-3.0.0.yaml
│       │   ├── circular-refs.yaml
│       │   ├── allof-oneof-anyof.yaml
│       │   ├── callbacks-webhooks.yaml
│       │   ├── discriminators.yaml
│       │   └── malformed-partial.yaml
│       ├── protobuf/
│       │   ├── proto2_basic.proto
│       │   ├── proto3_basic.proto
│       │   ├── well_known_types.proto
│       │   ├── streaming_rpcs.proto
│       │   └── nested_complex.proto
│       ├── graphql/
│       │   ├── basic_sdl.graphql
│       │   ├── federation.graphql
│       │   ├── introspection.json
│       │   └── complex_types.graphql
│       └── migrations/
│           └── 001_test_schema.sql
├── crates/
│   ├── valiforge-test-utils/           # Private crate: shared test helpers
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── builders.rs             # Test data builders
│   │       ├── strategies.rs           # proptest strategies
│   │       ├── mock_server.rs          # wiremock-rs helpers
│   │       └── assertions.rs           # Custom assertion macros
│   ├── valiforge-cli/
│   │   └── tests/                      # E2E tests (assert_cmd)
│   ├── valiforge-core/
│   │   └── tests/                      # Integration tests
│   ├── valiforge-schema/
│   │   └── tests/                      # Compatibility matrix tests
│   └── ...
├── examples/
│   └── ci/
│       ├── github-actions.yml
│       ├── gitlab-ci.yml
│       ├── Jenkinsfile
│       ├── circleci-config.yml
│       └── buildkite-pipeline.yml
├── tools/
│   └── dashboard/
│       ├── generate.py                 # Dashboard HTML generator
│       └── template.html               # Dashboard template
├── UNSAFE_AUDIT.md                     # Unsafe code audit log
├── DEPENDENCY_POLICY.md                # Dependency policy document
└── TESTING_STRATEGY.md                 # This document
```

---

## Appendix C: Key Decisions Log

| Decision | Choice | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Coverage tool | `cargo-llvm-cov` | `cargo-tarpaulin` | More accurate on complex Rust code; supports branch coverage; better performance |
| Test runner | `cargo-nextest` | `cargo test` | Process-per-test isolation; 2-4x faster; built-in flaky detection; JUnit output |
| Benchmark framework | `criterion` | `divan`, `iai` | Mature, statistical rigor, HTML reports, baseline comparison |
| Snapshot testing | `insta` | `expect-test`, manual `assert_eq!` | Interactive review workflow; multiple serialization formats; CI-friendly |
| Property testing | `proptest` | `quickcheck` | Better shrinking; composable strategies; more Rustic API |
| Fuzzing | `cargo-fuzz` (libFuzzer) | `afl.rs`, `honggfuzz` | Best ecosystem support; coverage-guided; integrated with OSS-Fuzz |
| Mock HTTP | `wiremock` | `mockito`, `httpmock` | Request matching; verification; dynamic stubs; good Tokio integration |
| Error display | `miette` | `ariadne`, `codespan-reporting` | Excellent CLI UX; line annotations; suggestion support; Rust ecosystem standard |
| Changelog | `git-cliff` | `conventional-changelog`, `release-please` | Rust-native; highly configurable; conventional commit parsing |

---

*This testing strategy is a living document. It will be updated as ValiForge evolves, new testing techniques emerge, and we learn from real-world usage. The overarching principle: ValiForge is a testing tool -- our own tests must be exemplary.*

*"A testing tool with bad tests is like a doctor who smokes."*
