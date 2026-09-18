# ValiForge — Developer Experience Implementation Guide

**For: Senior Developer Advocate / DX Engineer**
**Goal: Build every surface a developer touches — docs, install, community, launch**

---

## 1. README.md (Production-Ready)

Place this at the repository root as `README.md`.

````markdown
<p align="center">
  <img src="public/logo.svg" alt="ValiForge" width="200" />
</p>

<h1 align="center">ValiForge</h1>
<p align="center"><strong>Headless API validation engine. Rust-fast. AI-native. Zero config.</strong></p>

<p align="center">
  <a href="https://github.com/valiforge/valiforge/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/valiforge/valiforge/ci.yml?branch=main&label=CI&style=flat-square" alt="CI"></a>
  <a href="https://crates.io/crates/valiforge"><img src="https://img.shields.io/crates/v/valiforge?style=flat-square&color=orange" alt="crates.io"></a>
  <a href="https://www.npmjs.com/package/valiforge"><img src="https://img.shields.io/npm/v/valiforge?style=flat-square&color=red" alt="npm"></a>
  <a href="https://docs.valiforge.dev"><img src="https://img.shields.io/badge/docs-valiforge.dev-blue?style=flat-square" alt="docs"></a>
  <a href="https://github.com/valiforge/valiforge/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-green?style=flat-square" alt="license"></a>
  <a href="https://discord.gg/valiforge"><img src="https://img.shields.io/discord/000000000000000000?label=Discord&style=flat-square&color=5865F2" alt="Discord"></a>
</p>

<p align="center">
  <a href="https://docs.valiforge.dev/getting-started">Getting Started</a> ·
  <a href="https://docs.valiforge.dev/guides/ci-cd">CI/CD Guide</a> ·
  <a href="https://docs.valiforge.dev/reference/config">Config Reference</a> ·
  <a href="https://discord.gg/valiforge">Discord</a>
</p>

---

## Why ValiForge?

**AI generates code faster than humans can review it.** ValiForge catches what code review misses — broken API contracts, schema violations, and state mutation bugs — in milliseconds, not minutes.

| | ValiForge | Legacy Tools |
|---|---|---|
| **Cold start** | < 200ms | 5–30 seconds (JVM/Python/Electron) |
| **Binary size** | ~8 MB single static binary | 200 MB+ with runtimes |
| **CI overhead** | Zero — no Docker, no browser, no JVM | Browsers, runtimes, containers |

### Three things that make ValiForge different

1. **Rust-fast, zero-dependency.** One static binary. No runtime. No Docker. Cold start under 200ms. Validates the Stripe OpenAPI spec (15,000+ lines) in under 2 seconds.

2. **AI-native test data.** Built-in small language model integration generates context-aware test payloads — edge cases, adversarial inputs, and boundary values your team would never write by hand.

3. **Schema-first, not UI-first.** Validates API contracts (REST, gRPC, GraphQL) against source-of-truth schemas. No brittle CSS selectors. No flaky browser tests. Tests that survive every AI-generated refactor.

---

## Quick Install

```bash
# Recommended — one-line install
curl -fsSL https://install.valiforge.dev | sh

# Homebrew (macOS / Linux)
brew install valiforge/tap/valiforge

# Cargo (build from source)
cargo install valiforge

# npm (for JS/TS projects)
npm install -g valiforge
```

---

## 30-Second Quickstart

```bash
# 1. Initialize in your project (auto-detects API schemas)
valiforge init

# 2. Validate your API contracts
valiforge validate

# 3. View results
# ValiForge prints results to the terminal automatically:
```

```
 ValiForge v0.1.0

  Validating: api/openapi.yaml (OpenAPI 3.1)
  ─────────────────────────────────────────────

  ✓ GET  /pets         200 schema valid
  ✓ GET  /pets/{id}    200 schema valid
  ✗ POST /pets         400 missing response schema
    → error[VF0042]: No response schema defined for status 400
      help: Add a content block with media type and schema
      docs: https://docs.valiforge.dev/errors/VF0042
  ✓ PUT  /pets/{id}    200 schema valid
  ! GET  /pets         pagination missing
    → warn[VFW012]: Collection endpoint has no pagination parameters
      help: Add limit/offset or cursor parameters

  ─────────────────────────────────────────────
  4 passed · 1 error · 1 warning · 0.23s
```

---

## Feature Highlights

### Breaking Change Detection

```bash
valiforge diff --base main --head feature/new-api
```

```
 Breaking Changes (2)

  ✗ REMOVED  DELETE /pets/{id}
    This endpoint existed in main but is missing in feature/new-api

  ✗ CHANGED  POST /pets → field "name" type changed
    string (required) → integer (required)
    Consumers sending string values will receive 400 errors

 Non-Breaking Changes (3)

  + ADDED    GET  /pets/search     New endpoint
  + ADDED    POST /pets → field "tags" (optional)
  ~ CHANGED  GET  /pets → field "status" enum widened
    ["available","pending"] → ["available","pending","adopted"]

 Summary: 2 breaking · 3 non-breaking
 Recommendation: This change requires a MAJOR version bump
```

### AI-Powered Test Data Generation

```bash
valiforge datagen --schema api/openapi.yaml --endpoint "POST /pets"
```

```json
[
  {"name": "Buddy", "tag": "dog", "status": "available"},
  {"name": "", "tag": null, "status": "available"},
  {"name": "A".repeat(10000), "tag": "🐱", "status": "INVALID"},
  {"name": "Robert'); DROP TABLE pets;--", "tag": "sql", "status": "pending"}
]
```

### Multiple Output Formats

```bash
valiforge validate --format json     # Machine-readable JSON
valiforge validate --format junit    # JUnit XML for CI
valiforge validate --format sarif    # GitHub Code Scanning
valiforge validate --format tap      # Test Anything Protocol
```

---

## Comparison

| Feature | ValiForge | Postman | Dredd | Schemathesis | Spectral |
|---------|-----------|---------|-------|-------------|----------|
| Language | Rust | Electron/JS | JS | Python | JS |
| Cold start | < 200ms | ~5s | ~3s | ~500ms | ~1s |
| Binary size | ~8 MB | ~500 MB | ~150 MB | ~50 MB | ~80 MB |
| OpenAPI 3.1 | Yes | Partial | No | Yes | Yes |
| gRPC / Protobuf | Yes | Yes | No | No | No |
| GraphQL | Yes | Yes | No | No | No |
| Breaking change detection | Yes | No | No | No | No |
| AI test data generation | Built-in (SLM) | No | No | Property-based | No |
| CI-first design | Yes | Bolt-on | Yes | Yes | Yes |
| Offline / local-first | Yes | No (cloud) | Yes | Yes | Yes |
| Watch mode | Yes | No | No | No | Yes |
| License | Apache 2.0 | Proprietary | MIT | MIT | Apache 2.0 |
| Price | Free (OSS) | $19–49/user/mo | Free | Free | Free / Paid |

---

## CI/CD Integration

### GitHub Actions

```yaml
# .github/workflows/api-validation.yml
name: API Validation
on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install ValiForge
        run: curl -fsSL https://install.valiforge.dev | sh
      - name: Validate API contracts
        run: valiforge validate --format sarif --output results.sarif
      - name: Upload SARIF
        if: always()
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: results.sarif
```

### GitLab CI

```yaml
# .gitlab-ci.yml
api-validation:
  stage: test
  image: alpine:latest
  before_script:
    - curl -fsSL https://install.valiforge.dev | sh
    - export PATH="$HOME/.valiforge/bin:$PATH"
  script:
    - valiforge validate --format junit --output report.xml
  artifacts:
    reports:
      junit: report.xml
```

### Jenkins

```groovy
// Jenkinsfile
pipeline {
    agent any
    stages {
        stage('API Validation') {
            steps {
                sh 'curl -fsSL https://install.valiforge.dev | sh'
                sh '$HOME/.valiforge/bin/valiforge validate --format junit --output results.xml'
            }
            post {
                always {
                    junit 'results.xml'
                }
            }
        }
    }
}
```

---

## Configuration

ValiForge is configured via `valiforge.toml` in your project root:

```toml
[project]
name = "my-api"
schema = "api/openapi.yaml"

[rules]
strict = true                  # Treat warnings as errors
exclude = ["VFW012"]           # Suppress specific warnings

[output]
format = "text"                # text | json | junit | sarif | tap
color = "auto"                 # auto | always | never
```

Full reference: [docs.valiforge.dev/reference/config](https://docs.valiforge.dev/reference/config)

---

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) to get started.

