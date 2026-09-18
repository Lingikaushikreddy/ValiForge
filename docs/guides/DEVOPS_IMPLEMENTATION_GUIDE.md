# ValiForge — DevOps / Infrastructure Implementation Guide

**For: Senior Platform / DevOps Engineer**
**Goal: Build the complete CI/CD, packaging, distribution, and cloud infrastructure for ValiForge**

---

## 1. CI/CD Pipeline (`.github/workflows/`)

### `ci.yml` — Full CI Pipeline

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
  SCCACHE_GHA_ENABLED: "true"
  RUSTC_WRAPPER: "sccache"

permissions:
  contents: read
  checks: write
  pull-requests: write

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
      - uses: mozilla-actions/sccache-action@v0.0.6
      - uses: Swatinem/rust-cache@v2
        with:
          shared-key: "clippy"
      - run: cargo clippy --workspace --all-targets --all-features -- -D warnings

  test:
    name: Tests (${{ matrix.os }})
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: macos-14
            target: aarch64-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: mozilla-actions/sccache-action@v0.0.6
      - uses: Swatinem/rust-cache@v2
        with:
          shared-key: "test-${{ matrix.os }}"
      - name: Run tests
        run: cargo test --workspace --all-features -- --nocapture
      - name: Run doc tests
        run: cargo test --workspace --doc
      - name: Upload test results
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: test-results-${{ matrix.os }}
          path: target/nextest/default/*.xml
          retention-days: 7

  audit:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: taiki-e/install-action@cargo-deny
      - run: cargo deny check all

  coverage:
    name: Code Coverage
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: llvm-tools-preview
      - uses: mozilla-actions/sccache-action@v0.0.6
      - uses: Swatinem/rust-cache@v2
        with:
          shared-key: "coverage"
      - uses: taiki-e/install-action@cargo-llvm-cov
      - name: Generate coverage
        run: cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info
      - name: Upload to Codecov
        uses: codecov/codecov-action@v4
        with:
          files: lcov.info
          token: ${{ secrets.CODECOV_TOKEN }}
          fail_ci_if_error: false

  msrv:
    name: MSRV Check (1.75.0)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: "1.75.0"
      - uses: Swatinem/rust-cache@v2
        with:
          shared-key: "msrv"
      - run: cargo check --workspace --all-features

  doc:
    name: Documentation
    runs-on: ubuntu-latest
    env:
      RUSTDOCFLAGS: "-D warnings"
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
        with:
          shared-key: "doc"
      - run: cargo doc --workspace --no-deps --all-features
```

---

### `release.yml` — Cross-Platform Release Pipeline

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags: ["v*.*.*"]

permissions:
  contents: write
  packages: write
  id-token: write

env:
  CARGO_TERM_COLOR: always
  BINARY_NAME: valiforge

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
            cross: false
          - target: aarch64-unknown-linux-musl
            os: ubuntu-latest
            archive: tar.gz
            cross: true
          - target: x86_64-apple-darwin
            os: macos-13
            archive: tar.gz
            cross: false
          - target: aarch64-apple-darwin
            os: macos-14
            archive: tar.gz
            cross: false
          - target: x86_64-pc-windows-msvc
            os: windows-latest
            archive: zip
            cross: false
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - uses: Swatinem/rust-cache@v2
        with:
          key: release-${{ matrix.target }}

      # Install musl tools for native x86_64 linux musl build
      - name: Install musl-tools
        if: matrix.target == 'x86_64-unknown-linux-musl'
        run: sudo apt-get update && sudo apt-get install -y musl-tools

      # Cross-compilation setup for aarch64-linux
      - name: Install cross-compilation tools
        if: matrix.cross
        run: |
          cargo install cargo-zigbuild --locked
      - uses: goto-bus-stop/setup-zig@v2
        if: matrix.cross
        with:
          version: 0.13.0

      # Native build
      - name: Build (native)
        if: "!matrix.cross"
        run: cargo build --release --locked --target ${{ matrix.target }} -p valiforge-cli

      # Cross build (aarch64-linux via zigbuild)
      - name: Build (cross)
        if: matrix.cross
        run: cargo zigbuild --release --locked --target ${{ matrix.target }} -p valiforge-cli

      # Generate shell completions
      - name: Generate completions
        shell: bash
        run: |
          mkdir -p completions
          if [ "${{ matrix.cross }}" = "true" ]; then
            echo "Skipping completions for cross-compiled target"
          else
            BIN="target/${{ matrix.target }}/release/${{ env.BINARY_NAME }}"
            if [ "${{ runner.os }}" = "Windows" ]; then
              BIN="${BIN}.exe"
            fi
            $BIN completions bash > completions/valiforge.bash 2>/dev/null || true
            $BIN completions zsh > completions/_valiforge 2>/dev/null || true
            $BIN completions fish > completions/valiforge.fish 2>/dev/null || true
          fi

      # Package (Unix: tar.gz)
      - name: Package (Unix)
        if: matrix.archive == 'tar.gz'
        shell: bash
        run: |
          STAGING="valiforge-${{ github.ref_name }}-${{ matrix.target }}"
          mkdir -p "$STAGING"
          cp "target/${{ matrix.target }}/release/${{ env.BINARY_NAME }}" "$STAGING/"
          cp LICENSE-APACHE LICENSE-MIT README.md "$STAGING/" 2>/dev/null || true
          cp -r completions "$STAGING/" 2>/dev/null || true
          tar czf "${STAGING}.tar.gz" "$STAGING"
          shasum -a 256 "${STAGING}.tar.gz" > "${STAGING}.tar.gz.sha256"
          cat "${STAGING}.tar.gz.sha256"

      # Package (Windows: zip)
      - name: Package (Windows)
        if: matrix.archive == 'zip'
        shell: pwsh
        run: |
          $STAGING = "valiforge-${{ github.ref_name }}-${{ matrix.target }}"
          New-Item -ItemType Directory -Path $STAGING
          Copy-Item "target/${{ matrix.target }}/release/${{ env.BINARY_NAME }}.exe" "$STAGING/"
          Copy-Item "LICENSE-APACHE","LICENSE-MIT","README.md" "$STAGING/" -ErrorAction SilentlyContinue
          Copy-Item "completions" "$STAGING/" -Recurse -ErrorAction SilentlyContinue
          Compress-Archive -Path "$STAGING" -DestinationPath "${STAGING}.zip"
          $hash = (Get-FileHash "${STAGING}.zip" -Algorithm SHA256).Hash.ToLower()
          "${hash}  ${STAGING}.zip" | Out-File -Encoding ASCII "${STAGING}.zip.sha256"
          Get-Content "${STAGING}.zip.sha256"

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: dist-${{ matrix.target }}
          path: |
            valiforge-${{ github.ref_name }}-${{ matrix.target }}.*

  release:
    name: Create GitHub Release
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

      - name: Create combined checksums
        working-directory: dist
        run: |
          cat *.sha256 > checksums-${{ github.ref_name }}.sha256
          echo "=== Combined checksums ==="
          cat checksums-${{ github.ref_name }}.sha256

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
          files: |
            dist/*
          prerelease: ${{ contains(github.ref, 'rc') || contains(github.ref, 'beta') || contains(github.ref, 'alpha') }}
          generate_release_notes: false

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
          for crate in valiforge-core valiforge-schema valiforge-diff valiforge-datagen valiforge-report valiforge-cli; do
            echo "Publishing $crate..."
            cargo publish -p "$crate" --locked || echo "::warning::$crate publish failed or already published"
            sleep 30
          done

  publish-npm:
    name: Publish npm wrapper
    needs: release
    if: "!contains(github.ref, 'rc') && !contains(github.ref, 'beta') && !contains(github.ref, 'alpha')"
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          registry-url: https://registry.npmjs.org
      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          path: dist
          merge-multiple: true
      - name: Prepare and publish npm packages
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
        run: |
          VERSION="${GITHUB_REF_NAME#v}"
          cd npm
          node scripts/prepare-platform-packages.js "$VERSION" ../dist
          for pkg in valiforge-linux-x64 valiforge-linux-arm64 valiforge-darwin-x64 valiforge-darwin-arm64 valiforge-win32-x64; do
            cd "$pkg" && npm publish --access public && cd ..
          done
          cd valiforge
          npm version "$VERSION" --no-git-tag-version
          npm publish --access public

  update-homebrew:
    name: Update Homebrew formula
    needs: release
    if: "!contains(github.ref, 'rc') && !contains(github.ref, 'beta') && !contains(github.ref, 'alpha')"
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Download checksums
        uses: actions/download-artifact@v4
        with:
          path: dist
          merge-multiple: true
      - name: Update Homebrew tap
        env:
          HOMEBREW_TAP_TOKEN: ${{ secrets.HOMEBREW_TAP_TOKEN }}
        run: |
          VERSION="${GITHUB_REF_NAME#v}"
          SHA_DARWIN_ARM=$(grep "aarch64-apple-darwin" dist/checksums-${{ github.ref_name }}.sha256 | awk '{print $1}')
          SHA_DARWIN_X64=$(grep "x86_64-apple-darwin" dist/checksums-${{ github.ref_name }}.sha256 | awk '{print $1}')
          SHA_LINUX_ARM=$(grep "aarch64-unknown-linux-musl" dist/checksums-${{ github.ref_name }}.sha256 | awk '{print $1}')
          SHA_LINUX_X64=$(grep "x86_64-unknown-linux-musl" dist/checksums-${{ github.ref_name }}.sha256 | awk '{print $1}')

          git clone "https://x-access-token:${HOMEBREW_TAP_TOKEN}@github.com/valiforge/homebrew-tap.git" tap
          cd tap

          cat > Formula/valiforge.rb << FORMULA
          class Valiforge < Formula
            desc "Headless API-first validation engine for AI workloads"
            homepage "https://valiforge.dev"
            version "${VERSION}"
            license "Apache-2.0"

            on_macos do
              on_arm do
                url "https://github.com/valiforge/valiforge/releases/download/v#{version}/valiforge-v#{version}-aarch64-apple-darwin.tar.gz"
                sha256 "${SHA_DARWIN_ARM}"
              end
              on_intel do
                url "https://github.com/valiforge/valiforge/releases/download/v#{version}/valiforge-v#{version}-x86_64-apple-darwin.tar.gz"
                sha256 "${SHA_DARWIN_X64}"
              end
            end

            on_linux do
              on_arm do
                url "https://github.com/valiforge/valiforge/releases/download/v#{version}/valiforge-v#{version}-aarch64-unknown-linux-musl.tar.gz"
                sha256 "${SHA_LINUX_ARM}"
              end
              on_intel do
                url "https://github.com/valiforge/valiforge/releases/download/v#{version}/valiforge-v#{version}-x86_64-unknown-linux-musl.tar.gz"
                sha256 "${SHA_LINUX_X64}"
              end
            end

            def install
              bin.install "valiforge"
              bash_completion.install "completions/valiforge.bash" => "valiforge"
              zsh_completion.install "completions/_valiforge"
              fish_completion.install "completions/valiforge.fish"
            end

            test do
              assert_match version.to_s, shell_output("#{bin}/valiforge --version")
            end
          end
          FORMULA

          git config user.name "valiforge-bot"
          git config user.email "bot@valiforge.dev"
          git add Formula/valiforge.rb
          git commit -m "Update ValiForge to ${VERSION}"
          git push origin main
```

---

### `bench.yml` — Benchmark Pipeline

```yaml
# .github/workflows/bench.yml
name: Benchmarks

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

permissions:
  contents: read
  pull-requests: write

env:
  CARGO_TERM_COLOR: always

jobs:
  benchmark:
    name: Run benchmarks
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - uses: Swatinem/rust-cache@v2
        with:
          shared-key: "bench"

      - name: Run benchmarks
        run: cargo bench --workspace -- --output-format bencher | tee bench-output.txt

      - name: Download baseline
        if: github.event_name == 'pull_request'
        uses: actions/cache/restore@v4
        with:
          path: bench-baseline.txt
          key: bench-baseline-main

      - name: Compare against baseline
        if: github.event_name == 'pull_request' && hashFiles('bench-baseline.txt') != ''
        id: compare
        run: |
          python3 << 'PYEOF'
          import re, sys

          def parse_bencher(path):
              results = {}
              with open(path) as f:
                  for line in f:
                      m = re.match(r'^test\s+(\S+)\s+.*bench:\s+([\d,]+)\s+ns/iter', line)
                      if m:
                          name = m.group(1)
                          ns = int(m.group(2).replace(',', ''))
                          results[name] = ns
              return results

          baseline = parse_bencher('bench-baseline.txt')
          current = parse_bencher('bench-output.txt')

          regressions = []
          report_lines = ["| Benchmark | Baseline (ns) | Current (ns) | Change |", "|---|---|---|---|"]
          has_regression = False

          for name in sorted(set(list(baseline.keys()) + list(current.keys()))):
              base_ns = baseline.get(name)
              curr_ns = current.get(name)
              if base_ns and curr_ns:
                  pct = ((curr_ns - base_ns) / base_ns) * 100
                  emoji = "regression" if pct > 10 else ("improved" if pct < -5 else "unchanged")
                  report_lines.append(f"| `{name}` | {base_ns:,} | {curr_ns:,} | {pct:+.1f}% ({emoji}) |")
                  if pct > 10:
                      regressions.append(f"{name}: {pct:+.1f}%")
                      has_regression = True
              elif curr_ns:
                  report_lines.append(f"| `{name}` | N/A | {curr_ns:,} | new |")

          report = "\n".join(report_lines)
          with open("bench-report.md", "w") as f:
              f.write("## Benchmark Results\n\n")
              f.write(report)
              if regressions:
                  f.write("\n\n**Regressions (>10%):**\n")
                  for r in regressions:
                      f.write(f"- {r}\n")

          if has_regression:
              print("::error::Performance regression detected (>10%)")
              sys.exit(1)
          PYEOF

      - name: Post benchmark results as PR comment
        if: github.event_name == 'pull_request' && always()
        uses: actions/github-script@v7
        with:
          script: |
            const fs = require('fs');
            let body = '## Benchmark Results\n\nNo baseline available for comparison. Results will be stored for future PRs.';
            try {
              body = fs.readFileSync('bench-report.md', 'utf8');
            } catch (e) {}

            const { data: comments } = await github.rest.issues.listComments({
              owner: context.repo.owner,
              repo: context.repo.repo,
              issue_number: context.issue.number,
            });
            const existing = comments.find(c => c.body.includes('Benchmark Results'));
            const params = {
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: body,
            };
            if (existing) {
              await github.rest.issues.updateComment({ ...params, comment_id: existing.id });
            } else {
              await github.rest.issues.createComment({ ...params, issue_number: context.issue.number });
            }

      - name: Save baseline (main only)
        if: github.ref == 'refs/heads/main'
        uses: actions/cache/save@v4
        with:
          path: bench-output.txt
          key: bench-baseline-main
```

---

## 2. Dockerfile & Compose

### `docker/Dockerfile` — Multi-Stage Production Build

```dockerfile
# docker/Dockerfile
# ---- Builder stage ----
FROM rust:1.75-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    musl-tools \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /build

# Cache dependency builds: copy manifests first, build deps, then copy source
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/valiforge-core/Cargo.toml crates/valiforge-core/Cargo.toml
COPY crates/valiforge-schema/Cargo.toml crates/valiforge-schema/Cargo.toml
COPY crates/valiforge-diff/Cargo.toml crates/valiforge-diff/Cargo.toml
COPY crates/valiforge-report/Cargo.toml crates/valiforge-report/Cargo.toml
COPY crates/valiforge-datagen/Cargo.toml crates/valiforge-datagen/Cargo.toml
COPY crates/valiforge-cli/Cargo.toml crates/valiforge-cli/Cargo.toml

# Create stub sources so cargo can resolve the workspace
RUN for dir in crates/*/; do \
      mkdir -p "$dir/src"; \
      if [ -f "$dir/Cargo.toml" ] && grep -q '^\[\[bin\]\]' "$dir/Cargo.toml" 2>/dev/null; then \
        echo 'fn main() {}' > "$dir/src/main.rs"; \
      else \
        echo '' > "$dir/src/lib.rs"; \
      fi; \
    done && \
    echo 'fn main() {}' > crates/valiforge-cli/src/main.rs

