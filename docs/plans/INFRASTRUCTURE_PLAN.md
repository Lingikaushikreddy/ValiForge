# ValiForge Infrastructure & DevOps Engineering Plan

**Version:** 1.0
**Date:** March 13, 2026
**Role:** DevOps / Infrastructure Engineer
**Status:** Engineering Review

---

## Table of Contents

1. [Repository & CI/CD Setup](#1-repository--cicd-setup)
2. [Cross-Platform Binary Builds](#2-cross-platform-binary-builds)
3. [Distribution Channels](#3-distribution-channels)
4. [GitHub Actions Integration](#4-github-actions-integration)
5. [ValiForge Cloud Infrastructure](#5-valiforge-cloud-infrastructure)
6. [Telemetry & Analytics](#6-telemetry--analytics)
7. [Security & Compliance](#7-security--compliance)
8. [Performance Monitoring & Benchmarks](#8-performance-monitoring--benchmarks)
9. [Effort Summary & Sprint Plan](#9-effort-summary--sprint-plan)
10. [Critical Path & Key Risks](#10-critical-path--key-risks)

---

## 1. Repository & CI/CD Setup

### INFRA-001: Initialize Cargo Workspace Monorepo

**Description:**
Set up the GitHub repository with a Cargo workspace structure matching the PRD architecture. The workspace root `Cargo.toml` defines all member crates. Directory layout:

```
valiforge/
├── .github/
│   ├── workflows/
│   │   ├── ci.yml
│   │   ├── release.yml
│   │   ├── nightly.yml
│   │   └── benchmark.yml
│   ├── CODEOWNERS
│   ├── dependabot.yml
│   ├── labeler.yml
│   └── ISSUE_TEMPLATE/
├── .cargo/
│   └── config.toml          # Cross-compilation targets, linker config
├── Cargo.toml                # [workspace] members
├── Cargo.lock                # Committed (binary project)
├── rust-toolchain.toml       # Pin Rust version (MSRV)
├── deny.toml                 # cargo-deny config
├── clippy.toml               # Clippy lint config
├── rustfmt.toml              # Formatting config
├── crates/
│   ├── valiforge-cli/
│   ├── valiforge-core/
│   ├── valiforge-schema/
│   ├── valiforge-datagen/
│   ├── valiforge-diff/
│   ├── valiforge-report/
│   ├── valiforge-plugin/
│   └── valiforge-sdk/
├── plugins/
├── schemas/                  # Test fixtures
├── docs/
├── action/                   # GitHub Action composite action
├── installers/               # Shell installer, npm wrapper
├── docker/
│   └── Dockerfile
├── LICENSE-APACHE
├── LICENSE-MIT
└── SECURITY.md
```

**Estimated Effort:** 2 days
**Dependencies:** None (first task)
**Acceptance Criteria:**
- `cargo build --workspace` succeeds with placeholder crates
- `cargo test --workspace` runs (even if 0 tests)
- Each crate has a `lib.rs` or `main.rs` with doc comments
- `Cargo.lock` is committed (required for binary projects)
- `.gitignore` covers `/target`, `*.swp`, `.env`, model files (`*.gguf`)

**Best Practices:**
- Commit `Cargo.lock` for reproducible builds (this is a binary, not a library)
- Use `[workspace.dependencies]` to centralize shared dependency versions
- Set `[workspace.package]` for shared metadata (authors, license, edition, rust-version)
- Use `[profile.release]` optimizations from day one (LTO, codegen-units, strip)

**Pitfalls to Avoid:**
- Do NOT use path dependencies with absolute paths; always use relative paths
- Do NOT set `resolver = "1"`; use `resolver = "2"` (default in edition 2021+)
- Do NOT put `Cargo.lock` in `.gitignore` (common mistake for binary projects)

---

### INFRA-002: Rust Toolchain Pinning & MSRV Policy

**Description:**
Create `rust-toolchain.toml` to pin the Rust toolchain for all developers and CI. Establish Minimum Supported Rust Version (MSRV) policy.

```toml
# rust-toolchain.toml
[toolchain]
channel = "1.82.0"
components = ["rustfmt", "clippy", "rust-src"]
targets = [
  "x86_64-unknown-linux-musl",
  "aarch64-unknown-linux-musl",
  "x86_64-apple-darwin",
  "aarch64-apple-darwin",
  "x86_64-pc-windows-msvc",
]
```

MSRV policy: N-2 stable releases. Currently Rust 1.82 is latest, so MSRV = 1.80. Set `rust-version = "1.80"` in workspace Cargo.toml. CI will test against MSRV explicitly.

**Estimated Effort:** 0.5 days
**Dependencies:** INFRA-001
**Acceptance Criteria:**
- `rust-toolchain.toml` exists and `rustup show` reflects it
- `Cargo.toml` has `rust-version` field
- CI includes an explicit MSRV check job
- README documents the MSRV policy

**Best Practices:**
- Pin exact version, not channel (e.g., `1.82.0` not `stable`)
- Include all cross-compilation targets in toolchain file
- Test MSRV in CI with `cargo +1.80.0 check --workspace`

**Pitfalls to Avoid:**
- Do NOT use nightly features unless gated behind a feature flag
- Do NOT forget to update MSRV when adopting new Rust features

---

### INFRA-003: GitHub Actions CI Pipeline

**Description:**
Implement the primary CI workflow that runs on every push and PR. This is the quality gate that all code must pass.

```yaml
# .github/workflows/ci.yml
name: CI
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-D warnings"

jobs:
  fmt:
    name: Formatting
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - run: cargo fmt --all --check

  clippy:
    name: Clippy Lints
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy --workspace --all-targets --all-features -- -D warnings

  test:
    name: Tests (${{ matrix.os }})
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --workspace --all-features
      - run: cargo test --workspace --doc

  msrv:
    name: MSRV Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: "1.80.0"
      - uses: Swatinem/rust-cache@v2
      - run: cargo check --workspace --all-features

  audit:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: rustsec/audit-check@v2
        with:
          token: ${{ secrets.GITHUB_TOKEN }}

  deny:
    name: License & Supply Chain
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v1
        with:
          command: check all

  doc:
    name: Documentation
    runs-on: ubuntu-latest
    env:
      RUSTDOCFLAGS: "-D warnings"
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo doc --workspace --no-deps --all-features
```

**Estimated Effort:** 2 days
**Dependencies:** INFRA-001, INFRA-002
**Acceptance Criteria:**
- CI runs automatically on every PR and push to main
- All 7 jobs pass: fmt, clippy, test (3 OS), msrv, audit, deny, doc
- PR cannot merge if any job fails (branch protection rule)
- `Swatinem/rust-cache@v2` reduces build times by 40-60%
- Concurrency group cancels stale CI runs on force-push
- Total CI time < 15 minutes on a clean run

**Best Practices:**
- Use `dtolnay/rust-toolchain` instead of `actions-rs` (better maintained)
- Use `Swatinem/rust-cache@v2` for cargo registry + target caching
- Set `CARGO_TERM_COLOR: always` for readable CI logs
- Use `fail-fast: false` so all OS test results are visible
- Set `RUSTFLAGS: "-D warnings"` globally to fail on any warning
- Use `concurrency` to cancel redundant runs

**Pitfalls to Avoid:**
- Do NOT cache the entire `target/` directory manually; use `Swatinem/rust-cache`
- Do NOT use `actions-rs/toolchain` (unmaintained since 2022)
- Do NOT skip Windows tests; cross-platform bugs are real
- Do NOT run `cargo audit` with advisory-db from a fork (supply chain risk)

---

### INFRA-004: Branch Protection & PR Workflow

**Description:**
Configure GitHub branch protection rules and PR workflow automation.

Branch strategy:
- `main`: Always deployable. Protected. Requires PR.
- `release/vX.Y`: Created for release candidates. Protected. Cherry-pick hotfixes.
- `feat/*`, `fix/*`, `chore/*`: Feature branches. Deleted after merge.

Branch protection on `main`:
- Require status checks: all CI jobs must pass
- Require 1 approving review (relaxed to 0 for solo founder phase, increase to 1 at team size 3+)
- Require linear history (squash merge or rebase, no merge commits)
- Require signed commits (optional, enable when team uses GPG/SSH signing)
- Dismiss stale reviews on new pushes
- Restrict force pushes

Auto-labeling via `.github/labeler.yml`:
```yaml
cli:
  - changed-files:
    - any-glob-to-any-file: 'crates/valiforge-cli/**'
core:
  - changed-files:
    - any-glob-to-any-file: 'crates/valiforge-core/**'
schema:
  - changed-files:
    - any-glob-to-any-file: 'crates/valiforge-schema/**'
ci:
  - changed-files:
    - any-glob-to-any-file: '.github/**'
docs:
  - changed-files:
    - any-glob-to-any-file: 'docs/**'
```

**Estimated Effort:** 1 day
**Dependencies:** INFRA-003
**Acceptance Criteria:**
- Branch protection rules enforced on `main`
- PRs auto-labeled by file path
- Squash merge is the default merge strategy
- PR template exists with checklist (tests, docs, breaking changes)
- CODEOWNERS file assigns reviewers per crate

**Best Practices:**
- Use squash merge to keep `main` history clean
- Require status checks by exact name, not just "any CI"
- Set up CODEOWNERS early even if solo; it documents ownership

**Pitfalls to Avoid:**
- Do NOT allow merge commits; they pollute history for a small project
- Do NOT require signed commits until team has GPG/SSH keys set up
- Do NOT use auto-merge without CI protection (can merge broken code)

---

### INFRA-005: Dependency Management (Dependabot / Renovate)

**Description:**
Configure automated dependency update tooling. Use Dependabot (built into GitHub) initially; switch to Renovate if more control is needed.

```yaml
# .github/dependabot.yml
version: 2
updates:
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "weekly"
      day: "monday"
    open-pull-requests-limit: 10
    reviewers:
      - "kaushikreddy"
    labels:
      - "dependencies"
    groups:
      rust-dependencies:
        patterns:
          - "*"
        update-types:
          - "minor"
          - "patch"

  - package-ecosystem: "github-actions"
    directory: "/"
    schedule:
      interval: "weekly"
    labels:
      - "ci"
```

**Estimated Effort:** 0.5 days
**Dependencies:** INFRA-003
**Acceptance Criteria:**
- Dependabot creates weekly grouped PRs for Cargo dependencies
- GitHub Actions dependencies are updated separately
- Minor + patch updates are grouped into a single PR
- Major updates get individual PRs for careful review
- CI runs automatically on dependency update PRs

**Best Practices:**
- Group minor/patch updates to reduce PR noise
- Keep major version bumps as separate PRs for review
- Set `open-pull-requests-limit` to avoid PR flood
- Monitor `cargo audit` results on dependency PRs closely

**Pitfalls to Avoid:**
- Do NOT auto-merge dependency updates without CI passing
- Do NOT ignore Dependabot PRs; stale deps = security risk
- Do NOT group major version bumps with patches (breaking changes get lost)

---

### INFRA-006: cargo-deny Configuration

**Description:**
Configure `cargo-deny` for license compliance, supply chain security, and duplicate dependency detection.

```toml
# deny.toml
[advisories]
vulnerability = "deny"
unmaintained = "warn"
yanked = "deny"
notice = "warn"

[licenses]
unlicensed = "deny"
allow = [
  "MIT",
  "Apache-2.0",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "ISC",
  "Unicode-DFS-2016",
  "Zlib",
  "CC0-1.0",
  "OpenSSL",
  "BSL-1.0",
]
copyleft = "deny"
default = "deny"

[bans]
multiple-versions = "warn"
wildcards = "deny"
highlight = "all"
skip = []

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []
```

**Estimated Effort:** 1 day
**Dependencies:** INFRA-001
**Acceptance Criteria:**
- `cargo deny check all` passes in CI
- GPL/AGPL/LGPL crates are blocked (copyleft = "deny")
- Known vulnerabilities fail the build
- Yanked crates fail the build
- Duplicate dependency versions produce warnings
- Wildcard version requirements are denied

**Best Practices:**
- Start strict and relax as needed with `skip` entries (documented)
- Review `multiple-versions` warnings to minimize binary bloat
- Keep the allowed license list minimal and auditable
- Run `cargo deny list` to audit current license landscape before configuring

**Pitfalls to Avoid:**
- Do NOT set `copyleft = "allow"` without legal review
- Do NOT use `skip-tree` liberally; it hides real issues
- Do NOT forget `unknown-registry = "deny"` (prevents supply chain attacks)

---

### INFRA-007: Release Automation (Tag-Based)

**Description:**
Implement automated release workflow triggered by Git tags. When a tag matching `v*.*.*` is pushed, the pipeline builds all platform binaries, generates a changelog, creates a GitHub Release, and publishes to crates.io.

Tag convention: `vX.Y.Z` (semver). Pre-releases: `vX.Y.Z-rc.N` or `vX.Y.Z-beta.N`.

Changelog generation: Use `git-cliff` for conventional-commit-based changelogs. Requires commit messages follow Conventional Commits (`feat:`, `fix:`, `chore:`, `perf:`, `docs:`, `ci:`, `refactor:`, `test:`, `breaking:`).

Release workflow is detailed under INFRA-010 (cross-platform builds). This task covers the tagging, changelog, and GitHub Release creation.

```yaml
# Partial: release creation step
- name: Generate Changelog
  uses: orhun/git-cliff-action@v3
  with:
    config: cliff.toml
    args: --latest --strip header
  env:
    OUTPUT: CHANGELOG.md

- name: Create GitHub Release
  uses: softprops/action-gh-release@v2
  with:
    body_path: CHANGELOG.md
    files: |
      dist/*
    generate_release_notes: false
    prerelease: ${{ contains(github.ref, 'rc') || contains(github.ref, 'beta') }}
```

**Estimated Effort:** 2 days
**Dependencies:** INFRA-003, INFRA-010
**Acceptance Criteria:**
- Pushing `v1.0.0` tag triggers full release pipeline
- Changelog auto-generated from conventional commits since last tag
- GitHub Release created with changelog body
- All platform binaries attached as release assets
- SHA256 checksums file attached
- Pre-release tags (rc, beta) marked as pre-release on GitHub
- `cargo publish` runs for all public crates in dependency order

**Best Practices:**
- Use `git-cliff` with a custom `cliff.toml` to categorize commits
- Enforce conventional commits via a PR title check (use `commitlint` action)
- Publish crates in dependency order: core -> schema -> diff -> datagen -> report -> plugin -> sdk -> cli
- Use `cargo publish --dry-run` in CI before actual publish
- Tag releases from `main` only, never from feature branches

**Pitfalls to Avoid:**
- Do NOT manually create GitHub Releases; automation prevents human error
- Do NOT forget `--locked` flag when building release binaries
- Do NOT publish crates without `cargo publish --dry-run` first
- Do NOT forget to bump version in ALL workspace member Cargo.toml files (use `cargo-release` or `cargo-workspaces`)

---

## 2. Cross-Platform Binary Builds

### INFRA-008: Build Matrix Definition

**Description:**
Define the build matrix for all supported platforms. Each target produces a statically-linked (where possible) binary with optimized release profile.

| Target Triple | OS | Arch | Libc | Runner | Notes |
|---|---|---|---|---|---|
| `x86_64-unknown-linux-musl` | Linux | x86_64 | musl (static) | `ubuntu-latest` | Primary Linux build, fully static |
| `aarch64-unknown-linux-musl` | Linux | aarch64 | musl (static) | `ubuntu-latest` + cross | ARM64 servers, Graviton |
| `x86_64-apple-darwin` | macOS | x86_64 | system | `macos-13` | Intel Macs (legacy) |
| `aarch64-apple-darwin` | macOS | aarch64 | system | `macos-14` | Apple Silicon (M1+) |
| `x86_64-pc-windows-msvc` | Windows | x86_64 | MSVC | `windows-latest` | Windows native |

Release profile in workspace `Cargo.toml`:

```toml
[profile.release]
opt-level = "s"        # Optimize for size (switch to "3" if perf > size)
lto = "fat"            # Full link-time optimization
codegen-units = 1      # Single codegen unit for best optimization
panic = "abort"        # Smaller binary, no unwinding
strip = true           # Strip debug symbols
```

Naming convention for artifacts:
`valiforge-{version}-{target}.tar.gz` (Linux/macOS)
`valiforge-{version}-{target}.zip` (Windows)

**Estimated Effort:** 1 day
**Dependencies:** INFRA-001
**Acceptance Criteria:**
- Build matrix documented and tested
- Release profile produces binaries under 15MB (per PRD target)
- Linux musl binaries are fully static (`ldd` reports "not a dynamic executable")
- macOS binaries work on both Intel and Apple Silicon
- Windows binary runs without MSVC runtime installer
- Artifact naming is consistent and machine-parseable

**Best Practices:**
- Use `opt-level = "s"` (not `"z"`) for best size/perf tradeoff; `"z"` is too aggressive
- `panic = "abort"` saves ~10% binary size and is fine for a CLI tool
- `strip = true` in Cargo.toml replaces manual `strip` command
- Test that musl builds work on Alpine, Debian, and Ubuntu (different glibc expectations)

**Pitfalls to Avoid:**
- Do NOT use `opt-level = "z"` without benchmarking; it can degrade performance 20-30%
- Do NOT forget to test musl binaries inside Docker containers (common deployment target)
- Do NOT assume Apple Silicon runners use `macos-latest` (it may still be Intel)
- Do NOT ship debug builds as releases (check file size; debug builds are 5-10x larger)

---

### INFRA-009: Cross-Compilation Setup

**Description:**
Configure cross-compilation for targets that cannot build natively on GitHub Actions runners.

For `aarch64-unknown-linux-musl`: Use `cross` (https://github.com/cross-rs/cross) which provides pre-built Docker images with the correct toolchain. Alternative: use `cargo-zigbuild` with Zig as a cross-linker (simpler, newer approach).

Recommended approach: `cargo-zigbuild`
- Zig bundles libc for all targets, making cross-compilation trivial
- No Docker-in-Docker needed
- Faster than `cross` for musl targets

```yaml
# In release workflow
- name: Install Zig
  uses: goto-bus-stop/setup-zig@v2
  with:
    version: 0.13.0

- name: Install cargo-zigbuild
  run: cargo install cargo-zigbuild

- name: Build (Linux aarch64 musl)
  run: cargo zigbuild --release --target aarch64-unknown-linux-musl -p valiforge-cli
```

For macOS universal binary (optional, combines x86_64 + aarch64):
```bash
lipo -create -output valiforge-universal \
  target/x86_64-apple-darwin/release/valiforge \
  target/aarch64-apple-darwin/release/valiforge
```

**Estimated Effort:** 2 days
**Dependencies:** INFRA-008
**Acceptance Criteria:**
- `aarch64-unknown-linux-musl` binary builds in CI and passes smoke tests
- Cross-compiled binaries run correctly on target platform (test in Docker/QEMU)
- Build time for cross-compilation < 20 minutes
- No dynamic library dependencies in musl builds

**Best Practices:**
- Prefer `cargo-zigbuild` over `cross` for simplicity and speed
- Always smoke-test cross-compiled binaries with QEMU in CI
- Cache the Zig installation and cargo-zigbuild binary
- Pin Zig version explicitly (Zig has breaking changes between versions)

**Pitfalls to Avoid:**
- Do NOT assume cross-compiled binaries work without testing; endianness, syscall, and linking issues are common
- Do NOT use `cross` with Docker if the CI runner itself is in Docker (Docker-in-Docker is fragile)
- Do NOT forget to handle OpenSSL; musl + OpenSSL is painful -- use `rustls` instead

---

### INFRA-010: Release Build Workflow

**Description:**
Full release workflow that builds all platform binaries, generates checksums, and uploads to GitHub Releases.

```yaml
# .github/workflows/release.yml
name: Release
on:
  push:
    tags: ["v*.*.*"]

permissions:
  contents: write

jobs:
  build:
    name: Build ${{ matrix.target }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: x86_64-unknown-linux-musl
            os: ubuntu-latest
            archive: tar.gz
          - target: aarch64-unknown-linux-musl
            os: ubuntu-latest
            archive: tar.gz
            cross: true
          - target: x86_64-apple-darwin
            os: macos-13
            archive: tar.gz
          - target: aarch64-apple-darwin
            os: macos-14
            archive: tar.gz
          - target: x86_64-pc-windows-msvc
            os: windows-latest
            archive: zip
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@v2
        with:
          key: release-${{ matrix.target }}

      # Cross-compilation setup (Linux aarch64 only)
      - name: Install cross-compilation tools
        if: matrix.cross
        run: |
          cargo install cargo-zigbuild
          sudo snap install zig --classic --beta

      # Native build
      - name: Build (native)
        if: "!matrix.cross"
        run: cargo build --release --locked --target ${{ matrix.target }} -p valiforge-cli

      # Cross build
      - name: Build (cross)
        if: matrix.cross
        run: cargo zigbuild --release --locked --target ${{ matrix.target }} -p valiforge-cli

      # Package
      - name: Package (Unix)
        if: matrix.archive == 'tar.gz'
        run: |
          cd target/${{ matrix.target }}/release
          tar czf ../../../valiforge-${{ github.ref_name }}-${{ matrix.target }}.tar.gz valiforge
          cd ../../..
          sha256sum valiforge-${{ github.ref_name }}-${{ matrix.target }}.tar.gz > valiforge-${{ github.ref_name }}-${{ matrix.target }}.tar.gz.sha256

      - name: Package (Windows)
        if: matrix.archive == 'zip'
        shell: pwsh
        run: |
          Compress-Archive -Path target/${{ matrix.target }}/release/valiforge.exe -DestinationPath valiforge-${{ github.ref_name }}-${{ matrix.target }}.zip
          (Get-FileHash valiforge-${{ github.ref_name }}-${{ matrix.target }}.zip -Algorithm SHA256).Hash.ToLower() + "  valiforge-${{ github.ref_name }}-${{ matrix.target }}.zip" | Out-File valiforge-${{ github.ref_name }}-${{ matrix.target }}.zip.sha256

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: dist-${{ matrix.target }}
          path: valiforge-${{ github.ref_name }}-${{ matrix.target }}.*

  release:
    name: Create Release
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          path: dist
          merge-multiple: true

      - name: Generate changelog
        uses: orhun/git-cliff-action@v3
        with:
          config: cliff.toml
          args: --latest --strip header
        env:
          OUTPUT: CHANGELOG.md

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          body_path: CHANGELOG.md
          files: dist/*
          prerelease: ${{ contains(github.ref, 'rc') || contains(github.ref, 'beta') || contains(github.ref, 'alpha') }}

  publish-crates:
    name: Publish to crates.io
    needs: release
    if: "!contains(github.ref, 'rc') && !contains(github.ref, 'beta') && !contains(github.ref, 'alpha')"
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Publish crates (dependency order)
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
        run: |
          cargo publish -p valiforge-core --locked
          sleep 30
          cargo publish -p valiforge-schema --locked
          sleep 30
          cargo publish -p valiforge-diff --locked
          sleep 30
          cargo publish -p valiforge-datagen --locked
          sleep 30
          cargo publish -p valiforge-report --locked
          sleep 30
          cargo publish -p valiforge-plugin --locked
          sleep 30
          cargo publish -p valiforge-sdk --locked
          sleep 30
          cargo publish -p valiforge-cli --locked
```

**Estimated Effort:** 3 days
**Dependencies:** INFRA-008, INFRA-009, INFRA-007
**Acceptance Criteria:**
- Tag push triggers full build matrix (5 targets)
- All binaries are built with `--locked` flag
- SHA256 checksums generated for every artifact
- GitHub Release created with changelog and all artifacts
- crates.io publish succeeds for non-prerelease tags
- Total release pipeline completes in < 30 minutes

**Best Practices:**
- Use `--locked` to ensure reproducible builds from `Cargo.lock`
- Sleep 30s between crate publishes (crates.io index propagation)
- Generate per-artifact SHA256 files AND a combined checksums file
- Use `fetch-depth: 0` for changelog generation (needs full history)
- Separate build and release jobs so builds run in parallel

**Pitfalls to Avoid:**
- Do NOT publish to crates.io for pre-releases (rc/beta/alpha)
- Do NOT forget the sleep between crate publishes (index race condition)
- Do NOT use `cargo publish` without `--locked`
- Do NOT store `CARGO_REGISTRY_TOKEN` in workflow files; use GitHub Secrets

---

### INFRA-011: Binary Size Optimization & Tracking

**Description:**
Establish binary size budget (< 15MB per PRD) and track it across releases. Implement size optimization techniques and CI checks.

Optimization checklist:
1. `opt-level = "s"` -- optimize for size
2. `lto = "fat"` -- cross-crate dead code elimination
3. `codegen-units = 1` -- best optimization at cost of compile time
4. `panic = "abort"` -- no unwinding tables
5. `strip = true` -- remove debug symbols and symbol tables
6. Use `cargo-bloat` to identify largest contributors
7. Use `cargo tree --duplicates` to find duplicate dependencies
8. Consider `UPX` compression for further reduction (trade-off: slower startup)

CI size tracking:
```yaml
- name: Check binary size
  run: |
    SIZE=$(stat -c%s target/x86_64-unknown-linux-musl/release/valiforge 2>/dev/null || stat -f%z target/x86_64-unknown-linux-musl/release/valiforge)
    echo "Binary size: $((SIZE / 1024 / 1024))MB ($SIZE bytes)"
    if [ "$SIZE" -gt 15728640 ]; then
      echo "::error::Binary size exceeds 15MB budget"
      exit 1
    fi
```

**Estimated Effort:** 1 day
**Dependencies:** INFRA-008
**Acceptance Criteria:**
- Release binary < 15MB for all targets
- CI fails if binary exceeds size budget
- Binary size printed in CI logs for every build
- `cargo-bloat` report available for debugging size regressions
- Size tracking per release (stored as GitHub Release metadata)

**Best Practices:**
- Audit dependencies with `cargo tree --edges features` to identify unnecessary features
- Use `cargo bloat --release --crates` to find the biggest size contributors
- Prefer `rustls` over `openssl-sys` (smaller, no C dependency)
- Feature-gate heavy optional dependencies (e.g., SLM runtime)

**Pitfalls to Avoid:**
- Do NOT use UPX on macOS (breaks code signing and Gatekeeper)
- Do NOT set the size budget too tight initially; 15MB is reasonable, 10MB may be too aggressive with all protocol parsers
- Do NOT forget that musl builds may be slightly larger than glibc builds

---

### INFRA-012: macOS Code Signing & Notarization

**Description:**
Optional but recommended for distribution: sign macOS binaries with an Apple Developer certificate and submit for notarization. Unsigned binaries trigger Gatekeeper warnings.

Steps:
1. Enroll in Apple Developer Program ($99/year)
2. Generate Developer ID Application certificate
3. Store certificate + password in GitHub Secrets
4. Sign binary with `codesign` in CI
5. Submit for notarization with `xcrun notarytool`
6. Staple notarization ticket to binary

```yaml
- name: Sign macOS binary
  if: startsWith(matrix.target, 'aarch64-apple') || startsWith(matrix.target, 'x86_64-apple')
  env:
    APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE }}
    APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
    APPLE_ID: ${{ secrets.APPLE_ID }}
    APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
    APPLE_APP_PASSWORD: ${{ secrets.APPLE_APP_PASSWORD }}
  run: |
    echo "$APPLE_CERTIFICATE" | base64 --decode > certificate.p12
    security create-keychain -p temp build.keychain
    security import certificate.p12 -k build.keychain -P "$APPLE_CERTIFICATE_PASSWORD" -T /usr/bin/codesign
    security set-key-partition-list -S apple-tool:,apple: -s -k temp build.keychain
    codesign --sign "Developer ID Application" --keychain build.keychain --options runtime target/${{ matrix.target }}/release/valiforge
    # Notarization
    ditto -c -k --keepParent target/${{ matrix.target }}/release/valiforge valiforge-notarize.zip
    xcrun notarytool submit valiforge-notarize.zip --apple-id "$APPLE_ID" --team-id "$APPLE_TEAM_ID" --password "$APPLE_APP_PASSWORD" --wait
```

**Estimated Effort:** 2 days
**Dependencies:** INFRA-010
**Acceptance Criteria:**
- macOS binaries are signed with Developer ID certificate
- Notarization succeeds (no Gatekeeper warnings on download)
- Certificate and credentials stored securely in GitHub Secrets
- Signing step does not block non-macOS builds

**Best Practices:**
- Use `--options runtime` flag for Hardened Runtime (required for notarization)
- Create a dedicated keychain in CI to avoid conflicts
- Test notarized binary on a clean macOS install
- Cache nothing related to signing (security risk)

**Pitfalls to Avoid:**
- Do NOT store Apple certificates in the repository
- Do NOT skip `--options runtime` (notarization will fail)
- Do NOT forget to delete the temporary keychain after signing
- This is a Phase 2 nice-to-have; do NOT block MVP launch on this

---

## 3. Distribution Channels

### INFRA-013: GitHub Releases (Primary Channel)

**Description:**
GitHub Releases is the primary distribution channel, automated via INFRA-010. This task covers the release page formatting, asset organization, and download link structure.

Release page template:
```markdown
## ValiForge v{version}

{changelog from git-cliff}

### Installation

#### Quick Install (recommended)
```bash
curl -fsSL https://install.valiforge.dev | sh
```

#### Direct Download
| Platform | Architecture | Download |
|----------|-------------|----------|
| Linux | x86_64 | [valiforge-v{ver}-x86_64-unknown-linux-musl.tar.gz](...) |
| Linux | aarch64 | [valiforge-v{ver}-aarch64-unknown-linux-musl.tar.gz](...) |
| macOS | Intel | [valiforge-v{ver}-x86_64-apple-darwin.tar.gz](...) |
| macOS | Apple Silicon | [valiforge-v{ver}-aarch64-apple-darwin.tar.gz](...) |
| Windows | x86_64 | [valiforge-v{ver}-x86_64-pc-windows-msvc.zip](...) |

### Checksums
SHA256 checksums: [checksums.txt](...)

### Other Installation Methods
- `cargo install valiforge`
- `brew install valiforge/tap/valiforge`
- `npm install -g @valiforge/cli`
```

**Estimated Effort:** 0.5 days
**Dependencies:** INFRA-010
**Acceptance Criteria:**
- Release page has structured download table
- All 5 platform binaries available
- SHA256 checksums file included
- Installation instructions for all channels
- Latest release accessible via `https://github.com/valiforge/valiforge/releases/latest`

**Best Practices:**
- Use consistent artifact naming for machine parsing
- Include "latest" redirect URL in documentation
- Provide both per-file `.sha256` and combined `checksums.txt`

**Pitfalls to Avoid:**
- Do NOT change artifact naming convention after first release (breaks install scripts)

---

### INFRA-014: crates.io Publishing

**Description:**
Publish ValiForge crates to crates.io so users can `cargo install valiforge`. Configure `Cargo.toml` metadata for all public crates.

Required metadata per crate:
```toml
[package]
name = "valiforge"          # or valiforge-core, etc.
version = "0.1.0"
edition = "2021"
rust-version = "1.80"
license = "Apache-2.0 OR MIT"
description = "Headless API-first validation engine for AI workloads"
homepage = "https://valiforge.dev"
repository = "https://github.com/valiforge/valiforge"
documentation = "https://docs.rs/valiforge"
readme = "README.md"
keywords = ["api", "testing", "validation", "openapi", "ci"]
categories = ["development-tools::testing", "command-line-utilities", "web-programming"]
```

Add `cargo-binstall` metadata to CLI crate:
```toml
[package.metadata.binstall]
pkg-url = "{ repo }/releases/download/v{ version }/valiforge-v{ version }-{ target-triple }.tar.gz"
bin-dir = "valiforge-v{ version }-{ target-triple }/{ bin }{ binary-ext }"
pkg-fmt = "tgz"
```

**Estimated Effort:** 1 day
**Dependencies:** INFRA-001, INFRA-010
**Acceptance Criteria:**
- All public crates have complete metadata
- `cargo install valiforge` works from crates.io
- `cargo binstall valiforge` downloads pre-built binary instead of compiling
- Docs appear on docs.rs automatically after publish
- Keywords and categories are accurate for discoverability

**Best Practices:**
- Use dual license `Apache-2.0 OR MIT` (Rust ecosystem standard)
- Set `publish = false` for internal-only crates (e.g., integration test crates)
- Include `[package.metadata.binstall]` from day one -- free optimization
- Verify `cargo publish --dry-run` before every release

**Pitfalls to Avoid:**
- Do NOT publish with `publish = true` on internal crates
- Do NOT forget to reserve the crate name early (squatting is a real issue)
- Do NOT include large files in the crate (use `exclude` in Cargo.toml for test fixtures, docs, etc.)
- Do NOT forget `license` field (crates.io requires it)

---

### INFRA-015: Homebrew Tap

**Description:**
Create and maintain a Homebrew tap (`valiforge/homebrew-tap`) for macOS and Linux installation via `brew install valiforge/tap/valiforge`.

Repository: `github.com/valiforge/homebrew-tap`

Formula template (auto-generated by release CI):
```ruby
class Valiforge < Formula
  desc "Headless API-first validation engine for AI workloads"
  homepage "https://valiforge.dev"
  version "{version}"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/valiforge/valiforge/releases/download/v#{version}/valiforge-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "{sha256}"
    end
    on_intel do
      url "https://github.com/valiforge/valiforge/releases/download/v#{version}/valiforge-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "{sha256}"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/valiforge/valiforge/releases/download/v#{version}/valiforge-v#{version}-aarch64-unknown-linux-musl.tar.gz"
      sha256 "{sha256}"
    end
    on_intel do
      url "https://github.com/valiforge/valiforge/releases/download/v#{version}/valiforge-v#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "{sha256}"
    end
  end

  def install
    bin.install "valiforge"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/valiforge --version")
  end
end
```

CI automation: After GitHub Release is created, trigger a workflow that:
1. Downloads checksums from the release
2. Renders the formula template with correct SHA256 hashes
3. Commits and pushes to the `homebrew-tap` repository

**Estimated Effort:** 1.5 days
**Dependencies:** INFRA-010
**Acceptance Criteria:**
- `brew install valiforge/tap/valiforge` works on macOS (Intel + Apple Silicon)
- `brew install valiforge/tap/valiforge` works on Linux (x86_64 + aarch64)
- Formula auto-updated on every release
- `brew test valiforge` passes (version check)
- Tap repository is public and well-documented

**Best Practices:**
- Use `on_macos`/`on_linux` + `on_arm`/`on_intel` blocks for platform detection
- Include a `test` block in the formula (Homebrew CI runs it)
- Automate formula updates via CI (never update manually)
- Eventually submit to homebrew-core for `brew install valiforge` (requires popularity threshold)

**Pitfalls to Avoid:**
- Do NOT hardcode SHA256 hashes; compute them from release artifacts
- Do NOT forget to handle the tap repository permissions for CI pushes
- Do NOT use `head` builds in the formula (breaks reproducibility)

---

### INFRA-016: npm Wrapper Package

**Description:**
Create a thin npm package (`@valiforge/cli`) that downloads the correct platform binary on `npm install` / `npx`. Follow the esbuild distribution pattern: platform-specific optional dependencies.

Package structure:
```
npm/
├── valiforge/                      # Main package
│   ├── package.json
│   ├── bin/
│   │   └── valiforge               # Shell script wrapper
│   └── install.js                   # postinstall: downloads binary
├── valiforge-linux-x64/
│   ├── package.json
│   └── bin/
│       └── valiforge
├── valiforge-linux-arm64/
├── valiforge-darwin-x64/
├── valiforge-darwin-arm64/
└── valiforge-win32-x64/
```

Main package.json:
```json
{
  "name": "@valiforge/cli",
  "version": "0.1.0",
  "description": "Headless API-first validation engine for AI workloads",
  "bin": { "valiforge": "bin/valiforge" },
  "optionalDependencies": {
    "@valiforge/cli-linux-x64": "0.1.0",
    "@valiforge/cli-linux-arm64": "0.1.0",
    "@valiforge/cli-darwin-x64": "0.1.0",
    "@valiforge/cli-darwin-arm64": "0.1.0",
    "@valiforge/cli-win32-x64": "0.1.0"
  }
}
```

**Estimated Effort:** 2 days
**Dependencies:** INFRA-010
**Acceptance Criteria:**
- `npm install -g @valiforge/cli` works on all 5 platforms
- `npx @valiforge/cli validate --help` works without global install
- Binary is downloaded lazily via optionalDependencies (esbuild pattern)
- Package size < 1MB (binary is in platform-specific package)
- npm publish automated in release CI

**Best Practices:**
- Use the esbuild pattern (optionalDependencies) not the postinstall download pattern
- Each platform package contains only the binary for that platform
- The main package has a JS shim that locates and executes the binary
- Use `os` and `cpu` fields in platform package.json for npm's native resolution

**Pitfalls to Avoid:**
- Do NOT use `postinstall` scripts to download binaries (blocked by many corporate proxies, npm audit flags them)
- Do NOT ship all platform binaries in one package (bloats node_modules)
- Do NOT forget to publish all 6 packages (main + 5 platforms) atomically

---

### INFRA-017: Docker Image (Multi-Arch)

**Description:**
Build and publish multi-architecture Docker images to Docker Hub and GitHub Container Registry (ghcr.io). Use multi-stage build with `FROM scratch` for minimal image size.

```dockerfile
# docker/Dockerfile
FROM rust:1.82-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /build
COPY . .
RUN cargo build --release --locked -p valiforge-cli
RUN strip /build/target/release/valiforge

FROM scratch
COPY --from=builder /build/target/release/valiforge /valiforge
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
ENTRYPOINT ["/valiforge"]
```

Multi-arch build with `docker buildx`:
```yaml
- name: Build and push Docker image
  uses: docker/build-push-action@v5
  with:
    context: .
    file: docker/Dockerfile
    platforms: linux/amd64,linux/arm64
    push: true
    tags: |
      valiforge/valiforge:${{ github.ref_name }}
      valiforge/valiforge:latest
      ghcr.io/valiforge/valiforge:${{ github.ref_name }}
      ghcr.io/valiforge/valiforge:latest
    cache-from: type=gha
    cache-to: type=gha,mode=max
```

**Estimated Effort:** 1.5 days
**Dependencies:** INFRA-010
**Acceptance Criteria:**
- `docker run valiforge/valiforge:latest --version` works on amd64 and arm64
- Image size < 20MB (FROM scratch + static binary + CA certs)
- Published to both Docker Hub and ghcr.io
- Multi-arch manifest works (`docker pull` on ARM machine gets ARM image)
- CA certificates included (for HTTPS API validation)
- Tags: `latest`, semver tag, major version tag (e.g., `v1`)

**Best Practices:**
- Use `FROM scratch` for smallest possible image (no shell, no OS)
- Include CA certificates for HTTPS (copy from builder stage)
- Use GitHub Actions cache for Docker layer caching
- Tag with both specific version and `latest`
- Publish to both Docker Hub and ghcr.io for maximum reach

**Pitfalls to Avoid:**
- Do NOT use `FROM alpine` for the final stage (wastes 5MB, adds attack surface)
- Do NOT forget CA certificates (HTTPS requests will fail)
- Do NOT forget to set up Docker Hub credentials in GitHub Secrets
- Do NOT use `--platform` in Dockerfile (use `docker buildx` instead)

---

### INFRA-018: Shell Installer Script

**Description:**
Create an install script served at `https://install.valiforge.dev` that detects the platform and downloads the correct binary.

```bash
#!/bin/sh
# install.sh — ValiForge installer
# Usage: curl -fsSL https://install.valiforge.dev | sh

set -euo pipefail

INSTALL_DIR="${VALIFORGE_INSTALL_DIR:-/usr/local/bin}"
REPO="valiforge/valiforge"
BINARY="valiforge"

# Detect OS
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
case "$OS" in
  linux)  OS="unknown-linux-musl" ;;
  darwin) OS="apple-darwin" ;;
  *)      echo "Unsupported OS: $OS"; exit 1 ;;
esac

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
  x86_64|amd64) ARCH="x86_64" ;;
  aarch64|arm64) ARCH="aarch64" ;;
  *)             echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

TARGET="${ARCH}-${OS}"

# Get latest version
VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"v([^"]+)".*/\1/')

DOWNLOAD_URL="https://github.com/${REPO}/releases/download/v${VERSION}/${BINARY}-v${VERSION}-${TARGET}.tar.gz"
CHECKSUM_URL="${DOWNLOAD_URL}.sha256"

echo "Installing ValiForge v${VERSION} for ${TARGET}..."

# Download and verify
TMPDIR=$(mktemp -d)
curl -fsSL "$DOWNLOAD_URL" -o "${TMPDIR}/${BINARY}.tar.gz"
curl -fsSL "$CHECKSUM_URL" -o "${TMPDIR}/${BINARY}.tar.gz.sha256"

cd "$TMPDIR"
if command -v sha256sum > /dev/null; then
  sha256sum -c "${BINARY}.tar.gz.sha256"
elif command -v shasum > /dev/null; then
  shasum -a 256 -c "${BINARY}.tar.gz.sha256"
fi

tar xzf "${BINARY}.tar.gz"
install -d "$INSTALL_DIR"
install "${BINARY}" "$INSTALL_DIR/${BINARY}"

rm -rf "$TMPDIR"
echo "ValiForge v${VERSION} installed to ${INSTALL_DIR}/${BINARY}"
echo "Run 'valiforge --help' to get started."
```

Host via GitHub Pages at `install.valiforge.dev` or Cloudflare Pages.

**Estimated Effort:** 1 day
**Dependencies:** INFRA-010, INFRA-013
**Acceptance Criteria:**
- `curl -fsSL https://install.valiforge.dev | sh` works on macOS (Intel + ARM) and Linux (x86_64 + aarch64)
- SHA256 checksum verified before installation
- Custom install directory via `VALIFORGE_INSTALL_DIR` env var
- Clear error messages for unsupported platforms
- Script is POSIX-compliant (works in sh, bash, zsh)
- Windows is directed to alternative installation methods

**Best Practices:**
- ALWAYS verify checksums (defense against MITM/CDN compromise)
- Use `set -euo pipefail` for safe shell scripting
- Support `VALIFORGE_INSTALL_DIR` for custom locations
- Print clear instructions after installation
- Use `mktemp -d` for secure temporary directory

**Pitfalls to Avoid:**
- Do NOT assume `sha256sum` exists on macOS (use `shasum -a 256` as fallback)
- Do NOT pipe to `sh` without offering a download-and-inspect alternative
- Do NOT require `sudo` unless installing to a system directory
- Do NOT assume `curl` is available; consider `wget` fallback

---

### INFRA-019: GitHub Actions Marketplace Action

**Description:**
Create a composite GitHub Action (`valiforge/action@v1`) for first-class CI integration. This is a critical adoption driver per the PRD.

Detailed design in Section 4 (INFRA-023 through INFRA-027).

**Estimated Effort:** See Section 4
**Dependencies:** INFRA-010

---

### INFRA-020: GitLab CI Template

**Description:**
Create an includable GitLab CI template for ValiForge validation.

```yaml
# templates/gitlab-ci.yml
.valiforge:
  image: valiforge/valiforge:latest
  variables:
    VALIFORGE_SCHEMA: ""
    VALIFORGE_TARGET: ""
    VALIFORGE_FAIL_ON: "breaking-changes,schema-violations"
    VALIFORGE_FORMAT: "junit"
  script:
    - valiforge validate
      --schema "$VALIFORGE_SCHEMA"
      --target "$VALIFORGE_TARGET"
      --fail-on "$VALIFORGE_FAIL_ON"
      --format "$VALIFORGE_FORMAT"
      --output valiforge-results.xml
  artifacts:
    reports:
      junit: valiforge-results.xml
    when: always
```

Usage in `.gitlab-ci.yml`:
```yaml
include:
  - remote: 'https://raw.githubusercontent.com/valiforge/valiforge/main/templates/gitlab-ci.yml'

api-validation:
  extends: .valiforge
  variables:
    VALIFORGE_SCHEMA: ./openapi.yaml
    VALIFORGE_TARGET: http://localhost:3000
```

**Estimated Effort:** 0.5 days
**Dependencies:** INFRA-017 (Docker image)
**Acceptance Criteria:**
- Template is includable via `remote` URL
- Variables are configurable
- JUnit XML artifacts uploaded for GitLab test reporting
- Template works with GitLab.com and self-hosted GitLab
- Documentation includes copy-paste example

**Best Practices:**
- Use Docker image (not binary download) for GitLab CI simplicity
- Output JUnit XML for native GitLab test reporting integration
- Keep template minimal; users can extend with their own `before_script`

**Pitfalls to Avoid:**
- Do NOT hardcode URLs in the template; use variables
- Do NOT use `latest` tag in production templates; recommend pinning to version

---

### INFRA-021: AUR Package (Arch Linux)

**Description:**
Create and maintain an AUR package for Arch Linux users. Two packages: `valiforge-bin` (pre-built binary) and `valiforge` (build from source).

PKGBUILD for `valiforge-bin`:
```bash
pkgname=valiforge-bin
pkgver=0.1.0
pkgrel=1
pkgdesc="Headless API-first validation engine for AI workloads"
arch=('x86_64' 'aarch64')
url="https://github.com/valiforge/valiforge"
license=('Apache-2.0' 'MIT')
provides=('valiforge')
conflicts=('valiforge')
source_x86_64=("https://github.com/valiforge/valiforge/releases/download/v${pkgver}/valiforge-v${pkgver}-x86_64-unknown-linux-musl.tar.gz")
source_aarch64=("https://github.com/valiforge/valiforge/releases/download/v${pkgver}/valiforge-v${pkgver}-aarch64-unknown-linux-musl.tar.gz")
sha256sums_x86_64=('SKIP')
sha256sums_aarch64=('SKIP')

package() {
  install -Dm755 valiforge "${pkgdir}/usr/bin/valiforge"
}
```

**Estimated Effort:** 0.5 days
**Dependencies:** INFRA-010
**Acceptance Criteria:**
- `yay -S valiforge-bin` installs successfully on Arch Linux
- PKGBUILD passes `namcap` checks
- Both x86_64 and aarch64 supported

**Best Practices:**
- Maintain `-bin` (pre-built) and source-build variants
- Use `updpkgsums` for SHA256 updates
- Automate PKGBUILD updates via CI

**Pitfalls to Avoid:**
- Do NOT use `sha256sums=('SKIP')` in production; compute real checksums
- AUR is community-maintained; plan for occasional breakage

---

### INFRA-022: Nix Flake

**Description:**
Create a `flake.nix` for NixOS/Nix users. Provides both a package and a dev shell.

```nix
{
  description = "ValiForge — Headless API-first validation engine";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rustfmt" "clippy" ];
        };
      in {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "valiforge";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          meta = with pkgs.lib; {
            description = "Headless API-first validation engine for AI workloads";
            homepage = "https://valiforge.dev";
            license = with licenses; [ asl20 mit ];
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            pkg-config
            openssl
            cargo-deny
            cargo-audit
            cargo-bloat
            cargo-watch
          ];
        };
      }
    );
}
```

**Estimated Effort:** 1 day
**Dependencies:** INFRA-001
**Acceptance Criteria:**
- `nix build` produces working binary
- `nix develop` provides full development environment
- Flake works on Linux and macOS (x86_64 + aarch64)
- Flake check passes (`nix flake check`)
- Added to FlakeHub or NixOS packages eventually

**Best Practices:**
- Use `rust-overlay` for consistent Rust toolchain in Nix
- Include dev tools (cargo-deny, cargo-audit) in devShell
- Pin nixpkgs to a specific revision for reproducibility

**Pitfalls to Avoid:**
- Do NOT use `fetchFromGitHub` with `sha256 = lib.fakeSha256` in production
- Do NOT forget to update `cargoLock.lockFile` when dependencies change
- Nix builds are sandboxed; ensure no network access during build

---

## 4. GitHub Actions Integration (First-Class)

### INFRA-023: Composite Action Design & Implementation

**Description:**
Design and implement `valiforge/action@v1` as a composite GitHub Action. This is the primary adoption vehicle for CI/CD integration.

Repository: `github.com/valiforge/action` (separate repo, or `action/` directory in monorepo)

```yaml
# action/action.yml
name: "ValiForge API Validation"
description: "Validate API contracts, detect breaking changes, and generate test data"
branding:
  icon: "shield"
  color: "orange"

inputs:
  version:
    description: "ValiForge version to install (default: latest)"
    required: false
    default: "latest"
  schema:
    description: "Path to API schema file (OpenAPI, Protobuf, or GraphQL SDL)"
    required: true
  target:
    description: "Base URL of the API server to validate against"
    required: false
  config:
    description: "Path to valiforge.toml config file"
    required: false
  fail-on:
    description: "Comma-separated list of failure conditions"
    required: false
    default: "breaking-changes,schema-violations"
  format:
    description: "Output format (json, junit, markdown, terminal)"
    required: false
    default: "junit"
  generate-data:
    description: "Enable test data generation"
    required: false
    default: "false"
  comment-on-pr:
    description: "Post results as PR comment"
    required: false
    default: "true"
  sarif:
    description: "Generate SARIF output for GitHub Code Scanning"
    required: false
    default: "false"

outputs:
  result:
    description: "Validation result (pass/fail)"
    value: ${{ steps.validate.outputs.result }}
  report-path:
    description: "Path to the generated report file"
    value: ${{ steps.validate.outputs.report-path }}
  violations-count:
    description: "Number of violations found"
    value: ${{ steps.validate.outputs.violations-count }}

runs:
  using: "composite"
  steps:
    - name: Install ValiForge
      shell: bash
      run: |
        if [ "${{ inputs.version }}" = "latest" ]; then
          VERSION=$(curl -fsSL https://api.github.com/repos/valiforge/valiforge/releases/latest | grep '"tag_name"' | sed -E 's/.*"v([^"]+)".*/\1/')
        else
          VERSION="${{ inputs.version }}"
        fi

        OS=$(uname -s | tr '[:upper:]' '[:lower:]')
        ARCH=$(uname -m)
        case "$OS" in linux) OS="unknown-linux-musl" ;; darwin) OS="apple-darwin" ;; esac
        case "$ARCH" in x86_64|amd64) ARCH="x86_64" ;; aarch64|arm64) ARCH="aarch64" ;; esac
        TARGET="${ARCH}-${OS}"

        curl -fsSL "https://github.com/valiforge/valiforge/releases/download/v${VERSION}/valiforge-v${VERSION}-${TARGET}.tar.gz" | tar xz
        sudo install valiforge /usr/local/bin/valiforge
        echo "Installed ValiForge v${VERSION}"

    - name: Cache ValiForge data
      uses: actions/cache@v4
      with:
        path: ~/.valiforge
        key: valiforge-${{ runner.os }}-${{ hashFiles(inputs.schema) }}

    - name: Run Validation
      id: validate
      shell: bash
      run: |
        ARGS="validate"
        [ -n "${{ inputs.schema }}" ] && ARGS="$ARGS --schema ${{ inputs.schema }}"
        [ -n "${{ inputs.target }}" ] && ARGS="$ARGS --target ${{ inputs.target }}"
        [ -n "${{ inputs.config }}" ] && ARGS="$ARGS --config ${{ inputs.config }}"
        [ -n "${{ inputs.fail-on }}" ] && ARGS="$ARGS --fail-on ${{ inputs.fail-on }}"

        ARGS="$ARGS --format ${{ inputs.format }}"
        ARGS="$ARGS --output valiforge-report.${{ inputs.format == 'junit' && 'xml' || inputs.format }}"

        if [ "${{ inputs.generate-data }}" = "true" ]; then
          ARGS="$ARGS --generate-data"
        fi

        if [ "${{ inputs.sarif }}" = "true" ]; then
          ARGS="$ARGS --sarif valiforge-results.sarif"
        fi

        set +e
        valiforge $ARGS
        EXIT_CODE=$?
        set -e

        if [ $EXIT_CODE -eq 0 ]; then
          echo "result=pass" >> $GITHUB_OUTPUT
          echo "violations-count=0" >> $GITHUB_OUTPUT
        else
          echo "result=fail" >> $GITHUB_OUTPUT
          VIOLATIONS=$(valiforge $ARGS --format json 2>/dev/null | jq '.violations | length' 2>/dev/null || echo "unknown")
          echo "violations-count=$VIOLATIONS" >> $GITHUB_OUTPUT
        fi

        echo "report-path=valiforge-report.${{ inputs.format == 'junit' && 'xml' || inputs.format }}" >> $GITHUB_OUTPUT
        exit $EXIT_CODE

    - name: Upload JUnit Results
      if: always() && inputs.format == 'junit'
      uses: dorny/test-reporter@v1
      with:
        name: ValiForge Results
        path: valiforge-report.xml
        reporter: java-junit

    - name: Upload SARIF
      if: always() && inputs.sarif == 'true'
      uses: github/codeql-action/upload-sarif@v3
      with:
        sarif_file: valiforge-results.sarif

    - name: Comment on PR
      if: always() && github.event_name == 'pull_request' && inputs.comment-on-pr == 'true'
      uses: actions/github-script@v7
      with:
        script: |
          const fs = require('fs');
          const result = '${{ steps.validate.outputs.result }}';
          const violations = '${{ steps.validate.outputs.violations-count }}';
          const icon = result === 'pass' ? ':white_check_mark:' : ':x:';

          let body = `## ${icon} ValiForge API Validation\n\n`;
          body += `**Result:** ${result.toUpperCase()}\n`;
          body += `**Violations:** ${violations}\n\n`;

          try {
            const report = fs.readFileSync('valiforge-report.xml', 'utf8');
            body += '<details><summary>Full Report</summary>\n\n```\n' + report.substring(0, 60000) + '\n```\n</details>';
          } catch (e) {}

          // Find and update existing comment or create new one
          const { data: comments } = await github.rest.issues.listComments({
            owner: context.repo.owner,
            repo: context.repo.repo,
            issue_number: context.issue.number,
          });
          const existing = comments.find(c => c.body.includes('ValiForge API Validation'));
          if (existing) {
            await github.rest.issues.updateComment({
              owner: context.repo.owner,
              repo: context.repo.repo,
              comment_id: existing.id,
              body: body,
            });
          } else {
            await github.rest.issues.createComment({
              owner: context.repo.owner,
              repo: context.repo.repo,
              issue_number: context.issue.number,
              body: body,
            });
          }
```

**Estimated Effort:** 3 days
**Dependencies:** INFRA-010, INFRA-018
**Acceptance Criteria:**
- `uses: valiforge/action@v1` works in any GitHub Actions workflow
- Binary installation takes < 10 seconds (direct download, no compilation)
- Schema-based caching reduces repeated runs
- JUnit XML results appear in GitHub's test summary UI
- SARIF results appear in GitHub's Code Scanning tab
- PR comment posted/updated with validation results
- Action outputs expose result, report path, and violation count
- Works on `ubuntu-latest`, `macos-latest`, and `windows-latest` runners

**Best Practices:**
- Use composite action (not Docker action) for cross-platform support
- Update existing PR comment instead of creating duplicates
- Always run upload steps with `if: always()` so results post even on failure
- Cache the `~/.valiforge` directory for baselines and config
- Pin action to major version (`@v1`), use tags for minor/patch

**Pitfalls to Avoid:**
- Do NOT use Docker-based action (limits to Linux runners only)
- Do NOT require users to pre-install dependencies
- Do NOT forget `set +e` before validation command (must capture exit code)
- Do NOT post multi-megabyte reports as PR comments (GitHub has a 65536 char limit)

---

### INFRA-024: Example Workflow Files

**Description:**
Create ready-to-use example workflow files that users can copy into their repositories.

Examples to create:
1. **Basic validation** -- Validate OpenAPI schema on every PR
2. **Breaking change detection** -- Diff schema against main branch
3. **Full integration test** -- Start API server, run ValiForge, upload results
4. **Multi-protocol** -- Validate REST + gRPC + GraphQL in one workflow
5. **Scheduled validation** -- Nightly validation against staging environment

```yaml
# examples/workflows/basic-validation.yml
name: API Validation
on: [pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: valiforge/action@v1
        with:
          schema: ./openapi.yaml
          fail-on: breaking-changes,schema-violations
```

```yaml
# examples/workflows/breaking-change-detection.yml
name: Breaking Change Detection
on:
  pull_request:
    paths: ['**/openapi.yaml', '**/openapi.json']

jobs:
  diff:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - name: Get base schema
        run: git show origin/${{ github.base_ref }}:openapi.yaml > /tmp/base-schema.yaml
      - uses: valiforge/action@v1
        with:
          command: diff
          args: /tmp/base-schema.yaml openapi.yaml
          fail-on: breaking-changes
          comment-on-pr: true
```

```yaml
# examples/workflows/full-integration.yml
name: Full API Integration Test
on: [pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_PASSWORD: test
        ports: ['5432:5432']
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20
      - run: npm ci && npm run build && npm start &
      - run: sleep 5  # Wait for server
      - uses: valiforge/action@v1
        with:
          schema: ./openapi.yaml
          target: http://localhost:3000
          generate-data: true
          format: junit
          sarif: true
```

**Estimated Effort:** 1 day
**Dependencies:** INFRA-023
**Acceptance Criteria:**
- 5+ example workflows covering common use cases
- Each example is tested in a real repository
- Examples include inline comments explaining each step
- Examples are referenced from the main documentation

**Best Practices:**
- Keep examples minimal and focused on one use case each
- Use realistic file paths and URLs
- Include comments for non-obvious steps

**Pitfalls to Avoid:**
- Do NOT make examples too complex; users should be able to copy-paste
- Do NOT use deprecated actions in examples

---

### INFRA-025: Matrix Testing Against API Frameworks

**Description:**
Create a CI workflow that tests ValiForge against real API server starters for Node.js (Express/Fastify), Python (FastAPI/Flask), Go (Gin/Echo), and Rust (Axum/Actix). This validates ValiForge works with real-world APIs, not just mock schemas.

```yaml
# .github/workflows/integration-matrix.yml
name: Integration Matrix
on:
  push:
    branches: [main]
  schedule:
    - cron: '0 6 * * 1'  # Weekly Monday 6am UTC

jobs:
  test:
    name: ${{ matrix.framework }}
    strategy:
      matrix:
        include:
          - framework: express
            setup: "cd test-servers/express && npm ci && npm start &"
            port: 3000
          - framework: fastapi
            setup: "cd test-servers/fastapi && pip install -r requirements.txt && uvicorn main:app --port 3000 &"
            port: 3000
          - framework: gin
            setup: "cd test-servers/gin && go run . &"
            port: 3000
          - framework: axum
            setup: "cd test-servers/axum && cargo run --release &"
            port: 3000
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Start API server
        run: ${{ matrix.setup }}
      - name: Wait for server
        run: timeout 60 bash -c 'until curl -s http://localhost:${{ matrix.port }}/health; do sleep 1; done'
      - name: Run ValiForge
        run: |
          cargo run -p valiforge-cli -- validate \
            --schema test-servers/${{ matrix.framework }}/openapi.yaml \
            --target http://localhost:${{ matrix.port }}
```

**Estimated Effort:** 3 days
**Dependencies:** INFRA-003, requires ValiForge CLI to be functional
**Acceptance Criteria:**
- ValiForge validates successfully against 4+ real API frameworks
- Test servers include OpenAPI specs
- Integration tests run weekly and on main branch pushes
- Failures are reported and triaged

**Best Practices:**
- Use health-check polling instead of `sleep` for server readiness
- Keep test servers minimal (3-5 endpoints each)
- Store test servers in the monorepo under `test-servers/`

**Pitfalls to Avoid:**
- Do NOT assume server starts instantly; use health-check polling
- Do NOT run these on every PR (too slow); weekly + main branch is sufficient

---

## 5. ValiForge Cloud Infrastructure (Phase 2)

### INFRA-026: Cloud Architecture Design

**Description:**
Design the ValiForge Cloud architecture. This is the monetization layer built on top of the open-source CLI.

Architecture:
```
                    ┌─────────────────────────────────────────────┐
                    │            Cloudflare CDN / R2               │
                    │  (Binary downloads, static assets, edge)     │
                    └──────────────┬──────────────────────────────┘
                                   │
                    ┌──────────────▼──────────────────────────────┐
                    │         Load Balancer (Cloud LB / Fly.io)    │
                    └──────────────┬──────────────────────────────┘
                                   │
              ┌────────────────────┼────────────────────┐
              │                    │                     │
    ┌─────────▼────────┐ ┌────────▼─────────┐ ┌────────▼─────────┐
    │  API Server       │ │  API Server       │ │  API Server       │
    │  (Rust/Axum)      │ │  (Rust/Axum)      │ │  (Rust/Axum)      │
    │  - Auth           │ │  - Validation      │ │  - Reports         │
    │  - Billing        │ │  - Schema storage  │ │  - Dashboard API   │
    │  - Teams          │ │  - SLM inference   │ │  - Webhooks        │
    └─────────┬────────┘ └────────┬─────────┘ └────────┬─────────┘
              │                    │                     │
              └────────────────────┼────────────────────┘
                                   │
              ┌────────────────────┼────────────────────┐
              │                    │                     │
    ┌─────────▼────────┐ ┌────────▼─────────┐ ┌────────▼─────────┐
    │  PostgreSQL       │ │  Redis            │ │  Object Storage   │
    │  (Neon / RDS)     │ │  (Upstash / EC)   │ │  (Cloudflare R2)  │
    │  - Users/Teams    │ │  - Sessions       │ │  - Schemas         │
    │  - Schemas        │ │  - Rate limits    │ │  - Reports         │
    │  - Results        │ │  - Cache          │ │  - Binaries        │
    │  - Billing        │ │  - Pub/Sub        │ │  - SLM models      │
    └──────────────────┘ └──────────────────┘ └──────────────────┘
              │
    ┌─────────▼────────┐
    │  Dashboard        │
    │  (Next.js on      │
    │   Vercel)         │
    └──────────────────┘
```

Technology choices:
| Component | Choice | Rationale |
|-----------|--------|-----------|
| API server | Rust + Axum | Performance, type safety, shared code with CLI |
| Database | PostgreSQL (Neon serverless) | Serverless scaling, branching for dev/staging |
| Cache | Redis (Upstash serverless) | Session management, rate limiting, pub/sub |
| Object storage | Cloudflare R2 | S3-compatible, zero egress fees, global CDN |
| Auth | GitHub OAuth + API keys | Developer-first auth, no friction |
| Billing | Stripe | Industry standard, usage-based billing support |
| Dashboard | Next.js on Vercel | Fast iteration, SSR, great DX |
| Deployment | Fly.io (initial) -> Kubernetes (scale) | Fly.io for speed to market, K8s for enterprise |
| CDN | Cloudflare | Binary downloads, DDoS protection, edge caching |
| Monitoring | Grafana Cloud | Prometheus metrics, Loki logs, unified dashboards |
| Error tracking | Sentry | Rust + JS SDKs, release tracking |
| Logging | tracing crate -> JSON | Structured logs, OpenTelemetry compatible |

**Estimated Effort:** 3 days (design only)
**Dependencies:** None (design task)
**Acceptance Criteria:**
- Architecture document with diagram reviewed by team
- Technology choices documented with rationale
- Cost estimates for each tier (startup through scale)
- Data model draft for PostgreSQL
- API endpoint design for cloud features
- Security model documented (auth, multi-tenancy, data isolation)

**Best Practices:**
- Start with Fly.io for simplicity; migrate to K8s only when needed
- Use Neon for PostgreSQL (serverless, branching = free staging DBs)
- Use Cloudflare R2 for all object storage (zero egress = major cost savings)
- Share Rust code between CLI and cloud API server via workspace crates

**Pitfalls to Avoid:**
- Do NOT over-architect for scale on day one; start simple
- Do NOT build Kubernetes from day one (Fly.io is 10x simpler for <100 instances)
- Do NOT use a separate language for the API server (Rust reuse is a superpower)
- Do NOT build auth from scratch; use established providers

---

### INFRA-027: Authentication & Multi-Tenancy

**Description:**
Implement authentication (GitHub OAuth + API keys) and multi-tenant data isolation for ValiForge Cloud.

Auth flow:
1. User clicks "Sign in with GitHub" on dashboard
2. GitHub OAuth flow redirects to ValiForge callback
3. Create/update user record, issue JWT session token
4. For CI/CD: user generates API key in dashboard, passes as `VALIFORGE_API_KEY`

Multi-tenancy model:
- Row-level security in PostgreSQL (tenant_id on every table)
- API keys scoped to team
- All queries filtered by tenant_id via middleware
- No shared state between tenants

Database schema (core tables):
```sql
CREATE TABLE teams (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL,
  slug TEXT UNIQUE NOT NULL,
  plan TEXT NOT NULL DEFAULT 'free',
  stripe_customer_id TEXT,
  created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE users (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  github_id BIGINT UNIQUE NOT NULL,
  email TEXT NOT NULL,
  name TEXT,
  avatar_url TEXT,
  created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE team_members (
  team_id UUID REFERENCES teams(id),
  user_id UUID REFERENCES users(id),
  role TEXT NOT NULL DEFAULT 'member',
  PRIMARY KEY (team_id, user_id)
);

CREATE TABLE api_keys (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  team_id UUID REFERENCES teams(id) NOT NULL,
  name TEXT NOT NULL,
  key_hash TEXT UNIQUE NOT NULL,  -- SHA256 of the key
  prefix TEXT NOT NULL,            -- First 8 chars for identification
  scopes TEXT[] DEFAULT '{}',
  last_used_at TIMESTAMPTZ,
  expires_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE validation_results (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  team_id UUID REFERENCES teams(id) NOT NULL,
  schema_name TEXT NOT NULL,
  result TEXT NOT NULL,  -- 'pass' | 'fail'
  violations_count INT DEFAULT 0,
  report JSONB,
  ci_metadata JSONB,  -- PR number, commit SHA, branch, etc.
  created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Row-level security
ALTER TABLE validation_results ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON validation_results
  USING (team_id = current_setting('app.current_team_id')::UUID);
```

**Estimated Effort:** 5 days
**Dependencies:** INFRA-026
**Acceptance Criteria:**
- GitHub OAuth login works end-to-end
- API key generation, rotation, and revocation works
- Multi-tenant isolation verified (user A cannot see user B's data)
- Row-level security policies on all tenant-scoped tables
- API key authentication works in CI/CD environments
- Rate limiting per API key (configurable per plan)

**Best Practices:**
- NEVER store API keys in plaintext; store SHA256 hash only
- Use a `prefix` field (first 8 chars) so users can identify keys without exposing them
- Implement key rotation (create new, deprecate old, with grace period)
- Use PostgreSQL RLS for defense-in-depth (even if app layer filters correctly)

**Pitfalls to Avoid:**
- Do NOT use JWT for API keys (they cannot be revoked server-side)
- Do NOT skip RLS (application bugs can leak data between tenants)
- Do NOT hardcode rate limits; make them configurable per plan
- Do NOT store OAuth tokens longer than needed; use short-lived sessions

---

### INFRA-028: Billing Integration (Stripe)

**Description:**
Integrate Stripe for subscription management, usage-based billing, and team plan upgrades.

Billing model:
- Free tier: No Stripe integration needed
- Team/Business: Per-seat monthly subscription
- Enterprise: Custom invoicing (manual Stripe invoices)
- Usage add-on: Hosted SLM generations billed per 1K generations

Implementation:
1. Stripe Checkout for plan upgrades
2. Stripe Customer Portal for self-service management
3. Stripe Webhooks for subscription lifecycle events
4. Usage metering via Stripe Usage Records

**Estimated Effort:** 4 days
**Dependencies:** INFRA-027
**Acceptance Criteria:**
- Users can upgrade from Free to Team/Business via Stripe Checkout
- Subscription changes (upgrade, downgrade, cancel) reflected immediately
- Usage-based billing for SLM generations tracked accurately
- Stripe webhooks handle all lifecycle events (payment failure, cancellation, etc.)
- Billing page shows current plan, usage, and invoices

**Best Practices:**
- Use Stripe Checkout (hosted) instead of custom payment forms (PCI compliance)
- Use Stripe Customer Portal for billing management (zero UI maintenance)
- Implement webhook idempotency (Stripe retries on failure)
- Test with Stripe test mode extensively before going live

**Pitfalls to Avoid:**
- Do NOT build custom billing UI when Stripe Customer Portal exists
- Do NOT forget to handle failed payments (dunning flow)
- Do NOT assume webhooks arrive in order; use event timestamps
- Do NOT store credit card data anywhere (Stripe handles this)

---

### INFRA-029: Dashboard Frontend

**Description:**
Build the ValiForge Cloud dashboard as a Next.js application deployed on Vercel.

Pages:
- `/` -- Landing page (marketing)
- `/dashboard` -- Overview (validation trends, recent runs)
- `/dashboard/results` -- Validation results list + detail view
- `/dashboard/schemas` -- Managed schemas
- `/dashboard/team` -- Team management (members, roles)
- `/dashboard/settings` -- API keys, integrations, billing
- `/dashboard/billing` -- Plan management (Stripe Customer Portal redirect)

Tech stack:
- Next.js 14+ (App Router)
- Tailwind CSS + shadcn/ui
- React Query for data fetching
- Recharts for trend visualization

**Estimated Effort:** 10 days
**Dependencies:** INFRA-027, INFRA-028
**Acceptance Criteria:**
- Dashboard deployed on Vercel with custom domain
- GitHub OAuth login flow works
- Real-time validation result display
- Team management (invite, remove, role change)
- API key management (create, revoke, list)
- Billing integration (upgrade, manage subscription)
- Responsive design (desktop + mobile)

**Best Practices:**
- Use Next.js App Router with server components for performance
- Use shadcn/ui for consistent, accessible components
- Implement optimistic updates for better perceived performance
- Use React Query for caching and background refetch

**Pitfalls to Avoid:**
- Do NOT build the dashboard before the API is stable
- Do NOT over-build v1; ship minimal viable dashboard first
- Do NOT forget error states and loading states
- Do NOT skip mobile responsiveness (developers check dashboards on phones)

---

### INFRA-030: Cloud Deployment (Fly.io Initial)

**Description:**
Deploy ValiForge Cloud API to Fly.io for initial launch. Migrate to Kubernetes when scale demands it.

Fly.io configuration:
```toml
# fly.toml
app = "valiforge-api"
primary_region = "iad"

[build]
  dockerfile = "docker/Dockerfile.cloud"

[env]
  RUST_LOG = "info"
  DATABASE_URL = "postgres://..."  # Set via fly secrets

[http_service]
  internal_port = 8080
  force_https = true
  auto_stop_machines = true
  auto_start_machines = true
  min_machines_running = 1

[[vm]]
  cpu_kind = "shared"
  cpus = 2
  memory_mb = 512
```

Deployment pipeline:
1. Merge to `main` triggers deploy to staging (`valiforge-api-staging`)
2. Manual promotion to production (`valiforge-api`)
3. Fly.io handles rolling deployments with health checks

**Estimated Effort:** 3 days
**Dependencies:** INFRA-026, INFRA-027
**Acceptance Criteria:**
- API deployed and accessible at `api.valiforge.dev`
- Staging environment at `api-staging.valiforge.dev`
- Health check endpoint returns 200
- Auto-scaling works (min 1, max configurable)
- Secrets managed via `fly secrets`
- Deploy triggered by CI on merge to main

**Best Practices:**
- Use Fly.io's auto-start/auto-stop for cost savings during low traffic
- Deploy staging automatically, production manually
- Use Fly.io's built-in metrics before adding Grafana
- Set `min_machines_running = 1` to avoid cold starts

**Pitfalls to Avoid:**
- Do NOT use Fly.io volumes for data (use external DB)
- Do NOT deploy to production without staging verification
- Do NOT forget to set resource limits (OOM kills are silent)

---

### INFRA-031: CDN for Binary Downloads (Cloudflare R2)

**Description:**
Set up Cloudflare R2 as the CDN for binary downloads, replacing direct GitHub Release downloads for production traffic. This reduces GitHub API rate limiting exposure and provides global edge caching.

Setup:
1. Create R2 bucket `valiforge-releases`
2. Configure custom domain `dl.valiforge.dev` pointing to R2
3. Upload release artifacts to R2 as part of release CI
4. Update install script and Homebrew formula to use R2 URLs

**Estimated Effort:** 1 day
**Dependencies:** INFRA-010
**Acceptance Criteria:**
- `dl.valiforge.dev/v1.0.0/valiforge-v1.0.0-x86_64-unknown-linux-musl.tar.gz` serves binaries
- R2 bucket synced automatically on each release
- Fallback to GitHub Releases if R2 is unavailable
- Zero egress fees (Cloudflare R2 benefit)
- Global edge caching for fast downloads worldwide

**Best Practices:**
- Use Cloudflare R2 (not S3) for zero egress fees
- Keep GitHub Releases as the source of truth; R2 is a cache/CDN layer
- Set appropriate Cache-Control headers (immutable for versioned artifacts)

**Pitfalls to Avoid:**
- Do NOT delete GitHub Release artifacts when uploading to R2
- Do NOT forget to upload checksums alongside binaries

---

### INFRA-032: Monitoring & Observability Stack

**Description:**
Implement monitoring, logging, and alerting for ValiForge Cloud.

Stack:
- **Metrics:** Prometheus (via Grafana Cloud) -- HTTP latency, error rates, validation counts
- **Logging:** Structured JSON logs via `tracing` crate -> Grafana Loki
- **Error tracking:** Sentry (Rust + JS SDKs)
- **Uptime monitoring:** Grafana Synthetic Monitoring or Checkly
- **Alerting:** Grafana alerting -> Slack/PagerDuty

Key metrics to track:
- API request latency (P50, P95, P99)
- Error rate by endpoint
- Validation runs per minute
- Active connections
- Database query latency
- Redis hit/miss ratio
- Binary download count by platform

Rust implementation using `tracing`:
```rust
use tracing::{info, instrument};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[instrument(skip(pool))]
async fn validate_schema(pool: &PgPool, req: ValidateRequest) -> Result<ValidateResponse> {
    info!(schema = %req.schema_name, "Starting validation");
    // ...
}
```

**Estimated Effort:** 3 days
**Dependencies:** INFRA-030
**Acceptance Criteria:**
- Grafana dashboard showing key metrics
- Structured JSON logs searchable in Loki
- Sentry captures and groups errors automatically
- Uptime monitoring alerts within 2 minutes of downtime
- Alert channels configured (Slack + email)

**Best Practices:**
- Use `tracing` crate (not `log`) for structured, async-aware logging
- Export metrics in Prometheus format (industry standard)
- Set up dashboards before launch (not after first incident)
- Use Sentry release tracking to correlate errors with deploys

**Pitfalls to Avoid:**
- Do NOT log sensitive data (API keys, tokens, PII)
- Do NOT alert on every 4xx error (high noise, low signal)
- Do NOT use print statements for logging in Rust; always use `tracing`

---

## 6. Telemetry & Analytics (Opt-In)

### INFRA-033: Anonymous Telemetry System

**Description:**
Implement privacy-first, opt-in telemetry for the CLI to understand usage patterns and prioritize development.

What to collect (anonymous, no PII):
- ValiForge version
- OS and architecture
- Command invoked (validate, diff, generate, etc.)
- Schema format (openapi, protobuf, graphql)
- Endpoint count (bucket: 1-10, 11-50, 51-200, 200+)
- Validation duration (bucket: <1s, 1-5s, 5-30s, 30s+)
- Error category (if failed): config, schema-parse, network, validation
- CI environment detected (github-actions, gitlab-ci, jenkins, unknown, none)

What NOT to collect:
- IP addresses (use Cloudflare Workers to strip)
- Schema content
- API URLs
- File paths
- Error messages (could contain PII)

Implementation:
```rust
// Telemetry is off by default, enabled via:
// 1. valiforge telemetry enable
// 2. VALIFORGE_TELEMETRY=on environment variable
// 3. telemetry = true in valiforge.toml

pub struct TelemetryEvent {
    pub event: String,
    pub version: String,
    pub os: String,
    pub arch: String,
    pub schema_format: Option<String>,
    pub endpoint_count_bucket: Option<String>,
    pub duration_bucket: Option<String>,
    pub ci_environment: Option<String>,
    pub error_category: Option<String>,
    pub timestamp: i64,
    pub session_id: Uuid,  // Random per-invocation, not persistent
}
```

Backend: Cloudflare Workers -> Cloudflare Analytics Engine (or simple append to R2 for batch processing).

**Estimated Effort:** 3 days
**Dependencies:** INFRA-030 (for telemetry endpoint)
**Acceptance Criteria:**
- Telemetry is OFF by default
- `valiforge telemetry enable` / `valiforge telemetry disable` commands
- `VALIFORGE_TELEMETRY=off` env var disables telemetry
- Telemetry POST is fire-and-forget (async, non-blocking, no retries)
- No PII collected (verified by security review)
- Telemetry documentation page explains exactly what is collected
- Telemetry fails silently (never affects CLI functionality)
- 2-second timeout on telemetry HTTP request (never delays the CLI)

**Best Practices:**
- Model after `rustup` and `cargo`'s telemetry approach
- Use Cloudflare Workers to strip IP before storage
- Bucket numeric values instead of sending exact numbers
- Use a random session_id per invocation (not a persistent user ID)
- Document everything in a public telemetry page

**Pitfalls to Avoid:**
- Do NOT enable telemetry by default (trust erosion, potential GDPR issues)
- Do NOT collect error messages (may contain file paths or PII)
- Do NOT block CLI execution on telemetry (fire-and-forget only)
- Do NOT retry telemetry sends (if endpoint is down, data is lost, that is fine)
- Do NOT use a persistent device ID (privacy concern)

---

## 7. Security & Compliance

### INFRA-034: Security Audit Pipeline (Nightly)

**Description:**
Implement a nightly security audit pipeline that runs `cargo audit`, `cargo deny`, and container scanning on a schedule.

```yaml
# .github/workflows/nightly.yml
name: Nightly Security Audit
on:
  schedule:
    - cron: '0 4 * * *'  # 4am UTC daily
  workflow_dispatch:

jobs:
  cargo-audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: rustsec/audit-check@v2
        with:
          token: ${{ secrets.GITHUB_TOKEN }}

  cargo-deny:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v1

  container-scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build image
        run: docker build -t valiforge:scan -f docker/Dockerfile .
      - name: Run Trivy
        uses: aquasecurity/trivy-action@master
        with:
          image-ref: 'valiforge:scan'
          format: 'sarif'
          output: 'trivy-results.sarif'
          severity: 'CRITICAL,HIGH'
      - name: Upload SARIF
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: 'trivy-results.sarif'
```

**Estimated Effort:** 1 day
**Dependencies:** INFRA-003, INFRA-017
**Acceptance Criteria:**
- Nightly audit runs at 4am UTC
- `cargo audit` detects known vulnerabilities
- `cargo deny` checks license compliance
- Trivy scans Docker image for CVEs
- Results uploaded to GitHub Security tab (SARIF)
- Slack notification on critical/high findings
- Manual trigger available via `workflow_dispatch`

**Best Practices:**
- Run nightly, not just on PRs (new CVEs are published daily)
- Upload SARIF to GitHub Security tab for centralized view
- Set up Slack alerts for critical findings (do not rely on email)
- Use `--ignore` for acknowledged/accepted risks (with documented justification)

**Pitfalls to Avoid:**
- Do NOT ignore nightly audit failures (they become critical debt)
- Do NOT suppress all warnings; triage and document accepted risks

---

### INFRA-035: SBOM Generation

**Description:**
Generate Software Bill of Materials (SBOM) in CycloneDX or SPDX format for every release. Required for enterprise customers and SOC 2 compliance.

```yaml
# In release workflow
- name: Generate SBOM
  run: |
    cargo install cargo-cyclonedx
    cargo cyclonedx --format json --output-file valiforge-sbom.cdx.json
- name: Upload SBOM to release
  uses: softprops/action-gh-release@v2
  with:
    files: valiforge-sbom.cdx.json
```

**Estimated Effort:** 0.5 days
**Dependencies:** INFRA-010
**Acceptance Criteria:**
- CycloneDX SBOM generated for every release
- SBOM attached to GitHub Release as artifact
- SBOM includes all transitive dependencies
- SBOM validates against CycloneDX schema

**Best Practices:**
- Use CycloneDX (more adoption in security tooling than SPDX)
- Generate SBOM from Cargo.lock for reproducibility
- Include SBOM in Docker image labels

**Pitfalls to Avoid:**
- Do NOT generate SBOM from Cargo.toml (misses transitive deps); use Cargo.lock

---

### INFRA-036: Security Policy & Vulnerability Disclosure

**Description:**
Create SECURITY.md with vulnerability reporting instructions and establish a responsible disclosure process.

```markdown
# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.x.x   | Yes (current) |

## Reporting a Vulnerability

Please report security vulnerabilities via GitHub Security Advisories:
https://github.com/valiforge/valiforge/security/advisories/new

DO NOT open a public issue for security vulnerabilities.

We will respond within 48 hours and aim to release patches within 7 days for critical vulnerabilities.

## Security Practices
- All dependencies audited nightly via cargo audit
- License compliance checked via cargo deny
- Container images scanned via Trivy
- SBOM published with every release
```

**Estimated Effort:** 0.5 days
**Dependencies:** None
**Acceptance Criteria:**
- SECURITY.md in repository root
- GitHub Security Advisories enabled
- Process documented for receiving, triaging, and patching vulnerabilities
- Response time SLA documented (48 hours acknowledgment, 7 days for critical patches)

**Best Practices:**
- Use GitHub Security Advisories (private disclosure + CVE assignment)
- Define severity levels and response time SLAs
- Credit reporters in security advisories

**Pitfalls to Avoid:**
- Do NOT use a public issue tracker for vulnerability reports
- Do NOT skip this document; it is required for enterprise adoption

---

### INFRA-037: SOC 2 Type II Preparation Checklist

**Description:**
Create a preparation checklist and begin implementing controls for SOC 2 Type II certification (required for enterprise tier at Month 9-12).

Key control areas:
1. **Access Control:** RBAC, MFA enforcement, least privilege
2. **Change Management:** PR reviews, CI gates, deployment approvals
3. **Monitoring:** Logging, alerting, incident response
4. **Data Protection:** Encryption at rest and in transit, data retention policies
5. **Vendor Management:** Third-party risk assessment (Fly.io, Neon, Stripe, etc.)
6. **Incident Response:** Documented playbook, on-call rotation
7. **Business Continuity:** Backup strategy, disaster recovery

Implementation priority (Phase 2):
- Month 7-8: Access control, change management, monitoring (most already done via CI/CD practices)
- Month 9-10: Data protection, vendor management
- Month 11-12: Incident response, business continuity, auditor engagement

**Estimated Effort:** 5 days (checklist + initial controls; full SOC 2 is ongoing)
**Dependencies:** INFRA-027, INFRA-032
**Acceptance Criteria:**
- SOC 2 preparation checklist document created
- Gap analysis completed against Trust Services Criteria
- Priority controls implemented (access control, change management, monitoring)
- Auditor shortlist (Vanta, Drata, or Secureframe for automation)

**Best Practices:**
- Use SOC 2 automation platforms (Vanta/Drata/Secureframe) to reduce manual work by 70%
- Start early; SOC 2 Type II requires 3-6 months of evidence collection
- Focus on controls you already have (CI/CD, code review, monitoring) and formalize them

**Pitfalls to Avoid:**
- Do NOT start SOC 2 the month before you need it (minimum 6-month lead time)
- Do NOT try to do it manually; automation platforms pay for themselves
- Do NOT forget that SOC 2 Type I is a shortcut to Type II (get Type I first)

---

## 8. Performance Monitoring & Benchmarks

### INFRA-038: Criterion Benchmark Suite in CI

**Description:**
Implement Criterion benchmarks for critical hot paths and run them in CI to detect performance regressions.

Benchmarks to implement:
1. OpenAPI schema parsing (small, medium, large specs)
2. Schema validation (single endpoint, 100 endpoints)
3. Breaking change detection (diff two schemas)
4. Test data generation (grammar-based, no SLM)
5. Report generation (JSON, JUnit XML)
6. CLI cold start (time to first output)

```rust
// benches/schema_parse.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_parse_openapi_small(c: &mut Criterion) {
    let schema = include_str!("../fixtures/petstore.yaml");
    c.bench_function("parse_openapi_small", |b| {
        b.iter(|| valiforge_schema::parse_openapi(black_box(schema)))
    });
}

fn bench_parse_openapi_large(c: &mut Criterion) {
    let schema = include_str!("../fixtures/stripe-api.yaml");
    c.bench_function("parse_openapi_large", |b| {
        b.iter(|| valiforge_schema::parse_openapi(black_box(schema)))
    });
}

criterion_group!(benches, bench_parse_openapi_small, bench_parse_openapi_large);
criterion_main!(benches);
```

CI integration using `critcmp` or `github-action-benchmark`:
```yaml
# .github/workflows/benchmark.yml
name: Benchmarks
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Run benchmarks
        run: cargo bench --workspace -- --output-format bencher | tee bench-output.txt

      - name: Store benchmark result
        uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'cargo'
          output-file-path: bench-output.txt
          github-token: ${{ secrets.GITHUB_TOKEN }}
          auto-push: true
          alert-threshold: '110%'
          comment-on-alert: true
          fail-on-alert: false
          alert-comment-cc-users: '@kaushikreddy'
```

**Estimated Effort:** 2 days
**Dependencies:** INFRA-003, requires core crates to have benchmarkable functions
**Acceptance Criteria:**
- 6+ benchmarks covering critical paths
- Benchmarks run on every push to main
- Performance regression > 10% triggers PR comment alert
- Historical benchmark data stored in GitHub Pages
- Benchmark results visible at `https://valiforge.github.io/valiforge/dev/bench/`

**Best Practices:**
- Use `criterion` (not `#[bench]` which requires nightly)
- Use `black_box` to prevent compiler from optimizing away the benchmark
- Run benchmarks on a consistent machine (same GitHub Actions runner type)
- Set `alert-threshold: '110%'` (10% regression tolerance for CI noise)
- Do NOT fail builds on benchmark regression (alert only; CI runners are noisy)

**Pitfalls to Avoid:**
- Do NOT use `fail-on-alert: true` initially (CI runner variance causes false positives)
- Do NOT benchmark I/O-bound operations without controlling for network latency
- Do NOT run benchmarks on every PR (expensive); run on main + manual trigger for PRs
- Do NOT forget to pin the runner OS version (runner updates can change performance)

---

### INFRA-039: Binary Size Tracking Per Release

**Description:**
Track binary size for each target across releases and display trends.

Implementation: After each release build, record binary sizes and publish to a tracking file or GitHub Pages dashboard.

```yaml
- name: Record binary sizes
  run: |
    echo "## Binary Sizes for ${{ github.ref_name }}" >> size-report.md
    echo "| Target | Size |" >> size-report.md
    echo "|--------|------|" >> size-report.md
    for f in dist/*.tar.gz dist/*.zip; do
      SIZE=$(stat -c%s "$f" 2>/dev/null || stat -f%z "$f")
      echo "| $(basename $f) | $(numfmt --to=iec $SIZE) |" >> size-report.md
    done
```

**Estimated Effort:** 0.5 days
**Dependencies:** INFRA-010
**Acceptance Criteria:**
- Binary sizes recorded for every release
- Size trend visible in a dashboard or markdown file
- CI fails if any binary exceeds 15MB budget
- Size broken down by target triple

**Best Practices:**
- Track compressed and uncompressed sizes
- Alert on size increase > 20% between releases

**Pitfalls to Avoid:**
- Do NOT only track one platform; sizes vary significantly between targets

---

### INFRA-040: Memory & Runtime Profiling Infrastructure

**Description:**
Set up profiling infrastructure for development use (not CI). Provide scripts and documentation for profiling ValiForge with DHAT, Valgrind (Linux), and Instruments (macOS).

Profiling tools:
- **DHAT** (via `dhat-rs` crate): Heap profiling, works on all platforms
- **Valgrind/Massif**: Linux heap profiling
- **heaptrack**: Linux heap profiling with GUI
- **Instruments**: macOS CPU and memory profiling
- **perf**: Linux CPU profiling
- **cargo-flamegraph**: Flamegraph generation

Add feature-gated DHAT integration:
```toml
# Cargo.toml
[features]
dhat-heap = ["dhat"]

[dependencies]
dhat = { version = "0.3", optional = true }
```

```rust
#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    // ... normal main
}
```

**Estimated Effort:** 1 day
**Dependencies:** INFRA-001
**Acceptance Criteria:**
- `cargo run --features dhat-heap -- validate ...` produces heap profile
- Documentation for running each profiling tool
- Profiling scripts in `scripts/` directory
- Memory budget documented: < 50MB without SLM (per PRD)

**Best Practices:**
- Use `dhat-rs` for cross-platform heap profiling (works on macOS, Linux, Windows)
- Use feature flags so profiling code is zero-cost in release builds
- Profile with release builds (debug builds are not representative)

**Pitfalls to Avoid:**
- Do NOT leave profiling code enabled in production builds
- Do NOT profile debug builds (misleading results)
- Do NOT assume macOS and Linux memory profiles are identical

---

## 9. Effort Summary & Sprint Plan

### Total Estimated Effort

| Section | Tasks | Total Days |
|---------|-------|------------|
| 1. Repository & CI/CD | INFRA-001 to INFRA-007 | 9 days |
| 2. Cross-Platform Builds | INFRA-008 to INFRA-012 | 9 days |
| 3. Distribution Channels | INFRA-013 to INFRA-022 | 9.5 days |
| 4. GitHub Actions Integration | INFRA-023 to INFRA-025 | 7 days |
| 5. Cloud Infrastructure | INFRA-026 to INFRA-032 | 29 days |
| 6. Telemetry | INFRA-033 | 3 days |
| 7. Security & Compliance | INFRA-034 to INFRA-037 | 7 days |
| 8. Performance Monitoring | INFRA-038 to INFRA-040 | 3.5 days |
| **TOTAL** | **40 tasks** | **77 days** |

### Sprint Plan (2-Week Sprints)

#### Sprint 1 (Weeks 1-2): Foundation
| Task | Days | Priority |
|------|------|----------|
| INFRA-001: Cargo workspace | 2 | P0 |
| INFRA-002: Toolchain & MSRV | 0.5 | P0 |
| INFRA-003: CI pipeline | 2 | P0 |
| INFRA-004: Branch protection | 1 | P0 |
| INFRA-005: Dependabot | 0.5 | P0 |
| INFRA-006: cargo-deny | 1 | P0 |
| INFRA-008: Build matrix | 1 | P0 |
| INFRA-036: SECURITY.md | 0.5 | P0 |
| **Sprint Total** | **8.5 days** | |

#### Sprint 2 (Weeks 3-4): Build & Release
| Task | Days | Priority |
|------|------|----------|
| INFRA-009: Cross-compilation | 2 | P0 |
| INFRA-010: Release workflow | 3 | P0 |
| INFRA-011: Binary size optimization | 1 | P0 |
| INFRA-007: Release automation | 2 | P0 |
| INFRA-013: GitHub Releases formatting | 0.5 | P0 |
| INFRA-014: crates.io publishing | 1 | P0 |
| **Sprint Total** | **9.5 days** | |

#### Sprint 3 (Weeks 5-6): Distribution
| Task | Days | Priority |
|------|------|----------|
| INFRA-015: Homebrew tap | 1.5 | P0 |
| INFRA-018: Shell installer | 1 | P0 |
| INFRA-017: Docker image | 1.5 | P1 |
| INFRA-023: GitHub Action | 3 | P0 |
| INFRA-024: Example workflows | 1 | P0 |
| INFRA-020: GitLab CI template | 0.5 | P1 |
| **Sprint Total** | **8.5 days** | |

#### Sprint 4 (Weeks 7-8): Extended Distribution & Benchmarks
| Task | Days | Priority |
|------|------|----------|
| INFRA-016: npm wrapper | 2 | P1 |
| INFRA-022: Nix flake | 1 | P2 |
| INFRA-021: AUR package | 0.5 | P2 |
| INFRA-038: Criterion benchmarks | 2 | P1 |
| INFRA-039: Binary size tracking | 0.5 | P1 |
| INFRA-040: Profiling infrastructure | 1 | P2 |
| INFRA-025: Integration matrix tests | 3 | P1 |
| **Sprint Total** | **10 days** | |

#### Sprint 5 (Weeks 9-10): Security & Telemetry
| Task | Days | Priority |
|------|------|----------|
| INFRA-034: Nightly security audit | 1 | P0 |
| INFRA-035: SBOM generation | 0.5 | P1 |
| INFRA-033: Telemetry system | 3 | P1 |
| INFRA-012: macOS code signing | 2 | P2 |
| INFRA-031: Cloudflare R2 CDN | 1 | P1 |
| **Sprint Total** | **7.5 days** | |

#### Sprint 6-8 (Weeks 11-16): Cloud Infrastructure (Phase 2)
| Task | Days | Priority |
|------|------|----------|
| INFRA-026: Cloud architecture design | 3 | P1 |
| INFRA-027: Auth & multi-tenancy | 5 | P1 |
| INFRA-028: Stripe billing | 4 | P1 |
| INFRA-029: Dashboard frontend | 10 | P1 |
| INFRA-030: Fly.io deployment | 3 | P1 |
| INFRA-032: Monitoring stack | 3 | P1 |
| INFRA-037: SOC 2 preparation | 5 | P2 |
| **Sprint Total** | **33 days** | |

---

## 10. Critical Path & Key Risks

### Critical Path

The critical path for the open-source CLI launch (MVP) runs through:

```
INFRA-001 (Workspace)
  → INFRA-002 (Toolchain)
    → INFRA-003 (CI Pipeline)
      → INFRA-008 (Build Matrix)
        → INFRA-009 (Cross-Compilation)
          → INFRA-010 (Release Workflow)
            → INFRA-007 (Release Automation)
              → INFRA-013 (GitHub Releases)
              → INFRA-014 (crates.io)
              → INFRA-015 (Homebrew)
              → INFRA-018 (Shell Installer)
              → INFRA-023 (GitHub Action)
```

**Critical path duration: ~16 days** (Sprints 1-3, assuming sequential execution of dependent tasks)

The cloud infrastructure (Sprints 6-8) is on a separate critical path that begins after the CLI is stable:

```
INFRA-026 (Architecture)
  → INFRA-027 (Auth)
    → INFRA-028 (Billing)
    → INFRA-030 (Deployment)
      → INFRA-029 (Dashboard)
        → INFRA-032 (Monitoring)
          → INFRA-037 (SOC 2)
```

**Cloud critical path duration: ~28 days**

### Key Risks

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| R1 | **Cross-compilation breaks for aarch64-musl** | Medium | High | Maintain fallback to `cross` if `cargo-zigbuild` fails. Test in QEMU. Keep Docker images as distribution alternative. |
| R2 | **Binary size exceeds 15MB budget with all features** | Medium | Medium | Feature-gate heavy dependencies (SLM runtime, WASM plugin). Audit with `cargo-bloat`. Use `opt-level = "s"` not `"z"`. Accept 20MB if perf justifies it. |
| R3 | **CI pipeline becomes slow (>20 min)** | High | Medium | Aggressive caching with `Swatinem/rust-cache`. Split CI into parallel jobs. Run benchmarks only on main, not PRs. Use `cargo nextest` for faster test execution. |
| R4 | **crates.io name squatted** | Low | High | Reserve `valiforge`, `valiforge-core`, `valiforge-cli`, etc. immediately with placeholder crates. Do this in Sprint 1. |
| R5 | **macOS notarization breaks in CI** | Medium | Low | macOS signing is optional for launch. Users can bypass Gatekeeper. Prioritize after initial traction. |
| R6 | **Fly.io pricing or reliability issues** | Low | Medium | Architecture is portable. Rust binary runs anywhere. Can migrate to Railway, Render, or K8s within days. |
| R7 | **SOC 2 timeline slips** | High | Medium | Start evidence collection in Sprint 5 (not Sprint 8). Use Vanta/Drata for automation. Get Type I before Type II. |
| R8 | **Telemetry backlash from community** | Medium | High | Off by default, always. Transparent documentation. No PII whatsoever. Model after rustup's approach. Consider removing if community pushback is strong. |
| R9 | **GitHub Actions runner variance causes benchmark false positives** | High | Low | Set `fail-on-alert: false`. Use 10% threshold. Alert but do not block. Use dedicated benchmark runner for precise measurements if needed. |
| R10 | **Single engineer bottleneck** | High | High | Prioritize ruthlessly. Ship Sprints 1-3 first (CLI distribution). Cloud (Sprints 6-8) can wait. Hire infrastructure-aware Rust engineer by Sprint 4. |

### Immediate Action Items (Week 1)

1. **Reserve crate names on crates.io** -- `valiforge`, `valiforge-core`, `valiforge-cli`, `valiforge-schema`, `valiforge-diff`, `valiforge-datagen`, `valiforge-report`, `valiforge-plugin`, `valiforge-sdk`
2. **Reserve npm scope** -- `@valiforge/cli` on npmjs.com
3. **Create GitHub organization** -- `github.com/valiforge`
4. **Register domains** -- `valiforge.dev`, `install.valiforge.dev`, `dl.valiforge.dev`, `api.valiforge.dev`
5. **Set up repository** -- INFRA-001 through INFRA-004
6. **Write SECURITY.md** -- INFRA-036

---

## Appendix A: Tool Versions & Links

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | 1.82.0 (MSRV: 1.80.0) | Language |
| cargo-deny | latest | License compliance |
| cargo-audit | latest | Security vulnerabilities |
| cargo-zigbuild | latest | Cross-compilation |
| cargo-cyclonedx | latest | SBOM generation |
| cargo-bloat | latest | Binary size analysis |
| cargo-release | latest | Release management |
| criterion | 0.5+ | Benchmarking |
| git-cliff | latest | Changelog generation |
| Trivy | latest | Container scanning |
| Swatinem/rust-cache | v2 | CI caching |
| dtolnay/rust-toolchain | stable | Rust installation in CI |

## Appendix B: Cost Estimates (Monthly)

### Phase 1: Open Source Only
| Service | Cost | Notes |
|---------|------|-------|
| GitHub (Team) | $0 (OSS) | Free for public repos |
| GitHub Actions | $0 | Free tier: 2,000 min/month for public repos |
| Cloudflare R2 | ~$5 | 10GB storage, minimal egress |
| Domain (valiforge.dev) | ~$12/year | |
| **Total** | **~$6/month** | |

### Phase 2: Cloud Launch
| Service | Cost | Notes |
|---------|------|-------|
| Fly.io (API) | $30-100 | 2-4 shared CPU machines |
| Neon (PostgreSQL) | $0-19 | Free tier to Pro |
| Upstash (Redis) | $0-10 | Free tier to Pay-as-you-go |
| Cloudflare R2 | $5-20 | Growing storage |
| Vercel (Dashboard) | $0-20 | Free tier to Pro |
| Grafana Cloud | $0 | Free tier (10K metrics) |
| Sentry | $0-26 | Free tier to Team |
| Stripe | 2.9% + $0.30/txn | Transaction fees only |
| **Total** | **~$60-200/month** | Scales with usage |

### Phase 3: Scale
| Service | Cost | Notes |
|---------|------|-------|
| Fly.io / K8s | $200-1000 | Scaling API tier |
| Neon (PostgreSQL) | $69-300 | Scale plan |
| Upstash (Redis) | $50-200 | Higher throughput |
| Vanta (SOC 2) | ~$300/month | Compliance automation |
| **Total** | **$600-2000/month** | At $50K+ MRR, well within margin |

---

*End of Infrastructure Engineering Plan*
