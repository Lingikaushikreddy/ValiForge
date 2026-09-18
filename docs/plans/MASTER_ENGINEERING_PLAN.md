# ValiForge — Master Engineering Plan

### End-to-End Development Task Assignment & Best Practices

**Version:** 1.0
**Date:** March 13, 2026
**Author:** Kaushik Reddy, CEO & Founder
**Status:** Ready for Sprint Planning

---

## Executive Summary

This master plan consolidates **6 parallel engineering workstreams** into a unified development roadmap for ValiForge — a headless, Rust-powered API validation engine for AI-generated workloads.

### At a Glance

| Workstream | Engineer Role | Tasks | Effort (days) | Critical Path | Detailed Plan |
|------------|--------------|-------|---------------|---------------|---------------|
| **Rust Core Engine** | Lead Rust Engineer | 35 | 88.5 days | 29 days (6 weeks) | `ENGINEERING_PLAN.md` |
| **AI/ML Test Data Gen** | AI/ML Engineer | 34 | 95 days | 19 days (4 weeks) | `VALIFORGE_DATAGEN_ENGINEERING_PLAN.md` |
| **DevOps / Infrastructure** | Platform Engineer | 40 | 77 days | ~20 days (4 weeks) | `INFRASTRUCTURE_PLAN.md` |
| **Cloud Dashboard & Web** | Frontend/Cloud Engineer | ~45 | 220 days | 44 days (9 weeks) | `VALIFORGE_CLOUD_ENGINEERING_PLAN.md` |
| **DX / DevRel / Community** | Developer Advocate | 127 | 412 days | 69 days (14 weeks) | `VALIFORGE_DX_TASK_BREAKDOWN.md` |
| **QA / Testing Strategy** | QA Lead | 33 | 111 days | ~25 days (5 weeks) | `TESTING_STRATEGY.md` |
| **GRAND TOTAL** | **6 roles** | **~314 tasks** | **~1,003.5 eng-days** | — | **~12,000 lines of specs** |

### What This Means

- **Solo founder:** ~4 years of work (not recommended)
- **Team of 4 (Phase 1 focus):** MVP in **14 weeks** (3.5 months)
- **Team of 8 (full parallel):** Full v1.0 in **6 months**, Cloud launch in **9 months**
- **Recommended:** Start with 2-3 engineers for MVP, scale to 8-10 post-seed

---

## Team Structure & Role Assignments

### Phase 1: Founding Team (Months 1-4) — MVP