RUN cargo build --release --locked --target x86_64-unknown-linux-musl -p valiforge-cli 2>/dev/null || true

# Copy real source and rebuild
COPY crates/ crates/
RUN touch crates/*/src/*.rs && \
    cargo build --release --locked --target x86_64-unknown-linux-musl -p valiforge-cli

RUN strip /build/target/x86_64-unknown-linux-musl/release/valiforge

# ---- Runtime stage ----
FROM debian:bookworm-slim AS runtime

# OCI annotations
LABEL org.opencontainers.image.title="ValiForge" \
      org.opencontainers.image.description="Headless API-first validation engine for AI workloads" \
      org.opencontainers.image.url="https://valiforge.dev" \
      org.opencontainers.image.source="https://github.com/valiforge/valiforge" \
      org.opencontainers.image.vendor="ValiForge" \
      org.opencontainers.image.licenses="Apache-2.0"

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system valiforge \
    && useradd --system --gid valiforge --create-home valiforge

COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/valiforge /usr/local/bin/valiforge

USER valiforge
WORKDIR /home/valiforge

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/valiforge", "--version"]

ENTRYPOINT ["/usr/local/bin/valiforge"]
CMD ["--help"]
```

### `docker-compose.yml` — Local Development

```yaml
# docker-compose.yml
version: "3.9"

services:
  valiforge:
    build:
      context: .
      dockerfile: docker/Dockerfile
    volumes:
      - ./schemas:/schemas:ro
      - ./reports:/reports
    environment:
      - VALIFORGE_LOG=debug
      - VALIFORGE_FORMAT=json
    depends_on:
      mock-api:
        condition: service_healthy
    command:
      - validate
      - --schema
      - /schemas/openapi.yaml
      - --target
      - http://mock-api:4010
      - --output
      - /reports/results.json

  mock-api:
    image: stoplight/prism:5
    command: mock /schemas/openapi.yaml --host 0.0.0.0
    ports:
      - "4010:4010"
    volumes:
      - ./schemas:/schemas:ro
    healthcheck:
      test: ["CMD", "wget", "--spider", "-q", "http://localhost:4010"]
      interval: 5s
      timeout: 3s
      retries: 10
```

---

## 3. Cross-Platform Install Script

### `installers/install.sh` — Unix Installer

```bash
#!/bin/sh
# install.sh — ValiForge installer for Linux and macOS
# Usage: curl -fsSL https://install.valiforge.dev | sh
#        curl -fsSL https://install.valiforge.dev | sh -s -- --version 0.2.0
#        curl -fsSL https://install.valiforge.dev | sh -s -- --prefix ~/.local

set -eu

# ---- Configuration ----
REPO="valiforge/valiforge"
BINARY="valiforge"
GITHUB_API="https://api.github.com"
DEFAULT_INSTALL_DIR="/usr/local/bin"

# ---- Color output ----
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

info()  { printf "${CYAN}info${NC}  %s\n" "$1"; }
ok()    { printf "${GREEN}ok${NC}    %s\n" "$1"; }
warn()  { printf "${YELLOW}warn${NC}  %s\n" "$1"; }
err()   { printf "${RED}error${NC} %s\n" "$1" >&2; exit 1; }

# ---- Argument parsing ----
VERSION=""
PREFIX=""

while [ $# -gt 0 ]; do
  case "$1" in
    --version)  VERSION="$2"; shift 2 ;;
    --prefix)   PREFIX="$2"; shift 2 ;;
    --help|-h)
      echo "Usage: install.sh [--version VERSION] [--prefix DIR]"
      echo ""
      echo "Options:"
      echo "  --version VERSION   Install a specific version (default: latest)"
      echo "  --prefix DIR        Install to DIR/bin (default: /usr/local)"
      exit 0
      ;;
    *) err "Unknown argument: $1" ;;
  esac
done

# ---- Determine install directory ----
if [ -n "$PREFIX" ]; then
  INSTALL_DIR="${PREFIX}/bin"
elif [ -n "${VALIFORGE_INSTALL_DIR:-}" ]; then
  INSTALL_DIR="$VALIFORGE_INSTALL_DIR"
else
  # Use /usr/local/bin if writable, otherwise fall back to ~/.valiforge/bin
  if [ -w "$DEFAULT_INSTALL_DIR" ] || [ "$(id -u)" = "0" ]; then
    INSTALL_DIR="$DEFAULT_INSTALL_DIR"
  else
    INSTALL_DIR="$HOME/.valiforge/bin"
    warn "No write access to ${DEFAULT_INSTALL_DIR}; installing to ${INSTALL_DIR}"
  fi
fi

# ---- Detect platform ----
detect_os() {
  OS="$(uname -s)"
  case "$OS" in
    Linux)  echo "unknown-linux-musl" ;;
    Darwin) echo "apple-darwin" ;;
    *)      err "Unsupported operating system: $OS. Use Windows? See https://valiforge.dev/docs/install#windows" ;;
  esac
}

detect_arch() {
  ARCH="$(uname -m)"
  case "$ARCH" in
    x86_64|amd64)   echo "x86_64" ;;
    aarch64|arm64)   echo "aarch64" ;;
    *)               err "Unsupported architecture: $ARCH" ;;
  esac
}

# ---- Check dependencies ----
need_cmd() {
  if ! command -v "$1" > /dev/null 2>&1; then
    err "Required command '$1' not found. Please install it and retry."
  fi
}

need_cmd uname
need_cmd mktemp
need_cmd tar

# Prefer curl, fall back to wget
DOWNLOADER=""
if command -v curl > /dev/null 2>&1; then
  DOWNLOADER="curl"
elif command -v wget > /dev/null 2>&1; then
  DOWNLOADER="wget"
else
  err "Neither 'curl' nor 'wget' found. Please install one and retry."
fi

download() {
  local url="$1" dest="$2"
  if [ "$DOWNLOADER" = "curl" ]; then
    curl -fsSL "$url" -o "$dest"
  else
    wget -qO "$dest" "$url"
  fi
}

# ---- Resolve version ----
OS_SUFFIX=$(detect_os)
ARCH_PREFIX=$(detect_arch)
TARGET="${ARCH_PREFIX}-${OS_SUFFIX}"

info "Detected platform: ${TARGET}"

if [ -z "$VERSION" ]; then
  info "Fetching latest release version..."
  if [ "$DOWNLOADER" = "curl" ]; then
    VERSION=$(curl -fsSL "${GITHUB_API}/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"v([^"]+)".*/\1/')
  else
    VERSION=$(wget -qO- "${GITHUB_API}/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"v([^"]+)".*/\1/')
  fi
  if [ -z "$VERSION" ]; then
    err "Could not determine latest version. Specify one with --version."
  fi
fi

info "Installing ValiForge v${VERSION} for ${TARGET}"

# ---- Download and verify ----
ARCHIVE="${BINARY}-v${VERSION}-${TARGET}.tar.gz"
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/v${VERSION}/${ARCHIVE}"
CHECKSUM_URL="${DOWNLOAD_URL}.sha256"

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

info "Downloading ${DOWNLOAD_URL}..."
download "$DOWNLOAD_URL" "${TMPDIR}/${ARCHIVE}"
download "$CHECKSUM_URL" "${TMPDIR}/${ARCHIVE}.sha256"

info "Verifying SHA256 checksum..."
cd "$TMPDIR"
if command -v sha256sum > /dev/null 2>&1; then
  sha256sum -c "${ARCHIVE}.sha256"
elif command -v shasum > /dev/null 2>&1; then
  shasum -a 256 -c "${ARCHIVE}.sha256"
else
  warn "No checksum utility found; skipping verification"
fi

# ---- Extract and install ----
info "Extracting..."
tar xzf "${ARCHIVE}"

mkdir -p "$INSTALL_DIR"
install -m 755 "${BINARY}-v${VERSION}-${TARGET}/${BINARY}" "${INSTALL_DIR}/${BINARY}"

# ---- Verify installation ----
if "${INSTALL_DIR}/${BINARY}" --version > /dev/null 2>&1; then
  INSTALLED_VERSION=$("${INSTALL_DIR}/${BINARY}" --version 2>&1 | head -1)
  ok "ValiForge installed successfully: ${INSTALLED_VERSION}"
else
  err "Installation verification failed"
fi

ok "Binary location: ${INSTALL_DIR}/${BINARY}"

# ---- PATH hint ----
case ":${PATH}:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    echo ""
    warn "${INSTALL_DIR} is not in your PATH."
    echo ""
    echo "  Add it by running:"
    echo ""
    SHELL_NAME="$(basename "${SHELL:-/bin/sh}")"
    case "$SHELL_NAME" in
      zsh)  echo "    echo 'export PATH=\"${INSTALL_DIR}:\$PATH\"' >> ~/.zshrc && source ~/.zshrc" ;;
      fish) echo "    fish_add_path ${INSTALL_DIR}" ;;
      *)    echo "    echo 'export PATH=\"${INSTALL_DIR}:\$PATH\"' >> ~/.bashrc && source ~/.bashrc" ;;
    esac
    echo ""
    ;;
