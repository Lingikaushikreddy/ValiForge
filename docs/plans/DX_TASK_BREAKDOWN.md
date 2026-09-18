# ValiForge — Developer Experience & Go-to-Market Task Breakdown

**Role:** Developer Experience (DX) & Developer Relations Engineer
**Date:** 2026-03-13
**Status:** Planning Phase
**License:** Apache 2.0 (Open Source)
**Target Audience:** Developers using AI coding tools (Copilot, Cursor, Claude Code)
**GTM Motion:** Bottom-up adoption — CLI → Team → ValiForge Cloud

---

## Executive Summary

This document contains **127 discrete tasks** across 7 workstreams, totaling an estimated **412 engineering-days** (~82 weeks of solo work, or ~21 weeks with a team of 4). The critical path runs through DX Design → Documentation → Launch, with community and content running in parallel.

**Priority tiers:**
- **P0 (Must-have for launch):** 47 tasks, 168 days
- **P1 (Should-have within 30 days of launch):** 38 tasks, 112 days
- **P2 (Nice-to-have within 90 days):** 42 tasks, 132 days

---

## 1. DEVELOPER EXPERIENCE (DX) DESIGN

### 1.1 First-Run Experience

---

#### DX-001: One-Line Install Script (`curl | sh`)

**Description:**
Build a cross-platform install script hosted at `https://install.valiforge.dev` that detects OS (macOS, Linux) and architecture (x86_64, aarch64), downloads the correct pre-built binary from GitHub Releases, verifies the SHA-256 checksum, and places the binary in a user-writable `$PATH` location (`~/.valiforge/bin`). The script must handle edge cases: missing `curl`/`wget`, permission errors, existing installations (upgrade path), and corporate proxies. Additionally, provide a Windows install path via `irm https://install.valiforge.dev/ps | iex` (PowerShell) and package manager support (brew, cargo, nix, apt, scoop, winget).

**Estimated Effort:** 5 engineering-days

**Dependencies:** None (foundational)

**Acceptance Criteria:**
- `curl -fsSL https://install.valiforge.dev | sh` completes in < 15 seconds on broadband
- Script detects macOS (Intel + Apple Silicon), Linux (x86_64 + aarch64), and WSL
- SHA-256 checksum verification passes for every download
- Clear error message if architecture is unsupported, with link to build-from-source docs
- Upgrade path: re-running the script updates an existing installation without data loss
- Script is shellcheck-clean (no warnings)
- PowerShell script works on Windows 10+ and Windows Server 2019+
- `brew install valiforge` works via a Homebrew tap (`valiforge/tap`)
- `cargo install valiforge` works from crates.io
- Telemetry opt-in prompt on first install (never silent tracking)
- Install script is tested in CI across all target platforms (GitHub Actions matrix)

**Best Practices:**
- Follow the patterns established by `rustup`, `deno`, and `starship` install scripts
- Never require `sudo` — install to user-local directories
- Provide a `--no-modify-path` flag for users who manage their own `$PATH`
- Include `--version` flag to install specific versions
- Log every step to stderr so users can diagnose failures
- Provide an uninstall command: `valiforge self uninstall`

---

#### DX-002: `valiforge init` — Project Scaffolding Command

**Description:**
Implement an interactive (and non-interactive via `--yes`) project initialization command that creates a `.valiforge/` directory and a `valiforge.toml` configuration file. The command should auto-detect existing API schemas in the project (OpenAPI 3.x YAML/JSON, Swagger 2.0, Protocol Buffers `.proto`, GraphQL `.graphql`/`.gql` SDL files, AsyncAPI specs), present the detected schemas to the user for confirmation, and generate a starter validation suite. The experience should feel magical — the user runs one command and immediately has something working.

**Estimated Effort:** 8 engineering-days

**Dependencies:** DX-001 (install must work first)

**Acceptance Criteria:**
- `valiforge init` in a project with `openapi.yaml` auto-detects and proposes a config in < 2 seconds
- `valiforge init --yes` runs non-interactively (for CI/scripting)
- Generates `valiforge.toml` with detected schemas, default rules, and inline comments explaining each option
- Generates `.valiforge/rules/` directory with starter validation rules
- Generates `.valiforge/snapshots/` directory for breaking-change detection baselines
- Detects and respects existing `.gitignore`, appends `.valiforge/cache/` if not present
- Works in monorepos: detects multiple schemas across subdirectories, asks which to include
- Prints a "Next steps" banner showing the exact command to run next (`valiforge validate`)
- Supports `--template` flag for common setups: `rest-api`, `grpc`, `graphql`, `event-driven`
- Non-interactive mode exits with code 0 on success, non-zero on failure with structured JSON error

**Best Practices:**
- Use `dialoguer` crate for interactive prompts (consistent with Rust CLI ecosystem)
- Use `indicatif` for progress indicators during schema scanning
- Follow the `npm init` / `cargo init` UX pattern — smart defaults, minimal questions
- Generated TOML should be extensively commented so users learn by reading their config
- Never overwrite existing files without explicit confirmation
- Include a `# Generated by valiforge init — safe to edit` header in generated files

---

#### DX-003: Zero-Config Auto-Detection Engine

**Description:**
Build a schema auto-detection module that scans the project directory tree (respecting `.gitignore` and `.valiforge/ignore`) to find API definition files. The engine should identify file type by content inspection (not just extension), parse schema metadata (API title, version, base URL, endpoints count), and rank detected schemas by confidence score. This module powers both `valiforge init` and `valiforge validate` (when run without config).

**Estimated Effort:** 6 engineering-days

**Dependencies:** None (core library module)

**Acceptance Criteria:**
- Detects OpenAPI 3.0/3.1 (YAML and JSON), Swagger 2.0, protobuf (proto3), GraphQL SDL, AsyncAPI 2.x/3.x
- Content-based detection: a file named `api.json` with `"openapi": "3.0"` is detected as OpenAPI regardless of extension
- Respects `.gitignore` — never scans `node_modules/`, `.git/`, `vendor/`, `target/`, etc.
- Scans a 10,000-file project in < 500ms
- Returns structured results: `Vec<DetectedSchema>` with path, schema type, confidence (0.0-1.0), metadata
- Handles ambiguous cases: a YAML file that could be OpenAPI or a generic config is scored lower
- Reports broken/invalid schemas with parse error location (line:col) and suggestion
- Configurable scan depth via `valiforge.toml` (`max_scan_depth = 5`)
- Unit tests for each schema type with real-world example files
- Integration test against popular open-source repos (Stripe, GitHub, Twilio API specs)

**Best Practices:**
- Use `ignore` crate (from ripgrep) for efficient gitignore-aware directory walking
- Use `rayon` for parallel file scanning on multi-core machines
- Implement a trait `SchemaDetector` so new formats can be added without modifying existing code
- Cache scan results in `.valiforge/cache/scan.json` with filesystem mtime-based invalidation
- Never read file contents unless the file extension or name suggests it could be a schema

---

#### DX-004: `valiforge validate` — Core Validation Runner

**Description:**
Implement the primary validation command that loads configuration, resolves schemas, runs validation rules, and outputs results. This is the command users will run most frequently — it must be fast (cold start < 200ms), produce clear output, and integrate seamlessly with CI/CD. Support multiple output formats: human-readable terminal (default), JSON, JUnit XML, SARIF (for GitHub Code Scanning), and TAP (Test Anything Protocol).

**Estimated Effort:** 12 engineering-days

**Dependencies:** DX-003 (auto-detection), DX-002 (config file format)

**Acceptance Criteria:**
- Cold start to first output: < 200ms for a single-file OpenAPI spec
- Default terminal output uses colors, unicode symbols, and aligned columns (like `cargo test` output)
- `--format json` outputs machine-readable JSON (one object per validation result)
- `--format junit` outputs JUnit XML compatible with all major CI systems
- `--format sarif` outputs SARIF 2.1.0 for GitHub Advanced Security integration
- `--format tap` outputs TAP v13 for compatibility with TAP consumers
- Exit code 0 = all validations pass, 1 = validation failures found, 2 = configuration/runtime error
- `--fail-on warn` flag to treat warnings as failures (for strict CI)
- `--include` and `--exclude` flags for filtering rules by ID or tag
- Watch mode: `valiforge validate --watch` re-runs on file changes (using `notify` crate)
- Parallel validation of multiple schemas (configurable concurrency with `--jobs`)
- Summary line at end: "47 rules passed, 3 warnings, 1 error in 0.23s"
- `--fix` flag for auto-fixable issues (e.g., adding missing `description` fields)
- Diff-aware mode: `valiforge validate --changed` only validates files changed since last commit
- Performance: validates the Stripe OpenAPI spec (15,000+ lines) in < 2 seconds