| Role | Person | Primary Workstreams | Focus |
|------|--------|-------------------|-------|
| **CEO / Founder** | Kaushik Reddy | DX, GTM, Community | Vision, product decisions, investor relations, launch execution |
| **CTO / Co-Founder** (Hire #1) | TBD | Core Engine, Architecture | Rust workspace, schema parsers, validation engine, CLI |
| **Senior Rust Engineer** (Hire #2) | TBD | Core Engine, DataGen | Diff engine, report generation, property-based test data gen |
| **DevOps Engineer** (Hire #3 or contract) | TBD | Infrastructure, CI/CD | Build pipeline, cross-platform distribution, GitHub Actions |

### Phase 2: Post-Seed Team (Months 5-10) — Cloud Launch

| Role | Person | Primary Workstreams |
|------|--------|-------------------|
| All Phase 1 roles | — | Continue |
| **AI/ML Engineer** | TBD | SLM integration, Phi-4-mini, inference optimization |
| **Frontend Engineer** | TBD | Cloud dashboard, marketing site, docs |
| **Developer Advocate** | TBD | Content, community, conference talks, launch campaigns |
| **QA Engineer** | TBD | Testing framework, benchmarks, compatibility matrix |

---

## Engineer-Specific Task Assignments

### Engineer 1: CTO / Lead Rust Engineer

**Ownership:** Core Engine + Schema Parsers + CLI
**Key deliverables:** The binary that developers install and run.

#### Sprint-by-Sprint Assignment

| Sprint | Weeks | Tasks | Goal |
|--------|-------|-------|------|
| **Sprint 1** | 1-2 | SETUP-001 (Workspace), SETUP-002 (MSRV), SETUP-003 (Feature flags), SETUP-004 (CI), SCHEMA-002 start (OpenAPI parser) | Workspace compiles, CI green |
| **Sprint 2** | 3-4 | SCHEMA-002 finish, SCHEMA-005 (JSON Schema), SCHEMA-006 (Test infra), CLI-001 start | OpenAPI parsing complete |
| **Sprint 3** | 5-6 | CORE-001 (Traits), CORE-002 (HTTP executor), CORE-003 (Response validation) | Can validate endpoints against live server |
| **Sprint 4** | 7-8 | CORE-004 (Concurrency), CORE-005 (Retry), CLI-002 (Config loading) | Full concurrent validation pipeline |
| **Sprint 5** | 9-10 | REPORT-001 (Data model), REPORT-002 (JUnit), REPORT-003 (SARIF), REPORT-004 (JSON/MD/TAP) | All output formats working |
| **Sprint 6** | 11-12 | PLUGIN-001 (WASM runtime), PLUGIN-002 (Plugin mgmt) | Plugin system operational |
| **Sprint 7** | 13-14 | SDK-001, hardening, perf optimization, beta release prep | **MVP READY** |

**Total: 35 tasks, 88.5 engineering-days across 14 weeks**

**Best Practices for This Role:**
- Use `thiserror` for library crates (typed errors), `anyhow` only in CLI binary crate
- Every public API gets `#[must_use]` where appropriate
- Zero `unsafe` code policy — if needed, isolate in a dedicated module with `// SAFETY:` comments
- All async code uses `tokio` — no mixing runtimes
- Feature-gate heavy dependencies (grpc, graphql, wasm-plugins) from Sprint 1
- Benchmark with `criterion` from Sprint 3 — never ship without perf baselines
- Use `insta` snapshot tests for CLI output and report formats
- Test against real-world schemas: Petstore, Stripe, GitHub API, Kubernetes

---

### Engineer 2: Senior Rust Engineer

**Ownership:** Diff Engine + Data Generation + Protocols
**Key deliverables:** Breaking change detection + test data generation.

#### Sprint-by-Sprint Assignment

| Sprint | Weeks | Tasks | Goal |
|--------|-------|-------|------|
| **Sprint 1** | 1-2 | SETUP-005 (Error/logging), SETUP-006 (xtask), SCHEMA-001 (Core schema IR) | Error infra, schema abstraction layer |
| **Sprint 2** | 3-4 | SCHEMA-003 (Protobuf parser), SCHEMA-004 (GraphQL parser) | All 3 protocol parsers functional |
| **Sprint 3** | 5-6 | DIFF-001 (Core diff algorithm), DIFF-002 (OpenAPI breaking rules) | OpenAPI diff working |
| **Sprint 4** | 7-8 | DIFF-003 (Protobuf diff), DIFF-004 (GraphQL diff), DIFF-005 (Output formatting) | All protocol diffs complete |
| **Sprint 5** | 9-10 | DATAGEN-001 (Schema parser for gen), DATAGEN-002 (Type generators), DATAGEN-003 (Format generators), DATAGEN-004 (Constraint generators) | Grammar-based data gen working |
| **Sprint 6** | 11-12 | DATAGEN-006 (Composite gen), DATAGEN-007 (Seeding), CORE-007 (Perf baselines), SDK-001 assist | Deterministic data gen, perf tracking |
| **Sprint 7** | 13-14 | DATAGEN-008 to -013 (Negative test gen), integration testing, beta prep | **Negative testing complete, MVP READY** |

**Total: ~35 tasks, ~90 engineering-days across 14 weeks**

**Best Practices for This Role:**
- Diff algorithm: use `similar` crate patterns — compute minimal edit distance between schema versions
- Breaking change severity: follow oasdiff's 300+ rules as reference, implement top 50 first
- Data generation: `proptest` strategies compose — build small generators, combine them
- Use the `fake` crate for realistic names, emails, addresses, phone numbers
- Negative test gen: categorize every payload (type_error, constraint_violation, security_probe, malformed)
- Seed all random generators with configurable seed (default: 42) for deterministic CI
- Test schema diff against real API evolution (GitHub API v3→v4, Stripe changelog)

---

### Engineer 3: DevOps / Platform Engineer

**Ownership:** CI/CD + Distribution + Infrastructure
**Key deliverables:** Everything from `git push` to binary on user's machine.

#### Sprint-by-Sprint Assignment

| Sprint | Weeks | Tasks | Goal |
|--------|-------|-------|------|
| **Sprint 1** | 1-2 | INFRA-001 (Repo structure), INFRA-002 (Branch strategy), INFRA-003 (CI pipeline), INFRA-004 (PR workflow) | CI/CD pipeline operational |
| **Sprint 2** | 3-4 | INFRA-005 (Cross-platform builds), INFRA-006 (Static linking), INFRA-007 (Binary optimization), INFRA-008 (Release automation) | Binaries building for all platforms |
| **Sprint 3** | 5-6 | INFRA-009 (GitHub Releases), INFRA-010 (cargo publish), INFRA-011 (cargo-binstall), INFRA-012 (Homebrew tap) | 4 distribution channels live |
| **Sprint 4** | 7-8 | INFRA-013 (npm wrapper), INFRA-014 (Docker), INFRA-015 (Shell installer), INFRA-016 (GitHub Actions marketplace) | All distribution channels live |
| **Sprint 5** | 9-10 | INFRA-017 (GitLab template), INFRA-018 (Security audit CI), INFRA-019 (SBOM), INFRA-020 (Performance benchmarks in CI) | Security & perf automation |
| **Sprint 6** | 11-12 | INFRA-021 (Telemetry), INFRA-022 (Binary size tracking), launch infra prep | Telemetry + monitoring ready |
| **Sprint 7** | 13-14 | Launch support, CDN setup, install script testing, load testing | **Infrastructure launch-ready** |

**Total: 40 tasks, 77 engineering-days across 14 weeks**

**Best Practices for This Role:**
- Use `cross` crate for cross-compilation — don't fight native toolchains
- Static link with musl for Linux: `RUSTFLAGS="-C target-feature=+crt-static"` + `x86_64-unknown-linux-musl`
- Binary optimization: `lto = true`, `codegen-units = 1`, `strip = true` in release profile → ~10-15MB binary
- GitHub Actions: cache `~/.cargo/registry` and `target/` — saves 2-5 min per CI run
- Use `cargo-dist` for automated release workflows (handles checksums, manifests, Homebrew formula)
- Docker: multi-stage build, `FROM scratch` final stage, `COPY --from=builder /app/valiforge /valiforge`
- npm wrapper: follow esbuild's pattern — `postinstall` script downloads platform binary
- Shell installer: test in CI with `shellcheck` and across Ubuntu, Debian, Alpine, macOS, WSL
- Never require `sudo` for installation — install to `~/.valiforge/bin/`
- `cargo audit` on every PR + nightly cron job for CVE detection
- `cargo deny` for license compliance — reject GPL dependencies in Apache 2.0 project

---

### Engineer 4: AI/ML Engineer (Phase 2, Month 5+)

**Ownership:** SLM Integration + Intelligent Data Generation
**Key deliverables:** AI-powered test data that's smarter than grammar-based.

#### Sprint-by-Sprint Assignment

| Sprint | Weeks | Tasks | Goal |
|--------|-------|-------|------|
| **Sprint 1** | 1-2 | DATAGEN-014 (Inference abstraction trait), DATAGEN-015 (Model management), DATAGEN-018 (Resource manager) | SLM backend compiles, loads model |
| **Sprint 2** | 3-4 | DATAGEN-016 (Prompt engineering), DATAGEN-017 (Output parser), DATAGEN-019 (Batch generation) | Generate valid JSON from SLM |
| **Sprint 3** | 5-6 | DATAGEN-020 (Fallback logic), DATAGEN-021 (Domain detection), DATAGEN-022 (Domain-aware gen) | Smart domain-specific data |
| **Sprint 4** | 7-8 | DATAGEN-024 (Quantization tuning), DATAGEN-025 (Memory optimization), DATAGEN-026 (CPU threading) | Perf-optimized inference |
| **Sprint 5** | 9-10 | DATAGEN-027 (GPU acceleration), DATAGEN-028 (Lazy loading), DATAGEN-023 (Locale-aware gen) | GPU path + internationalization |
| **Sprint 6** | 11-12 | DATAGEN-029-034 (Testing suite), benchmarks, documentation | **SLM integration complete** |

**Total: 34 tasks, 95 engineering-days across 12 weeks**

**Best Practices for This Role:**
- **Model selection:** Phi-4-mini (3.8B, Q4_K_M) as default — 83.7% ARC-C, runs on 3GB RAM
- **CI mode:** TinyLlama (1.1B, Q4) for resource-constrained runners — ~1GB RAM
- **Inference runtime:** `llama-cpp-2` crate as primary (best GGUF support), `ort` as ONNX alternative
- **Prompt design:** System prompt forces JSON output matching the schema; few-shot examples from parsed schema
- **Temperature:** 0.7 for diverse data, 0.3 for high-validity data — make configurable
- **Fallback chain:** SLM → grammar-based → proptest → fail with helpful error
- **Model caching:** Store in `~/.valiforge/models/`, check SHA-256 on download, support `VALIFORGE_MODEL_DIR` env var
- **Feature flag:** Entire SLM subsystem behind `--features slm` — binary without SLM is <15MB
- **NEVER** send user data to external services — all inference runs locally
- Benchmark: target <2s per generation on Apple M-series, <5s on x86 CPU

---

### Engineer 5: Frontend / Cloud Product Engineer (Phase 2, Month 6+)

**Ownership:** ValiForge Cloud Dashboard + Marketing Site + Docs
**Key deliverables:** The web products that monetize the CLI.

#### Sprint-by-Sprint Assignment

| Sprint | Weeks | Tasks | Goal |
|--------|-------|-------|------|
| **Sprint 1-2** | 1-4 | CLOUD-001 (Design system), CLOUD-002 (Marketing site), CLOUD-003 (Auth), CLOUD-004 (Landing page) | Marketing site live, auth working |
| **Sprint 3-4** | 5-8 | CLOUD-005 (Docs site), CLOUD-006 (Dashboard overview), CLOUD-007 (Project detail), CLOUD-008 (Run detail) | Core dashboard pages functional |
| **Sprint 5-6** | 9-12 | CLOUD-009 (Schema explorer), CLOUD-010 (Team mgmt), CLOUD-011 (Settings), CLOUD-012 (PR bot) | Team features + GitHub integration |
| **Sprint 7-8** | 13-16 | CLOUD-013 (Billing/Stripe), CLOUD-014 (Usage dashboard), CLOUD-015 (API server), CLOUD-016 (WebSocket) | Billing live, real-time updates |
| **Sprint 9-10** | 17-20 | CLOUD-017 (SAML SSO), CLOUD-018 (RBAC), CLOUD-019 (Audit logs), CLOUD-020 (Interactive playground) | Enterprise features |
| **Sprint 11-12** | 21-24 | CLOUD-021 (On-prem Helm chart), polish, perf optimization | **Cloud GA ready** |

**Total: ~45 tasks, 220 engineering-days across 24 weeks**

**Best Practices for This Role:**
- **Stack recommendation:** Next.js 15 (App Router) + Tailwind CSS + shadcn/ui + Vercel deployment
- **Docs framework:** Starlight (Astro-based) — excellent for Rust tool documentation, fast, searchable
- **Auth:** NextAuth.js with GitHub OAuth as primary, then Google, then email/password
- **Billing:** Stripe Billing + Stripe Elements for checkout, usage-based metering via Stripe Meter API
- **Real-time:** Server-Sent Events (SSE) for validation run status (simpler than WebSocket for unidirectional)
- **Performance:** Lighthouse 95+ on marketing site, <2s TTFB on dashboard pages
- **Interactive playground:** Compile ValiForge core to WASM, run validation in the browser
- **Dark mode:** Support from day 1 — developers expect it
- **API server:** Rust (Axum) for performance, or TypeScript (Hono) for velocity — choose based on team strength

---

### Engineer 6: QA Lead / Testing Strategist (Phase 2, Month 5+)

**Ownership:** Testing Framework + Quality Gates + Benchmarks + Release Process
**Key deliverables:** Confidence that ValiForge works correctly everywhere.

#### Sprint-by-Sprint Assignment

| Sprint | Weeks | Tasks | Goal |
|--------|-------|-------|------|
| **Sprint 1** | 1-2 | QA-001 (Test pyramid design), QA-002 (Test fixture collection), QA-003 (Mock server setup) | Testing foundation |
| **Sprint 2** | 3-4 | QA-004 (Unit test framework), QA-005 (Integration test suite), QA-006 (E2E CLI tests), QA-007 (Property-based tests) | Core test suites running |
| **Sprint 3** | 5-6 | QA-008 (Quality gates), QA-009 (Coverage thresholds), QA-010 (Perf regression detection), QA-011 (Compatibility matrix) | Quality gates enforced in CI |
| **Sprint 4** | 7-8 | QA-012 (Fuzzing), QA-013 (Security testing), QA-014 (Error handling tests), QA-015 (Network failure scenarios) | Security + resilience testing |
| **Sprint 5** | 9-10 | QA-016 (Benchmark framework), QA-017 (Competitor benchmarks), QA-018 (Snapshot tests), QA-019 (OS compatibility) | Benchmarks + cross-platform |
| **Sprint 6** | 11-12 | QA-020 (Release process), QA-021 (Changelog automation), QA-022 (Dogfooding), QA-023-033 remaining | **Release process formalized** |

**Total: 33 tasks, 111 engineering-days across 12 weeks**

**Best Practices for This Role:**
- **Test pyramid:** 60% unit, 25% integration, 10% E2E, 5% fuzzing/property
- **Mock server:** `wiremock-rs` for HTTP mocking — async, isolated per test, extensible matchers
- **E2E testing:** `assert_cmd` + `predicates` crates for CLI invocation testing
- **Snapshot testing:** `insta` crate — review workflow for CLI output changes
- **Property testing:** `proptest` — "if we generate a valid schema, parsing and re-serializing produces equivalent output"
- **Fuzzing:** `cargo-fuzz` on all input parsers (OpenAPI YAML, Protobuf, GraphQL SDL, TOML config)
- **Coverage:** `cargo-llvm-cov` (more accurate than tarpaulin), target ≥80% line coverage
- **Test runner:** `cargo-nextest` for faster parallel execution + better output
- **Quality gates (block PR merge):**
  - `cargo fmt --check`
  - `cargo clippy -- -D warnings`
  - `cargo test --workspace`
  - `cargo audit`
  - `cargo deny check`
  - Coverage ≥80%
  - Benchmark within 10% of baseline
  - Binary size within 20% of baseline
- **Real-world fixtures:** Petstore, Stripe API, GitHub API, Kubernetes OpenAPI, Twilio, Slack
- **Dogfooding:** Use ValiForge to validate ValiForge Cloud's own API

---

### CEO / Founder: Kaushik Reddy

**Ownership:** DX, Community, GTM, Fundraising
**Key deliverables:** Users, community, revenue, funding.

#### Sprint-by-Sprint Assignment

| Sprint | Weeks | Focus | Key Actions |
|--------|-------|-------|-------------|
| **Sprint 1** | 1-2 | Hiring + Fundraising | Recruit CTO/co-founder, begin pre-seed conversations, set up Discord |
| **Sprint 2** | 3-4 | Community Foundation | README polish, CONTRIBUTING.md, "Good First Issue" seeding, begin blog |
| **Sprint 3** | 5-6 | Content + Beta Prep | Write "Why We Built ValiForge in Rust", recruit 20-50 beta testers |
| **Sprint 4** | 7-8 | Beta Launch | Private beta with design partners, collect feedback + testimonials |
| **Sprint 5** | 9-10 | Launch Prep | Prepare Show HN, Product Hunt, social assets, "Postman Migration Guide" |
| **Sprint 6** | 11-12 | Pre-Launch | Seed Discord, test install flow, finalize docs, record demo video |
| **Sprint 7** | 13-14 | **LAUNCH** | Show HN (9 AM ET Tuesday), Product Hunt, Reddit, Twitter thread |
| **Sprint 8** | 15-16 | Post-Launch | Respond to every issue, iterate on feedback, blog "What We Learned" |

---

## Cross-Workstream Dependencies

```
                    ┌─────────────────────────────────────────┐
                    │           SETUP (Sprint 1)               │
                    │  Workspace, CI, Error handling           │
                    └─────────┬──────────┬────────────────────┘
                              │          │
                    ┌─────────▼──┐  ┌────▼─────────────────┐
                    │  SCHEMA     │  │  CLI                  │
                    │  Parsers    │  │  Framework            │
                    │  (Sprint 2) │  │  (Sprint 2-3)         │
                    └──┬──┬──┬───┘  └────┬─────────────────┘
                       │  │  │           │
              ┌────────▼┐ │ ┌▼────┐  ┌──▼──────────┐
              │  CORE    │ │ │DIFF │  │  REPORT      │
              │  Engine  │ │ │     │  │  Generation   │
              │ (S3-4)   │ │ │(S3-4)│ │  (Sprint 5)  │
              └────┬─────┘ │ └─────┘  └──────────────┘
                   │       │
              ┌────▼─────┐ │
              │ DATAGEN  │ │
              │ Grammar  │ │
              │ (S5-6)   │ │
              └────┬─────┘ │
                   │       │
              ┌────▼──────────▼───────┐
              │  DATAGEN SLM          │
              │  (Phase 2, Month 5+)  │
              └───────────────────────┘

    PARALLEL TRACKS (no dependency on core):
    ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
    │  INFRA       │  │  DX/DevRel   │  │  QA          │
    │  CI/CD +     │  │  Docs +      │  │  Testing +   │
    │  Distribution│  │  Community   │  │  Benchmarks  │
    │  (S1-7)      │  │  (S1-8)      │  │  (S1-6)      │
    └──────────────┘  └──────────────┘  └──────────────┘

    PHASE 2 (Month 6+):
    ┌──────────────────────────────────┐
    │  CLOUD                           │
    │  Marketing Site + Dashboard +    │
    │  Billing + Enterprise Features   │
    │  (S1-12, 24 weeks)              │
    └──────────────────────────────────┘
```

---

## Master Timeline

### Phase 1: MVP (Weeks 1-14) — Open Source Launch

```
Week  1-2   ████ Setup + Schema IR + CI Pipeline
Week  3-4   ████ Schema Parsers (OpenAPI, Proto, GraphQL) + CLI skeleton
Week  5-6   ████ Validation Engine + Diff Algorithm + Docs start
Week  7-8   ████ Concurrent Validation + All Protocol Diffs + Beta
Week  9-10  ████ Report Generation + Grammar DataGen + Launch Prep
Week 11-12  ████ WASM Plugins + SDK + Distribution Channels
Week 13-14  ████ Hardening + Beta Feedback + LAUNCH 🚀
```

**MVP includes:**
- OpenAPI 3.0/3.1 validation against live endpoints
- Breaking change detection (OpenAPI, Protobuf, GraphQL)
- Grammar-based test data generation (deterministic, no AI required)
- CLI: `init`, `validate`, `diff`, `generate`, `report`
- Output: JSON, JUnit XML, SARIF, Markdown, Terminal
- Distribution: GitHub Releases, cargo install, Homebrew, Docker, GitHub Actions
- Documentation: Quick Start, CLI Reference, CI/CD Guides

### Phase 2: Intelligence + Cloud (Weeks 15-32)

```
Week 15-18  ████ SLM Integration (Phi-4-mini) + Marketing Site
Week 19-22  ████ Cloud Dashboard + Auth + Core Pages
Week 23-26  ████ Team Features + PR Bot + GitHub Integration
Week 27-30  ████ Billing (Stripe) + Usage Metering + Enterprise SSO
Week 31-32  ████ Cloud GA Launch 🚀
```

### Phase 3: Scale (Weeks 33-52)

```
Week 33-40  ████ Enterprise features (RBAC, Audit, On-prem)
Week 41-48  ████ Plugin marketplace + IDE integrations
Week 49-52  ████ Series A prep + Category leadership
```

---

## Best Practices — Cross-Cutting Concerns

### 1. Code Quality Standards

```toml
# clippy.toml
avoid-breaking-exported-api = true
cognitive-complexity-threshold = 25
too-many-arguments-threshold = 7

# Cargo.toml [workspace.lints.clippy]
pedantic = "warn"
nursery = "warn"
unwrap_used = "deny"
expect_used = "warn"
panic = "deny"
```

- **No `.unwrap()` in library code** — use `?` operator with typed errors
- **`#[must_use]` on all fallible functions** returning `Result`
- **Documentation on all public items** — `#[warn(missing_docs)]` in library crates
- **Zero `unsafe`** unless in isolated, audited modules with `// SAFETY:` justification

### 2. Git & PR Workflow

- **Branch naming:** `feat/TASK-ID-description`, `fix/TASK-ID-description`
- **Commit format:** Conventional Commits (`feat:`, `fix:`, `docs:`, `chore:`, `perf:`, `test:`)
- **PR requirements:** CI green + 1 review + no `clippy` warnings + coverage ≥80%
- **Merge strategy:** Squash merge to `main` for clean history
- **Release:** Tag-based (`v0.1.0`), automated via `cargo-dist`
- **Changelog:** Auto-generated from conventional commits via `git-cliff`

### 3. Error Handling Philosophy

```rust
// Library crates: typed errors with thiserror
#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error("Failed to parse OpenAPI spec at {path}: {source}")]
    ParseFailed {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("Unsupported OpenAPI version: {version}. ValiForge supports 3.0.x and 3.1.x")]
    UnsupportedVersion { version: String },
}

// CLI crate: anyhow for ergonomic error handling
fn main() -> anyhow::Result<()> {
    // anyhow wraps all errors with context
    let config = load_config().context("Failed to load valiforge.toml")?;
    Ok(())
}
```

Every error message must:
1. Say **what** went wrong
2. Say **where** it went wrong (file path, line number, endpoint URL)
3. Suggest **how to fix it**
4. Link to **documentation** for complex cases

### 4. Performance Principles

- **Benchmark from Sprint 3** — never optimize without data
- **Use `criterion`** for statistical benchmarks (not wall-clock timing)
- **Profile with `cargo flamegraph`** before optimization
- **Hot path priorities:** Schema parsing → validation → report generation
- **Lazy initialization:** Don't load SLM until requested, don't parse unused protocols
- **Parallel by default:** `tokio::spawn` for each endpoint validation, `Semaphore` for concurrency limit

### 5. Security Checklist (Every Sprint)

- [ ] `cargo audit` — no known vulnerabilities
- [ ] `cargo deny check` — no GPL/copyleft dependencies (Apache 2.0 project)
- [ ] No secrets in code (API keys, tokens)
- [ ] No `unsafe` without justification
- [ ] No user data sent to external services (SLM runs locally)
- [ ] Fuzz all input parsers (schemas, configs)
- [ ] Generated test data with injection payloads is clearly labeled, never auto-executed

### 6. Documentation-First Development

- **Write the Getting Started guide before the code** (Stripe's approach)
- **CLI help text is documentation** — invest in clear `--help` output
- **Code examples tested in CI** — use `mdbook-test` or custom test harness
- **Every feature ships with:** user guide + CLI reference update + changelog entry

---

## Key Metrics to Track

### Engineering Metrics (Weekly)

| Metric | Target | Tool |
|--------|--------|------|
| CI pass rate | >95% | GitHub Actions |
| PR merge time (P50) | <24 hours | GitHub |
| Test coverage | ≥80% | cargo-llvm-cov |
| Open bugs (P0/P1) | <5 | GitHub Issues |
| Binary size | <15MB (no SLM) | CI tracking |
| P95 validation time (100 endpoints) | <5 seconds | criterion |

### Adoption Metrics (Post-Launch, Weekly)

| Metric | 1-Month Target | 3-Month Target |
|--------|---------------|----------------|
| GitHub stars | 500 | 3,000 |
| CLI installs (cumulative) | 1,000 | 10,000 |
| CI integrations (weekly active) | 50 | 500 |
| Discord members | 100 | 500 |
| Contributors | 5 | 20 |

---

## Risk Register (Top 10)

| # | Risk | Owner | Likelihood | Impact | Mitigation |
|---|------|-------|-----------|--------|------------|
| 1 | **CTO/co-founder hiring takes too long** | CEO | High | Critical | Start outreach immediately, consider Rust communities (r/rust jobs, RustConf), offer equity |
| 2 | **OpenAPI 3.1 parser edge cases** | Eng 1 | Medium | High | Use real-world schemas as test fixtures from day 1, contribute upstream to `openapiv3` |
| 3 | **SLM inference too slow for CI** | AI/ML | Medium | Medium | Grammar-based fallback is default; SLM is opt-in enhancement. TinyLlama for CI runners |
| 4 | **Launch underperforms on HN** | CEO | Medium | Medium | Don't bet everything on HN — run multi-channel launch (PH, Reddit, Twitter, Dev.to simultaneously) |
| 5 | **Postman ships competitive feature** | CEO | High | Medium | Our Rust perf + local-first + zero-dep binary are deep moats they can't replicate |
| 6 | **Binary size bloat from SLM deps** | Eng 3 | Medium | Medium | Feature-gate SLM entirely. Ship two binaries: `valiforge` (slim) and `valiforge-ai` (with SLM) |
| 7 | **Cross-platform build failures** | Eng 3 | Medium | Medium | Test cross-compilation in CI from week 1. Use `rustls` not `openssl`. Use `cross` crate |
| 8 | **Contributor onboarding friction (Rust)** | CEO | Medium | Medium | WASM plugin system allows non-Rust contributors. Excellent CONTRIBUTING.md. Mentorship in Discord |
| 9 | **Cloud dashboard scope creep** | Eng 5 | High | Medium | Ship minimal viable dashboard (overview + run detail + billing) first. No enterprise features until paying customers demand them |
| 10 | **Funding environment tightens** | CEO | Medium | High | Bootstrap-friendly model — Rust = low infra costs. Target profitability by Month 18 with $2.4M ARR |

---

## File Index

All detailed plans are in the ValiForge project directory:

| File | Lines | Content |
|------|-------|---------|
| `PRD.md` | 1,039 | Product Requirements Document — market, personas, features, pricing |
| `ENGINEERING_PLAN.md` | 1,559 | Rust Core Engine — 35 tasks, 88.5 days, 7 sprints |
| `VALIFORGE_DATAGEN_ENGINEERING_PLAN.md` | 2,017 | AI/ML Data Generation — 34 tasks, 95 days, 6 sprints |
| `INFRASTRUCTURE_PLAN.md` | 2,926 | DevOps/Infra — 40 tasks, 77 days, 7 sprints |
| `VALIFORGE_CLOUD_ENGINEERING_PLAN.md` | 1,542 | Cloud Dashboard — ~45 tasks, 220 days, 12 sprints |
| `VALIFORGE_DX_TASK_BREAKDOWN.md` | 2,015 | DX/DevRel — 127 tasks, 412 days, 8 sprints |
| `TESTING_STRATEGY.md` | 1,847 | QA/Testing — 33 tasks, 111 days, 6 sprints |
| **MASTER_ENGINEERING_PLAN.md** | **This file** | **Consolidated cross-team plan** |
| **TOTAL** | **~13,000 lines** | **Complete end-to-end engineering specification** |

---

*ValiForge: Validate APIs, not pixels.*

*This master plan is a living document. Review weekly during sprint planning.*