esac

echo ""
info "Get started: ${BOLD}valiforge --help${NC}"
```

### `installers/install.ps1` — Windows PowerShell Installer

```powershell
# install.ps1 — ValiForge installer for Windows
# Usage: irm https://install.valiforge.dev/windows | iex
#        .\install.ps1 -Version "0.2.0"
#        .\install.ps1 -InstallDir "C:\tools\valiforge"

param(
    [string]$Version = "",
    [string]$InstallDir = ""
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$Repo = "valiforge/valiforge"
$Binary = "valiforge"
$Target = "x86_64-pc-windows-msvc"
$GithubApi = "https://api.github.com"

function Write-Info  { Write-Host "[info]  $args" -ForegroundColor Cyan }
function Write-Ok    { Write-Host "[ok]    $args" -ForegroundColor Green }
function Write-Warn  { Write-Host "[warn]  $args" -ForegroundColor Yellow }
function Write-Err   { Write-Host "[error] $args" -ForegroundColor Red; exit 1 }

# Determine install directory
if (-not $InstallDir) {
    $InstallDir = Join-Path $env:LOCALAPPDATA "ValiForge\bin"
}

# Resolve version
if (-not $Version) {
    Write-Info "Fetching latest release version..."
    $release = Invoke-RestMethod -Uri "$GithubApi/repos/$Repo/releases/latest"
    $Version = $release.tag_name -replace '^v', ''
    if (-not $Version) {
        Write-Err "Could not determine latest version."
    }
}

Write-Info "Installing ValiForge v$Version for $Target"

# Download
$Archive = "$Binary-v$Version-$Target.zip"
$DownloadUrl = "https://github.com/$Repo/releases/download/v$Version/$Archive"
$ChecksumUrl = "$DownloadUrl.sha256"

$TempDir = New-Item -ItemType Directory -Path (Join-Path $env:TEMP "valiforge-install-$(Get-Random)")

try {
    Write-Info "Downloading $DownloadUrl..."
    Invoke-WebRequest -Uri $DownloadUrl -OutFile (Join-Path $TempDir $Archive)
    Invoke-WebRequest -Uri $ChecksumUrl -OutFile (Join-Path $TempDir "$Archive.sha256")

    # Verify checksum
    Write-Info "Verifying SHA256 checksum..."
    $expectedHash = (Get-Content (Join-Path $TempDir "$Archive.sha256") -Raw).Split(" ")[0].Trim()
    $actualHash = (Get-FileHash (Join-Path $TempDir $Archive) -Algorithm SHA256).Hash.ToLower()
    if ($expectedHash -ne $actualHash) {
        Write-Err "Checksum mismatch! Expected: $expectedHash Got: $actualHash"
    }
    Write-Ok "Checksum verified"

    # Extract
    Write-Info "Extracting..."
    Expand-Archive -Path (Join-Path $TempDir $Archive) -DestinationPath $TempDir -Force

    # Install
    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }
    $BinSrc = Get-ChildItem -Path $TempDir -Filter "$Binary.exe" -Recurse | Select-Object -First 1
    Copy-Item -Path $BinSrc.FullName -Destination (Join-Path $InstallDir "$Binary.exe") -Force

    # Verify
    $installed = & (Join-Path $InstallDir "$Binary.exe") --version 2>&1
    Write-Ok "ValiForge installed: $installed"
    Write-Ok "Binary location: $(Join-Path $InstallDir "$Binary.exe")"

    # Add to PATH if needed
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($currentPath -notlike "*$InstallDir*") {
        Write-Warn "$InstallDir is not in your PATH."
        $addToPath = Read-Host "Add to PATH? [Y/n]"
        if ($addToPath -ne "n") {
            [Environment]::SetEnvironmentVariable("Path", "$currentPath;$InstallDir", "User")
            $env:Path = "$env:Path;$InstallDir"
            Write-Ok "Added $InstallDir to user PATH. Restart your terminal for changes to take effect."
        }
    }
}
finally {
    Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Info "Get started: valiforge --help"
```

---

## 4. Homebrew Formula

### `Formula/valiforge.rb`

```ruby
# typed: false
# frozen_string_literal: true

class Valiforge < Formula
  desc "Headless API-first validation engine for AI workloads"
  homepage "https://valiforge.dev"
  version "0.1.0"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/valiforge/valiforge/releases/download/v#{version}/valiforge-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_DARWIN_ARM64"

      def install
        bin.install "valiforge-v#{version}-aarch64-apple-darwin/valiforge"
        generate_completions_from_executable(bin/"valiforge", "completions")
      end
    end
    on_intel do
      url "https://github.com/valiforge/valiforge/releases/download/v#{version}/valiforge-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_DARWIN_X64"

      def install
        bin.install "valiforge-v#{version}-x86_64-apple-darwin/valiforge"
        generate_completions_from_executable(bin/"valiforge", "completions")
      end
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/valiforge/valiforge/releases/download/v#{version}/valiforge-v#{version}-aarch64-unknown-linux-musl.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_ARM64"

      def install
        bin.install "valiforge-v#{version}-aarch64-unknown-linux-musl/valiforge"
        generate_completions_from_executable(bin/"valiforge", "completions")
      end
    end
    on_intel do
      url "https://github.com/valiforge/valiforge/releases/download/v#{version}/valiforge-v#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_X64"

      def install
        bin.install "valiforge-v#{version}-x86_64-unknown-linux-musl/valiforge"
        generate_completions_from_executable(bin/"valiforge", "completions")
      end
    end
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/valiforge --version")
    # Validate that a minimal schema parse works
    (testpath/"minimal.yaml").write <<~YAML
      openapi: "3.0.0"
      info:
        title: Test
        version: "1.0"
      paths: {}
    YAML
    output = shell_output("#{bin}/valiforge validate --schema #{testpath}/minimal.yaml --dry-run 2>&1", 0)
    assert_match "schema parsed", output.downcase
  end
end
```

---

## 5. npm Wrapper Package

### `npm/valiforge/package.json`

```json
{
  "name": "@valiforge/cli",
  "version": "0.1.0",
  "description": "Headless API-first validation engine for AI workloads",
  "license": "Apache-2.0",
  "repository": {
    "type": "git",
    "url": "https://github.com/valiforge/valiforge"
  },
  "homepage": "https://valiforge.dev",
  "keywords": ["api", "testing", "validation", "openapi", "ci", "rust"],
  "bin": {
    "valiforge": "bin/valiforge.js"
  },
  "files": ["bin/", "postinstall.js", "README.md"],
  "scripts": {
    "postinstall": "node postinstall.js"
  },
  "optionalDependencies": {
    "@valiforge/cli-linux-x64": "0.1.0",
    "@valiforge/cli-linux-arm64": "0.1.0",
    "@valiforge/cli-darwin-x64": "0.1.0",
    "@valiforge/cli-darwin-arm64": "0.1.0",
    "@valiforge/cli-win32-x64": "0.1.0"
  },
  "engines": {
    "node": ">=16"
  }
}
```

### `npm/valiforge/postinstall.js`

```javascript
#!/usr/bin/env node
"use strict";

const { existsSync } = require("fs");
const { join } = require("path");

// Map Node.js os/arch to package names
const PLATFORM_MAP = {
  "linux-x64": "@valiforge/cli-linux-x64",
  "linux-arm64": "@valiforge/cli-linux-arm64",
  "darwin-x64": "@valiforge/cli-darwin-x64",
  "darwin-arm64": "@valiforge/cli-darwin-arm64",
  "win32-x64": "@valiforge/cli-win32-x64",
};

const platform = `${process.platform}-${process.arch}`;
const pkg = PLATFORM_MAP[platform];

if (!pkg) {
  console.error(
    `ValiForge does not have a prebuilt binary for ${platform}.\n` +
    `Try installing from source: cargo install valiforge`
  );
  process.exit(0); // Exit 0 so npm install doesn't fail
}

// Check if the optional platform package was installed
try {
  const binPath = require.resolve(`${pkg}/bin/valiforge${process.platform === "win32" ? ".exe" : ""}`);
  if (existsSync(binPath)) {
    // Binary available via optional dependency - nothing to do
    return;
  }
} catch (e) {
  // Optional dependency not installed; download manually
  console.log(`Platform package ${pkg} not found. Downloading binary directly...`);
  downloadBinary().catch((err) => {
    console.error(`Failed to download ValiForge binary: ${err.message}`);
    console.error(`Try installing from source: cargo install valiforge`);
    process.exit(0);
  });
}

async function downloadBinary() {
  const https = require("https");
  const { createWriteStream, chmodSync, mkdirSync } = require("fs");
  const { pipeline } = require("stream/promises");
  const path = require("path");

  const version = require("./package.json").version;
  const targetMap = {
    "linux-x64": "x86_64-unknown-linux-musl",
    "linux-arm64": "aarch64-unknown-linux-musl",
    "darwin-x64": "x86_64-apple-darwin",
    "darwin-arm64": "aarch64-apple-darwin",
    "win32-x64": "x86_64-pc-windows-msvc",
  };
  const target = targetMap[platform];
  const ext = process.platform === "win32" ? "zip" : "tar.gz";
  const url = `https://github.com/valiforge/valiforge/releases/download/v${version}/valiforge-v${version}-${target}.${ext}`;

  const binDir = path.join(__dirname, "bin");
  mkdirSync(binDir, { recursive: true });

  // For simplicity, use tar on Unix, PowerShell on Windows
  const { execSync } = require("child_process");
  const tmpFile = path.join(require("os").tmpdir(), `valiforge-download.${ext}`);

  execSync(`curl -fsSL "${url}" -o "${tmpFile}"`, { stdio: "inherit" });

  if (process.platform === "win32") {
    execSync(`powershell -Command "Expand-Archive -Path '${tmpFile}' -DestinationPath '${binDir}' -Force"`, { stdio: "inherit" });
  } else {
    execSync(`tar xzf "${tmpFile}" -C "${binDir}" --strip-components=1`, { stdio: "inherit" });
  }

  if (process.platform !== "win32") {
    chmodSync(path.join(binDir, "valiforge"), 0o755);
  }
}
```

### `npm/valiforge/bin/valiforge.js`

```javascript
#!/usr/bin/env node
"use strict";

const { execFileSync } = require("child_process");
const { join } = require("path");
const { existsSync } = require("fs");

const PLATFORM_MAP = {
  "linux-x64": "@valiforge/cli-linux-x64",
  "linux-arm64": "@valiforge/cli-linux-arm64",
  "darwin-x64": "@valiforge/cli-darwin-x64",
  "darwin-arm64": "@valiforge/cli-darwin-arm64",
  "win32-x64": "@valiforge/cli-win32-x64",
};

const platform = `${process.platform}-${process.arch}`;
const ext = process.platform === "win32" ? ".exe" : "";
const binaryName = `valiforge${ext}`;

// Strategy 1: Look in optional platform dependency
const pkg = PLATFORM_MAP[platform];
if (pkg) {
  try {
    const pkgBin = join(require.resolve(`${pkg}/package.json`), "..", "bin", binaryName);
    if (existsSync(pkgBin)) {
      runBinary(pkgBin);
    }
  } catch (e) {
    // Fall through
  }
}

// Strategy 2: Look in local bin directory (postinstall download)
const localBin = join(__dirname, binaryName);
if (existsSync(localBin)) {
  runBinary(localBin);
}

// Strategy 3: Look on PATH
try {
  execFileSync("valiforge", process.argv.slice(2), { stdio: "inherit" });
  process.exit(0);
} catch (e) {
  if (e.status !== null) process.exit(e.status);
}

console.error(
  "Error: Could not find ValiForge binary.\n" +
  "Try reinstalling: npm install -g @valiforge/cli\n" +
  "Or install from source: cargo install valiforge"
);
process.exit(1);

function runBinary(binPath) {
  try {
    execFileSync(binPath, process.argv.slice(2), { stdio: "inherit" });
    process.exit(0);
  } catch (e) {
    if (e.status !== null) process.exit(e.status);
    throw e;
  }
}
```

### Platform Package Example: `npm/valiforge-linux-x64/package.json`

```json
{
  "name": "@valiforge/cli-linux-x64",
  "version": "0.1.0",
  "description": "ValiForge binary for Linux x64",
  "license": "Apache-2.0",
  "repository": {
    "type": "git",
    "url": "https://github.com/valiforge/valiforge"
  },
  "os": ["linux"],
  "cpu": ["x64"],
  "files": ["bin/"],
  "preferUnplugged": true
}
```

Each platform package (`valiforge-linux-arm64`, `valiforge-darwin-x64`, `valiforge-darwin-arm64`, `valiforge-win32-x64`) follows the same structure, changing `os` and `cpu` fields accordingly. The release CI copies the correct native binary into each package's `bin/` directory before publish.

---

## 6. GitHub Action (`action.yml`)

### `action/action.yml` — Composite Action for Marketplace

```yaml
# action/action.yml
name: "ValiForge API Validation"
description: "Validate API contracts, detect breaking changes, and test data integrity with ValiForge"
author: "ValiForge"

branding:
  icon: "shield"
  color: "orange"

inputs:
  version:
    description: "ValiForge version to install (default: latest)"
    required: false
    default: "latest"
  schema:
    description: "Path to API schema file (OpenAPI YAML/JSON, Protobuf, or GraphQL SDL)"
    required: true
  target:
    description: "Base URL of the running API server to validate against"
    required: false
    default: ""
  config:
    description: "Path to valiforge.toml configuration file"
    required: false
    default: ""
  args:
    description: "Additional CLI arguments passed directly to valiforge"
    required: false
    default: ""
  fail-on-error:
    description: "Fail the workflow step if validation errors are found"
    required: false
    default: "true"
  format:
    description: "Output format: json, junit, markdown, sarif, terminal"
    required: false
    default: "junit"
  sarif:
    description: "Upload SARIF results to GitHub Code Scanning"
    required: false
    default: "false"
  comment-on-pr:
    description: "Post validation results as a PR comment"
    required: false
    default: "true"

outputs:
  result:
    description: "Validation result: pass or fail"
    value: ${{ steps.validate.outputs.result }}
  report-path:
    description: "Path to the generated report file"
    value: ${{ steps.validate.outputs.report_path }}
  violations-count:
    description: "Number of violations found"
    value: ${{ steps.validate.outputs.violations_count }}
  exit-code:
    description: "ValiForge process exit code"
    value: ${{ steps.validate.outputs.exit_code }}

runs:
  using: "composite"
  steps:
    - name: Detect platform
      id: platform
      shell: bash
      run: |
        OS=$(uname -s | tr '[:upper:]' '[:lower:]')
        ARCH=$(uname -m)
        case "$OS" in
          linux)  OS_SUFFIX="unknown-linux-musl" ;;
          darwin) OS_SUFFIX="apple-darwin" ;;
          *)      echo "::error::Unsupported OS: $OS"; exit 1 ;;
        esac
        case "$ARCH" in
          x86_64|amd64)  ARCH_PREFIX="x86_64" ;;
          aarch64|arm64) ARCH_PREFIX="aarch64" ;;
          *)             echo "::error::Unsupported arch: $ARCH"; exit 1 ;;
        esac
        echo "target=${ARCH_PREFIX}-${OS_SUFFIX}" >> $GITHUB_OUTPUT

    - name: Install ValiForge
      shell: bash
      run: |
        if [ "${{ inputs.version }}" = "latest" ]; then
          VERSION=$(curl -fsSL https://api.github.com/repos/valiforge/valiforge/releases/latest | grep '"tag_name"' | sed -E 's/.*"v([^"]+)".*/\1/')
        else
          VERSION="${{ inputs.version }}"
        fi
        TARGET="${{ steps.platform.outputs.target }}"
        ARCHIVE="valiforge-v${VERSION}-${TARGET}.tar.gz"
        URL="https://github.com/valiforge/valiforge/releases/download/v${VERSION}/${ARCHIVE}"

        echo "::group::Downloading ValiForge v${VERSION}"
        curl -fsSL "$URL" | tar xz
        sudo install "valiforge-v${VERSION}-${TARGET}/valiforge" /usr/local/bin/valiforge
        echo "::endgroup::"
        valiforge --version

    - name: Cache ValiForge data
      uses: actions/cache@v4
      with:
        path: ~/.valiforge
        key: valiforge-${{ runner.os }}-${{ hashFiles(inputs.schema) }}
        restore-keys: |
          valiforge-${{ runner.os }}-

    - name: Run validation
      id: validate
      shell: bash
      run: |
        # Build argument list
        ARGS="validate --schema ${{ inputs.schema }}"

        if [ -n "${{ inputs.target }}" ]; then
          ARGS="$ARGS --target ${{ inputs.target }}"
        fi
        if [ -n "${{ inputs.config }}" ]; then
          ARGS="$ARGS --config ${{ inputs.config }}"
        fi

        # Determine output file extension
        case "${{ inputs.format }}" in
          junit) EXT="xml" ;;
          sarif) EXT="sarif" ;;
          json)  EXT="json" ;;
          markdown) EXT="md" ;;
          *) EXT="txt" ;;
        esac

        REPORT_PATH="valiforge-report.${EXT}"
        ARGS="$ARGS --format ${{ inputs.format }} --output ${REPORT_PATH}"

        if [ "${{ inputs.sarif }}" = "true" ]; then
          ARGS="$ARGS --sarif valiforge-results.sarif"
        fi

        if [ -n "${{ inputs.args }}" ]; then
          ARGS="$ARGS ${{ inputs.args }}"
        fi

        echo "::group::ValiForge validation"
        echo "Running: valiforge $ARGS"

        set +e
        valiforge $ARGS 2>&1
        EXIT_CODE=$?
        set -e

        echo "::endgroup::"

        echo "exit_code=${EXIT_CODE}" >> $GITHUB_OUTPUT
        echo "report_path=${REPORT_PATH}" >> $GITHUB_OUTPUT

        if [ $EXIT_CODE -eq 0 ]; then
          echo "result=pass" >> $GITHUB_OUTPUT
          echo "violations_count=0" >> $GITHUB_OUTPUT
        else
          echo "result=fail" >> $GITHUB_OUTPUT
          # Attempt to extract violation count from JSON
          COUNT=$(valiforge $ARGS --format json 2>/dev/null | jq -r '.summary.violations // 0' 2>/dev/null || echo "unknown")
          echo "violations_count=${COUNT}" >> $GITHUB_OUTPUT
        fi

        if [ "${{ inputs.fail-on-error }}" = "true" ] && [ $EXIT_CODE -ne 0 ]; then
          exit $EXIT_CODE
        fi

    - name: Upload test results
      if: always() && inputs.format == 'junit'
      uses: dorny/test-reporter@v1
      with:
        name: ValiForge Validation
        path: valiforge-report.xml
        reporter: java-junit
        fail-on-error: false

    - name: Upload SARIF to Code Scanning
      if: always() && inputs.sarif == 'true'
      uses: github/codeql-action/upload-sarif@v3
      with:
        sarif_file: valiforge-results.sarif
        category: valiforge

    - name: Comment on PR
      if: always() && github.event_name == 'pull_request' && inputs.comment-on-pr == 'true'
      uses: actions/github-script@v7
      with:
        script: |
          const fs = require('fs');
          const result = '${{ steps.validate.outputs.result }}';
          const violations = '${{ steps.validate.outputs.violations_count }}';
          const icon = result === 'pass' ? ':white_check_mark:' : ':x:';
          const status = result === 'pass' ? 'PASSED' : 'FAILED';

          let body = `## ${icon} ValiForge API Validation — ${status}\n\n`;
          body += `| Metric | Value |\n|--------|-------|\n`;
          body += `| **Result** | ${status} |\n`;
          body += `| **Violations** | ${violations} |\n`;
          body += `| **Schema** | \`${{ inputs.schema }}\` |\n`;

          try {
            const reportPath = '${{ steps.validate.outputs.report_path }}';
            const report = fs.readFileSync(reportPath, 'utf8');
            const truncated = report.length > 50000 ? report.substring(0, 50000) + '\n\n...(truncated)' : report;
            body += `\n<details><summary>Full Report</summary>\n\n\`\`\`\n${truncated}\n\`\`\`\n</details>\n`;
          } catch (e) {
            body += '\n*Report file not available.*\n';
          }

          body += '\n---\n*Generated by [ValiForge](https://valiforge.dev)*';

          const { data: comments } = await github.rest.issues.listComments({
            owner: context.repo.owner,
            repo: context.repo.repo,
            issue_number: context.issue.number,
          });
          const existing = comments.find(c => c.body && c.body.includes('ValiForge API Validation'));

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