**Best Practices:**
- Use `owo-colors` or `termcolor` for cross-platform colored output
- Use `clap`'s derive API for argument parsing (enables auto-generated completions and docs)
- Implement the `Visitor` pattern for rule evaluation (each rule visits the schema tree)
- Stream results as they complete rather than waiting for all validations to finish
- Include `--verbose` and `--debug` flags for troubleshooting (but default output is minimal and clean)
- Respect `NO_COLOR` environment variable (https://no-color.org)
- Respect `CI=true` environment variable to disable interactive features in CI

---

#### DX-005: Actionable Error Messages System

**Description:**
Design and implement a comprehensive error message system that follows the Rust compiler's famously helpful error format. Every error must include: (1) a unique error code (e.g., `VF0042`), (2) a clear description of what went wrong, (3) the exact location in the file (path:line:col), (4) a code snippet with the error highlighted, (5) a specific suggestion for how to fix it, and (6) a link to the relevant documentation page. Build a central error catalog that maps every error code to its documentation.

**Estimated Effort:** 8 engineering-days

**Dependencies:** DX-004 (validation runner)

**Acceptance Criteria:**
- Every error has a unique, stable code: `VF` prefix + 4-digit number (e.g., `VF0001` through `VF9999`)
- Error output format matches Rust compiler style:
  ```
  error[VF0042]: Missing response schema for status code 200
    --> api/openapi.yaml:47:5
     |
  47 |     responses:
  48 |       200:
     |       ^^^ this response is missing a `content` schema definition
     |
     = help: Add a `content` block with a media type and schema
     = docs: https://docs.valiforge.dev/errors/VF0042
  ```
- Suggestions are context-aware: if a field is misspelled, suggest the correct spelling (Levenshtein distance)
- `valiforge explain VF0042` command prints detailed documentation for any error code
- Error catalog is auto-generated from source code annotations into the docs site
- Warnings use `warning[VFW001]` format with the same structure
- All error messages are tested: unit tests assert the exact output for known inputs
- Error codes are never reused or renumbered after release (stability guarantee)
- Error messages work without color (plain text fallback is still readable)
- Internationalization-ready: error message strings are in a separate resource file

**Best Practices:**
- Study `rustc`, `elm`, and `deno` error messages as gold standards
- Use `miette` or `ariadne` crate for beautiful error rendering with source spans
- Include "Did you mean?" suggestions for common typos using `strsim` crate
- Group related errors: if 15 endpoints all have the same issue, show it once with "... and 14 more"
- Never show a stack trace to end users — log it at `--debug` level only
- Test error messages with real users (developer usability testing)

---

#### DX-006: CLI Help Text Design

**Description:**
Design the CLI help text system following conventions established by modern Rust CLIs (ripgrep, bat, fd, starship). Help text should be scannable, example-rich, and consistently formatted. Implement a hierarchical help system: top-level `valiforge --help` shows subcommands, `valiforge validate --help` shows detailed options, and `valiforge help validate` provides extended documentation with examples.

**Estimated Effort:** 4 engineering-days

**Dependencies:** DX-004 (commands must exist to document)

**Acceptance Criteria:**
- `valiforge --help` fits in one terminal screen (< 40 lines) and groups subcommands logically
- Each subcommand help includes at minimum: description, usage, 2-3 examples, and common flags
- Examples use realistic data (not `foo`/`bar` — use real API paths and filenames)
- Color-coded help text: command names in bold, flags in cyan, descriptions in default color
- `valiforge --version` shows version, git commit hash (short), build date, and Rust compiler version
- `valiforge help` without subcommand shows the same as `--help` but with more detail
- `valiforge help <subcommand>` shows extended help with full examples and notes
- Long help (`--help`) and short help (`-h`) are different: short is concise, long is comprehensive
- Help text passes automated readability check (no lines > 80 chars in short help)
- Man pages generated from clap and installable via `valiforge man`

**Best Practices:**
- Use `clap`'s `about`, `long_about`, `before_help`, `after_help` for structured help sections
- Follow the "inverted pyramid" pattern: most important info first
- Include a `EXAMPLES` section at the bottom of every subcommand help (most-read section)
- Use consistent terminology throughout (pick "schema" not "spec" and stick with it)
- Test help text rendering at 80 and 120 column widths

---

#### DX-007: Shell Completions (bash, zsh, fish, PowerShell)

**Description:**
Generate and distribute shell completions for all major shells. Completions should cover subcommands, flags, flag values (where enumerable), and file path arguments (with smart filtering — e.g., `--schema` only completes `.yaml`, `.json`, `.proto`, `.graphql` files). Completions must be auto-installable via `valiforge completions install` and also available for manual installation.

**Estimated Effort:** 3 engineering-days

**Dependencies:** DX-006 (CLI structure must be finalized)

**Acceptance Criteria:**
- `valiforge completions bash/zsh/fish/powershell` outputs completion script to stdout
- `valiforge completions install` auto-detects shell and installs completions to the correct location
- Tab-completing `valiforge ` shows all subcommands with descriptions
- Tab-completing `valiforge validate --` shows all flags with descriptions
- Tab-completing `--format` shows available formats: `json`, `junit`, `sarif`, `tap`, `text`
- File argument completions filter by relevant extensions (e.g., `--schema` shows `.yaml` files)
- Completions are bundled in Homebrew formula, deb/rpm packages, and AUR package
- Tested in CI: a script verifies completions parse correctly for each shell
- Zsh completions include group headers (Options, Output, Filtering)
- Fish completions include condition-based completions (different completions per subcommand)

**Best Practices:**
- Use `clap_complete` for generation, `clap_complete_fig` for Fig/Warp integration
- Generate completions at build time (not runtime) for consistency
- Include completions in the install script (DX-001) — offer to install them automatically
- Test with real users on each shell — automated generation can miss UX issues
- Update completions in CI whenever the CLI interface changes

---

#### DX-008: `valiforge doctor` — Setup Diagnostics Command

**Description:**
Build a diagnostic command that checks the user's environment for common issues and reports them with clear fix instructions. Modeled after `flutter doctor` and `brew doctor`, this command should verify: installation integrity, PATH configuration, config file validity, schema file accessibility, network connectivity (for cloud features), and toolchain compatibility (CI runner detection).

**Estimated Effort:** 4 engineering-days

**Dependencies:** DX-001, DX-002

**Acceptance Criteria:**
- Output uses checkmark/cross/warning symbols with colors:
  ```
  ValiForge Doctor
  ✓ ValiForge CLI v0.1.0 (installed at ~/.valiforge/bin/valiforge)
  ✓ Configuration file found (./valiforge.toml)
  ✓ 3 API schemas detected (2 OpenAPI, 1 GraphQL)
  ✗ Schema validation: api/openapi.yaml has 2 parse errors
    → Run `valiforge validate --schema api/openapi.yaml` for details
  ✓ Shell completions installed (zsh)
  ! Update available: v0.2.0 (run `valiforge self update`)
  ```
- Checks: binary integrity, PATH configuration, config file parse, schema file parse, network (ping cloud API), disk space, shell completions, git integration
- `--json` flag for machine-readable output (useful in CI for automated environment validation)
- Exit code 0 = all checks pass, 1 = warnings only, 2 = errors found
- Each failed check includes the exact command to fix it
- Detects CI environment (GitHub Actions, GitLab CI, Jenkins, CircleCI) and shows CI-specific advice
- Detects conflicting tools (e.g., old Swagger validators) and suggests migration steps
- Total execution time < 3 seconds (network checks have 2-second timeout)

**Best Practices:**
- Run checks in parallel where possible (network check shouldn't block filesystem checks)
- Cache doctor results for 1 hour to avoid redundant checks
- Include `--fix` flag that automatically resolves fixable issues (with confirmation)
- Log detailed diagnostic info to `~/.valiforge/doctor.log` for support ticket attachment
- Make it easy to copy-paste doctor output into a GitHub issue

---

#### DX-009: VS Code Extension — Inline Validation

**Description:**
Build a Visual Studio Code extension that provides real-time inline validation for API schema files. The extension should show validation errors/warnings as diagnostics (red/yellow squiggles), provide hover documentation for schema elements, offer quick-fix code actions, and integrate with the ValiForge CLI as the validation backend. The extension should work immediately after installation without configuration for projects that already have `valiforge.toml`.

**Estimated Effort:** 15 engineering-days

**Dependencies:** DX-004, DX-005 (CLI must produce machine-readable output)

**Acceptance Criteria:**
- Published on VS Code Marketplace and Open VSX Registry (for VS Code forks like VSCodium)
- Extension activates automatically when `.valiforge/` directory or `valiforge.toml` is detected
- Real-time diagnostics: validation errors appear as you type (debounced, < 500ms after last keystroke)
- Hover on schema elements shows: type info, constraints, description, and link to docs
- Code actions (Quick Fix): offer `valiforge --fix` suggestions as clickable fixes
- Status bar item showing validation status (passing/failing/running)
- Command palette: "ValiForge: Validate Current File", "ValiForge: Validate All", "ValiForge: Init"
- Settings: configurable rule severity overrides, custom rule paths, binary path
- Codelens on endpoint definitions showing validation status
- Problem panel integration: all ValiForge diagnostics appear in the Problems tab
- Tree view in sidebar: schema explorer showing all endpoints/types with validation status
- Extension size < 5MB, startup time < 1 second
- Works offline (uses local CLI binary, no network required)
- Telemetry opt-in only, with clear privacy documentation
- Extension is open-source (same Apache 2.0 license)

**Best Practices:**
- Use the Language Server Protocol (LSP) for the backend — this enables Neovim reuse (DX-010)
- Build the LSP server in Rust (reuse core validation logic) for performance
- Follow VS Code extension guidelines: https://code.visualstudio.com/api/references/extension-guidelines
- Use `esbuild` for extension bundling (faster than webpack)
- Implement a test suite using `@vscode/test-electron`
- Publish CI/CD pipeline that auto-publishes on GitHub Release tags

---

#### DX-010: Neovim Integration via LSP

**Description:**
Ensure the ValiForge Language Server (built for VS Code in DX-009) works with Neovim's built-in LSP client and popular Neovim plugin managers. Provide configuration snippets for `nvim-lspconfig`, `mason.nvim`, and `lazy.nvim`. Build a dedicated Neovim plugin (`valiforge.nvim`) that adds enhanced features beyond basic LSP: Telescope integration for schema search, floating window previews, and integration with `trouble.nvim` for the diagnostics list.

**Estimated Effort:** 5 engineering-days

**Dependencies:** DX-009 (LSP server)

**Acceptance Criteria:**
- ValiForge LSP server binary is installable via `mason.nvim`
- 5-line `nvim-lspconfig` configuration snippet works out of the box
- Diagnostics appear inline using Neovim's native diagnostic framework
- Hover documentation works via `vim.lsp.buf.hover()`
- Code actions work via `vim.lsp.buf.code_action()`
- `valiforge.nvim` plugin provides:
  - `:ValiForge validate` command
  - `:ValiForge doctor` command
  - Telescope picker for searching endpoints/schemas
  - Floating window for validation summary
- Plugin works with Neovim 0.9+ (current stable and one prior version)
- README includes configuration for popular setups: `lazy.nvim`, `packer.nvim`, `vim-plug`
- Integration test suite using `plenary.nvim` test harness
- Published to `luarocks` for `lazy.nvim` native install

**Best Practices:**
- Reuse the LSP server from DX-009 — do not build a separate backend
- Follow Neovim plugin conventions: Lua-first, no Vimscript
- Test with popular Neovim distributions: LazyVim, AstroNvim, NvChad, LunarVim
- Include GIF demos in the README showing each feature
- Provide a `recommended.lua` config file that users can copy wholesale

---

#### DX-011: `valiforge self update` — Auto-Update Mechanism

**Description:**
Implement a self-update command that checks for new versions, downloads the update, and replaces the current binary. Include an optional auto-update check that runs in the background (at most once per 24 hours) and notifies the user of available updates without blocking their workflow.

**Estimated Effort:** 3 engineering-days

**Dependencies:** DX-001 (install infrastructure)

**Acceptance Criteria:**
- `valiforge self update` checks GitHub Releases API, downloads latest, verifies checksum, replaces binary
- Update check is atomic: if download fails, the old binary remains intact
- Background update check: on any command, if > 24 hours since last check, spawn a background process to check (does not slow down the current command)
- Update notification: "A new version of ValiForge is available (v0.2.0). Run `valiforge self update` to upgrade."
- `valiforge self update --check` only checks without installing
- Respects `VALIFORGE_NO_UPDATE_CHECK=1` environment variable
- Works behind corporate proxies (respects `HTTP_PROXY`/`HTTPS_PROXY`)
- `valiforge self update --version 0.1.5` installs a specific version (for downgrading)
- Update preserves configuration and cached data

**Best Practices:**
- Use `self_update` crate as a starting point
- Implement update verification: after replacing the binary, run `valiforge --version` to verify it works
- If the new binary fails to start, automatically roll back to the previous version
- Rate-limit update checks to avoid hitting GitHub API limits
- Log update history to `~/.valiforge/update.log`

---

#### DX-012: `valiforge diff` — Breaking Change Detection

**Description:**
Implement a command that compares two versions of an API schema and detects breaking changes. This is a high-value feature that differentiates ValiForge from simpler linting tools. The command should classify changes as breaking, non-breaking, or informational, and integrate with CI to block merges that introduce breaking changes without explicit approval.

**Estimated Effort:** 10 engineering-days

**Dependencies:** DX-003 (schema parsing), DX-004 (output formatting)

**Acceptance Criteria:**
- `valiforge diff api/openapi.yaml` compares current file against the last committed version (git integration)
- `valiforge diff --base main --head feature/new-api` compares between git branches
- `valiforge diff old.yaml new.yaml` compares two explicit files
- Detects breaking changes: removed endpoints, removed required fields, type changes, narrowed enums, changed auth requirements
- Detects non-breaking changes: new endpoints, new optional fields, widened enums, added descriptions
- Output shows a summary table and detailed per-change breakdown
- `--format json` for CI integration
- GitHub Actions integration: posts a PR comment with the diff summary
- `--allow-breaking` flag with a reason string for intentional breaking changes (logged)
- Generates a changelog entry from the diff
- Supports OpenAPI, GraphQL, and Protobuf schema diffing
- Performance: diffs the Stripe API spec in < 1 second

**Best Practices:**
- Use semver-aware diffing: breaking changes suggest a major version bump
- Integrate with the `valiforge snapshot` command for baseline management
- Provide a `breaking-changes.yaml` allowlist file for intentional breaks
- Show the full context of each change (not just "field removed" — show which endpoint is affected)

---

### 1.1 DX Design — Summary Table

| Task ID | Title | Effort (days) | Priority | Dependencies |
|---------|-------|---------------|----------|--------------|
| DX-001 | One-Line Install Script | 5 | P0 | None |
| DX-002 | `valiforge init` | 8 | P0 | DX-001 |
| DX-003 | Zero-Config Auto-Detection | 6 | P0 | None |
| DX-004 | `valiforge validate` | 12 | P0 | DX-003, DX-002 |
| DX-005 | Actionable Error Messages | 8 | P0 | DX-004 |
| DX-006 | CLI Help Text Design | 4 | P0 | DX-004 |
| DX-007 | Shell Completions | 3 | P1 | DX-006 |
| DX-008 | `valiforge doctor` | 4 | P1 | DX-001, DX-002 |
| DX-009 | VS Code Extension | 15 | P1 | DX-004, DX-005 |
| DX-010 | Neovim Integration | 5 | P2 | DX-009 |
| DX-011 | Self-Update Mechanism | 3 | P1 | DX-001 |
| DX-012 | Breaking Change Detection | 10 | P0 | DX-003, DX-004 |
| **Total** | | **83** | | |

---

## 2. DOCUMENTATION STRATEGY

---

#### DOC-001: Documentation Framework Setup (Starlight/Astro)

**Description:**
Set up the documentation site using Astro with the Starlight theme — the leading documentation framework for developer tools, used by Astro itself, Biome, and other Rust-ecosystem tools. Configure the site with: dark/light mode, full-text search (Pagefind), versioning support, sidebar navigation, and deployment to `docs.valiforge.dev` via Cloudflare Pages or Vercel.

**Estimated Effort:** 4 engineering-days

**Dependencies:** None

**Acceptance Criteria:**
- Docs site builds with `npm run build` in < 30 seconds
- Lighthouse score: Performance > 95, Accessibility > 95, Best Practices > 95, SEO > 95
- Dark mode toggle with system preference detection and manual override
- Full-text search powered by Pagefind (built at build time, no external service)
- Sidebar navigation with collapsible sections matching the information architecture
- Mobile-responsive: readable and navigable on phones (hamburger menu for sidebar)
- Version selector dropdown (v0.1, v0.2, latest, next)
- "Edit this page on GitHub" link on every page
- Automatic table of contents on right sidebar for long pages
- Code blocks with syntax highlighting (shiki), copy button, and file name labels
- Custom components: `<Callout>` (tip/warning/danger), `<Tabs>` (for OS/language switching), `<Steps>`
- Deployed via CI on every merge to main, preview deployments on PRs
- Custom 404 page with search box and common links
- RSS feed for blog/changelog
- OpenGraph meta tags for social sharing (custom images per section)
- Sitemap.xml auto-generated

**Best Practices:**
- Use Starlight's i18n support from day one (even if only English initially)
- Put docs in the main repo (`docs/` directory) so documentation is reviewed alongside code changes
- Use MDX for pages that need interactive components
- Implement a docs linting pipeline: check for broken links, unused images, consistent terminology
- Set up Algolia DocSearch as a future upgrade path (Pagefind for launch, Algolia at scale)

---

#### DOC-002: Quick Start Guide (< 5 Minutes)

**Description:**
Write the most important page on the entire docs site: the Quick Start guide. This page must take a developer from zero to a successful validation run in under 5 minutes. It should work for the three most common scenarios: (1) a Node.js project with an OpenAPI spec, (2) a Go project with protobuf, and (3) a Python project with a GraphQL schema. Use tabs to let users pick their scenario.

**Estimated Effort:** 3 engineering-days

**Dependencies:** DOC-001 (site setup), DX-001/002/004 (CLI commands)

**Acceptance Criteria:**
- Page reads in under 5 minutes (measured by average reading speed tool)
- Three tabs: REST/OpenAPI, gRPC/Protobuf, GraphQL
- Step 1: Install (one-liner with OS tabs: macOS, Linux, Windows)
- Step 2: Initialize (`valiforge init` with expected output shown)
- Step 3: Validate (`valiforge validate` with expected output shown)
- Step 4: Next steps (links to guides for CI setup, custom rules, etc.)
- Every code block is copy-pasteable and tested in CI
- Includes a "What just happened?" expandable section explaining what ValiForge checked
- Terminal output screenshots use `asciinema` or `vhs` (Charm) for animated recordings
- Page has a "Was this helpful?" feedback widget at the bottom
- No jargon without explanation — defines "schema", "validation", "breaking change" on first use
- Available in a single-page printable format for offline use

**Best Practices:**
- Write for a developer who has never heard of ValiForge — assume zero context
- Test the guide with 5+ real developers and iterate based on where they get stuck
- Include a "Common Issues" section at the bottom addressing the top 3 setup problems
- Use progressive disclosure: basic path is simple, advanced options are in expandable sections
- Update this page on every release — it is the first thing new users see

---

#### DOC-003: Concepts & Architecture Documentation

**Description:**
Write the conceptual documentation that explains ValiForge's philosophy, architecture, and mental model. This section should cover: schema-first testing philosophy, the validation pipeline (detect → parse → validate → report), rule system (built-in vs. custom), the trait-based architecture (why traits not plugins), and how ValiForge compares to other approaches (Postman, Spectral, Dredd).

**Estimated Effort:** 5 engineering-days

**Dependencies:** DOC-001

**Acceptance Criteria:**
- Pages: "How ValiForge Works" (overview), "Schema-First Testing", "The Validation Pipeline", "Rules & Rulesets", "Custom Rules", "Architecture Overview"
- Architecture page includes a clear diagram (Mermaid or SVG) of the validation pipeline
- Each concept page is self-contained (can be read independently) but cross-links to related pages
- Comparison page is factual, fair, and acknowledges where alternatives are better
- Code examples illustrate every concept (not just text descriptions)
- Glossary page defining all ValiForge-specific terms
- Reading time indicated on each page
- Reviewed by at least 2 people who are NOT on the core team (outsider readability check)

**Best Practices:**
- Use the Divio documentation system: separate concepts from how-to guides from reference
- Include "Why?" explanations, not just "What?" — developers want to understand design decisions
- Use analogies to familiar concepts (e.g., "Rules are like ESLint rules for your API schema")
- Avoid marketing language in technical docs — be precise and honest
- Include a "Design Decisions" page explaining key tradeoffs (FAQ format)

---

#### DOC-004: How-To Guides (CI/CD, Migration, Integrations)

**Description:**
Write task-oriented guides that solve specific developer problems. Each guide should be a complete, copy-pasteable solution for a real-world scenario. Priority guides: (1) GitHub Actions CI setup, (2) GitLab CI setup, (3) Breaking change detection in PRs, (4) Custom rule authoring, (5) Migrating from Postman, (6) Migrating from Spectral, (7) Test data generation with SLMs, (8) Monorepo setup.

**Estimated Effort:** 10 engineering-days

**Dependencies:** DOC-001, DX-004, DX-012

**Acceptance Criteria:**
- Minimum 8 guides at launch, each with:
  - Clear problem statement ("You want to...")
  - Prerequisites section
  - Step-by-step instructions (numbered, not paragraphs)
  - Complete code examples (not snippets — full config files)
  - "Verify it works" section at the end
  - Troubleshooting section for common issues
- GitHub Actions guide includes a complete `.github/workflows/api-validation.yml`
- GitLab CI guide includes a complete `.gitlab-ci.yml`
- Migration guides include feature-mapping tables (Postman feature → ValiForge equivalent)
- All code examples tested in CI using the `docs-test` job
- Each guide is tagged with difficulty level: Beginner, Intermediate, Advanced
- Guides are findable via search (good titles, descriptions, and keywords)

**Best Practices:**
- Write guides in order of user need: CI setup is #1 because it is asked for most often
- Include the "Why would I do this?" context, not just the "How"
- Use realistic project structures in examples (not toy projects)
- Link to Concepts pages for deeper understanding (but guides are self-sufficient)
- Update guides when the CLI interface changes — treat them as production code

---

#### DOC-005: CLI Reference (Auto-Generated from Clap)

**Description:**
Build a pipeline that auto-generates CLI reference documentation from `clap` derive annotations in the Rust source code. Every command, subcommand, flag, and argument should have a dedicated reference page with: syntax, description, examples, default values, environment variable overrides, and related commands. The generation should run in CI so docs are always in sync with the code.

**Estimated Effort:** 4 engineering-days

**Dependencies:** DOC-001, DX-006 (CLI help text)

**Acceptance Criteria:**
- Every CLI command has a reference page at `docs.valiforge.dev/reference/cli/<command>`
- Pages are auto-generated from `clap` annotations — manual edits are overwritten
- Each page includes: syntax diagram, description, all flags with defaults, examples, environment variables
- `clap`'s `long_about` maps to the page description, `help` maps to the flag description
- Generation runs as a CI step: `cargo run -- generate-docs --output docs/src/content/docs/reference/cli/`
- Diff check in CI: if generated docs differ from committed docs, the CI job fails (forces regeneration)
- Cross-links between related commands (e.g., `validate` page links to `diff` and `init`)
- Search indexes these pages with correct titles and descriptions
- Man pages (`valiforge.1`) are generated from the same source and included in the install package

**Best Practices:**
- Use `clap_mangen` for man page generation
- Write a custom `clap` visitor that extracts all metadata into a structured JSON intermediate format
- Keep examples in the `clap` annotations (not in a separate file) so they are in one place
- Include "See also" sections linking to relevant How-To guides
- Version the CLI reference: older versions remain accessible

---

#### DOC-006: Configuration Reference

**Description:**
Write a comprehensive reference for the `valiforge.toml` configuration file. Every configuration key should be documented with: type, default value, description, example, and any validation constraints. Include a complete example configuration file with inline comments and a minimal configuration for common use cases.

**Estimated Effort:** 3 engineering-days

**Dependencies:** DOC-001, DX-002

**Acceptance Criteria:**
- Every `valiforge.toml` key is documented on a single reference page
- Each key includes: name, type (string/bool/int/array/table), default, description, since (version), example
- Keys are grouped by section: `[project]`, `[schemas]`, `[rules]`, `[output]`, `[cloud]`
- Complete example `valiforge.toml` with all keys and inline comments
- Minimal example configs for common use cases (REST API, GraphQL, monorepo)
- JSON Schema for `valiforge.toml` provided for editor auto-completion
- Configuration is validated by the `valiforge doctor` command with helpful error messages
- Page includes a "Frequently Changed Settings" section at the top for quick access
- Environment variable overrides documented for every key (e.g., `VALIFORGE_RULES_STRICT=true`)

**Best Practices:**
- Auto-generate from a single source of truth (Rust struct with `serde` derive)
- Provide a `valiforge config --dump-default` command that outputs the full default config
- Include migration notes when config format changes between versions
- Test that the documented defaults match the actual code defaults (CI check)

---

#### DOC-007: API/SDK Reference for `valiforge-sdk` Crate

**Description:**
Set up and publish comprehensive Rust API documentation for the `valiforge-sdk` crate on docs.rs. This is for developers who want to embed ValiForge validation in their own Rust applications or build custom tools on top of the ValiForge engine. Ensure every public type, function, trait, and module has documentation with examples.

**Estimated Effort:** 5 engineering-days

**Dependencies:** Core library must have stable public API

**Acceptance Criteria:**
- `cargo doc` produces clean documentation with zero warnings
- Every public item has a doc comment (`///`) with: description, examples, panics (if any), errors (if any)
- Top-level crate documentation (`//!`) includes: overview, quick start example, feature flags, MSRV
- All examples in doc comments compile and run (`cargo test --doc` passes)
- Feature flags documented: which features are default vs. optional
- Links from the Starlight docs site to docs.rs for API reference
- `README.md` on crates.io includes a quick start for the SDK
- Published to crates.io with appropriate categories and keywords
- Changelog maintained in `CHANGELOG.md` following Keep a Changelog format

**Best Practices:**
- Use `#[doc(cfg(...))]` to indicate which feature flags enable which items
- Use `#[doc(alias = "...")]` for discoverability of items with multiple names
- Run `cargo doc --document-private-items` for internal development docs
- Include architecture doc comments on modules (`//!` at top of `mod.rs`)
- Use intra-doc links (`[`SchemaDetector`]`) for cross-referencing

---

#### DOC-008: Troubleshooting & FAQ

**Description:**
Create a living troubleshooting guide and FAQ based on real user questions. Seed it with anticipated issues (from beta testing) and establish a process for continuously adding new entries as questions arise in GitHub Issues and Discord.

**Estimated Effort:** 3 engineering-days

**Dependencies:** DOC-001

**Acceptance Criteria:**
- Initial FAQ with 20+ entries covering installation, configuration, common errors, and CI setup
- Each entry follows the format: Question → Short Answer → Detailed Explanation → Related Links
- Troubleshooting page organized by symptom (not by component)
- Searchable: each entry has keywords for the search index
- "Still stuck?" section at the bottom linking to Discord #help channel and GitHub Issues
- Process documented for adding new FAQ entries (team playbook)
- FAQ entries link to error codes (DX-005) where relevant
- Community-contributed FAQ entries are accepted via PR

**Best Practices:**
- Write FAQ entries in the user's language (their question), not in technical jargon
- Review GitHub Issues weekly and extract FAQ candidates
- Use analytics to identify top search queries with no results — these indicate missing FAQ entries
- Keep answers concise — link to detailed docs rather than duplicating content

---

#### DOC-009: Contributing Guide

**Description:**
Write a comprehensive guide for open-source contributors that covers: development environment setup, build instructions, project structure, testing strategy, PR process, code style, commit message conventions, and the path from first contribution to maintainership. This guide should make it possible for a new contributor to submit their first PR within 30 minutes.

**Estimated Effort:** 3 engineering-days

**Dependencies:** DOC-001, Community setup (COM-001)

**Acceptance Criteria:**
- `CONTRIBUTING.md` in repo root with: quick start, dev setup, testing, PR process, code style
- Dev setup works on macOS, Linux, and Windows (with WSL) — tested in CI
- `just setup` (or `make setup`) installs all development dependencies
- Project structure documented: what each crate/directory does
- Testing guide: how to run unit tests, integration tests, doc tests, and benchmarks
- PR process: branch naming, commit messages, review process, merge strategy
- Code style: Rust edition, formatting (`rustfmt` config), linting (`clippy` config), naming conventions
- "Good First Issue" guide: where to look, how to claim an issue, expected response time
- Architecture Decision Records (ADRs) explained: how to propose changes via ADR
- Contributor License Agreement (CLA) process if required (or DCO sign-off)
- Estimated time from "clone" to "tests passing": < 15 minutes on a modern machine

**Best Practices:**
- Include a `DEVELOPMENT.md` for detailed setup that `CONTRIBUTING.md` links to
- Provide a `Justfile` (or `Makefile`) with common commands: `just build`, `just test`, `just lint`, `just fmt`
- Use `cargo-insta` for snapshot testing — document how to update snapshots
- Include a "How to write a good bug report" section
- Thank contributors by name in release notes

---

#### DOC-010: Internationalization (i18n) Planning

**Description:**
Design the internationalization strategy for documentation. Plan for English-first with a framework for community-contributed translations. Identify the top 5 developer languages by market size and community demand. Set up the translation toolchain and contribution process.

**Estimated Effort:** 3 engineering-days

**Dependencies:** DOC-001

**Acceptance Criteria:**
- Starlight i18n configured with English as default locale
- Translation-ready: all user-facing strings in CLI are extractable (using `fluent` or `i18n-embed`)
- Target languages identified with rationale: Chinese (zh), Japanese (ja), Korean (ko), Spanish (es), Portuguese (pt-BR)
- Translation contribution guide: how to add a new language, how to update existing translations
- Translation progress dashboard (which pages are translated, which are outdated)
- CI check: if English source changes, mark corresponding translations as "needs update"
- Machine translation baseline: use LLM to generate initial translations, human-reviewed before publish
- URL structure: `docs.valiforge.dev/zh/quick-start` (language prefix)

**Best Practices:**
- Start with CLI error messages and Quick Start guide (highest impact)
- Use Crowdin or Weblate for community translation management
- Never block a release on translations — English is always the source of truth
- Establish a "Translation Champions" program for each language
- Include cultural adaptation, not just word-for-word translation

---

### Documentation — Summary Table

| Task ID | Title | Effort (days) | Priority | Dependencies |
|---------|-------|---------------|----------|--------------|
| DOC-001 | Documentation Framework Setup | 4 | P0 | None |
| DOC-002 | Quick Start Guide | 3 | P0 | DOC-001, DX-001/002/004 |
| DOC-003 | Concepts & Architecture | 5 | P0 | DOC-001 |
| DOC-004 | How-To Guides | 10 | P0 | DOC-001, DX-004, DX-012 |
| DOC-005 | CLI Reference (Auto-Gen) | 4 | P0 | DOC-001, DX-006 |
| DOC-006 | Configuration Reference | 3 | P0 | DOC-001, DX-002 |
| DOC-007 | API/SDK Reference | 5 | P1 | Stable API |
| DOC-008 | Troubleshooting & FAQ | 3 | P1 | DOC-001 |
| DOC-009 | Contributing Guide | 3 | P0 | DOC-001 |
| DOC-010 | i18n Planning | 3 | P2 | DOC-001 |
| **Total** | | **43** | | |

---

## 3. OPEN SOURCE COMMUNITY BUILDING

---

#### COM-001: GitHub Repository Setup & Polish

**Description:**
Set up the GitHub repository with all the standard open-source project files, templates, and automation that signal a well-maintained project. First impressions matter: a developer who lands on the repo should immediately see a professional, active project with clear contribution paths.

**Estimated Effort:** 5 engineering-days

**Dependencies:** None

**Acceptance Criteria:**
- `README.md` includes:
  - Badges: CI status, crates.io version, docs.rs, Discord members, license
  - One-line description: "Headless API validation engine. Rust-fast. AI-native. Zero config."
  - Animated GIF/SVG demo showing `valiforge init` → `valiforge validate` flow (created with `vhs`)
  - Quick start (3-step install/init/validate)
  - Feature list with brief descriptions
  - Comparison table (vs. Postman, Spectral, Dredd)
  - Links to docs, Discord, contributing guide
  - "Who uses ValiForge" section (populated after launch)
  - Sponsors section (for GitHub Sponsors / Open Collective)
- `CONTRIBUTING.md` linking to full docs
- `CODE_OF_CONDUCT.md` (Contributor Covenant v2.1)
- `SECURITY.md` with responsible disclosure policy and security@valiforge.dev contact
- `LICENSE` (Apache 2.0)
- `CODEOWNERS` mapping directories to team members
- `.github/ISSUE_TEMPLATE/bug_report.yml` (YAML-based form with OS, version, repro steps)
- `.github/ISSUE_TEMPLATE/feature_request.yml` (YAML-based form with use case, proposal)
- `.github/ISSUE_TEMPLATE/question.yml` (redirects to GitHub Discussions)
- `.github/PULL_REQUEST_TEMPLATE.md` with checklist (tests, docs, changelog)
- `.github/FUNDING.yml` for GitHub Sponsors
- GitHub repository settings: Discussions enabled, Wiki disabled, Projects enabled, branch protection on `main`
- Repository topics: `api-validation`, `rust`, `openapi`, `graphql`, `protobuf`, `developer-tools`, `cli`
- Social preview image (1280x640) for link sharing

**Best Practices:**
- Study top Rust repos for conventions: `ripgrep`, `bat`, `deno`, `zed`, `nushell`
- Use GitHub's YAML-based issue templates (not markdown) for structured input
- Keep README under 500 lines — link to docs for details
- Update README badges automatically via CI
- Pin 3-4 key issues for newcomers

---

#### COM-002: GitHub Labels & Issue Triage System

**Description:**
Design a comprehensive GitHub label system and issue triage workflow. Labels should help contributors find work, help maintainers prioritize, and help users track progress on their requests.

**Estimated Effort:** 2 engineering-days

**Dependencies:** COM-001

**Acceptance Criteria:**
- Label categories with consistent color coding:
  - **Type:** `bug` (red), `feature` (blue), `enhancement` (cyan), `docs` (purple), `performance` (orange)
  - **Priority:** `P0-critical` (red), `P1-high` (orange), `P2-medium` (yellow), `P3-low` (green)
  - **Status:** `needs-triage`, `confirmed`, `in-progress`, `blocked`, `wontfix`
  - **Effort:** `good-first-issue` (green), `help-wanted` (green), `mentor-available` (teal), `complex` (red)
  - **Component:** `cli`, `core-engine`, `rules`, `docs`, `vscode-extension`, `ci-cd`
  - **Schema type:** `openapi`, `graphql`, `protobuf`, `asyncapi`
- GitHub Actions bot that auto-labels issues based on template type
- Stale issue bot: comment after 30 days of inactivity, close after 60 days (with "stale" label)
- "Good First Issue" issues always have: clear description, expected behavior, implementation hints, mentor name
- Triage SLA: new issues triaged within 48 hours (label + comment)
- Weekly triage meeting process documented

**Best Practices:**
- Keep label names lowercase and hyphenated for consistency
- Limit labels to ~30 total — too many labels means none are useful
- Use GitHub Projects (v2) for sprint planning and roadmap tracking
- Automate what you can: auto-assign reviewers, auto-label by file path changed
- Rotate triage duty weekly among maintainers

---

#### COM-003: Discord Community Setup

**Description:**
Set up and configure a Discord server as the primary real-time community hub for ValiForge. Design channel structure, roles, moderation rules, and onboarding flow to create a welcoming, productive community space.

**Estimated Effort:** 3 engineering-days

**Dependencies:** None

**Acceptance Criteria:**
- Server structure:
  - **Info:** #welcome, #rules, #announcements, #changelog
  - **Community:** #general, #showcase, #off-topic
  - **Support:** #help, #troubleshooting (with forum-style threads)
  - **Development:** #contributors, #rfc, #architecture, #code-review
  - **Voice:** "Office Hours" voice channel, "Pair Programming" voice channel
- Roles: `@Team` (core team), `@Maintainer`, `@Contributor`, `@Champion`, `@Member`
- Onboarding: welcome message with rules acknowledgment, auto-role assignment
- AutoMod rules: spam filter, link allowlist, profanity filter, duplicate message prevention
- Bots:
  - GitHub bot: posts new releases, high-priority issues, and merged PRs
  - Welcome bot: greets new members with getting-started links
  - Thread bot: auto-creates threads for support questions
- Custom emoji set: ValiForge logo variants, check/cross/warning for reactions
- Vanity URL: `discord.gg/valiforge`
- Server boost perks: contributor recognition, custom roles
- Moderation playbook: how to handle Code of Conduct violations

**Best Practices:**
- Keep channel count low initially (< 15) — split when a channel gets too noisy
- Use forum-style channels for #help (each question is a thread with solved/unsolved status)
- Pin "How to ask for help" guide in support channels
- Core team should be visibly active daily during launch month
- Archive inactive channels rather than deleting them
- Set up analytics: track member growth, message volume, response time

---

#### COM-004: GitHub Discussions Configuration

**Description:**
Enable and configure GitHub Discussions as the structured, searchable alternative to Discord for long-form Q&A, feature proposals (RFCs), and community announcements. Discussions persist and are SEO-indexed, making them valuable for common questions that Discord messages would bury.

**Estimated Effort:** 1 engineering-day

**Dependencies:** COM-001

**Acceptance Criteria:**
- Discussion categories:
  - **Q&A** (question/answer format with "answered" marking)
  - **Ideas** (feature ideas and proposals)
  - **RFC** (formal Request for Comments with template)
  - **Show & Tell** (community projects and integrations)
  - **Announcements** (team announcements, pinned)
  - **General** (catch-all)
- Category descriptions explain what belongs where
- RFC template: Problem, Proposal, Alternatives Considered, Implementation Notes
- Discussion pinning for active RFCs and important announcements
- Link to Discussions from issue templates (questions redirect to Discussions)
- GitHub Actions: auto-label discussions, notify Discord of new RFCs
- Moderation guidelines: when to convert a Discussion to an Issue

**Best Practices:**
- Use Q&A category aggressively — mark answers so future searchers find solutions
- Convert accepted RFCs to tracking issues when implementation begins
- Cross-post important Discussions to Discord for visibility
- Regularly review unanswered discussions (weekly)

---

#### COM-005: "Good First Issue" Pipeline

**Description:**
Establish a systematic process for creating, curating, and supporting "Good First Issue" (GFI) contributions. This is the primary on-ramp for new contributors and directly impacts the project's ability to grow its contributor base. Each GFI should be self-contained, well-documented, and have a named mentor.

**Estimated Effort:** 3 engineering-days (initial setup + first batch of 10 issues)

**Dependencies:** COM-001, COM-002, DOC-009

**Acceptance Criteria:**
- 10 "Good First Issue" issues created before public launch
- Each GFI includes:
  - Clear problem description
  - Expected behavior / acceptance criteria
  - Implementation hints (which files to modify, which tests to add)
  - Links to relevant documentation
  - Named mentor (who to ask for help)
  - Estimated difficulty (1-3 scale) and estimated time (hours)
- GFIs span multiple areas: CLI, docs, tests, error messages, rules
- "Claim" process documented: comment to claim, 7-day reservation, mentor check-in at day 3
- Automation: bot comments on claimed issues with helpful links
- Dashboard tracking GFI completion rate, time-to-merge, contributor retention
- Monthly GFI creation target: 5 new issues per month
- Mentors trained on how to give helpful code review for new contributors

**Best Practices:**
- Break large tasks into multiple small GFIs (e.g., "Add 5 error message tests" → 5 separate issues)
- Include test-only GFIs (lower barrier: just add a test, don't change production code)
- Include docs-only GFIs (even lower barrier: fix typos, add examples)
- Celebrate first-time contributors: bot posts a welcome message on their first merged PR
- Track which GFIs lead to repeat contributors (optimize for retention)

---

#### COM-006: Community Programs — "ValiForge Champions"

**Description:**
Design and launch a community recognition program that identifies, rewards, and empowers active community members. Champions get early access to features, direct access to the core team, speaking opportunities, and public recognition. This program creates a flywheel of contribution and advocacy.

**Estimated Effort:** 3 engineering-days

**Dependencies:** COM-003, COM-004

**Acceptance Criteria:**
- Champion tiers: Contributor → Regular Contributor → Champion → Maintainer → Core Team
- Criteria for each tier:
  - **Contributor:** 1+ merged PR or accepted Discussion answer
  - **Regular Contributor:** 5+ merged PRs over 3+ months
  - **Champion:** 10+ merged PRs OR significant community contributions (organizing events, writing tutorials)
  - **Maintainer:** Invited by core team, has merge permissions on specific areas
  - **Core Team:** Full repository access, roadmap input, paid (if applicable)
- Champion benefits: Discord role, logo on README, early access, swag, conference sponsorship
- Application process: self-nomination or team-nomination, reviewed quarterly
- Public Champions page on docs site with photos, bios, and contribution highlights
- Quarterly "Contributor Spotlight" blog post featuring top contributors
- Release notes always credit contributors by username with links to their PRs
- Annual "State of ValiForge" report includes contributor statistics

**Best Practices:**
- Start small: 5-10 champions in the first cohort
- Give champions real power (not just a title): RFC voting rights, roadmap input sessions
- Invest in champion relationships: 1:1 monthly calls with core team
- Create a private #champions Discord channel for early discussions
- Model after: Astro Champions, Deno Contributors, Next.js Gold Sponsors

---

#### COM-007: Monthly Community Calls / Office Hours

**Description:**
Establish a monthly community call cadence for live interaction between the core team and community. These calls build trust, provide a forum for feedback, and create content (recordings) for asynchronous consumption.

**Estimated Effort:** 2 engineering-days (setup + first call prep)

**Dependencies:** COM-003

**Acceptance Criteria:**
- Monthly cadence: first Thursday of each month, 10 AM PT (friendly to US/EU timezones)
- Alternating format: community call (updates + Q&A) and office hours (open discussion)
- Community call agenda: roadmap update, release highlights, contributor spotlight, Q&A
- Live on Discord (Stage Channel) with simultaneous YouTube Live stream
- Recorded and published to YouTube within 24 hours with chapters and timestamps
- Calendar invite link on docs site and Discord #announcements
- Agenda published 1 week before; community can submit questions/topics
- Meeting notes published to GitHub Discussions within 48 hours
- Quarterly "Deep Dive" sessions on specific technical topics (architecture, performance, etc.)

**Best Practices:**
- Keep calls to 45 minutes max (30 min content + 15 min Q&A)
- Rotate hosts/presenters to avoid single point of failure
- Record a 5-minute highlight clip for social media
- Ask for feedback after every call (1-question survey)
- Time zone rotation: quarterly shift to accommodate APAC community members

---

### Community Building — Summary Table

| Task ID | Title | Effort (days) | Priority | Dependencies |
|---------|-------|---------------|----------|--------------|
| COM-001 | GitHub Repository Setup | 5 | P0 | None |
| COM-002 | Labels & Triage System | 2 | P0 | COM-001 |
| COM-003 | Discord Community Setup | 3 | P0 | None |
| COM-004 | GitHub Discussions | 1 | P1 | COM-001 |
| COM-005 | Good First Issue Pipeline | 3 | P0 | COM-001, COM-002, DOC-009 |
| COM-006 | ValiForge Champions Program | 3 | P2 | COM-003, COM-004 |
| COM-007 | Monthly Community Calls | 2 | P2 | COM-003 |
| **Total** | | **19** | | |

---

## 4. CONTENT MARKETING PLAN (FIRST 6 MONTHS)

---

#### CTN-001: Blog Platform Setup

**Description:**
Set up a blog within the Starlight docs site (or a dedicated Astro blog) at `blog.valiforge.dev` or `docs.valiforge.dev/blog`. Configure RSS, social meta tags, author profiles, reading time estimates, and a content calendar system.

**Estimated Effort:** 2 engineering-days

**Dependencies:** DOC-001

**Acceptance Criteria:**
- Blog supports MDX with custom components (code blocks, callouts, images, embeds)
- Author profiles with photo, bio, social links (displays on each post)
- Reading time estimate on each post
- RSS feed at `/blog/rss.xml`
- OpenGraph and Twitter Card meta tags (custom image per post)
- Category tags: Engineering, Community, Tutorial, Case Study, Benchmark, Announcement
- Pagination (10 posts per page)
- Newsletter signup form (integrate with Buttondown or Resend)
- Draft preview: authors can preview unpublished posts via PR preview deployment
- Canonical URL support for cross-posted content (Dev.to, Hashnode)

**Best Practices:**
- Write in a conversational, technical tone (not corporate marketing)
- Every post should teach something — even announcements should include technical depth
- Include code examples that readers can try themselves
- Target 5-10 minute read times (1200-2400 words)
- Optimize for sharing: strong titles, compelling first paragraphs, shareable images

---

#### CTN-002: Month 1 Blog Posts

**Description:**
Write and publish the two foundational blog posts that establish ValiForge's narrative and technical credibility.

**Estimated Effort:** 5 engineering-days

**Dependencies:** CTN-001

**Acceptance Criteria:**

**Post 1: "Why We Built ValiForge in Rust"**
- Covers: performance requirements, safety guarantees, ecosystem (crates), cross-compilation, Wasm future
- Includes benchmarks: ValiForge vs. Node.js-based tools (startup time, throughput, memory usage)
- Tone: honest about tradeoffs (Rust's learning curve, smaller contributor pool)
- 1500-2000 words, includes at least 3 code snippets and 1 benchmark chart
- Published on blog + cross-posted to Dev.to and r/rust

**Post 2: "API Testing Is Broken: Here's How We Fix It"**
- Covers: problems with current API testing (manual, brittle, slow, disconnected from schemas)
- Introduces ValiForge's approach: schema-first, automated, shift-left, AI-native
- Includes a comparison workflow: before (Postman manual testing) vs. after (ValiForge CI validation)
- 1500-2000 words, includes workflow diagrams and terminal output screenshots
- Published on blog + cross-posted to Hashnode and r/programming

**Best Practices:**
- Do not bash competitors — acknowledge what they do well, explain where ValiForge differs
- Use real benchmarks with reproducible methodology (link to benchmark repo)
- Include a "Try it yourself" section with install command
- End with a call-to-action: GitHub star, Discord join, or newsletter signup

---

#### CTN-003: Month 2 Blog Posts

**Description:**
Write posts that deepen technical credibility and begin competitive positioning.

**Estimated Effort:** 5 engineering-days

**Dependencies:** CTN-001

**Acceptance Criteria:**

**Post 3: "How Small Language Models Generate Smarter Test Data"**
- Covers: ValiForge's SLM integration for context-aware test data generation
- Technical depth: model selection, prompt engineering, validation of generated data, privacy considerations
- Includes examples: generated test data for a payment API, healthcare API, etc.
- Discusses edge cases: adversarial inputs, boundary values, format-specific data (SSN, email, etc.)
- 2000-2500 words, code examples, architecture diagram

**Post 4: "ValiForge vs. Postman: An Honest Comparison"**
- Feature-by-feature comparison table
- Acknowledges Postman's strengths: UI, collections, team collaboration, market share
- Highlights ValiForge's strengths: speed, CI-native, schema-first, open-source, AI-native
- "When to use Postman" vs. "When to use ValiForge" decision framework
- Migration guide teaser (links to full migration doc)
- 2000-2500 words, comparison table, decision flowchart

**Best Practices:**
- The comparison post must be factually accurate — errors here destroy credibility permanently
- Have someone from outside the team review the comparison for fairness
- Update the comparison when Postman ships new features (or link to latest docs)
- Include "Updated: [date]" at the top for evergreen content

---

#### CTN-004: Month 3 Blog Posts

**Description:**
Write posts targeting practical adoption and the AI-generated code use case.

**Estimated Effort:** 5 engineering-days

**Dependencies:** CTN-001

**Acceptance Criteria:**

**Post 5: "Zero to API Validation in 90 Seconds"**
- Speed-run tutorial with animated terminal recording (asciinema/vhs)
- Shows: install → init → validate → fix → CI setup in 90 seconds of real time
- Embeds the terminal recording directly in the blog post
- Includes "Pause and try it" checkpoints
- 1000-1500 words (short and punchy)

**Post 6: "Shift-Left Testing for AI-Generated Code"**
- Addresses the specific problem: Copilot/Cursor generates API endpoints, but who validates the schema?
- Demonstrates ValiForge as the "linter for AI-generated APIs"
- Shows integration with popular AI coding tools
- Discusses: trust but verify, guardrails for AI-generated code, schema drift detection
- 2000-2500 words, workflow diagrams, code examples

**Best Practices:**
- Post 5 should be the most shareable post — optimized for Twitter/X and Reddit
- Post 6 positions ValiForge for the AI-native developer audience — this is a differentiator
- Include relevant SEO keywords: "API validation", "AI code quality", "shift-left testing"

---

#### CTN-005: Month 4 Blog Posts

**Description:**
Deep technical content for the Rust and systems engineering audience.

**Estimated Effort:** 5 engineering-days

**Dependencies:** CTN-001

**Acceptance Criteria:**

**Post 7: "Inside ValiForge's Architecture: Traits, Not Plugins"**
- Deep dive into the trait-based extension system
- Explains why traits are better than dynamic plugins for this use case
- Includes Rust code examples showing how to implement a custom rule
- Covers: compilation guarantees, zero-cost abstractions, composability
- Targeted at r/rust audience — assumes Rust knowledge
- 2500-3000 words, architecture diagrams, Rust code examples

**Post 8: "Benchmarking ValiForge: How We Validate 10,000 Endpoints in 200ms"**
- Detailed benchmark methodology (hardware, methodology, statistical significance)
- Comparison against: Spectral, openapi-diff, protolint, graphql-inspector
- Flamegraph analysis showing where time is spent
- Optimization story: what we tried, what worked, what didn't
- Reproducible: link to benchmark harness repo
- 2000-2500 words, charts, flamegraphs, benchmark tables

**Best Practices:**
- Architecture post should be a reference that Rust learners can study
- Benchmark post must be reproducible — unfair benchmarks are worse than no benchmarks
- Include caveats: "benchmarks are synthetic, your mileage may vary"
- Publish benchmark harness as open source for community verification

---

#### CTN-006: Month 5 — Case Study

**Description:**
Produce the first customer/user case study demonstrating real-world value.

**Estimated Effort:** 5 engineering-days

**Dependencies:** Requires at least one production user

**Acceptance Criteria:**

**Post 9: "How [Company X] Saved 40% on CI Costs with ValiForge"**
- Real metrics from a real user (with their permission and review)
- Structure: Challenge → Solution → Results → Lessons Learned
- Specific numbers: CI time reduction, bugs caught, developer satisfaction
- Quotes from the engineering team
- Architecture diagram showing ValiForge in their pipeline
- 1500-2000 words
- If no enterprise user available, use an open-source project as the case study

**Best Practices:**
- Get written approval from the company before publishing
- Let them review the draft before publication
- Focus on the developer experience, not just the metrics
- Include a "How to replicate this" section for readers

---

#### CTN-007: Month 6 — Launch Week (v1.0)

**Description:**
Plan and execute a 5-day "Launch Week" for the v1.0 release, modeled after Supabase Launch Week and Vercel Ship. Each day features a blog post, a feature announcement, and social media content.

**Estimated Effort:** 15 engineering-days (writing + coordination + design)

**Dependencies:** All prior content, all P0 tasks

**Acceptance Criteria:**
- **Day 1 (Monday):** "ValiForge v1.0: The Headless API Validation Engine" — overview post + announcement
- **Day 2 (Tuesday):** "ValiForge Cloud: Team Validation Dashboards" — cloud product announcement
- **Day 3 (Wednesday):** "New: AI-Powered Test Data Generation" — SLM feature deep dive
- **Day 4 (Thursday):** "ValiForge for GraphQL & Protobuf" — multi-schema support announcement
- **Day 5 (Friday):** "The Road to v2: Community Roadmap" — roadmap + contributor call-to-action
- Each post published at 9 AM ET with coordinated social media
- Landing page: `valiforge.dev/launch-week` with day-by-day schedule and countdown timer
- Daily Twitter thread, Reddit post, and Discord announcement
- Launch Week recap video (5-minute summary)
- Swag giveaway: stickers, t-shirts for top contributors and early adopters

**Best Practices:**
- Pre-write all 5 posts and have them reviewed before day 1
- Schedule social media posts using Buffer or Typefully
- Have someone monitoring Hacker News, Reddit, and Twitter for mentions throughout the week
- Create a shared war room (Discord channel or Slack) for real-time coordination
- Post-launch retrospective: what worked, what to improve for next launch week

---

#### CTN-008: Video Content Production

**Description:**
Produce video content for YouTube and social media: demo videos, tutorial series, architecture deep dives, and conference talk recordings.

**Estimated Effort:** 10 engineering-days

**Dependencies:** DX-004 (CLI must work for demos)

**Acceptance Criteria:**
- **"ValiForge in 3 Minutes" demo video:** Quick, polished demo showing install → validate → CI setup
  - Professional quality: good audio, screen recording, subtle background music
  - Captions/subtitles for accessibility
  - Published on YouTube with SEO-optimized title, description, tags
  - Short versions (60s, 30s) for Twitter and LinkedIn
- **"API Validation 101" tutorial series (4 episodes):**
  - Episode 1: What is API validation and why it matters
  - Episode 2: Setting up ValiForge (install, init, first validation)
  - Episode 3: Custom rules and configuration
  - Episode 4: CI/CD integration and breaking change detection
  - Each episode: 8-12 minutes, screen recording + talking head
- **Architecture deep dive:**
  - 20-minute video explaining ValiForge internals for Rust enthusiasts
  - Covers: trait system, parser pipeline, rule engine, error rendering
  - Whiteboard-style diagrams + code walkthrough
- All videos have: thumbnails, descriptions, timestamps/chapters, end screens, cards

**Best Practices:**
- Invest in audio quality — bad audio is the #1 reason people click away
- Use `asciinema` or `vhs` for terminal recordings (consistent, reproducible)
- Publish transcripts for accessibility and SEO
- Create a YouTube playlist for organized viewing
- Cross-promote videos in docs (embed relevant videos in guide pages)

---

#### CTN-009: Social Media Strategy & Execution

**Description:**
Establish ValiForge's social media presence with a consistent posting cadence, voice, and content strategy across Twitter/X, Reddit, Dev.to, Hashnode, and LinkedIn.

**Estimated Effort:** 3 engineering-days (setup + first month content calendar)

**Dependencies:** None

**Acceptance Criteria:**
- **Twitter/X (@valiforge):**
  - Daily posting during launch month, 3-5x/week ongoing
  - Content mix: 40% technical (CLI GIFs, benchmarks), 30% community (shoutouts, contributor highlights), 20% engagement (polls, questions), 10% announcements
  - Thread format for major announcements (5-7 tweets with visuals)
  - Engage with mentions within 4 hours during business hours
- **Reddit:**
  - Weekly posts on r/rust, r/devops, r/programming (only when genuinely valuable content)
  - Follow each subreddit's self-promotion rules strictly
  - Engage in comments on related posts (without spamming)
- **Dev.to / Hashnode:**
  - Cross-post blog content 2x/month with canonical URL pointing to valiforge blog
  - Optimize titles and tags for each platform's discovery algorithm
- **LinkedIn:**
  - Weekly posts targeting engineering managers and DevOps leads
  - Tone: more business-oriented (CI cost savings, team productivity, compliance)
  - Share case studies and benchmark results
- Content calendar template (monthly) with themes and key dates
- Social media guidelines document (voice, tone, do's and don'ts)

**Best Practices:**
- Authenticity over polish: developer audiences detect and reject corporate social media
- Show the building process: share WIP, design decisions, bugs (vulnerability creates connection)
- Engage genuinely in conversations — don't just broadcast
- Use Typefully or Buffer for scheduling, but respond to replies manually
- Track metrics weekly: impressions, engagement rate, link clicks, follower growth

---

### Content Marketing — Summary Table

| Task ID | Title | Effort (days) | Priority | Dependencies |
|---------|-------|---------------|----------|--------------|
| CTN-001 | Blog Platform Setup | 2 | P0 | DOC-001 |
| CTN-002 | Month 1 Blog Posts | 5 | P0 | CTN-001 |
| CTN-003 | Month 2 Blog Posts | 5 | P1 | CTN-001 |
| CTN-004 | Month 3 Blog Posts | 5 | P1 | CTN-001 |
| CTN-005 | Month 4 Blog Posts | 5 | P1 | CTN-001 |
| CTN-006 | Month 5 Case Study | 5 | P1 | Production user |
| CTN-007 | Month 6 Launch Week | 15 | P0 | All P0 tasks |
| CTN-008 | Video Content Production | 10 | P1 | DX-004 |
| CTN-009 | Social Media Strategy | 3 | P0 | None |
| **Total** | | **55** | | |

---

## 5. LAUNCH PLAYBOOK

---

#### LCH-001: Private Beta Program (Weeks -6 to -2)

**Description:**
Recruit and manage a private beta of 20-50 design partners who will test ValiForge before public launch. These beta users provide critical feedback, testimonials, and initial community seeding.

**Estimated Effort:** 8 engineering-days

**Dependencies:** DX-001 through DX-005 (core CLI must work)

**Acceptance Criteria:**
- 20-50 beta users recruited from: personal network, Twitter DMs, relevant Discord communities, Reddit
- Target profile: senior engineers at companies with > 10 API endpoints, active open-source contributors
- Beta onboarding: personal 15-minute call with each beta user to set up ValiForge and gather context
- Private Discord channel for beta users (separate from public community)
- Weekly check-in survey: What worked? What's broken? What's missing?
- Bug tracking: all beta-reported bugs triaged within 24 hours
- Feedback synthesis: weekly summary of themes and trends shared with engineering team
- By beta end: collect 5+ written testimonials (with permission to publish)
- By beta end: collect 3+ "before/after" stories for case study material
- Beta users invited to provide GitHub stars on launch day (genuine ask, not coerced)
- Beta graduation: all critical bugs reported by beta users are fixed before public launch
- NDA not required (open-source), but beta users agree to a feedback commitment

**Best Practices:**
- Recruit diverse beta users: different company sizes, industries, API technologies, experience levels
- Personally respond to every piece of feedback (builds loyalty and advocacy)
- Beta users who give the most feedback should be offered Champion status
- Use a shared Notion/Linear board for beta feedback so the team can see all themes
- End beta with a "Thank You" post crediting all beta participants

---

#### LCH-002: Pre-Launch Asset Preparation

**Description:**
Create all marketing and community assets needed for launch day. Everything should be ready, reviewed, and scheduled before launch so the team can focus on engagement.

**Estimated Effort:** 5 engineering-days

**Dependencies:** CTN-008 (demo video), COM-001 (repo polish)

**Acceptance Criteria:**
- **Show HN post:** draft written, reviewed by 3 people, scheduled for 9 AM ET Tuesday
  - Title format: "Show HN: ValiForge – Open-Source API Validation Engine in Rust"
  - Post body: 3 paragraphs (problem, solution, ask for feedback), link to GitHub and docs
- **Product Hunt listing:** title, tagline, description, screenshots, maker profile, categories
  - 5+ screenshots showing: CLI demo, VS Code extension, CI output, docs site, error messages
  - First comment from maker explaining the "why" story
  - 10+ beta users ready to leave genuine reviews on launch day
- **Twitter thread:** 7-tweet thread with: hook, problem, solution, demo GIF, benchmarks, CTA
- **Reddit posts:** drafts for r/rust, r/devops, r/programming (tailored to each subreddit's culture)
- **Email to beta users:** personalized email thanking them and asking for GitHub star + testimonial
- **Press kit:** logo (SVG, PNG), brand colors, one-pager, founder bios, screenshots
- **Social media image assets:** Open Graph images, Twitter cards, banner images (all sizes)
- **Discord announcement:** ready to post, includes quick start link and feedback request

**Best Practices:**
- Time HN post for Tuesday 9 AM ET (highest engagement, avoids Monday rush)
- Product Hunt launch should be coordinated with a "maker" who is the face of the launch
- Reddit posts should be genuinely helpful, not promotional — share insights, link to repo casually
- Have all assets reviewed by someone who has NOT been working on the project (fresh eyes)
- Prepare a "launch day FAQ" document for the team to handle common questions

---

#### LCH-003: Launch Day Execution

**Description:**
Execute the launch plan with a coordinated, time-sequenced rollout across all channels. Have the entire team aligned and ready to engage.

**Estimated Effort:** 3 engineering-days (all hands on deck)

**Dependencies:** LCH-001, LCH-002

**Acceptance Criteria:**
- **9:00 AM ET:** Show HN post published, team immediately engages with comments
- **9:15 AM ET:** Product Hunt listing goes live, beta users notified to visit and review
- **9:30 AM ET:** Twitter thread posted from @valiforge account
- **10:00 AM ET:** Reddit posts published (r/rust, r/devops — one team member per subreddit)
- **10:30 AM ET:** Email blast to beta users and newsletter subscribers
- **11:00 AM ET:** Discord announcement posted
- **All day:** Core team monitors all channels, responds to every comment/question within 30 minutes
- **All day:** Engineering on standby for critical bugs (deploy hotfixes within 2 hours)
- **All day:** Track metrics: GitHub stars, npm installs, Docker pulls, docs page views, Discord joins
- **6:00 PM ET:** Internal debrief — what's working, what needs response, plan for day 2
- **End of day:** Metrics snapshot shared on Discord #announcements
- Response SLA: HN comments < 30 min, GitHub issues < 2 hours, Discord < 15 min

**Best Practices:**
- Do NOT ask people to upvote on HN (it violates HN rules and will get you flagged)
- Genuinely engage with criticism — every negative comment is a product insight
- If something breaks, post a transparent update rather than going silent
- Celebrate publicly: share star count milestones on Twitter as they happen
- Designate one person as the "traffic controller" who routes questions to the right team member

---

#### LCH-004: Post-Launch Follow-Through (Weeks +1 to +4)

**Description:**
The two weeks after launch are more important than launch day. Execute a rapid-response plan to maintain momentum, fix issues, and convert interest into sustained adoption.

**Estimated Effort:** 10 engineering-days

**Dependencies:** LCH-003

**Acceptance Criteria:**
- **Week 1:**
  - Every GitHub issue responded to within 24 hours (even if just "thanks, we're looking into this")
  - Every HN comment responded to (even late ones — HN threads stay active for days)
  - Ship 1-2 quick-fix releases addressing top-reported issues
  - Blog post: "What We Learned from Launch Day" (metrics, feedback themes, next steps)
  - Send personal thank-you messages to top 10 community engagers
- **Week 2:**
  - "Postman Migration Guide" blog post + campaign (email + social)
  - "Getting Started" tutorial video published on YouTube
  - First "Good First Issue" batch published (capitalizing on contributor interest)
  - Community call: "Ask Us Anything" session for new users
- **Week 3-4:**
  - Analyze retention: who installed on launch day and is still using ValiForge?
  - Survey active users: what made them stay? what almost made them leave?
  - Iterate on onboarding: update Quick Start based on common stumbling points
  - v0.2 release with top-requested features (show responsiveness)
- Metrics tracked: daily installs, GitHub stars (cumulative), issues opened/closed ratio, Discord growth

**Best Practices:**
- Momentum fades fast — have content and releases pre-queued for weeks 2-4
- Convert launch traffic into email subscribers (they can be re-engaged later)
- Speed of response matters more than quality of response in week 1 (acknowledge fast, fix fast)
- Share a public roadmap informed by launch feedback (shows you listen)
- Do a "launch retrospective" and document learnings for the next launch

---

### Launch Playbook — Summary Table

| Task ID | Title | Effort (days) | Priority | Dependencies |
|---------|-------|---------------|----------|--------------|
| LCH-001 | Private Beta Program | 8 | P0 | DX-001 through DX-005 |
| LCH-002 | Pre-Launch Asset Prep | 5 | P0 | CTN-008, COM-001 |
| LCH-003 | Launch Day Execution | 3 | P0 | LCH-001, LCH-002 |
| LCH-004 | Post-Launch Follow-Through | 10 | P0 | LCH-003 |
| **Total** | | **26** | | |

---

## 6. CONFERENCE & SPEAKING STRATEGY

---

#### SPK-001: CFP Calendar & Submission Pipeline

**Description:**
Create a comprehensive calendar of target conferences with CFP deadlines, and establish a pipeline for writing and submitting talk proposals. Aim for 6-8 submissions per quarter with a 20-30% acceptance rate target.

**Estimated Effort:** 5 engineering-days (initial + first quarter submissions)

**Dependencies:** None (can start before product is ready)

**Acceptance Criteria:**
- Conference target list with CFP deadlines, dates, locations, and audience profiles:
  - **Rust:** RustConf, RustNation, EuroRust, Oxidize
  - **DevOps/Platform:** KubeCon, DevOpsDays (multiple cities), PlatformCon
  - **API/Testing:** API World, TestBash, APIdays, API Specification Conference
  - **General Dev:** QCon, DeveloperWeek, FOSDEM, Open Source Summit, Strange Loop
  - **Regional:** local meetups in SF, NYC, London, Berlin, Bangalore
- 3 polished talk abstracts ready to submit:
  - "Shift-Left API Testing in the AI Era" (20 min, intermediate, any dev conference)
  - "Why Rust Is Eating Developer Tools" (30 min, advanced, Rust conferences)
  - "Headless Validation: Beyond the GUI" (20 min, intermediate, API/testing conferences)
- Each abstract includes: title, description (200 words), outline, speaker bio, talk format preferences
- Submission tracker (spreadsheet/Notion) tracking: conference, deadline, submission date, status
- Speaker training: for team members who haven't spoken at conferences, provide coaching resources
- Talk slide templates matching ValiForge brand
- Speaker kit: bio, headshot, social links, company description

**Best Practices:**
- Submit to 3x more conferences than you expect to speak at (acceptance rate is ~25%)
- Tailor each abstract to the specific conference's audience and themes
- Include a 2-minute video with the CFP submission (many conferences require this)
- Start with local meetups to practice before major conferences
- Record every talk and publish on YouTube (extends reach 10-100x beyond the room)

---

#### SPK-002: Conference Talk Development

**Description:**
Develop polished, rehearsed conference talks with supporting materials. Each talk should be a standalone piece of marketing: entertaining, educational, and memorable.

**Estimated Effort:** 10 engineering-days (for 3 talks)

**Dependencies:** SPK-001

**Acceptance Criteria:**
- 3 fully developed talks with:
  - Slide deck (clean design, code-heavy slides use large fonts, demo sections marked)
  - Speaker notes for each slide
  - Live demo script with backup plan (pre-recorded demo video if live demo fails)
  - 2+ practice runs recorded and reviewed
  - Post-talk resources page: `valiforge.dev/talks/<talk-slug>` with slides, links, and resources
- Each talk includes:
  - Hook in first 60 seconds (a surprising fact, a demo, or a relatable pain point)
  - At least one live demo section (not just slides)
  - Clear takeaway that the audience can act on ("try `valiforge validate` on your project today")
  - QR code on last slide linking to GitHub repo
- Talks rehearsed with timing (within 2 minutes of target length)
- Feedback collected from practice audience and incorporated

**Best Practices:**
- The live demo is the most memorable part — invest heavily in making it flawless
- Have a pre-recorded backup of every demo (tech failures happen at every conference)
- End with a clear CTA, not "any questions?" (e.g., "Star us on GitHub, join our Discord")
- Share slides publicly after every talk (SlideShare, Speaker Deck, or docs site)
- Follow up with attendees who tweet about the talk — connect and engage

---

#### SPK-003: Workshop Development

**Description:**
Create a hands-on workshop format: "Build an API Validation Pipeline in 30 Minutes." This workshop should work for conference workshops (90 min), meetup sessions (60 min), and self-paced online formats.

**Estimated Effort:** 8 engineering-days

**Dependencies:** DX-001 through DX-004, DOC-002

**Acceptance Criteria:**
- Workshop materials:
  - Facilitator guide (timing, setup requirements, troubleshooting)
  - Participant handout (step-by-step instructions, NOT slides-only)
  - Pre-built sample project (GitHub repo) that participants clone
  - Solution branch for each exercise (so stuck participants can catch up)
  - Feedback survey (Google Form or similar)
- Workshop structure:
  - Intro & setup (5 min): install ValiForge, clone sample project
  - Exercise 1 (10 min): `valiforge init` and first validation
  - Exercise 2 (10 min): fix validation errors, understand error messages
  - Exercise 3 (10 min): write a custom validation rule
  - Exercise 4 (10 min): set up CI with GitHub Actions
  - Wrap-up & next steps (5 min)
- Works on macOS, Linux, and Windows (WSL) — tested pre-workshop
- Self-paced online version published at `docs.valiforge.dev/workshop`
- Workshop delivered 2x (meetups or conference) and iterated on feedback

**Best Practices:**
- Over-prepare: have solutions ready for every possible failure point
- Assume 20% of participants will have setup issues — budget time for this
- Pair struggling participants with those who finish quickly
- Give participants something to take away (GitHub repo, cheat sheet, sticker)
- Record the workshop and publish as a YouTube tutorial

---

#### SPK-004: Hackathon Sponsorship & Participation

**Description:**
Sponsor and participate in developer hackathons to introduce ValiForge to new developers. Focus on hackathons that attract the target audience: API developers, DevOps engineers, and AI/ML developers.

**Estimated Effort:** 5 engineering-days (per hackathon, budget for 2-3)

**Dependencies:** DX-001 through DX-004

**Acceptance Criteria:**
- Target 2-3 hackathons in the first 6 months
- Target hackathons: DeveloperWeek Hackathon, MLH events, API-focused hackathons
- Sponsorship package: "API Validation" challenge track with prizes
  - Prize: ValiForge swag + ValiForge Cloud credits + feature on blog
  - Judging criteria: creativity, API design quality, ValiForge integration
- Hackathon booth materials: banner, stickers, one-pager, QR code to quick start
- Workshop at hackathon: 30-minute "Getting Started with ValiForge" session
- Mentors on-site to help teams integrate ValiForge
- Post-hackathon blog post showcasing winning projects
- Follow-up with hackathon participants: invite to Discord, newsletter

**Best Practices:**
- Choose hackathons where ValiForge naturally fits the problem space
- Make integration as easy as possible: pre-built templates for common hackathon stacks
- Be genuinely helpful as mentors, not just promotional
- Collect emails (opt-in) for follow-up nurturing
- Feature winning projects in "Showcase" on the docs site

---

### Conference & Speaking — Summary Table

| Task ID | Title | Effort (days) | Priority | Dependencies |
|---------|-------|---------------|----------|--------------|
| SPK-001 | CFP Calendar & Submissions | 5 | P1 | None |
| SPK-002 | Conference Talk Development | 10 | P1 | SPK-001 |
| SPK-003 | Workshop Development | 8 | P2 | DX-001 through DX-004 |
| SPK-004 | Hackathon Sponsorship | 5 | P2 | DX-001 through DX-004 |
| **Total** | | **28** | | |

---

## 7. METRICS & MEASUREMENT

---

#### MET-001: DX Metrics Infrastructure

**Description:**
Set up instrumentation and dashboards to measure developer experience quality. Track the full funnel from first touch to successful validation, identifying where developers drop off or get stuck.

**Estimated Effort:** 5 engineering-days

**Dependencies:** DX-001, DX-004, DOC-001

**Acceptance Criteria:**
- **Time-to-First-Validation (TTFV):** measured from install to first successful `valiforge validate`
  - Target: < 2 minutes for auto-detected schemas, < 5 minutes with manual config
  - Measured via opt-in anonymous telemetry (not tracking by default)
- **Docs metrics:** page views, time on page, bounce rate, search queries (via Plausible or Fathom)
  - Search query analysis: identify terms with no results (indicates missing content)
  - Page exit analysis: identify pages where users leave the docs (indicates confusion)
- **CLI telemetry (opt-in only):**
  - Command usage frequency (which commands are most/least used)
  - Error frequency by error code (which errors frustrate users most)
  - Schema types validated (market intelligence)
  - Anonymized, aggregated, open methodology document published
- **Dashboard:** Grafana or Metabase dashboard with real-time metrics
  - Daily/weekly/monthly active CLI users
  - TTFV distribution (p50, p90, p99)
  - Top errors by frequency
  - Docs search queries without results
- Privacy policy published at `valiforge.dev/privacy`
- Telemetry opt-in flow: first run asks, respects `DO_NOT_TRACK` env var, can be disabled anytime

**Best Practices:**
- Default to NO telemetry — make opt-in the first-class path
- Publish the exact data schema collected (full transparency)
- Never collect: file names, file contents, IP addresses, user identifiers
- Use Plausible or Fathom for docs analytics (privacy-friendly, no cookies)
- Review metrics weekly as a team — create action items from each review

---

#### MET-002: Community Metrics Dashboard

**Description:**
Set up automated tracking of community health metrics across GitHub, Discord, and social media. Create a dashboard that the team reviews weekly and shares publicly quarterly.

**Estimated Effort:** 3 engineering-days

**Dependencies:** COM-001, COM-003

**Acceptance Criteria:**
- **GitHub metrics (automated):**
  - Stars (daily delta and cumulative)
  - Forks (daily delta and cumulative)
  - Unique contributors per month (new vs. returning)
  - Issues opened vs. closed (ratio and trend)
  - PR merge time (p50, p90)
  - Time-to-first-response on issues
- **Discord metrics:**
  - Total members (daily delta and cumulative)
  - Daily active members
  - Messages per day
  - #help channel response time
  - New member retention (% still active after 30 days)
- **Social media metrics:**
  - Twitter: followers, impressions, engagement rate
  - Reddit: post karma, comment engagement
  - YouTube: subscribers, views, watch time
- Dashboard tool: Grafana, Orbit, or custom Astro page
- Weekly review ritual: 15-minute team check-in on metrics every Monday
- Quarterly public report: "State of the ValiForge Community" blog post with charts

**Best Practices:**
- Focus on trends, not absolute numbers (growth rate matters more than total count)
- Set targets for each metric and track progress against targets
- Use Orbit or Common Room for community intelligence (identifies key community members)
- Compare against benchmarks: similar-stage open-source projects (Biome, Turbopack, etc.)
- Automate data collection — manual tracking will not be maintained

---

#### MET-003: Conversion Funnel Tracking

**Description:**
Set up end-to-end funnel tracking from first touch through cloud conversion. This data drives business decisions about where to invest in the funnel.

**Estimated Effort:** 4 engineering-days

**Dependencies:** DX-001, MET-001

**Acceptance Criteria:**
- Funnel stages defined and tracked:
  1. **Awareness:** docs page view, blog view, social media impression
  2. **Interest:** GitHub star, Discord join, newsletter signup
  3. **Activation:** CLI install (from install script analytics)
  4. **Engagement:** first `valiforge validate` run (opt-in telemetry)
  5. **Retention:** 7-day active (ran valiforge at least twice in 7 days)
  6. **Revenue:** cloud signup, paid conversion (when cloud launches)
- Conversion rates between each stage tracked weekly
- Cohort analysis: weekly cohorts tracked through the funnel over 90 days
- Attribution: track which channel (HN, Reddit, Twitter, organic search) drives each funnel stage
- Dashboard showing funnel with conversion rates and week-over-week trends
- Alerts set up for significant drops in any funnel stage (> 20% WoW decline)
- Monthly funnel analysis shared with the team with action items

**Best Practices:**
- Do not over-optimize for vanity metrics (stars) — focus on activation and retention
- Use UTM parameters on all links for attribution tracking
- Define "activation" precisely (first successful validation, not just first command run)
- Compare funnel against industry benchmarks (developer tools typically see 5-15% star-to-install)
- Test funnel improvements with A/B tests (e.g., different Quick Start page versions)

---

#### MET-004: Content Performance Measurement

**Description:**
Set up measurement for all content (blog, video, social) to understand what resonates with the audience and optimize future content production.

**Estimated Effort:** 2 engineering-days

**Dependencies:** CTN-001, CTN-008, CTN-009

**Acceptance Criteria:**
- **Blog metrics per post:** page views, time on page, scroll depth, social shares, referral source
- **Video metrics per video:** views, watch time, retention curve, click-through rate, subscriber conversion
- **Social media metrics per post:** impressions, engagement rate, link clicks, follower delta
- **HN/Reddit metrics:** rank achieved, points, comment count, referral traffic
- Content performance dashboard updated weekly
- Top-performing content identified monthly → produce more content in that format/topic
- Underperforming content analyzed → why didn't it work? What to change?
- ROI calculation: engineer-days invested per content piece → traffic/engagement generated
- A/B test titles and thumbnails for key content (use Twitter to test before publishing)

**Best Practices:**
- Track "content half-life" — how long does a blog post continue generating traffic?
- Evergreen content (guides, comparisons) is more valuable than news content
- Republish and update top-performing content rather than always creating new content
- Use Google Search Console to identify organic search opportunities
- Content that generates GitHub stars or Discord joins is the highest-value content

---

### Metrics & Measurement — Summary Table

| Task ID | Title | Effort (days) | Priority | Dependencies |
|---------|-------|---------------|----------|--------------|
| MET-001 | DX Metrics Infrastructure | 5 | P1 | DX-001, DX-004, DOC-001 |
| MET-002 | Community Metrics Dashboard | 3 | P1 | COM-001, COM-003 |
| MET-003 | Conversion Funnel Tracking | 4 | P1 | DX-001, MET-001 |
| MET-004 | Content Performance | 2 | P2 | CTN-001 |
| **Total** | | **14** | | |

---

## EFFORT SUMMARY

| Workstream | Tasks | Total Effort (days) | P0 (days) | P1 (days) | P2 (days) |
|------------|-------|--------------------:|----------:|----------:|----------:|
| 1. DX Design | 12 | 83 | 39 | 30 | 14 |
| 2. Documentation | 10 | 43 | 32 | 8 | 3 |
| 3. Community Building | 7 | 19 | 10 | 4 | 5 |
| 4. Content Marketing | 9 | 55 | 22 | 25 | 8 |
| 5. Launch Playbook | 4 | 26 | 26 | 0 | 0 |
| 6. Conference & Speaking | 4 | 28 | 0 | 15 | 13 |
| 7. Metrics & Measurement | 4 | 14 | 0 | 12 | 2 |
| **GRAND TOTAL** | **50** | **268** | **129** | **94** | **45** |

---

## CRITICAL PATH

The critical path determines the minimum time to launch. Tasks on this path cannot be delayed without delaying the entire launch.

```
DX-001 (Install, 5d)
  └─→ DX-002 (Init, 8d)
       └─→ DX-004 (Validate, 12d)  ← requires DX-003 (Auto-detect, 6d) in parallel
            └─→ DX-005 (Error Messages, 8d)
                 └─→ DX-012 (Breaking Change, 10d)
                      └─→ DOC-004 (How-To Guides, 10d)
                           └─→ LCH-001 (Beta Program, 8d)
                                └─→ LCH-002 (Pre-Launch Assets, 5d)
                                     └─→ LCH-003 (Launch Day, 3d)

Critical Path Total: 69 engineering-days (~14 weeks)
```

**Parallel tracks (off critical path):**
- Documentation setup (DOC-001) can start immediately, in parallel with DX-001
- Community setup (COM-001, COM-003) can start immediately, in parallel with everything
- Content creation (CTN-001 through CTN-009) starts after DOC-001 and can run continuously
- Conference strategy (SPK-001) can start immediately
- Metrics (MET-001 through MET-004) start after core DX is built

---

## SPRINT PLAN (2-Week Sprints)

### Sprint 1 (Weeks 1-2): Foundation
| Task | Effort | Assignee Focus |
|------|--------|----------------|
| DX-001: Install Script | 5d | DX Engineer |
| DX-003: Auto-Detection Engine | 6d | DX Engineer |
| DOC-001: Docs Framework Setup | 4d | DX/Docs Engineer |
| COM-001: GitHub Repo Setup | 5d | DevRel |
| COM-003: Discord Setup | 3d | DevRel |
| CTN-009: Social Media Setup | 3d | DevRel |
| **Sprint total:** | **26d** | **Team of 3-4** |

### Sprint 2 (Weeks 3-4): Core CLI
| Task | Effort | Assignee Focus |
|------|--------|----------------|
| DX-002: `valiforge init` | 8d | DX Engineer |
| DX-004: `valiforge validate` (start) | 12d (partial) | DX Engineer |
| DOC-002: Quick Start Guide (draft) | 3d | Docs Engineer |
| DOC-009: Contributing Guide | 3d | DevRel |
| COM-002: Labels & Triage | 2d | DevRel |
| CTN-001: Blog Platform | 2d | DevRel |
| **Sprint total:** | **24d** | **Team of 3-4** |

### Sprint 3 (Weeks 5-6): Polish & Errors
| Task | Effort | Assignee Focus |
|------|--------|----------------|
| DX-004: `valiforge validate` (complete) | remaining | DX Engineer |
| DX-005: Error Messages | 8d | DX Engineer |
| DX-006: CLI Help Text | 4d | DX Engineer |
| DOC-003: Concepts & Architecture | 5d | Docs Engineer |
| CTN-002: Month 1 Blog Posts | 5d | DevRel |
| COM-005: Good First Issues | 3d | DevRel |
| **Sprint total:** | **25d** | **Team of 3-4** |

### Sprint 4 (Weeks 7-8): Advanced Features & Docs
| Task | Effort | Assignee Focus |
|------|--------|----------------|
| DX-012: Breaking Change Detection | 10d | DX Engineer |
| DX-007: Shell Completions | 3d | DX Engineer |
| DX-008: `valiforge doctor` | 4d | DX Engineer |
| DOC-004: How-To Guides (start) | 10d (partial) | Docs Engineer |
| DOC-005: CLI Reference | 4d | Docs Engineer |
| CTN-003: Month 2 Blog Posts | 5d | DevRel |
| **Sprint total:** | **26d** | **Team of 3-4** |

### Sprint 5 (Weeks 9-10): Editor Integration & Beta Prep
| Task | Effort | Assignee Focus |
|------|--------|----------------|
| DX-009: VS Code Extension (start) | 15d (partial) | DX Engineer |
| DX-011: Self-Update | 3d | DX Engineer |
| DOC-004: How-To Guides (complete) | remaining | Docs Engineer |
| DOC-006: Configuration Reference | 3d | Docs Engineer |
| LCH-001: Beta Program (start recruiting) | 8d (partial) | DevRel |
| CTN-008: Demo Video | 5d (partial) | DevRel |
| **Sprint total:** | **24d** | **Team of 3-4** |

### Sprint 6 (Weeks 11-12): Beta & Content
| Task | Effort | Assignee Focus |
|------|--------|----------------|
| DX-009: VS Code Extension (complete) | remaining | DX Engineer |
| LCH-001: Beta Program (run beta) | remaining | DevRel |
| DOC-007: SDK Reference | 5d | Docs Engineer |
| DOC-008: FAQ & Troubleshooting | 3d | Docs Engineer |
| CTN-004: Month 3 Blog Posts | 5d | DevRel |
| CTN-008: Tutorial Videos | 5d (partial) | DevRel |
| **Sprint total:** | **24d** | **Team of 3-4** |

### Sprint 7 (Weeks 13-14): Launch Prep
| Task | Effort | Assignee Focus |
|------|--------|----------------|
| LCH-002: Pre-Launch Assets | 5d | All hands |
| MET-001: DX Metrics | 5d | DX Engineer |
| MET-002: Community Metrics | 3d | DevRel |
| SPK-001: CFP Submissions | 5d | DevRel |
| Final polish, bug fixes, testing | 5d | All hands |
| **Sprint total:** | **23d** | **Team of 3-4** |

### Sprint 8 (Week 15): LAUNCH WEEK
| Task | Effort | Assignee Focus |
|------|--------|----------------|
| LCH-003: Launch Day Execution | 3d | All hands |
| LCH-004: Post-Launch (start) | 5d (partial) | All hands |
| **Sprint total:** | **8d** | **All hands on deck** |

---

## KEY RISKS & MITIGATIONS

### Risk 1: Time-to-First-Validation Exceeds 2-Minute Target
**Likelihood:** Medium | **Impact:** High
**Description:** The < 2 minute TTFV target is aggressive. Complex project structures, slow networks, or confusing auto-detection results could push this well past the target, causing first-run abandonment.
**Mitigations:**
- Measure TTFV with every beta user and optimize the worst cases
- Provide a "playground" mode: `valiforge demo` runs against a built-in sample schema (zero setup)
- Offline-first: pre-bundle common rulesets so no network required for first run
- Continuous monitoring: track TTFV in production and alert if p90 exceeds 3 minutes

### Risk 2: Hacker News Launch Falls Flat
**Likelihood:** Medium | **Impact:** Medium
**Description:** HN is unpredictable. The post might not gain traction due to timing, competition, or title choice, missing a major awareness opportunity.
**Mitigations:**
- Have a backup launch plan that does not depend on HN (Reddit, Twitter, Product Hunt)
- Test titles with the beta community before launch day
- If the first post doesn't gain traction, it is acceptable to post again weeks later with a different angle
- Build an email list during beta so launch is not entirely dependent on external platforms

### Risk 3: Contributor Onboarding Too Difficult
**Likelihood:** Medium | **Impact:** High
**Description:** Rust's learning curve plus a complex codebase could deter potential contributors, stunting community growth and creating an unsustainable maintenance burden.
**Mitigations:**
- Invest heavily in "Good First Issues" that do not require deep Rust knowledge (docs, tests, configs)
- Provide mentorship for first-time Rust contributors
- Create a `valiforge-contrib` playground crate for learning the codebase
- Consider accepting contributions in other languages for non-core components (docs tooling, CI scripts)

### Risk 4: VS Code Extension Quality Issues
**Likelihood:** Medium | **Impact:** Medium
**Description:** VS Code extensions have high quality expectations. A buggy or slow extension will generate negative reviews that are difficult to recover from.
**Mitigations:**
- Ship the extension as "Preview" initially to set expectations
- Extensive testing on macOS, Linux, and Windows before publishing
- Fast issue response time for extension bugs (< 24 hours)
- Telemetry (opt-in) specifically for extension crashes and performance

### Risk 5: Content Marketing Does Not Convert to Adoption
**Likelihood:** Medium | **Impact:** Medium
**Description:** Blog posts generate traffic but visitors do not install ValiForge or join the community, resulting in wasted effort.
**Mitigations:**
- Every blog post includes a clear CTA (install command, not just "check it out")
- Track content-to-install attribution rigorously (UTM parameters on every link)
- Focus on tutorial content (high conversion) over thought leadership (low conversion)
- A/B test CTAs to optimize conversion

### Risk 6: Solo Maintainer Burnout
**Likelihood:** High | **Impact:** Critical
**Description:** If the team is small (1-2 people), the scope of this plan risks burnout. Community management, content creation, bug fixes, and feature development compete for limited time.
**Mitigations:**
- Ruthlessly prioritize P0 tasks and defer P2 tasks until the community can help
- Automate everything possible (CI, bots, scheduled posts, metric collection)
- Set boundaries: designate "no community management" days for focused engineering
- Recruit champions early who can handle community support
- It is acceptable to ship less and ship well, rather than shipping everything poorly

---

## QUARTERLY OKRs

### Q1: Foundation (Months 1-3)
**Objective:** Ship a polished CLI with world-class DX and establish community presence.
- **KR1:** Time-to-First-Validation p50 < 2 minutes, p90 < 5 minutes
- **KR2:** 500 GitHub stars
- **KR3:** 100 Discord members
- **KR4:** 50 CLI installs per week (organic)
- **KR5:** 6 blog posts published with > 5,000 total views

### Q2: Growth (Months 4-6)
**Objective:** Launch v1.0, grow community, and establish content authority.
- **KR1:** 2,500 GitHub stars
- **KR2:** 500 Discord members
- **KR3:** 20 external contributors (at least 1 merged PR each)
- **KR4:** 500 CLI installs per week
- **KR5:** 1 conference talk accepted and delivered
- **KR6:** HN front page during Launch Week
- **KR7:** VS Code extension: 1,000 installs

---

*This plan is a living document. Review and update monthly based on actual metrics and community feedback. The best DX strategy is one that adapts to real developer behavior, not one that follows a plan blindly.*
