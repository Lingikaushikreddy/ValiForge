# ValiForge

A headless API validation engine written in Rust. ValiForge reads an OpenAPI 3.x
spec, calls the live API it describes, and reports where the responses drift
from the contract. It can also compare two spec versions and flag breaking changes.

[![CI](https://github.com/Lingikaushikreddy/ValiForge/actions/workflows/ci.yml/badge.svg)](https://github.com/Lingikaushikreddy/ValiForge/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

> **Status:** early prototype (0.1.0). The `init`, `validate` and `diff` commands
> work today; see [What works today](#what-works-today) for the exact scope.

## Quick Start

```bash
git clone https://github.com/Lingikaushikreddy/ValiForge.git
cd ValiForge
cargo build --release

# Detect an OpenAPI spec in the current directory and write valiforge.toml
./target/release/valiforge init

# Call every endpoint in the spec and check status codes and response bodies
./target/release/valiforge validate --schema openapi.yaml --target http://localhost:3000

# Compare two versions of a spec for breaking changes
./target/release/valiforge diff old.yaml new.yaml
```

Example output from `validate`:

```
info: Validating 2 endpoints against http://127.0.0.1:8091

Results:
  PASS GET /users (4ms)
  FAIL DELETE /users/{id} (2ms)
       -> Expected status 204, got 501

Summary: 2 total, 1 passed, 1 failed, 0 errors
```

Exit codes: `0` on success, `1` when validation fails or breaking changes are
found, `2` on usage or runtime errors -- so it can gate a CI job directly.

## What works today

| Command | Status |
|---------|--------|
| `init` | Finds `openapi.yaml` / `swagger.json`-style files and writes a starter `valiforge.toml` |
| `validate` | Sends a request per operation (with retries and a concurrency limit), checks the expected status code and validates JSON bodies against the response schema |
| `diff` | Detects **removed endpoints** only; field-level and parameter-level changes are not compared yet |
| `generate` | Placeholder -- prints a notice, generates nothing yet |

Known gaps:

- Path parameters are not filled in, so `/users/{id}` is requested literally.
- `--format json|junit|markdown` is parsed, and the formatters exist in
  `valiforge-report`, but the CLI always prints terminal output.
- Only `schema.path` and `target.base_url` are read from `valiforge.toml`.

## Workspace layout

```
crates/
  valiforge-cli/      -- `valiforge` binary (clap)
  valiforge-core/     -- validation engine, shared types and errors
  valiforge-schema/   -- OpenAPI parsing and spec auto-detection
  valiforge-report/   -- JSON, JUnit and Markdown report formatters
  valiforge-diff/     -- breaking-change detection (stub)
  valiforge-datagen/  -- test data generation (stub)
docs/
  PRD.md              -- product requirements
  plans/              -- engineering, infrastructure and testing plans
  guides/             -- per-area implementation guides
```

The documents in `docs/` describe the full roadmap; most of it is not built yet.

## Development

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