### Example Workflow Using the Action

```yaml
# .github/workflows/api-validation.yml (in a consumer repository)
name: API Validation

on:
  pull_request:
    paths:
      - "openapi.yaml"
      - "src/**"

jobs:
  validate:
    name: Validate API Contracts
    runs-on: ubuntu-latest
    services:
      api:
        image: your-api-image:test
        ports: ["3000:3000"]
    steps:
      - uses: actions/checkout@v4

      - uses: valiforge/action@v1
        with:
          schema: ./openapi.yaml
          target: http://localhost:3000
          fail-on-error: true
          format: junit
          sarif: true
          comment-on-pr: true
```

---

## 7. Terraform / Infrastructure (Cloud Tier)

### `terraform/terraform.tf` — Provider and Backend

```hcl
# terraform/terraform.tf
terraform {
  required_version = ">= 1.6"

  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 5.0"
    }
    google-beta = {
      source  = "hashicorp/google-beta"
      version = "~> 5.0"
    }
    random = {
      source  = "hashicorp/random"
      version = "~> 3.6"
    }
  }

  backend "gcs" {
    bucket = "valiforge-terraform-state"
    prefix = "prod"
  }
}

provider "google" {
  project = var.project_id
  region  = var.region
}

provider "google-beta" {
  project = var.project_id
  region  = var.region
}
```