**Quick links:**
- [Good First Issues](https://github.com/valiforge/valiforge/labels/good-first-issue)
- [Development Setup](CONTRIBUTING.md#development-setup)
- [Discord #contributors](https://discord.gg/valiforge)

---

## License

Licensed under [Apache License 2.0](LICENSE).

Copyright 2026 ValiForge Contributors.
````

---

## 2. CONTRIBUTING.md

Place this at the repository root as `CONTRIBUTING.md`.

````markdown
# Contributing to ValiForge

Thank you for your interest in contributing to ValiForge! This guide will get you from clone to passing tests in under 15 minutes.

---

## Development Setup

### Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| Rust | 1.75+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Just | 1.0+ | `cargo install just` |
| Node.js | 20+ | For docs site only — `brew install node` |

### Clone and Build

```bash
git clone https://github.com/valiforge/valiforge.git
cd valiforge

# Install dev dependencies and build
just setup

# Run the full test suite
just test

# Run the CLI locally
cargo run -- validate --help
```

### Useful Commands

```bash
just build          # Build all crates (debug)
just release        # Build all crates (release, optimized)
just test           # Run unit + integration tests
just test-doc       # Run doc tests
just lint           # Run clippy + rustfmt check
just fmt            # Auto-format code
just bench          # Run benchmarks
just docs           # Build and open rustdoc
just docs-site      # Build the Starlight docs site
just ci             # Run the full CI pipeline locally
```

---

## Architecture Overview

ValiForge is a Cargo workspace with six crates:

```
crates/
├── valiforge-cli/       # CLI binary — argument parsing, output formatting
├── valiforge-core/      # Core engine — validation pipeline, rule execution
├── valiforge-schema/    # Schema parsing — OpenAPI, Protobuf, GraphQL, AsyncAPI
├── valiforge-diff/      # Breaking change detection — schema comparison
├── valiforge-report/    # Output formats — JSON, JUnit, SARIF, TAP
└── valiforge-datagen/   # Test data generation — SLM integration, property-based
```

### Data Flow

```
                 valiforge-schema
                    (parse)
                       │
  CLI input ──→ valiforge-core ──→ valiforge-report ──→ Output
                  (validate)          (format)
                       │
                 valiforge-diff
                  (compare)
```

**Key design principles:**
- Each crate has a single responsibility and a well-defined public API.
- `valiforge-core` depends on `valiforge-schema` but NOT on `valiforge-cli`.
- `valiforge-cli` is the only crate that does I/O (filesystem, network, terminal).
- All business logic lives in `valiforge-core` and is testable without a CLI.

---

## Code Style

### Formatting

We use `rustfmt` with the project's `rustfmt.toml`:

```toml
edition = "2021"
max_width = 100
use_field_init_shorthand = true
use_try_shorthand = true
```

Run `just fmt` before committing. CI will reject unformatted code.

### Clippy Lints

The workspace enforces strict Clippy lints:

```toml
[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
unwrap_used = "deny"
panic = "deny"
missing_errors_doc = "warn"
```

- Never use `.unwrap()` — use `.expect("reason")` in tests, `?` or `anyhow` in application code.
- Never use `panic!()` in library code.
- Document all `pub fn` that return `Result` with an `# Errors` section.

### Naming Conventions

- Types: `PascalCase` (Rust standard)
- Functions / methods: `snake_case` (Rust standard)
- Error variants: descriptive — `SchemaNotFound(PathBuf)`, not `NotFound(PathBuf)`
- Feature flags: `kebab-case` — `ai-datagen`, `grpc-support`

---

## PR Process

### Branch Naming

```
feat/short-description     # New feature
fix/short-description      # Bug fix
docs/short-description     # Documentation only
refactor/short-description # Code refactor (no behavior change)
test/short-description     # Tests only
perf/short-description     # Performance improvement
```

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(schema): add AsyncAPI 3.0 parser support
fix(cli): respect NO_COLOR environment variable
docs(readme): add GitLab CI example
test(core): add property tests for enum validation
perf(schema): parallelize OpenAPI endpoint parsing
```

### Review Checklist

Every PR must satisfy:

- [ ] `just ci` passes locally (build, test, lint, fmt)
- [ ] New public API has doc comments with examples
- [ ] New features have unit tests (and integration tests if applicable)
- [ ] Error messages follow the VF error format (error code, location, suggestion)
- [ ] CHANGELOG.md updated (under `## [Unreleased]`)
- [ ] No new `unwrap()` calls in library code
- [ ] Breaking changes documented in PR description

### Review Timeline

- **First review:** Within 48 hours on business days.
- **Iteration:** Aim for resolution within 1 week.
- **Merge:** Squash-merge to `main` with a clean commit message.

---

## Issue Templates

### Bug Report Fields

When filing a bug, include:
1. **ValiForge version** (`valiforge --version`)
2. **OS and architecture** (`uname -a` or equivalent)
3. **Schema type** (OpenAPI, GraphQL, Protobuf)
4. **Minimal reproduction** (smallest config + schema that triggers the bug)
5. **Expected vs. actual behavior**
6. **Full terminal output** (with `--debug` flag)

### Feature Request Fields

When proposing a feature, include:
1. **Use case** — what problem are you solving?
2. **Proposed solution** — how should it work?
3. **Alternatives considered** — what else did you try?
4. **Willingness to implement** — would you like to submit a PR?

---

## First Good Issues

Look for issues labeled [`good-first-issue`](https://github.com/valiforge/valiforge/labels/good-first-issue). Each one includes:

- Clear description of what to change
- Which files to modify
- Which tests to add or update
- A named mentor you can tag for help

**Claim process:** Comment "I'd like to work on this" on the issue. You have 7 days to open a PR. If you need more time, just comment — we are happy to extend.

---

## Code of Conduct

This project follows the [Contributor Covenant v2.1](https://www.contributor-covenant.org/version/2/1/code_of_conduct/). By participating, you agree to uphold this code. Report issues to conduct@valiforge.dev.
````

---

## 3. Cross-Platform Install Script (`install.sh`)

Place this at `scripts/install.sh` and host at `https://install.valiforge.dev`.

```bash
#!/usr/bin/env bash
# ValiForge installer — https://install.valiforge.dev
# Usage: curl -fsSL https://install.valiforge.dev | sh
#
# Flags:
#   --help       Show this help message
#   --version    Install a specific version (e.g., --version 0.2.0)
#   --prefix     Install to a custom directory (default: ~/.valiforge/bin)
#
# Environment:
#   VALIFORGE_VERSION   Same as --version
#   VALIFORGE_PREFIX    Same as --prefix
#   HTTP_PROXY          Respected for downloads behind a proxy
#   HTTPS_PROXY         Respected for downloads behind a proxy

set -euo pipefail

# --- Configuration ---
REPO="valiforge/valiforge"
INSTALL_DIR="${VALIFORGE_PREFIX:-$HOME/.valiforge/bin}"
BASE_URL="https://github.com/${REPO}/releases/download"
VERSION="${VALIFORGE_VERSION:-latest}"
BINARY_NAME="valiforge"

# --- Colors ---
if [ -t 1 ] && [ "${NO_COLOR:-}" = "" ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    BLUE='\033[0;34m'
    BOLD='\033[1m'
    RESET='\033[0m'
else
    RED='' GREEN='' YELLOW='' BLUE='' BOLD='' RESET=''
fi

# --- Helpers ---
info()  { printf "${BLUE}info${RESET}  %s\n" "$1"; }
ok()    { printf "${GREEN}  ok${RESET}  %s\n" "$1"; }
warn()  { printf "${YELLOW}warn${RESET}  %s\n" "$1"; }
err()   { printf "${RED}error${RESET} %s\n" "$1" >&2; }
die()   { err "$1"; exit 1; }

usage() {
    cat <<'USAGE'
ValiForge Installer

USAGE:
    curl -fsSL https://install.valiforge.dev | sh [-- OPTIONS]

OPTIONS:
    --help               Show this help message
    --version <VERSION>  Install a specific version (e.g., 0.2.0)
    --prefix <DIR>       Install directory (default: ~/.valiforge/bin)

EXAMPLES:
    # Install latest
    curl -fsSL https://install.valiforge.dev | sh

    # Install specific version
    curl -fsSL https://install.valiforge.dev | sh -s -- --version 0.2.0

    # Install to custom directory
    curl -fsSL https://install.valiforge.dev | sh -s -- --prefix /usr/local/bin

ENVIRONMENT:
    VALIFORGE_VERSION    Same as --version
    VALIFORGE_PREFIX     Same as --prefix
    NO_COLOR             Disable colored output
    HTTP_PROXY           Proxy for downloads
    HTTPS_PROXY          Proxy for downloads
USAGE
    exit 0
}

# --- Parse arguments ---
while [ $# -gt 0 ]; do
    case "$1" in
        --help)    usage ;;
        --version) VERSION="$2"; shift 2 ;;
        --prefix)  INSTALL_DIR="$2"; shift 2 ;;
        *)         die "Unknown option: $1. Run with --help for usage." ;;
    esac
done

# --- Detect OS ---
detect_os() {
    local os
    os="$(uname -s)"
    case "$os" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "darwin" ;;
        MINGW*|MSYS*|CYGWIN*) die "Windows detected. Use PowerShell instead:\n  irm https://install.valiforge.dev/ps | iex" ;;
        *)       die "Unsupported operating system: $os\nSee https://docs.valiforge.dev/getting-started/install#build-from-source" ;;
    esac
}

# --- Detect architecture ---
detect_arch() {
    local arch
    arch="$(uname -m)"
    case "$arch" in
        x86_64|amd64)   echo "x86_64" ;;
        aarch64|arm64)   echo "aarch64" ;;
        *)               die "Unsupported architecture: $arch\nSee https://docs.valiforge.dev/getting-started/install#build-from-source" ;;
    esac
}

# --- Detect download tool ---
detect_downloader() {
    if command -v curl >/dev/null 2>&1; then
        echo "curl"
    elif command -v wget >/dev/null 2>&1; then
        echo "wget"
    else
        die "Neither curl nor wget found. Install one and retry.\n  sudo apt install curl   # Debian/Ubuntu\n  sudo yum install curl   # RHEL/CentOS\n  brew install curl       # macOS"
    fi
}

# --- Download file ---
download() {
    local url="$1" dest="$2"
    case "$(detect_downloader)" in
        curl) curl -fsSL --progress-bar -o "$dest" "$url" ;;
        wget) wget -q --show-progress -O "$dest" "$url" ;;
    esac
}

# --- Resolve latest version ---
resolve_version() {
    if [ "$VERSION" = "latest" ]; then
        info "Resolving latest version..."
        local url="https://api.github.com/repos/${REPO}/releases/latest"
        case "$(detect_downloader)" in
            curl) VERSION="$(curl -fsSL "$url" | grep '"tag_name"' | head -1 | sed 's/.*"v\([^"]*\)".*/\1/')" ;;
            wget) VERSION="$(wget -qO- "$url" | grep '"tag_name"' | head -1 | sed 's/.*"v\([^"]*\)".*/\1/')" ;;
        esac
        if [ -z "$VERSION" ]; then
            die "Failed to resolve latest version. Check your network connection.\nOr specify a version: --version 0.1.0"
        fi
    fi
    ok "Version: v${VERSION}"
}

# --- Main ---
main() {
    printf "\n${BOLD}ValiForge Installer${RESET}\n\n"

    local os arch target archive_name archive_url checksum_url tmpdir

    os="$(detect_os)"
    arch="$(detect_arch)"
    target="${arch}-unknown-${os}-gnu"
    if [ "$os" = "darwin" ]; then
        target="${arch}-apple-darwin"
    fi

    ok "Detected: ${os} ${arch} (${target})"

    resolve_version

    archive_name="valiforge-v${VERSION}-${target}.tar.gz"
    archive_url="${BASE_URL}/v${VERSION}/${archive_name}"
    checksum_url="${BASE_URL}/v${VERSION}/SHA256SUMS"

    # Create temp directory
    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT

    # Download archive
    info "Downloading ${archive_name}..."
    download "$archive_url" "${tmpdir}/${archive_name}" || \
        die "Download failed. URL: ${archive_url}\nCheck that version v${VERSION} exists at:\n  https://github.com/${REPO}/releases"

    ok "Downloaded ${archive_name}"

    # Download and verify checksum
    info "Verifying SHA-256 checksum..."
    download "$checksum_url" "${tmpdir}/SHA256SUMS" || \
        die "Checksum download failed. URL: ${checksum_url}"

    local expected_checksum actual_checksum
    expected_checksum="$(grep "${archive_name}" "${tmpdir}/SHA256SUMS" | awk '{print $1}')"

    if [ -z "$expected_checksum" ]; then
        die "Checksum not found for ${archive_name} in SHA256SUMS"
    fi

    if command -v sha256sum >/dev/null 2>&1; then
        actual_checksum="$(sha256sum "${tmpdir}/${archive_name}" | awk '{print $1}')"
    elif command -v shasum >/dev/null 2>&1; then
        actual_checksum="$(shasum -a 256 "${tmpdir}/${archive_name}" | awk '{print $1}')"
    else
        warn "No sha256sum or shasum found — skipping checksum verification"
        actual_checksum="$expected_checksum"
    fi

    if [ "$expected_checksum" != "$actual_checksum" ]; then
        die "Checksum verification failed!\n  Expected: ${expected_checksum}\n  Actual:   ${actual_checksum}\nThe download may be corrupted. Please retry."
    fi

    ok "Checksum verified"

    # Extract and install
    info "Installing to ${INSTALL_DIR}..."
    mkdir -p "$INSTALL_DIR"
    tar -xzf "${tmpdir}/${archive_name}" -C "$tmpdir"
    mv "${tmpdir}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

    ok "Installed valiforge to ${INSTALL_DIR}/${BINARY_NAME}"

    # PATH check
    if ! echo "$PATH" | tr ':' '\n' | grep -q "^${INSTALL_DIR}$"; then
        printf "\n"
        warn "\"${INSTALL_DIR}\" is not in your PATH."
        printf "\n  Add it to your shell profile:\n\n"

        local shell_name
        shell_name="$(basename "${SHELL:-/bin/bash}")"
        case "$shell_name" in
            zsh)  printf "    echo 'export PATH=\"%s:\$PATH\"' >> ~/.zshrc && source ~/.zshrc\n" "$INSTALL_DIR" ;;
            bash) printf "    echo 'export PATH=\"%s:\$PATH\"' >> ~/.bashrc && source ~/.bashrc\n" "$INSTALL_DIR" ;;
            fish) printf "    set -Ux fish_user_paths %s \$fish_user_paths\n" "$INSTALL_DIR" ;;
            *)    printf "    export PATH=\"%s:\$PATH\"\n" "$INSTALL_DIR" ;;
        esac
        printf "\n"
    fi

    # Verify installation
    if "${INSTALL_DIR}/${BINARY_NAME}" --version >/dev/null 2>&1; then
        local installed_version
        installed_version="$("${INSTALL_DIR}/${BINARY_NAME}" --version 2>/dev/null | head -1)"
        ok "Verified: ${installed_version}"
    else
        warn "Installation succeeded but binary failed to execute. Check your system for missing libraries."
    fi

    # Done
    printf "\n${GREEN}${BOLD}ValiForge v${VERSION} installed successfully!${RESET}\n\n"
    printf "  Get started:\n"
    printf "    ${BOLD}valiforge init${RESET}       Initialize in your project\n"
    printf "    ${BOLD}valiforge validate${RESET}   Run API validation\n"
    printf "    ${BOLD}valiforge --help${RESET}     See all commands\n\n"
    printf "  Docs:    https://docs.valiforge.dev\n"
    printf "  Discord: https://discord.gg/valiforge\n\n"
}

main
```

---

## 4. Documentation Site Content (Astro Starlight)

All documentation goes in `docs/src/content/docs/`. The site uses [Astro Starlight](https://starlight.astro.build/).

### 4.1 Getting Started Guide

**File:** `docs/src/content/docs/getting-started/index.mdx`

````mdx
---
title: Getting Started
description: Install ValiForge and run your first API validation in under 5 minutes.
---

import { Tabs, TabItem, Steps, Callout } from '@astrojs/starlight/components';

# Getting Started

This guide takes you from zero to a passing API validation in under 5 minutes.

## Installation

<Tabs>
  <TabItem label="macOS / Linux">
    ```bash
    curl -fsSL https://install.valiforge.dev | sh
    ```
  </TabItem>
  <TabItem label="Homebrew">
    ```bash
    brew install valiforge/tap/valiforge
    ```
  </TabItem>
  <TabItem label="Cargo">
    ```bash
    cargo install valiforge
    ```
  </TabItem>
  <TabItem label="npm">
    ```bash
    npm install -g valiforge
    ```
  </TabItem>
</Tabs>

Verify the installation:

```bash
valiforge --version
# valiforge 0.1.0 (a1b2c3d 2026-03-15, rustc 1.75.0)
```

## Your First Validation

We will validate a sample Petstore API. This works even without an existing project.

<Steps>

### Create a sample project

```bash
mkdir petstore-demo && cd petstore-demo
```

### Create an OpenAPI spec

Save the following as `openapi.yaml`:

```yaml
openapi: "3.1.0"
info:
  title: Petstore
  version: "1.0.0"
paths:
  /pets:
    get:
      summary: List all pets
      operationId: listPets
      parameters:
        - name: limit
          in: query
          required: false
          schema:
            type: integer
            maximum: 100
      responses:
        "200":
          description: A list of pets
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: "#/components/schemas/Pet"
    post:
      summary: Create a pet
      operationId: createPet
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: "#/components/schemas/Pet"
      responses:
        "201":
          description: Pet created
  /pets/{petId}:
    get:
      summary: Get a pet by ID
      operationId: getPet
      parameters:
        - name: petId
          in: path
          required: true
          schema:
            type: string
      responses:
        "200":
          description: A single pet
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/Pet"
components:
  schemas:
    Pet:
      type: object
      required:
        - id
        - name
      properties:
        id:
          type: integer
          format: int64
        name:
          type: string
        tag:
          type: string
```

### Initialize ValiForge

```bash
valiforge init
```

Expected output:

```
 ValiForge Init

  Scanning for API schemas...
  ✓ Found: openapi.yaml (OpenAPI 3.1, 3 endpoints)

  Created: valiforge.toml
  Created: .valiforge/rules/
  Created: .valiforge/snapshots/

  Next steps:
    Run valiforge validate to check your API contracts.
```

### Run validation

```bash
valiforge validate
```

Expected output:

```
 ValiForge v0.1.0

  Validating: openapi.yaml (OpenAPI 3.1)
  ─────────────────────────────────────────────

  ✓ GET  /pets           200 schema valid
  ✓ GET  /pets/{petId}   200 schema valid
  ✗ POST /pets           201 missing response schema
    → error[VF0042]: Response 201 has no content schema
      help: Add a content block with media type and schema
      docs: https://docs.valiforge.dev/errors/VF0042
  ! GET  /pets           pagination incomplete
    → warn[VFW012]: limit parameter has no default value
      help: Add a default value to prevent unbounded queries

  ─────────────────────────────────────────────
  2 passed · 1 error · 1 warning · 0.18s
```

</Steps>

## Understanding Results

| Symbol | Meaning |
|--------|---------|
| ✓ (green) | Validation passed — this endpoint is correct |
| ✗ (red) | Validation error — must be fixed before merge |
| ! (yellow) | Warning — should be fixed but will not block CI |

Each issue includes:
- **Error code** (e.g., `VF0042`) — stable, searchable, linked to documentation
- **Location** — file, line, and column where the issue was found
- **Suggestion** — a concrete fix you can apply immediately
- **Docs link** — detailed explanation with examples

Run `valiforge explain VF0042` for full documentation on any error code.

## Next Steps

- [Add to CI/CD](/guides/ci-cd) — Block PRs that break API contracts
- [Configuration reference](/reference/config) — Customize rules and output
- [Breaking change detection](/guides/breaking-changes) — Compare schema versions
- [AI test data generation](/guides/datagen) — Generate edge-case payloads
````

### 4.2 Configuration Reference

**File:** `docs/src/content/docs/reference/config.mdx`

````mdx
---
title: Configuration Reference
description: Complete reference for valiforge.toml and CLI flags.
---

import { Tabs, TabItem, Callout } from '@astrojs/starlight/components';

# Configuration Reference

ValiForge reads configuration from three sources, in order of precedence (highest first):

1. **CLI flags** — `valiforge validate --strict`
2. **Environment variables** — `VALIFORGE_RULES_STRICT=true`
3. **Config file** — `valiforge.toml` in the project root

## `valiforge.toml` Full Reference

```toml
# =============================================================================
# ValiForge Configuration
# Generated by valiforge init — safe to edit
# Full reference: https://docs.valiforge.dev/reference/config
# =============================================================================

# --- Project Metadata ---
[project]
# Project name (used in reports and output)
name = "my-api"

# Path to the primary API schema file
# Supports: OpenAPI 3.x (.yaml/.json), Protobuf (.proto), GraphQL (.graphql)
schema = "api/openapi.yaml"

# Additional schemas (for monorepos or multi-schema projects)
# schemas = ["api/v1/openapi.yaml", "api/v2/openapi.yaml"]

# Base URL of the running API (for live validation)
# base_url = "http://localhost:3000"

# --- Validation Rules ---
[rules]
# Treat warnings as errors (recommended for CI)
# CLI: --strict | Env: VALIFORGE_RULES_STRICT
strict = false

# Rule IDs to exclude from validation
# exclude = ["VFW012", "VFW015"]

# Rule IDs to include (if set, only these rules run)
# include = ["VF0001", "VF0042"]

# Tags to filter rules by category
# tags = ["security", "breaking-change"]

# Minimum severity to report: "error", "warning", "info"
# CLI: --min-severity | Env: VALIFORGE_RULES_MIN_SEVERITY
min_severity = "warning"

# Custom rules directory (Rust WASM plugins or YAML rule definitions)
# custom_rules_dir = ".valiforge/rules/"

# --- Output ---
[output]
# Output format: "text", "json", "junit", "sarif", "tap"
# CLI: --format | Env: VALIFORGE_OUTPUT_FORMAT
format = "text"

# Output file path (default: stdout)
# CLI: --output | Env: VALIFORGE_OUTPUT_FILE
# file = "results.json"

# Color mode: "auto", "always", "never"
# CLI: --color | Env: VALIFORGE_OUTPUT_COLOR
# Respects NO_COLOR environment variable (https://no-color.org)
color = "auto"

# Show verbose output (internal timings, rule evaluation details)
# CLI: --verbose | Env: VALIFORGE_OUTPUT_VERBOSE
verbose = false

# Group results by: "endpoint", "rule", "severity", "file"
group_by = "endpoint"

# --- Performance ---
[performance]
# Maximum number of parallel validation jobs
# CLI: --jobs | Env: VALIFORGE_PERFORMANCE_JOBS
# Default: number of CPU cores
# jobs = 4

# Request timeout in milliseconds (for live validation)
# CLI: --timeout | Env: VALIFORGE_PERFORMANCE_TIMEOUT
timeout_ms = 30000

# Maximum number of retries for failed HTTP requests
retries = 2

# --- Diff / Breaking Change Detection ---
[diff]
# Default base branch for comparisons
# CLI: --base | Env: VALIFORGE_DIFF_BASE
base = "main"

# Path to allowlist file for intentional breaking changes
# allowlist = ".valiforge/breaking-changes.yaml"

# --- Test Data Generation ---
[datagen]
# Enable AI-powered test data generation
# enabled = true

# SLM model to use: "built-in", "ollama:mistral", "ollama:llama3"
# model = "built-in"

# Number of test cases to generate per endpoint
# count = 10

# Include adversarial/negative test cases
# adversarial = true

# --- Scanning ---
[scan]
# Maximum directory depth for schema auto-detection
max_depth = 5

# Additional directories to ignore (beyond .gitignore)
ignore = ["vendor/", "third_party/", "node_modules/"]

# --- Telemetry ---
[telemetry]
# Opt-in anonymous usage telemetry
# See: https://docs.valiforge.dev/privacy
enabled = false
```

## Environment Variables

Every config key maps to an environment variable using the pattern `VALIFORGE_<SECTION>_<KEY>` in uppercase:

| Config Key | Environment Variable | Example |
|---|---|---|
| `project.name` | `VALIFORGE_PROJECT_NAME` | `VALIFORGE_PROJECT_NAME=my-api` |
| `rules.strict` | `VALIFORGE_RULES_STRICT` | `VALIFORGE_RULES_STRICT=true` |
| `output.format` | `VALIFORGE_OUTPUT_FORMAT` | `VALIFORGE_OUTPUT_FORMAT=json` |
| `output.color` | `VALIFORGE_OUTPUT_COLOR` | `VALIFORGE_OUTPUT_COLOR=never` |
| `performance.timeout_ms` | `VALIFORGE_PERFORMANCE_TIMEOUT` | `VALIFORGE_PERFORMANCE_TIMEOUT=60000` |
| `diff.base` | `VALIFORGE_DIFF_BASE` | `VALIFORGE_DIFF_BASE=develop` |
| `telemetry.enabled` | `VALIFORGE_TELEMETRY_ENABLED` | `VALIFORGE_TELEMETRY_ENABLED=false` |

<Callout type="tip">
Environment variables are useful for CI/CD where you want different settings per environment without modifying `valiforge.toml`.
</Callout>

## CLI Flags Reference

### `valiforge validate`

```
USAGE:
    valiforge validate [OPTIONS] [SCHEMA_PATH]

ARGS:
    [SCHEMA_PATH]  Path to schema file (overrides valiforge.toml)

OPTIONS:
    -f, --format <FORMAT>      Output format [default: text]
                               [possible values: text, json, junit, sarif, tap]
    -o, --output <FILE>        Write output to file instead of stdout
    -s, --strict               Treat warnings as errors (exit code 1)
        --fail-on <LEVEL>      Fail on this severity or above [default: error]
                               [possible values: error, warning, info]
        --include <RULES>      Only run these rules (comma-separated IDs or tags)
        --exclude <RULES>      Skip these rules (comma-separated IDs or tags)
    -j, --jobs <N>             Parallel validation jobs [default: num_cpus]
        --changed              Only validate files changed since last commit
    -w, --watch                Re-run validation on file changes
        --fix                  Auto-fix fixable issues
        --color <WHEN>         Color output [default: auto]
                               [possible values: auto, always, never]
    -v, --verbose              Show detailed output
        --debug                Show debug-level output (for troubleshooting)
        --timeout <MS>         Request timeout in milliseconds [default: 30000]
    -h, --help                 Print help (short)
        --help                 Print help (detailed with examples)
    -V, --version              Print version

EXAMPLES:
    # Validate using config from valiforge.toml
    valiforge validate

    # Validate a specific schema with strict mode
    valiforge validate api/openapi.yaml --strict

    # Output JUnit XML for CI
    valiforge validate --format junit --output results.xml

    # Only validate changed files in a PR
    valiforge validate --changed --strict

    # Watch mode for development
    valiforge validate --watch
```

### `valiforge init`

```
USAGE:
    valiforge init [OPTIONS]

OPTIONS:
    -y, --yes                  Accept defaults without prompting
        --template <TEMPLATE>  Use a starter template
                               [possible values: rest-api, grpc, graphql, event-driven]
        --schema <PATH>        Specify schema path (skip auto-detection)
    -h, --help                 Print help

EXAMPLES:
    # Interactive init (recommended)
    valiforge init

    # Non-interactive with defaults
    valiforge init --yes

    # Use the GraphQL template
    valiforge init --template graphql
```

### `valiforge diff`

```
USAGE:
    valiforge diff [OPTIONS] [OLD_SCHEMA] [NEW_SCHEMA]

ARGS:
    [OLD_SCHEMA]  Base schema file (or use --base for git branch)
    [NEW_SCHEMA]  Head schema file (or uses current working tree)

OPTIONS:
        --base <BRANCH>        Git branch/ref for base comparison [default: main]
        --head <BRANCH>        Git branch/ref for head comparison [default: HEAD]
    -f, --format <FORMAT>      Output format [default: text]
                               [possible values: text, json, markdown]
        --allow-breaking <MSG> Allow breaking changes with a reason
    -h, --help                 Print help

EXAMPLES:
    # Compare current branch against main
    valiforge diff

    # Compare two specific files
    valiforge diff api/v1.yaml api/v2.yaml

    # Compare branches with JSON output for CI
    valiforge diff --base main --head feature/v2 --format json
```

### `valiforge doctor`

```
USAGE:
    valiforge doctor [OPTIONS]

OPTIONS:
        --json   Output in JSON format (for CI)
        --fix    Attempt to auto-fix detected issues
    -h, --help   Print help

EXAMPLES:
    valiforge doctor
    valiforge doctor --json
    valiforge doctor --fix
```

## Precedence Order

When the same setting is specified in multiple places:

```
CLI flags          (highest priority — always wins)
  ↓
Environment vars   (overrides config file)
  ↓
valiforge.toml     (project-level defaults)
  ↓
Built-in defaults  (lowest priority — used when nothing else is set)
```

<Callout type="caution">
The `--strict` CLI flag overrides `rules.strict` in `valiforge.toml`, which overrides `VALIFORGE_RULES_STRICT`. This matches the behavior of tools like `rustfmt` and `eslint`.
</Callout>
````

### 4.3 CI/CD Integration Guide

**File:** `docs/src/content/docs/guides/ci-cd.mdx`

````mdx
---
title: CI/CD Integration
description: Add ValiForge to your CI/CD pipeline — GitHub Actions, GitLab CI, Jenkins, and more.
---

import { Tabs, TabItem, Callout, Steps } from '@astrojs/starlight/components';

# CI/CD Integration

ValiForge is built CI-first. No Docker. No browser. No JVM. One binary, zero dependencies.

## GitHub Actions

### Basic Workflow

```yaml
# .github/workflows/api-validation.yml
name: API Validation

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  validate:
    name: Validate API Contracts
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install ValiForge
        run: curl -fsSL https://install.valiforge.dev | sh

      - name: Validate schemas
        run: |
          export PATH="$HOME/.valiforge/bin:$PATH"
          valiforge validate --strict --format sarif --output results.sarif

      - name: Upload SARIF to GitHub Security
        if: always()
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: results.sarif
```

### Reusable Workflow (recommended for organizations)

```yaml
# .github/workflows/valiforge-reusable.yml
name: ValiForge Validation (Reusable)

on:
  workflow_call:
    inputs:
      schema_path:
        description: "Path to API schema"
        required: false
        type: string
        default: ""
      strict:
        description: "Treat warnings as errors"
        required: false
        type: boolean
        default: true
      valiforge_version:
        description: "ValiForge version to install"
        required: false
        type: string
        default: "latest"

jobs:
  validate:
    name: API Validation
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Cache ValiForge binary
        uses: actions/cache@v4
        with:
          path: ~/.valiforge
          key: valiforge-${{ inputs.valiforge_version }}-${{ runner.os }}

      - name: Install ValiForge
        run: |
          if [ ! -f "$HOME/.valiforge/bin/valiforge" ]; then
            curl -fsSL https://install.valiforge.dev | sh -s -- --version ${{ inputs.valiforge_version }}
          fi
          echo "$HOME/.valiforge/bin" >> $GITHUB_PATH

      - name: Validate
        run: |
          ARGS="--format junit --output results.xml"
          if [ "${{ inputs.strict }}" = "true" ]; then ARGS="$ARGS --strict"; fi
          if [ -n "${{ inputs.schema_path }}" ]; then ARGS="$ARGS ${{ inputs.schema_path }}"; fi
          valiforge validate $ARGS

      - name: Publish Test Results
        if: always()
        uses: dorny/test-reporter@v1
        with:
          name: ValiForge Results
          path: results.xml
          reporter: java-junit
```

Call it from any repo:

```yaml
# .github/workflows/ci.yml
jobs:
  api-validation:
    uses: valiforge/valiforge/.github/workflows/valiforge-reusable.yml@main
    with:
      strict: true
```

### PR Comment with Breaking Changes

```yaml
# .github/workflows/breaking-changes.yml
name: Breaking Change Check

on:
  pull_request:
    paths:
      - "api/**"

jobs:
  diff:
    name: Detect Breaking Changes
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Install ValiForge
        run: curl -fsSL https://install.valiforge.dev | sh && echo "$HOME/.valiforge/bin" >> $GITHUB_PATH

      - name: Run diff
        id: diff
        run: |
          valiforge diff --base origin/main --format markdown --output diff-report.md
          echo "has_breaking=$(grep -c 'BREAKING' diff-report.md || true)" >> $GITHUB_OUTPUT

      - name: Comment on PR
        if: steps.diff.outputs.has_breaking != '0'
        uses: marocchino/sticky-pull-request-comment@v2
        with:
          path: diff-report.md
```

## GitLab CI

```yaml
# .gitlab-ci.yml
stages:
  - validate

api-validation:
  stage: validate
  image: alpine:latest
  variables:
    VALIFORGE_RULES_STRICT: "true"
  before_script:
    - apk add --no-cache curl
    - curl -fsSL https://install.valiforge.dev | sh
    - export PATH="$HOME/.valiforge/bin:$PATH"
  script:
    - valiforge validate --format junit --output report.xml
  artifacts:
    when: always
    reports:
      junit: report.xml
  cache:
    key: valiforge
    paths:
      - $HOME/.valiforge/
  rules:
    - changes:
        - "api/**"
        - "valiforge.toml"
```

## Jenkins

```groovy
// Jenkinsfile
pipeline {
    agent any

    environment {
        PATH = "${env.HOME}/.valiforge/bin:${env.PATH}"
    }

    stages {
        stage('Install ValiForge') {
            steps {
                sh 'curl -fsSL https://install.valiforge.dev | sh'
            }
        }

        stage('Validate API Contracts') {
            steps {
                sh 'valiforge validate --strict --format junit --output results.xml'
            }
            post {
                always {
                    junit 'results.xml'
                }
            }
        }

        stage('Breaking Change Check') {
            when {
                changeRequest()
            }
            steps {
                sh 'valiforge diff --base origin/main --format json --output diff.json'
            }
        }
    }
}
```

## CircleCI

```yaml
# .circleci/config.yml
version: 2.1

jobs:
  api-validation:
    docker:
      - image: cimg/base:current
    steps:
      - checkout
      - restore_cache:
          keys:
            - valiforge-v1
      - run:
          name: Install ValiForge
          command: |
            if [ ! -f "$HOME/.valiforge/bin/valiforge" ]; then
              curl -fsSL https://install.valiforge.dev | sh
            fi
      - save_cache:
          key: valiforge-v1
          paths:
            - ~/.valiforge
      - run:
          name: Validate API contracts
          command: |
            export PATH="$HOME/.valiforge/bin:$PATH"
            valiforge validate --strict --format junit --output results.xml
      - store_test_results:
          path: results.xml

workflows:
  main:
    jobs:
      - api-validation
```

## Azure DevOps

```yaml
# azure-pipelines.yml
trigger:
  branches:
    include:
      - main
  paths:
    include:
      - api/*
      - valiforge.toml

pool:
  vmImage: "ubuntu-latest"

steps:
  - script: |
      curl -fsSL https://install.valiforge.dev | sh
      export PATH="$HOME/.valiforge/bin:$PATH"
      valiforge validate --strict --format junit --output $(Build.ArtifactStagingDirectory)/results.xml
    displayName: "ValiForge API Validation"

  - task: PublishTestResults@2
    condition: always()
    inputs:
      testResultsFormat: "JUnit"
      testResultsFiles: "$(Build.ArtifactStagingDirectory)/results.xml"
      testRunTitle: "ValiForge API Validation"
```

## Bitbucket Pipelines

```yaml
# bitbucket-pipelines.yml
pipelines:
  default:
    - step:
        name: API Validation
        image: alpine:latest
        script:
          - apk add --no-cache curl bash
          - curl -fsSL https://install.valiforge.dev | sh
          - export PATH="$HOME/.valiforge/bin:$PATH"
          - valiforge validate --strict --format junit --output test-results.xml
        after-script:
          - pipe: atlassian/junit-report:1.0.0
            variables:
              REPORT_PATH: "test-results.xml"
  pull-requests:
    "**":
      - step:
          name: Breaking Change Check
          script:
            - curl -fsSL https://install.valiforge.dev | sh
            - export PATH="$HOME/.valiforge/bin:$PATH"
            - valiforge diff --base origin/main --strict
```

## Best Practices

**Fail fast.** Run ValiForge early in your pipeline, before slower test suites. Schema validation takes seconds and catches contract issues immediately.

**Cache the binary.** ValiForge is a single binary. Cache `~/.valiforge/` between CI runs to skip the download step (~15 seconds saved).

**Use `--strict` in CI.** Always pass `--strict` in CI to treat warnings as errors. Warnings in local development become blockers in the pipeline.

**Pin the version.** Use `--version 0.1.0` in the install command to ensure reproducible builds. Unpin periodically to pick up improvements.

**Use SARIF for GitHub.** If you use GitHub, output SARIF format and upload to GitHub Code Scanning. Validation issues appear inline on the PR diff.

**Run breaking change detection on PRs only.** The `valiforge diff` command requires git history. Run it on PRs against the base branch, not on push-to-main (where the diff is already merged).

**Parallelize across schemas.** In monorepos with multiple schemas, use a CI matrix to validate each schema in parallel.
````

### 4.4 OpenAPI Validation Guide

**File:** `docs/src/content/docs/guides/openapi-validation.mdx`

````mdx
---
title: OpenAPI Validation
description: What ValiForge checks, common violations, and how to write better OpenAPI specs.
---

import { Callout } from '@astrojs/starlight/components';

# OpenAPI Validation Guide

ValiForge validates OpenAPI 3.0 and 3.1 specifications for correctness, completeness, and best practices. This guide explains what gets checked, common issues, and how to fix them.

## What Gets Validated

ValiForge runs four categories of validation rules against your OpenAPI spec:

### 1. Structural Validity
Checks that the spec is valid according to the OpenAPI specification itself.

| Rule | Code | Description |
|------|------|-------------|
| Valid OpenAPI version | VF0001 | `openapi` field must be `3.0.x` or `3.1.x` |
| Required fields present | VF0002 | `info`, `paths` (or `webhooks` in 3.1) must exist |
| Valid JSON Schema | VF0003 | All schemas must be valid JSON Schema |
| Valid `$ref` targets | VF0004 | All `$ref` pointers must resolve to existing definitions |
| Valid path parameters | VF0005 | Path parameters in URL must have corresponding parameter objects |
| Unique operationIds | VF0006 | Every operation must have a unique `operationId` |

### 2. Response Completeness
Checks that every endpoint defines its responses properly.

| Rule | Code | Description |
|------|------|-------------|
| Response schema defined | VF0042 | Every response with a body must have a content schema |
| Error responses defined | VF0043 | Endpoints should define 4xx and 5xx responses |
| Success response defined | VF0044 | Every operation must define at least one 2xx response |
| Content-type specified | VF0045 | Response content must declare a media type |

### 3. Security
Checks for security best practices.

| Rule | Code | Description |
|------|------|-------------|
| Auth defined | VF0060 | API should define at least one security scheme |
| Auth applied | VF0061 | Endpoints should have security requirements (or explicit opt-out) |
| HTTPS only | VF0062 | Server URLs should use `https://` |
| No credentials in URL | VF0063 | API keys should not be passed in query parameters |

### 4. Best Practices (Warnings)
Style and convention checks that improve API quality.

| Rule | Code | Description |
|------|------|-------------|
| Descriptions present | VFW010 | Operations, parameters, and schemas should have descriptions |
| Pagination parameters | VFW012 | Collection endpoints should include pagination |
| Consistent naming | VFW013 | Property names should follow a consistent convention |
| Examples provided | VFW014 | Schemas should include example values |
| Tags used | VFW015 | Operations should be organized with tags |

## Common Violations and How to Fix Them

### VF0042 — Missing Response Schema

**Problem:** A response code is defined but has no content schema.

```yaml
# Bad — no schema for 200
responses:
  "200":
    description: Success
```

**Fix:** Add a `content` block with a media type and schema.

```yaml
# Good
responses:
  "200":
    description: Success
    content:
      application/json:
        schema:
          type: object
          properties:
            id:
              type: integer
            name:
              type: string
```

### VF0004 — Broken $ref Reference

**Problem:** A `$ref` points to a definition that does not exist.

```yaml
# Bad — "Pets" doesn't exist, should be "Pet"
schema:
  $ref: "#/components/schemas/Pets"
```

**Fix:** ValiForge suggests the closest match (Levenshtein distance).

```
error[VF0004]: Unresolved reference "#/components/schemas/Pets"
  --> api/openapi.yaml:34:15
   |
34 |         $ref: "#/components/schemas/Pets"
   |               ^^^^^^^^^^^^^^^^^^^^^^^^^^ reference target not found
   |
   = help: Did you mean "#/components/schemas/Pet"?
```

### VF0005 — Path Parameter Mismatch

**Problem:** A path contains `{petId}` but no corresponding parameter object exists.

```yaml
# Bad — parameter named "id" but path uses "petId"
/pets/{petId}:
  get:
    parameters:
      - name: id          # Should be "petId"
        in: path
```

**Fix:** Ensure the parameter `name` matches the path template exactly.

### VFW012 — Missing Pagination

**Problem:** A `GET` endpoint returns an array but has no pagination parameters.

```yaml
# Triggers warning
/pets:
  get:
    responses:
      "200":
        content:
          application/json:
            schema:
              type: array    # Unbounded array — could return millions of records
```

**Fix:** Add `limit` and `offset` (or `cursor`) query parameters.

```yaml
/pets:
  get:
    parameters:
      - name: limit
        in: query
        schema:
          type: integer
          default: 20
          maximum: 100
      - name: offset
        in: query
        schema:
          type: integer
          default: 0
```

## OpenAPI 3.0 vs 3.1

ValiForge supports both versions. Key differences that affect validation:

| Feature | OpenAPI 3.0 | OpenAPI 3.1 |
|---------|-------------|-------------|
| JSON Schema dialect | Modified subset | Standard 2020-12 |
| `nullable` keyword | `nullable: true` | `type: ["string", "null"]` |
| `exclusiveMinimum` | `boolean` | `number` |
| Webhooks | Not supported | `webhooks` field |
| `pathItems` in components | Not supported | Supported |

<Callout type="tip">
ValiForge auto-detects the version from the `openapi` field and applies the correct validation rules. You do not need to specify the version in `valiforge.toml`.
</Callout>

### Migrating 3.0 to 3.1

If you are migrating, ValiForge can help:

```bash
valiforge validate --include VF-3.1-compat
```

This runs additional rules that flag 3.0 constructs that are invalid or deprecated in 3.1, with suggestions for the 3.1 equivalent.

## Schema Design Tips

1. **Always define `operationId`.** ValiForge uses it to identify endpoints in output. Without it, endpoints are identified by method + path, which is harder to read.

2. **Use `$ref` for shared schemas.** Define types once in `components/schemas` and reference them. ValiForge validates that all references resolve.

3. **Include examples.** ValiForge's test data generator uses `example` values as seeds for better test data. Specs without examples get generic test data.

4. **Define error responses.** At minimum, define `400`, `401`, `404`, and `500` responses. ValiForge flags endpoints that only define the happy path.

5. **Use enums for constrained values.** `status: { type: string, enum: [active, inactive] }` is more useful than `status: { type: string }` for validation and test data generation.
````

---

## 5. Launch Playbook

### T-14 Days: Prep Checklist

- [ ] All `P0` issues in DX, DOC, and COM workstreams are complete
- [ ] `README.md` reviewed by 3 people who have never seen ValiForge
- [ ] `valiforge init` + `valiforge validate` works on macOS (Intel + Apple Silicon), Ubuntu 22.04, Ubuntu 24.04
- [ ] Documentation site (`docs.valiforge.dev`) deployed with: Getting Started, Config Reference, CI/CD Guide, OpenAPI Guide
- [ ] Install script tested on fresh VMs (macOS, Ubuntu, Debian, Fedora, Alpine)
- [ ] `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `LICENSE` in repo root
- [ ] 10 "Good First Issue" issues created with mentors assigned
- [ ] Demo video scripted: 90-second terminal recording showing install, init, validate, diff
- [ ] Record demo with `vhs` (Charm) — save as `demo.gif` for README and social
- [ ] Social media accounts created: Twitter/X `@valiforge`, Discord server, GitHub org
- [ ] Blog post drafts written: "Why We Built ValiForge in Rust", "API Testing Is Broken"
- [ ] Press kit: logo (SVG, PNG), one-liner, founder photo, product screenshots

### T-7 Days: Seed Community

- [ ] Publish DEV.to draft (unlisted): "API Testing Is Broken: Here's How We Fix It"
- [ ] Draft Twitter/X launch thread (see below)
- [ ] Draft Reddit posts for r/rust and r/programming (see below)
- [ ] Draft Hacker News post (see below)
- [ ] Invite 20 beta users from personal network — ask for GitHub stars and feedback
- [ ] Set up Product Hunt listing (ship page, first 5 upvotes from team)
- [ ] Email newsletter list: send "launching next week" teaser to waitlist
- [ ] Test all install methods one final time on fresh machines
- [ ] Verify all documentation links work (run link checker)

### T-3 Days: Final Checks

- [ ] Tag release `v0.1.0` on GitHub with pre-built binaries for all targets
- [ ] Publish to crates.io: `cargo publish`
- [ ] Publish to npm: `npm publish`
- [ ] Submit Homebrew formula to `valiforge/homebrew-tap`
- [ ] Run `valiforge doctor` on 3 different machines — verify all green
- [ ] Product Hunt listing finalized: tagline, description, 4 screenshots, demo GIF
- [ ] Schedule tweets (if using a scheduler)
- [ ] Brief any supporters who will amplify launch day

### Launch Day (T+0)

#### Hacker News Post

**Title:** `ValiForge: Headless API Validation Engine Written in Rust`

**Body (HN comment):**

```
Hi HN, I'm building ValiForge — a headless API validation engine written in Rust.

The problem: AI coding tools (Copilot, Cursor, Claude Code) generate API endpoints
faster than teams can review them. Existing test tools (Postman, Selenium, Cypress)
are too slow, too heavy, or too brittle for this pace. E2E test suites break on 60%+
of AI-generated PRs.

ValiForge takes a different approach:
- Validates API contracts (OpenAPI, gRPC, GraphQL) against schemas — not UI behavior
- Ships as a single ~8MB static binary — no Docker, no JVM, no browser
- Cold start under 200ms — validates the Stripe API spec (15K+ lines) in <2 seconds
- Detects breaking changes between schema versions
- Generates edge-case test data using small language models
- Outputs JUnit/SARIF/JSON for any CI system

Quick start:
  curl -fsSL https://install.valiforge.dev | sh
  valiforge init
  valiforge validate

It's open source (Apache 2.0). Would love feedback from anyone doing API testing
at scale — especially interested in what validation rules would be most useful.

GitHub: https://github.com/valiforge/valiforge
Docs: https://docs.valiforge.dev
```

#### Reddit r/rust Post

**Title:** `ValiForge: a headless API validation engine — our first production Rust project`

**Body:**

```
We just open-sourced ValiForge — a headless API validation engine built entirely in Rust.

Why Rust: We needed sub-200ms cold starts, zero dependencies (no Docker, no JVM), and
cross-compilation to every major platform from a single CI matrix. Rust delivered on all
three. The release binary is ~8MB with LTO.

Architecture:
- Cargo workspace with 6 crates (cli, core, schema, diff, report, datagen)
- openapiv3 crate for OpenAPI parsing
- jsonschema for response validation
- clap derive for CLI
- owo-colors for terminal output
- proptest for property-based testing

We'd love feedback on:
1. Crate structure — are we splitting too fine or too coarse?
2. Error handling — we use thiserror in libraries, anyhow in the CLI binary
3. Performance — any suggestions for the schema parsing hot path?

GitHub: https://github.com/valiforge/valiforge
Crate: https://crates.io/crates/valiforge

Licensed Apache 2.0. Happy to answer any questions about the Rust choices.
```

#### DEV.to Article Intro

**Title:** "API Testing Is Broken. Here's How We're Fixing It."

```
AI coding assistants now generate 30–50% of production code at leading engineering
organizations. GitHub Copilot has 20M+ users. Cursor is at $500M+ ARR. The code
generation problem is solved.

But what about the code *validation* problem?

Teams using AI coding tools report that their E2E test suites break on 60%+ of
AI-generated PRs. Selenium tests fail because AI changed a CSS class name. Cypress
tests time out because AI restructured a component. The tests that were supposed to
catch bugs are now the biggest source of friction.

We built ValiForge to fix this. Instead of testing pixels, we test contracts...

[Continue reading →]
```

#### Twitter/X Thread

**Tweet 1 (anchor):**
```
We just open-sourced ValiForge — a headless API validation engine written in Rust.

One binary. Zero dependencies. Sub-200ms cold start.

Validates API contracts against OpenAPI, gRPC, and GraphQL schemas.

Here's why we built it 🧵

github.com/valiforge/valiforge
```

**Tweet 2:**
```
AI generates code faster than humans can review it.

At scale, 60%+ of AI-generated PRs break existing E2E tests — not because the code
is wrong, but because the tests are brittle.

Selenium and Cypress test UI pixels. ValiForge tests API contracts.
```

**Tweet 3:**
```
ValiForge in 30 seconds:

curl -fsSL https://install.valiforge.dev | sh
valiforge init
valiforge validate

That's it. Auto-detects your API schema, runs 50+ validation rules, outputs
results in <1 second.

[attach demo.gif]
```

**Tweet 4:**
```
Why Rust?

- ~8MB static binary (vs 200MB+ for Postman/JVM tools)
- <200ms cold start (vs 5-30 seconds)
- Validates the Stripe API spec (15K+ lines) in <2 seconds
- Cross-compiles to Linux, macOS, Windows from one CI job
- Memory-safe by default
```

**Tweet 5:**
```
Built for CI/CD, not GUIs:

- GitHub Actions, GitLab CI, Jenkins, CircleCI — works everywhere
- JUnit, SARIF, JSON, TAP output formats
- Detects breaking changes between schema versions
- Blocks PRs that violate API contracts

Docs: docs.valiforge.dev/guides/ci-cd
```

**Tweet 6:**
```
ValiForge is open source (Apache 2.0).

We'd love your feedback:
- What validation rules would be most useful?
- What CI system should we support better?
- What API schema format do you use most?

Star, fork, or file an issue:
github.com/valiforge/valiforge

Discord: discord.gg/valiforge
```

#### Product Hunt Listing

- **Tagline:** "Headless API validation — Rust-fast, AI-native, zero config"
- **Description:** "ValiForge validates API contracts (REST, gRPC, GraphQL) against schemas in milliseconds. One binary. Zero dependencies. Built for the AI coding era where E2E tests break faster than humans can fix them. Drop it into any CI/CD pipeline — GitHub Actions, GitLab CI, Jenkins — and catch breaking changes, missing schemas, and security issues before they reach production."
- **Topics:** Developer Tools, API, Testing, Open Source, Rust
- **First Comment (maker):** "Hi PH! I built ValiForge because..."

### T+1 to T+7: Follow-Up Engagement

| Day | Action |
|-----|--------|
| T+1 | Respond to every HN comment, Reddit comment, and GitHub issue within 4 hours |
| T+1 | Retweet/share anyone who posts about ValiForge |
| T+2 | Publish "Why We Built ValiForge in Rust" blog post, cross-post to DEV.to and r/rust |
| T+3 | Write a "Thank you + what we learned" tweet thread summarizing launch feedback |
| T+4 | Open 5 new issues based on launch feedback, label them `good-first-issue` |
| T+5 | Reach out to any reviewer/commenter who identified themselves as a potential user — offer a 1:1 onboarding call |
| T+6 | Submit to Rust newsletter "This Week in Rust" |
| T+7 | Publish launch week retrospective internally: stars, installs, issues, PRs, traffic |

### T+30: Retrospective Metrics

| Metric | Target | How to Measure |
|--------|--------|----------------|
| GitHub stars | 500+ | GitHub API |
| crates.io downloads | 300+ | crates.io API |
| npm downloads | 200+ | npm API |
| Discord members | 100+ | Discord server stats |
| GitHub Issues opened | 30+ | GitHub API |
| External PRs merged | 5+ | GitHub contributor graph |
| Blog post views | 5,000+ | Analytics |
| HN post points | 50+ | HN API |
| Documentation page views | 2,000+ | Plausible/Umami |
| CI integrations (by schema count in telemetry) | 50+ | Opt-in telemetry |

---

## 6. Demo & Examples

### Directory Structure

```
examples/
├── petstore/
│   ├── README.md
│   ├── openapi.yaml
│   └── valiforge.toml
├── github-action/
│   ├── README.md
│   └── .github/
│       └── workflows/
│           └── api-validation.yml
└── docker/
    ├── README.md
    ├── Dockerfile
    └── docker-compose.yml
```

### `examples/petstore/openapi.yaml`

```yaml
openapi: "3.1.0"
info:
  title: Petstore
  description: A sample API for ValiForge demonstration
  version: "1.0.0"
  license:
    name: Apache 2.0
    url: https://www.apache.org/licenses/LICENSE-2.0
servers:
  - url: https://petstore.example.com/v1
paths:
  /pets:
    get:
      summary: List all pets
      operationId: listPets
      tags:
        - pets
      parameters:
        - name: limit
          in: query
          description: Maximum number of items to return
          required: false
          schema:
            type: integer
            default: 20
            maximum: 100
      responses:
        "200":
          description: A paginated list of pets
          content:
            application/json:
              schema:
                type: object
                required:
                  - data
                  - pagination
                properties:
                  data:
                    type: array
                    items:
                      $ref: "#/components/schemas/Pet"
                  pagination:
                    $ref: "#/components/schemas/Pagination"
        "500":
          description: Internal server error
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/Error"
    post:
      summary: Create a pet
      operationId: createPet
      tags:
        - pets
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: "#/components/schemas/CreatePetRequest"
      responses:
        "201":
          description: Pet created
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/Pet"
        "400":
          description: Invalid input
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/Error"
  /pets/{petId}:
    get:
      summary: Get a pet by ID
      operationId: getPetById
      tags:
        - pets
      parameters:
        - name: petId
          in: path
          required: true
          description: The ID of the pet to retrieve
          schema:
            type: string
            format: uuid
      responses:
        "200":
          description: A single pet
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/Pet"
        "404":
          description: Pet not found
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/Error"
components:
  schemas:
    Pet:
      type: object
      required:
        - id
        - name
        - status
      properties:
        id:
          type: string
          format: uuid
          example: "550e8400-e29b-41d4-a716-446655440000"
        name:
          type: string
          example: "Buddy"
        tag:
          type: string
          example: "dog"
        status:
          type: string
          enum:
            - available
            - pending
            - adopted
          example: "available"
    CreatePetRequest:
      type: object
      required:
        - name
      properties:
        name:
          type: string
          minLength: 1
          maxLength: 100
        tag:
          type: string
        status:
          type: string
          enum:
            - available
            - pending
            - adopted
          default: available
    Pagination:
      type: object
      properties:
        total:
          type: integer
        limit:
          type: integer
        offset:
          type: integer
    Error:
      type: object
      required:
        - code
        - message
      properties:
        code:
          type: integer
        message:
          type: string
  securitySchemes:
    BearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT
security:
  - BearerAuth: []
```

### `examples/petstore/valiforge.toml`

```toml
# ValiForge configuration for the Petstore example
# Run: valiforge validate

[project]
name = "petstore"
schema = "openapi.yaml"

[rules]
strict = false

[output]
format = "text"
color = "auto"
```

### `examples/petstore/README.md`

````markdown
# Petstore Example

Demonstrates ValiForge validating a simple Petstore OpenAPI 3.1 spec.

## Run

```bash
cd examples/petstore
valiforge validate
```

## Expected Output

```
 ValiForge v0.1.0

  Validating: openapi.yaml (OpenAPI 3.1)
  ─────────────────────────────────────────────

  ✓ GET  /pets           200 schema valid
  ✓ GET  /pets           500 schema valid
  ✓ POST /pets           201 schema valid
  ✓ POST /pets           400 schema valid
  ✓ GET  /pets/{petId}   200 schema valid
  ✓ GET  /pets/{petId}   404 schema valid

  ─────────────────────────────────────────────
  6 passed · 0 errors · 0 warnings · 0.12s
```

All validations pass because this spec follows best practices: every response has a schema, error responses are defined, pagination is included, security is configured, and descriptions are present.
````

### `examples/github-action/.github/workflows/api-validation.yml`

```yaml
name: API Validation
on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install ValiForge
        run: curl -fsSL https://install.valiforge.dev | sh
      - name: Validate
        run: |
          export PATH="$HOME/.valiforge/bin:$PATH"
          valiforge validate --strict
```

### `examples/docker/Dockerfile`

```dockerfile
FROM rust:1.75-slim AS builder
RUN cargo install valiforge

FROM debian:bookworm-slim
COPY --from=builder /usr/local/cargo/bin/valiforge /usr/local/bin/valiforge
WORKDIR /workspace
ENTRYPOINT ["valiforge"]
CMD ["validate"]
```

### `examples/docker/docker-compose.yml`

```yaml
version: "3.8"
services:
  valiforge:
    build: .
    volumes:
      - ./api:/workspace/api:ro
      - ./valiforge.toml:/workspace/valiforge.toml:ro
    command: validate --strict --format json
```

---

## 7. Community Templates

### `.github/ISSUE_TEMPLATE/bug_report.yml`

```yaml
name: Bug Report
description: Report a bug in ValiForge
title: "[Bug]: "
labels: ["bug", "needs-triage"]
body:
  - type: markdown
    attributes:
      value: |
        Thank you for reporting a bug. Please fill out the sections below so we can reproduce and fix it.

  - type: input
    id: version
    attributes:
      label: ValiForge Version
      description: "Output of `valiforge --version`"
      placeholder: "valiforge 0.1.0 (a1b2c3d 2026-03-15, rustc 1.75.0)"
    validations:
      required: true

  - type: dropdown
    id: os
    attributes:
      label: Operating System
      options:
        - macOS (Apple Silicon)
        - macOS (Intel)
        - Ubuntu / Debian
        - Fedora / RHEL
        - Alpine Linux
        - Arch Linux
        - Windows (WSL)
        - Other (specify below)
    validations:
      required: true

  - type: dropdown
    id: schema_type
    attributes:
      label: Schema Type
      options:
        - OpenAPI 3.0
        - OpenAPI 3.1
        - GraphQL
        - Protobuf
        - AsyncAPI
        - N/A
    validations:
      required: true

  - type: textarea
    id: description
    attributes:
      label: What happened?
      description: A clear description of the bug.
      placeholder: "When I run `valiforge validate` on my OpenAPI spec, I get..."
    validations:
      required: true

  - type: textarea
    id: expected
    attributes:
      label: Expected behavior
      description: What should have happened instead?
    validations:
      required: true

  - type: textarea
    id: reproduction
    attributes:
      label: Minimal reproduction
      description: "The smallest config + schema that triggers the bug. Use code blocks."
      render: yaml
    validations:
      required: true

  - type: textarea
    id: logs
    attributes:
      label: Debug output
      description: "Output of `valiforge validate --debug` (paste full output)"
      render: shell
    validations:
      required: false
```

### `.github/ISSUE_TEMPLATE/feature_request.yml`

```yaml
name: Feature Request
description: Suggest a new feature or improvement
title: "[Feature]: "
labels: ["feature", "needs-triage"]
body:
  - type: textarea
    id: problem
    attributes:
      label: What problem does this solve?
      description: "Describe the use case. What are you trying to do?"
      placeholder: "I need to validate AsyncAPI specs in my event-driven architecture..."
    validations:
      required: true

  - type: textarea
    id: proposal
    attributes:
      label: Proposed solution
      description: "How should this work? CLI syntax, config options, expected output?"
    validations:
      required: true

  - type: textarea
    id: alternatives
    attributes:
      label: Alternatives considered
      description: "What else have you tried? Other tools, workarounds?"
    validations:
      required: false

  - type: dropdown
    id: willingness
    attributes:
      label: Would you like to submit a PR?
      options:
        - "Yes — I can implement this"
        - "Yes — but I need guidance"
        - "No — just a suggestion"
    validations:
      required: true
```

### `.github/ISSUE_TEMPLATE/question.yml`

```yaml
name: Question
description: Ask a question about ValiForge
title: "[Question]: "
labels: ["question"]
body:
  - type: markdown
    attributes:
      value: |
        For quick questions, consider joining our [Discord](https://discord.gg/valiforge).

        For questions that might help others, please continue here — this will be searchable.

  - type: textarea
    id: question
    attributes:
      label: Your question
      description: "What would you like to know?"
    validations:
      required: true

  - type: textarea
    id: context
    attributes:
      label: Context
      description: "What are you trying to accomplish? What have you tried?"
    validations:
      required: false
```

### `.github/PULL_REQUEST_TEMPLATE.md`

```markdown
## What does this PR do?

<!-- Brief description of the change -->

## Related Issues

<!-- Link to related issues: Fixes #123, Relates to #456 -->

## Checklist

- [ ] `just ci` passes locally (build, test, lint, fmt)
- [ ] New/changed public APIs have doc comments with examples
- [ ] New features have unit tests
- [ ] Error messages follow VF error format (code, location, suggestion)
- [ ] CHANGELOG.md updated (under `## [Unreleased]`)
- [ ] No new `.unwrap()` calls in library code
- [ ] Breaking changes documented in this PR description

## Testing

<!-- How did you test this? What should reviewers check? -->

## Screenshots / Terminal Output

<!-- If relevant, show the before/after terminal output -->
```

### Discord Channel Structure

```
INFO
  #welcome          Read-only. Server rules, links, getting started.
  #announcements    Read-only. Releases, blog posts, events.
  #changelog        Read-only. Bot-posted release notes.

COMMUNITY
  #general          Open discussion about ValiForge and API testing.
  #showcase         Share what you've built with ValiForge.
  #off-topic        Non-ValiForge chat.

SUPPORT (Forum-style)
  #help             Ask questions. Each question becomes a thread.
  #troubleshooting  Post error output and get help debugging.

DEVELOPMENT
  #contributors     Contributor discussion, PR reviews, pairing.
  #rfc              Discuss proposed features and architecture changes.
  #ci-cd            CI/CD integration questions and tips.

VOICE
  Office Hours      Monthly live Q&A with the core team.
  Pair Programming  Open voice channel for pairing sessions.
```

### `SECURITY.md`

```markdown
# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in ValiForge, please report it responsibly.

**Do NOT open a public GitHub issue for security vulnerabilities.**

Instead, email us at: **security@valiforge.dev**

Include:
1. Description of the vulnerability
2. Steps to reproduce
3. Potential impact
4. Suggested fix (if you have one)

## Response Timeline

| Action | Timeline |
|--------|----------|
| Acknowledgment | Within 48 hours |
| Initial assessment | Within 5 business days |
| Fix development | Within 30 days for critical, 90 days for non-critical |
| Public disclosure | After fix is released, coordinated with reporter |

## Scope

The following are in scope:
- ValiForge CLI binary
- ValiForge SDK crate
- Install script (`install.valiforge.dev`)
- Documentation site (`docs.valiforge.dev`)

The following are out of scope:
- Third-party dependencies (report to the dependency maintainer)
- Social engineering attacks
- Denial of service attacks

## Recognition

We credit security researchers in our release notes (unless you prefer to remain anonymous).

## Supported Versions

| Version | Supported |
|---------|-----------|
| Latest release | Yes |
| Previous minor | Security fixes only |
| Older | No |
```

---

## 8. Terminal UX Specification

### Color Scheme

| Element | Color | ANSI Code | Usage |
|---------|-------|-----------|-------|
| Pass | Green | `\x1b[32m` | Checkmark and passing rule text |
| Fail | Red | `\x1b[31m` | Cross mark and error text |
| Warning | Yellow | `\x1b[33m` | Exclamation mark and warning text |
| Info | Blue | `\x1b[34m` | Informational messages, progress |
| Dimmed | Gray | `\x1b[2m` | Secondary text, separators, hints |
| Bold | Bold | `\x1b[1m` | Section headers, emphasis |
| Error code | Cyan | `\x1b[36m` | `VF0042`, `VFW012` |
| Path / URL | Underline | `\x1b[4m` | File paths and documentation links |

All colors respect:
- `NO_COLOR` environment variable (disable all colors)
- `--color never` flag
- Non-TTY output (pipe to file automatically disables colors)

### Progress Bar Format

```
  Validating 3 schemas...
  ████████████████████░░░░░░░░░░ 2/3 api/v2.yaml
```

Uses `indicatif` crate with:
- Bar width: 30 characters
- Update frequency: 100ms
- Cleared on completion (replaced by summary)
- Hidden when `CI=true` environment variable is set (CI systems render poorly)

### Table Layout for Results

```
  Validating: api/openapi.yaml (OpenAPI 3.1)
  ─────────────────────────────────────────────

  ✓ GET  /pets           200 schema valid
  ✓ GET  /pets/{petId}   200 schema valid
  ✗ POST /pets           400 missing response schema
  ! GET  /pets           pagination missing

  ─────────────────────────────────────────────
  3 passed · 1 error · 1 warning · 0.23s
```

Column alignment rules:
- Method: left-aligned, 6 chars wide (padded: `GET   `, `POST  `, `DELETE`)
- Path: left-aligned, truncated at 30 chars with ellipsis
- Status code: right-aligned, 3 chars
- Description: remainder of line

### Error Message Format

Follow Rust compiler conventions:

```
error[VF0042]: Missing response schema for status code 400
  --> api/openapi.yaml:47:5
   |
47 |     responses:
48 |       400:
   |       ^^^ this response is missing a `content` schema definition
   |
   = help: Add a `content` block with a media type and JSON Schema
   = docs: https://docs.valiforge.dev/errors/VF0042
```

### ASCII Art Logo (`--version` output)

```
 __     __    _ _ _____
 \ \   / /_ _| (_)  ___|__  _ __ __ _  ___
  \ \ / / _` | | | |_ / _ \| '__/ _` |/ _ \
   \ V / (_| | | |  _| (_) | | | (_| |  __/
    \_/ \__,_|_|_|_|  \___/|_|  \__, |\___|
                                |___/
  v0.1.0 (a1b2c3d 2026-03-15, rustc 1.75.0)
```

---

## 9. Changelog Format

### `CHANGELOG.md` Template

```markdown
# Changelog

All notable changes to ValiForge are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- New validation rule VF0065: detect missing `Content-Type` headers in responses

### Changed
- Improved error message for VF0042 to include a code snippet suggestion

### Fixed
- Fixed crash when parsing OpenAPI specs with circular `$ref` references (#142)

## [0.1.0] - 2026-03-15

### Added
- Initial release
- OpenAPI 3.0 and 3.1 validation (50+ built-in rules)
- Breaking change detection (`valiforge diff`)
- Output formats: text, JSON, JUnit XML, SARIF, TAP
- Cross-platform install script
- GitHub Actions, GitLab CI, Jenkins integration examples
- Shell completions for bash, zsh, fish, PowerShell
- `valiforge init` with auto-detection of API schemas
- `valiforge doctor` for setup diagnostics
- `valiforge explain` for error code documentation

[Unreleased]: https://github.com/valiforge/valiforge/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/valiforge/valiforge/releases/tag/v0.1.0
```

### Release Notes Template

```markdown
# ValiForge v0.X.0

## Highlights

**One sentence about the most exciting thing in this release.**

Detailed paragraph about the highlight feature, with a terminal output example or code snippet.

## What's New

- **Feature name** — one-line description ([#PR](link))
- **Feature name** — one-line description ([#PR](link))

## Bug Fixes

- Fixed description of bug ([#issue](link))

## Breaking Changes

- Description of breaking change and migration path

## Contributors

Thank you to the N contributors who made this release possible:

@username1, @username2, @username3

**Full Changelog:** https://github.com/valiforge/valiforge/compare/v0.X-1.0...v0.X.0
```

### Migration Guide Template

```markdown
# Migrating from v0.X to v0.Y

## Breaking Changes

### 1. Config key renamed: `rules.fail_on` → `rules.min_severity`

**Before (v0.X):**
```toml
[rules]
fail_on = "warning"
```

**After (v0.Y):**
```toml
[rules]
min_severity = "warning"
```

### 2. CLI flag changed: `--fail-on` → `--min-severity`

**Before:** `valiforge validate --fail-on warning`
**After:** `valiforge validate --min-severity warning`

## Deprecations

- `--json` flag is deprecated. Use `--format json` instead. Will be removed in v0.Z.

## New Features Available After Migration

- Feature X is now available after upgrading
```

---

## 10. Analytics & Metrics

### What to Track

| Category | Metric | Purpose |
|----------|--------|---------|
| Adoption | Install count (by method: curl, cargo, brew, npm) | Track growth |
| Adoption | `valiforge init` runs per week | Track activation |
| Usage | `valiforge validate` runs per week | Track engagement |
| Usage | Schema types validated (OpenAPI, gRPC, GraphQL) | Prioritize support |
| Usage | Output formats used (text, json, junit, sarif) | Prioritize formats |
| Performance | Cold start time (P50, P95) | Monitor regressions |
| Performance | Validation time per 1K endpoints | Benchmark improvements |
| Errors | Top 10 error codes triggered | Improve error messages |
| Errors | Crash reports (panics, if any) | Fix stability issues |
| Environment | OS / architecture distribution | Prioritize platforms |
| Environment | CI system detection (GitHub Actions, GitLab, etc.) | Prioritize integrations |

### Privacy-First Telemetry Design

ValiForge telemetry follows these principles:

1. **Opt-in only.** Telemetry is disabled by default. It is never enabled silently.
2. **No PII.** Never collect: file paths, schema contents, API URLs, IP addresses, usernames.
3. **Aggregated only.** Individual events are aggregated before transmission. No individual session tracking.
4. **Transparent.** The exact payload is logged locally before sending. Users can inspect what is sent.
5. **Offline-safe.** Telemetry failures never affect CLI behavior. Fire-and-forget with a 2-second timeout.
6. **Open source.** The telemetry module is open source and auditable.

### Opt-In / Opt-Out Mechanism

**First run prompt (interactive terminals only):**

```
ValiForge collects anonymous usage statistics to improve the tool.
No personal data, file paths, or API content is ever collected.

Learn more: https://docs.valiforge.dev/privacy

Enable anonymous telemetry? [y/N]:
```

**Configuration:**

```toml
# valiforge.toml
[telemetry]
enabled = false  # default: false (opt-in)
```

**Environment variable override:**

```bash
VALIFORGE_TELEMETRY_ENABLED=false  # Disable globally
```

**CLI commands:**

```bash
valiforge telemetry status    # Show current telemetry status
valiforge telemetry enable    # Opt in
valiforge telemetry disable   # Opt out
```

### Telemetry Payload Example

```json
{
  "schema_version": 1,
  "timestamp": "2026-03-15T10:30:00Z",
  "valiforge_version": "0.1.0",
  "os": "darwin",
  "arch": "aarch64",
  "command": "validate",
  "duration_ms": 230,
  "schema_type": "openapi_3_1",
  "rules_passed": 47,
  "rules_failed": 1,
  "rules_warned": 3,
  "output_format": "text",
  "ci_detected": false,
  "session_id": "random-uuid-not-linked-to-user"
}
```

Fields that are **never** collected:
- File paths or file names
- Schema content or API definitions
- API URLs or server addresses
- IP addresses (stripped at ingestion)
- Error message content (only error codes)
- Any form of user identity

### Metrics Dashboard Requirements

Build a simple internal dashboard (Grafana + ClickHouse, or PostHog open source) showing:

1. **Daily active installations** — unique session IDs per day
2. **Command usage distribution** — pie chart of validate / init / diff / doctor
3. **Schema type distribution** — OpenAPI 3.0 vs 3.1 vs GraphQL vs Protobuf
4. **OS / arch distribution** — platform support prioritization
5. **Validation performance** — P50/P95 duration over time
6. **Top error codes** — most frequently triggered validation rules
7. **CI detection rate** — percentage of runs in CI vs local development
8. **Version adoption curve** — how quickly users upgrade to new versions