### `terraform/variables.tf`

```hcl
# terraform/variables.tf
variable "project_id" {
  type        = string
  description = "GCP project ID"
}

variable "region" {
  type        = string
  default     = "us-central1"
  description = "GCP region"
}

variable "environment" {
  type        = string
  default     = "prod"
  description = "Environment name (prod, staging)"
  validation {
    condition     = contains(["prod", "staging"], var.environment)
    error_message = "Environment must be 'prod' or 'staging'."
  }
}

variable "cloud_run_image" {
  type        = string
  description = "Docker image for Cloud Run (e.g., gcr.io/valiforge/api:v1.0.0)"
}

variable "db_tier" {
  type        = string
  default     = "db-f1-micro"
  description = "Cloud SQL instance tier"
}

variable "db_password" {
  type        = string
  sensitive   = true
  description = "PostgreSQL root password"
}

variable "domain" {
  type        = string
  default     = "api.valiforge.dev"
  description = "Custom domain for the API"
}

variable "min_instances" {
  type    = number
  default = 1
}

variable "max_instances" {
  type    = number
  default = 10
}
```

### `terraform/main.tf`

```hcl
# terraform/main.tf

# ---- Networking ----
resource "google_compute_network" "vpc" {
  name                    = "valiforge-vpc-${var.environment}"
  auto_create_subnetworks = false
}

resource "google_compute_subnetwork" "subnet" {
  name          = "valiforge-subnet-${var.environment}"
  ip_cidr_range = "10.0.0.0/24"
  region        = var.region
  network       = google_compute_network.vpc.id

  private_ip_google_access = true
}

resource "google_compute_global_address" "private_ip" {
  name          = "valiforge-db-ip-${var.environment}"
  purpose       = "VPC_PEERING"
  address_type  = "INTERNAL"
  prefix_length = 16
  network       = google_compute_network.vpc.id
}

resource "google_service_networking_connection" "private_vpc" {
  network                 = google_compute_network.vpc.id
  service                 = "servicenetworking.googleapis.com"
  reserved_peering_ranges = [google_compute_global_address.private_ip.name]
}

# ---- VPC Connector (for Cloud Run -> Cloud SQL) ----
resource "google_vpc_access_connector" "connector" {
  name          = "valiforge-connector-${var.environment}"
  region        = var.region
  ip_cidr_range = "10.8.0.0/28"
  network       = google_compute_network.vpc.name

  min_instances = 2
  max_instances = 3
}

# ---- Cloud SQL (PostgreSQL) ----
resource "random_id" "db_suffix" {
  byte_length = 4
}

resource "google_sql_database_instance" "postgres" {
  name             = "valiforge-db-${var.environment}-${random_id.db_suffix.hex}"
  database_version = "POSTGRES_16"
  region           = var.region

  depends_on = [google_service_networking_connection.private_vpc]

  settings {
    tier              = var.db_tier
    availability_type = var.environment == "prod" ? "REGIONAL" : "ZONAL"
    disk_autoresize   = true
    disk_size         = 20

    ip_configuration {
      ipv4_enabled    = false
      private_network = google_compute_network.vpc.id
    }

    backup_configuration {
      enabled                        = true
      start_time                     = "03:00"
      point_in_time_recovery_enabled = var.environment == "prod"
      transaction_log_retention_days = 7
      backup_retention_settings {
        retained_backups = 14
      }
    }

    database_flags {
      name  = "log_min_duration_statement"
      value = "1000"
    }

    maintenance_window {
      day          = 7
      hour         = 4
      update_track = "stable"
    }
  }

  deletion_protection = var.environment == "prod"
}

resource "google_sql_database" "valiforge" {
  name     = "valiforge"
  instance = google_sql_database_instance.postgres.name
}

resource "google_sql_user" "valiforge" {
  name     = "valiforge"
  instance = google_sql_database_instance.postgres.name
  password = var.db_password
}

# ---- Cloud Storage (Reports) ----
resource "google_storage_bucket" "reports" {
  name     = "valiforge-reports-${var.environment}-${var.project_id}"
  location = var.region

  uniform_bucket_level_access = true
  force_destroy               = var.environment != "prod"

  versioning {
    enabled = true
  }

  lifecycle_rule {
    condition {
      age = 90
    }
    action {
      type          = "SetStorageClass"
      storage_class = "NEARLINE"
    }
  }

  lifecycle_rule {
    condition {
      age = 365
    }
    action {
      type = "Delete"
    }
  }
}

# ---- IAM: Service Accounts ----
resource "google_service_account" "cloud_run" {
  account_id   = "valiforge-api-${var.environment}"
  display_name = "ValiForge Cloud Run Service Account (${var.environment})"
}

resource "google_project_iam_member" "cloud_sql_client" {
  project = var.project_id
  role    = "roles/cloudsql.client"
  member  = "serviceAccount:${google_service_account.cloud_run.email}"
}

resource "google_storage_bucket_iam_member" "reports_writer" {
  bucket = google_storage_bucket.reports.name
  role   = "roles/storage.objectAdmin"
  member = "serviceAccount:${google_service_account.cloud_run.email}"
}

# ---- Cloud Run Service ----
resource "google_cloud_run_v2_service" "api" {
  name     = "valiforge-api-${var.environment}"
  location = var.region

  template {
    service_account = google_service_account.cloud_run.email

    scaling {
      min_instance_count = var.min_instances
      max_instance_count = var.max_instances
    }

    vpc_access {
      connector = google_vpc_access_connector.connector.id
      egress    = "PRIVATE_RANGES_ONLY"
    }

    containers {
      image = var.cloud_run_image

      ports {
        container_port = 8080
      }

      resources {
        limits = {
          cpu    = "2"
          memory = "1Gi"
        }
        cpu_idle          = true
        startup_cpu_boost = true
      }

      env {
        name  = "ENVIRONMENT"
        value = var.environment
      }
      env {
        name  = "DATABASE_URL"
        value = "postgres://valiforge:${var.db_password}@${google_sql_database_instance.postgres.private_ip_address}:5432/valiforge"
      }
      env {
        name  = "REPORTS_BUCKET"
        value = google_storage_bucket.reports.name
      }
      env {
        name  = "RUST_LOG"
        value = var.environment == "prod" ? "info" : "debug"
      }

      startup_probe {
        http_get {
          path = "/health"
        }
        initial_delay_seconds = 5
        period_seconds        = 5
        failure_threshold     = 3
      }

      liveness_probe {
        http_get {
          path = "/health"
        }
        period_seconds = 30
      }
    }
  }

  traffic {
    type    = "TRAFFIC_TARGET_ALLOCATION_TYPE_LATEST"
    percent = 100
  }
}

# ---- Public access (unauthenticated) ----
resource "google_cloud_run_v2_service_iam_member" "public" {
  name     = google_cloud_run_v2_service.api.name
  location = var.region
  role     = "roles/run.invoker"
  member   = "allUsers"
}
```

### `terraform/outputs.tf`

```hcl
# terraform/outputs.tf
output "api_url" {
  description = "Cloud Run service URL"
  value       = google_cloud_run_v2_service.api.uri
}

output "database_instance" {
  description = "Cloud SQL instance name"
  value       = google_sql_database_instance.postgres.name
}

output "database_private_ip" {
  description = "Cloud SQL private IP"
  value       = google_sql_database_instance.postgres.private_ip_address
  sensitive   = true
}

output "reports_bucket" {
  description = "Cloud Storage bucket for reports"
  value       = google_storage_bucket.reports.name
}

output "service_account_email" {
  description = "Cloud Run service account"
  value       = google_service_account.cloud_run.email
}
```

---

## 8. Monitoring and Observability

### Prometheus Metrics Endpoint (Rust Implementation)

The API server exposes a `/metrics` endpoint in Prometheus exposition format. Add the following to `Cargo.toml` for the cloud API crate:

```toml
[dependencies]
metrics = "0.23"
metrics-exporter-prometheus = "0.15"
axum = "0.7"
```

Register and serve the metrics endpoint:

```rust
// src/metrics.rs
use axum::{routing::get, Router};
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use std::time::Instant;

pub fn setup_metrics() -> PrometheusHandle {
    let builder = PrometheusBuilder::new()
        .set_buckets_for_metric(
            Matcher::Full("http_request_duration_seconds".to_string()),
            &[0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0],
        )
        .expect("bucket config valid");

    builder.install_recorder().expect("metrics recorder installed")
}

pub fn metrics_route(handle: PrometheusHandle) -> Router {
    Router::new().route("/metrics", get(move || {
        let handle = handle.clone();
        async move { handle.render() }
    }))
}

/// Call from request middleware to record HTTP metrics.
pub fn record_request(method: &str, path: &str, status: u16, duration: std::time::Duration) {
    let labels = [
        ("method", method.to_string()),
        ("path", path.to_string()),
        ("status", status.to_string()),
    ];
    counter!("http_requests_total", &labels).increment(1);
    histogram!("http_request_duration_seconds", &labels).record(duration.as_secs_f64());
}

pub fn set_active_connections(count: usize) {
    gauge!("http_active_connections").set(count as f64);
}

pub fn record_validation(schema_format: &str, result: &str, duration: std::time::Duration) {
    let labels = [
        ("format", schema_format.to_string()),
        ("result", result.to_string()),
    ];
    counter!("validations_total", &labels).increment(1);
    histogram!("validation_duration_seconds", &labels).record(duration.as_secs_f64());
}
```

### Grafana Dashboard JSON

```json
{
  "dashboard": {
    "id": null,
    "uid": "valiforge-overview",
    "title": "ValiForge API Overview",
    "tags": ["valiforge", "api"],
    "timezone": "browser",
    "refresh": "30s",
    "time": { "from": "now-6h", "to": "now" },
    "panels": [
      {
        "title": "Request Rate (req/s)",
        "type": "timeseries",
        "gridPos": { "h": 8, "w": 12, "x": 0, "y": 0 },
        "targets": [
          {
            "expr": "sum(rate(http_requests_total[5m]))",
            "legendFormat": "Total"
          },
          {
            "expr": "sum(rate(http_requests_total{status=~\"5..\"}[5m]))",
            "legendFormat": "5xx Errors"
          }
        ]
      },
      {
        "title": "Latency P99 (seconds)",
        "type": "timeseries",
        "gridPos": { "h": 8, "w": 12, "x": 12, "y": 0 },
        "targets": [
          {
            "expr": "histogram_quantile(0.99, sum(rate(http_request_duration_seconds_bucket[5m])) by (le))",
            "legendFormat": "P99"
          },
          {
            "expr": "histogram_quantile(0.95, sum(rate(http_request_duration_seconds_bucket[5m])) by (le))",
            "legendFormat": "P95"
          },
          {
            "expr": "histogram_quantile(0.50, sum(rate(http_request_duration_seconds_bucket[5m])) by (le))",
            "legendFormat": "P50"
          }
        ]
      },
      {
        "title": "Validations per Minute",
        "type": "stat",
        "gridPos": { "h": 4, "w": 6, "x": 0, "y": 8 },
        "targets": [
          {
            "expr": "sum(rate(validations_total[5m])) * 60",
            "legendFormat": "Validations/min"
          }
        ]
      },
      {
        "title": "Validation Pass Rate",
        "type": "gauge",
        "gridPos": { "h": 4, "w": 6, "x": 6, "y": 8 },
        "targets": [
          {
            "expr": "sum(rate(validations_total{result=\"pass\"}[1h])) / sum(rate(validations_total[1h])) * 100"
          }
        ],
        "fieldConfig": {
          "defaults": {
            "thresholds": {
              "steps": [
                { "color": "red", "value": 0 },
                { "color": "yellow", "value": 80 },
                { "color": "green", "value": 95 }
              ]
            },
            "unit": "percent",
            "min": 0,
            "max": 100
          }
        }
      },
      {
        "title": "Active Connections",
        "type": "stat",
        "gridPos": { "h": 4, "w": 6, "x": 12, "y": 8 },
        "targets": [
          {
            "expr": "http_active_connections",
            "legendFormat": "Connections"
          }
        ]
      },
      {
        "title": "Error Rate by Path",
        "type": "table",
        "gridPos": { "h": 8, "w": 12, "x": 0, "y": 12 },
        "targets": [
          {
            "expr": "sum(rate(http_requests_total{status=~\"5..\"}[5m])) by (path) / sum(rate(http_requests_total[5m])) by (path) * 100",
            "legendFormat": "{{ path }}"
          }
        ]
      },
      {
        "title": "Validation Duration by Schema Format",
        "type": "timeseries",
        "gridPos": { "h": 8, "w": 12, "x": 12, "y": 12 },
        "targets": [
          {
            "expr": "histogram_quantile(0.99, sum(rate(validation_duration_seconds_bucket[5m])) by (le, format))",
            "legendFormat": "{{ format }} P99"
          }
        ]
      }
    ]
  }
}
```

### Alert Rules (Prometheus/Grafana)

```yaml
# monitoring/alert-rules.yml
groups:
  - name: valiforge-api
    interval: 30s
    rules:
      - alert: HighLatencyP99
        expr: histogram_quantile(0.99, sum(rate(http_request_duration_seconds_bucket[5m])) by (le)) > 2.0
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "API P99 latency above 2s"
          description: "P99 latency is {{ $value | humanizeDuration }} (threshold: 2s)"

      - alert: HighErrorRate
        expr: sum(rate(http_requests_total{status=~"5.."}[5m])) / sum(rate(http_requests_total[5m])) > 0.05
        for: 3m
        labels:
          severity: critical
        annotations:
          summary: "Error rate above 5%"
          description: "5xx error rate is {{ $value | humanizePercentage }}"

      - alert: HighValidationFailureRate
        expr: sum(rate(validations_total{result="fail"}[15m])) / sum(rate(validations_total[15m])) > 0.50
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Validation failure rate above 50%"
          description: "This may indicate a systemic issue with schema parsing or API connectivity."

      - alert: ServiceDown
        expr: up{job="valiforge-api"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "ValiForge API is down"
          description: "The ValiForge API service has been unreachable for over 1 minute."

      - alert: HighMemoryUsage
        expr: process_resident_memory_bytes{job="valiforge-api"} > 800 * 1024 * 1024
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Memory usage above 800MB"
          description: "Current memory: {{ $value | humanize1024 }}B"

      - alert: DatabaseConnectionPoolExhausted
        expr: db_pool_active_connections / db_pool_max_connections > 0.9
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "Database connection pool >90% utilized"
```

---

## 9. Security

### `deny.toml` — cargo-deny Configuration

```toml
# deny.toml — Supply chain security, license compliance, and dependency hygiene

[graph]
targets = [
    "x86_64-unknown-linux-musl",
    "aarch64-unknown-linux-musl",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
]

[advisories]
vulnerability = "deny"
unmaintained = "warn"
yanked = "deny"
notice = "warn"
ignore = []

[licenses]
unlicensed = "deny"
allow = [
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Unicode-DFS-2016",
    "Unicode-3.0",
    "Zlib",
    "CC0-1.0",
    "OpenSSL",
    "BSL-1.0",
    "MPL-2.0",
]
copyleft = "deny"
default = "deny"
confidence-threshold = 0.8

[[licenses.clarify]]
name = "ring"
expression = "MIT AND ISC AND OpenSSL"
license-files = [{ path = "LICENSE", hash = 0xbd0eed23 }]

[bans]
multiple-versions = "warn"
wildcards = "deny"
highlight = "all"
allow-wildcard-paths = false
skip = []
skip-tree = []

[bans.deny]
# Block known-problematic crates
# name = "openssl-sys"  # Uncomment to enforce rustls-only

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []
```

### SBOM Generation & Container Scanning

Add these steps to the release pipeline:

```yaml
# Add to .github/workflows/release.yml under the release job

  sbom:
    name: Generate SBOM
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install cargo-cyclonedx
        run: cargo install cargo-cyclonedx --locked
      - name: Generate SBOM (CycloneDX)
        run: cargo cyclonedx --format json --output-file valiforge-sbom.cdx.json
      - name: Upload SBOM
        uses: actions/upload-artifact@v4
        with:
          name: sbom
          path: valiforge-sbom.cdx.json
      - name: Attach SBOM to release
        if: startsWith(github.ref, 'refs/tags/')
        uses: softprops/action-gh-release@v2
        with:
          files: valiforge-sbom.cdx.json

  container-scan:
    name: Container Security Scan
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build Docker image for scanning
        run: docker build -t valiforge:scan -f docker/Dockerfile .
      - name: Run Trivy vulnerability scanner
        uses: aquasecurity/trivy-action@master
        with:
          image-ref: "valiforge:scan"
          format: "sarif"
          output: "trivy-results.sarif"
          severity: "CRITICAL,HIGH"
          exit-code: "1"
      - name: Upload Trivy SARIF
        if: always()
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: trivy-results.sarif
          category: trivy
```

### SLSA Provenance

```yaml
# Add to .github/workflows/release.yml

  provenance:
    name: SLSA Provenance
    needs: build
    permissions:
      actions: read
      id-token: write
      contents: write
    uses: slsa-framework/slsa-github-generator/.github/workflows/generator_generic_slsa3.yml@v2.0.0
    with:
      base64-subjects: "${{ needs.build.outputs.hashes }}"
      upload-assets: true
```

---

## 10. Makefile / justfile

### `justfile` — Development Commands

```just
# justfile — ValiForge development commands
# Install just: cargo install just

set dotenv-load
set shell := ["bash", "-euo", "pipefail", "-c"]

# Default recipe: show help
default:
    @just --list

# ---- Build ----

# Build all workspace crates in debug mode
build:
    cargo build --workspace

# Build the CLI in release mode
build-release:
    cargo build --release --locked -p valiforge-cli

# Build for a specific target (cross-compilation)
build-target target:
    cargo build --release --locked --target {{target}} -p valiforge-cli

# ---- Test ----

# Run all tests
test:
    cargo test --workspace --all-features

# Run tests with output visible
test-verbose:
    cargo test --workspace --all-features -- --nocapture

# Run tests for a specific crate
test-crate crate:
    cargo test -p {{crate}} --all-features

# Run doc tests only
test-doc:
    cargo test --workspace --doc

# ---- Lint & Format ----

# Run all linters (fmt check + clippy)
lint: fmt-check clippy

# Check formatting
fmt-check:
    cargo fmt --all --check

# Format all code
fmt:
    cargo fmt --all

# Run clippy with strict warnings
clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# ---- Security ----

# Run all security checks
audit: deny
    cargo audit

# Run cargo-deny checks (licenses, advisories, bans, sources)
deny:
    cargo deny check all

# Generate SBOM
sbom:
    cargo cyclonedx --format json --output-file valiforge-sbom.cdx.json

# ---- Coverage ----

# Generate code coverage report
coverage:
    cargo llvm-cov --workspace --all-features --html
    @echo "Report: target/llvm-cov/html/index.html"

# Generate coverage as LCOV
coverage-lcov:
    cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info

# ---- Bench ----

# Run all benchmarks
bench:
    cargo bench --workspace

# Run benchmarks and save baseline
bench-save:
    cargo bench --workspace -- --save-baseline main

# Compare benchmarks against saved baseline
bench-compare:
    cargo bench --workspace -- --baseline main

# ---- Documentation ----

# Build and open documentation
doc:
    cargo doc --workspace --no-deps --all-features --open

# Check documentation builds without warnings
doc-check:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features

# ---- Docker ----

# Build Docker image
docker-build tag="latest":
    docker build -t valiforge:{{tag}} -f docker/Dockerfile .

# Run ValiForge in Docker
docker-run *args:
    docker run --rm valiforge:latest {{args}}

# Build and start local dev stack (ValiForge + mock API)
docker-dev:
    docker compose up --build

# Stop local dev stack
docker-dev-down:
    docker compose down

# ---- Release ----

# Dry-run a release (checks everything without publishing)
release-dry-run:
    cargo publish --dry-run -p valiforge-core
    cargo publish --dry-run -p valiforge-schema
    cargo publish --dry-run -p valiforge-diff
    cargo publish --dry-run -p valiforge-datagen
    cargo publish --dry-run -p valiforge-report
    cargo publish --dry-run -p valiforge-cli

# Check binary size for the release build
size:
    cargo build --release -p valiforge-cli
    @echo "Binary size:"
    @ls -lh target/release/valiforge | awk '{print $5, $NF}'

# Analyze what is taking up space in the binary
bloat:
    cargo bloat --release -p valiforge-cli --crates

# ---- MSRV ----

# Check against minimum supported Rust version
msrv:
    cargo +1.75.0 check --workspace --all-features

# ---- Development Helpers ----

# Watch for changes and run tests
watch:
    cargo watch -x 'test --workspace'

# Watch for changes and run clippy
watch-clippy:
    cargo watch -x 'clippy --workspace --all-targets --all-features -- -D warnings'

# Clean all build artifacts
clean:
    cargo clean
    rm -rf dist/ *.tar.gz *.zip

# Show dependency tree
deps:
    cargo tree --workspace --depth 1

# Show duplicate dependencies
deps-dupes:
    cargo tree --workspace --duplicates

# Run the full CI check locally before pushing
ci: fmt-check clippy test doc-check deny msrv
    @echo ""
    @echo "All CI checks passed locally."
```

### Equivalent `Makefile` (for environments without `just`)

```makefile
# Makefile — ValiForge development commands
.PHONY: build build-release test lint fmt clippy audit deny coverage bench doc \
        docker-build docker-dev clean ci msrv size bloat

# ---- Build ----
build:
	cargo build --workspace

build-release:
	cargo build --release --locked -p valiforge-cli

# ---- Test ----
test:
	cargo test --workspace --all-features

test-verbose:
	cargo test --workspace --all-features -- --nocapture

# ---- Lint & Format ----
lint: fmt-check clippy

fmt-check:
	cargo fmt --all --check

fmt:
	cargo fmt --all

clippy:
	cargo clippy --workspace --all-targets --all-features -- -D warnings

# ---- Security ----
audit: deny
	cargo audit

deny:
	cargo deny check all

# ---- Coverage ----
coverage:
	cargo llvm-cov --workspace --all-features --html
	@echo "Report: target/llvm-cov/html/index.html"

# ---- Bench ----
bench:
	cargo bench --workspace

# ---- Documentation ----
doc:
	cargo doc --workspace --no-deps --all-features --open

doc-check:
	RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features

# ---- Docker ----
docker-build:
	docker build -t valiforge:latest -f docker/Dockerfile .

docker-dev:
	docker compose up --build

docker-dev-down:
	docker compose down

# ---- Release ----
release-dry-run:
	@for crate in valiforge-core valiforge-schema valiforge-diff valiforge-datagen valiforge-report valiforge-cli; do \
		echo "Dry-run publishing $$crate..."; \
		cargo publish --dry-run -p $$crate; \
	done

size:
	cargo build --release -p valiforge-cli
	@echo "Binary size:"
	@ls -lh target/release/valiforge | awk '{print $$5, $$NF}'

bloat:
	cargo bloat --release -p valiforge-cli --crates

# ---- MSRV ----
msrv:
	cargo +1.75.0 check --workspace --all-features

# ---- Development ----
watch:
	cargo watch -x 'test --workspace'

clean:
	cargo clean
	rm -rf dist/ *.tar.gz *.zip

deps:
	cargo tree --workspace --depth 1

deps-dupes:
	cargo tree --workspace --duplicates

# ---- Full CI locally ----
ci: fmt-check clippy test doc-check deny msrv
	@echo ""
	@echo "All CI checks passed locally."
```

---

## Appendix A: File Placement Summary

```
valiforge/
├── .github/
│   └── workflows/
│       ├── ci.yml              # Section 1 — CI pipeline
│       ├── release.yml         # Section 1 — Release pipeline
│       └── bench.yml           # Section 1 — Benchmark pipeline
├── action/
│   └── action.yml              # Section 6 — GitHub Action
├── docker/
│   └── Dockerfile              # Section 2 — Multi-stage Docker build
├── docker-compose.yml          # Section 2 — Local dev compose
├── installers/
│   ├── install.sh              # Section 3 — Unix installer
│   └── install.ps1             # Section 3 — Windows installer
├── Formula/
│   └── valiforge.rb            # Section 4 — Homebrew formula (in homebrew-tap repo)
├── npm/
│   ├── valiforge/
│   │   ├── package.json        # Section 5 — npm main package
│   │   ├── postinstall.js      # Section 5 — Fallback binary download
│   │   └── bin/
│   │       └── valiforge.js    # Section 5 — Bin stub
│   ├── valiforge-linux-x64/
│   │   └── package.json        # Section 5 — Platform package
│   └── ...                     # (other platform packages)
├── terraform/
│   ├── terraform.tf            # Section 7 — Provider config
│   ├── variables.tf            # Section 7 — Input variables
│   ├── main.tf                 # Section 7 — GCP resources
│   └── outputs.tf              # Section 7 — Outputs
├── monitoring/
│   ├── grafana-dashboard.json  # Section 8 — Grafana dashboard
│   └── alert-rules.yml         # Section 8 — Prometheus alert rules
├── deny.toml                   # Section 9 — cargo-deny config
├── justfile                    # Section 10 — Development commands
├── Makefile                    # Section 10 — Make alternative
└── ...
```

## Appendix B: Required GitHub Secrets

| Secret | Used By | Description |
|--------|---------|-------------|
| `CARGO_REGISTRY_TOKEN` | `release.yml` | crates.io API token for publishing |
| `NPM_TOKEN` | `release.yml` | npm automation token for @valiforge scope |
| `HOMEBREW_TAP_TOKEN` | `release.yml` | GitHub PAT with push access to homebrew-tap repo |
| `CODECOV_TOKEN` | `ci.yml` | Codecov upload token |
| `APPLE_CERTIFICATE` | `release.yml` (future) | Base64-encoded .p12 Developer ID cert |
| `APPLE_CERTIFICATE_PASSWORD` | `release.yml` (future) | Password for the .p12 certificate |
| `APPLE_ID` | `release.yml` (future) | Apple ID email for notarization |
| `APPLE_TEAM_ID` | `release.yml` (future) | Apple Developer Team ID |
| `APPLE_APP_PASSWORD` | `release.yml` (future) | App-specific password for notarization |
| `DOCKERHUB_USERNAME` | `release.yml` (future) | Docker Hub username |
| `DOCKERHUB_TOKEN` | `release.yml` (future) | Docker Hub access token |

## Appendix C: Implementation Order

Execute in this sequence to minimize blocked work:

1. **justfile / Makefile** (Section 10) -- Immediate developer productivity
2. **deny.toml** (Section 9) -- Supply chain security from day zero
3. **ci.yml** (Section 1) -- Quality gate before any code merges
4. **Dockerfile** (Section 2) -- Local dev environment with mock API
5. **release.yml** (Section 1) -- Automates the first release
6. **install.sh / install.ps1** (Section 3) -- Primary user-facing install path
7. **Homebrew formula** (Section 4) -- macOS/Linux secondary install
8. **npm wrapper** (Section 5) -- JavaScript ecosystem reach
9. **action.yml** (Section 6) -- CI/CD adoption vehicle
10. **bench.yml** (Section 1) -- Performance regression guard
11. **Terraform** (Section 7) -- Cloud tier (Phase 2)
12. **Monitoring** (Section 8) -- Required before cloud launch
13. **SBOM / SLSA / Trivy** (Section 9) -- Enterprise security requirements
