# ValiForge Cloud — Frontend & Cloud Product Engineering Plan

**Document Version:** 1.0
**Author:** Frontend / Cloud Product Engineer
**Date:** 2026-03-13
**Status:** Draft — Pending Architecture Review

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Tech Stack Rationale](#tech-stack-rationale)
3. [Section 1: Marketing Website (valiforge.dev)](#section-1-marketing-website)
4. [Section 2: Documentation Architecture](#section-2-documentation-architecture)
5. [Section 3: ValiForge Cloud Dashboard](#section-3-cloud-dashboard)
6. [Section 4: PR Bot / GitHub Integration](#section-4-pr-bot--github-integration)
7. [Section 5: API Server (Backend-for-Frontend)](#section-5-api-server)
8. [Section 6: Billing & Subscription Management](#section-6-billing--subscription-management)
9. [Section 7: Enterprise Features](#section-7-enterprise-features)
10. [Total Effort Summary](#total-effort-summary)
11. [Critical Path Analysis](#critical-path-analysis)
12. [Sprint Plan (16 Sprints / 32 Weeks)](#sprint-plan)
13. [Key Risks & Mitigations](#key-risks--mitigations)
14. [Appendix: ADRs and Open Questions](#appendix)

---

## Executive Summary

ValiForge Cloud is the monetization layer atop the open-source ValiForge Rust CLI. This plan covers 119 discrete tasks across 7 workstreams, estimated at **~398 engineering-days** (roughly 80 person-weeks). With a team of 3 engineers (1 senior frontend, 1 full-stack, 1 backend/infra), the critical path yields a **32-week delivery** to GA, with an MVP ("Cloud Beta") achievable by **Week 16**.

The plan is sequenced so that the marketing website and docs ship first (generating organic traffic and waitlist signups), followed by the dashboard MVP, then billing, then enterprise features.

---

## Tech Stack Rationale

### Marketing Website: Astro 5 + Starlight
- **Why Astro over Next.js for marketing:** Astro ships zero JS by default, producing static HTML that trivially hits Lighthouse 95+. The marketing site has no interactive state — it's content. Astro's island architecture lets us embed the WASM playground as an isolated interactive island without hydrating the whole page.
- **Why Starlight for docs:** First-party Astro integration, built-in versioning, search (Pagefind), i18n, sidebar generation from filesystem. Avoids the React dependency of Docusaurus and the opinionated MDX setup of Nextra.
- **MDX for blog:** Astro has first-class MDX support. We reuse the same content pipeline for blog and docs.

### Cloud Dashboard: Next.js 15 (App Router) + Tailwind CSS v4 + shadcn/ui
- **Why Next.js over SvelteKit:** Ecosystem depth. shadcn/ui provides 50+ accessible, unstyled components (tables, dialogs, command palettes) that match the dashboard-heavy UI. The React ecosystem has mature libraries for charts (Recharts/Tremor), real-time data (TanStack Query), and form handling (React Hook Form + Zod). SvelteKit is excellent but the component ecosystem is thinner for enterprise dashboards.
- **Why App Router:** Server Components reduce client bundle (schema data, run history can stream from server). Route groups cleanly separate (marketing) from (dashboard) from (auth) layouts.
- **Why shadcn/ui over Radix/Mantine/Chakra:** Copy-paste model means zero dependency lock-in. Components live in our repo, fully customizable. Built on Radix primitives for accessibility. Tailwind v4 integration is seamless.
- **Why Tailwind v4:** CSS-first configuration (no `tailwind.config.js`), native CSS cascade layers, faster builds. The `@theme` directive replaces the config file.

### API Server: Rust (Axum)
- **Why Rust over TypeScript (Hono/Fastify):** ValiForge's core is Rust. The validation engine, schema parser, and SLM inference are all Rust crates. By using Axum, we call into `valiforge-core` directly without FFI or subprocess overhead. This also means one language for the entire backend, simplifying CI and hiring for the Rust-centric team.
- **Why Axum over Actix-Web:** Tower middleware ecosystem, cleaner async API, better compiler error messages, official Tokio-maintained.

### Database: PostgreSQL 16 + pgvector
- **Why Postgres:** Battle-tested for multi-tenant SaaS. Row-level security for tenant isolation. JSONB for flexible validation result storage. pgvector enables future semantic search over API schemas.
- **Migration tool:** `sqlx` with compile-time query checking.

### Auth: Clerk or custom (JWT + refresh tokens via Axum middleware)
- For MVP: **Clerk** (hosted auth, GitHub/Google OAuth, JWT verification, org management). Reduces auth engineering by ~15 days.
- For Enterprise: Replace with custom SAML + OIDC provider integration when SSO becomes a requirement.

### Real-time: Server-Sent Events (SSE) over WebSocket
- **Why SSE over WebSocket:** Validation run updates are server-to-client only (no bidirectional need). SSE works over HTTP/2 multiplexing, requires no upgrade handshake, auto-reconnects, and passes through corporate proxies that block WebSocket. Axum supports SSE natively via `axum::response::Sse`.

### Payments: Stripe
- No viable alternative for SaaS billing at this scale. Stripe Billing for subscriptions, Stripe Metering for usage-based pricing, Stripe Tax for compliance.

---

## Section 1: Marketing Website (valiforge.dev)

### SITE-001: Project Scaffolding & Monorepo Setup
- **Description:** Initialize an Nx or Turborepo monorepo with three packages: `@valiforge/web` (Astro marketing site), `@valiforge/dashboard` (Next.js cloud app), `@valiforge/api` (Axum BFF). Configure shared Tailwind theme tokens, ESLint, Prettier, and CI (GitHub Actions). Use pnpm workspaces for the JS packages and a Cargo workspace for Rust.
- **Estimated Effort:** 3 days
- **Dependencies:** None (Day 0 task)
- **Acceptance Criteria:**
  - `pnpm dev` boots the marketing site at `localhost:4321` and the dashboard at `localhost:3000`
  - `cargo run` boots the API server at `localhost:8080`
  - CI pipeline runs lint, typecheck, and build for all packages on every PR
  - Shared Tailwind design tokens (colors, fonts, spacing) are consumed by both Astro and Next.js
- **Best Practices:**
  - Pin all dependency versions in `pnpm-lock.yaml` (no `^` ranges in production deps)
  - Use `.nvmrc` to lock Node version (22 LTS)
  - Use `rust-toolchain.toml` to lock Rust nightly/stable channel
  - Configure Turborepo remote caching for CI speed

### SITE-002: Design System & Shared Component Library
- **Description:** Create a shared design token system and reusable component library. Define brand colors (primary: electric indigo `#6366F1`, accent: emerald `#10B981`, neutral grays), typography scale (Inter for body, JetBrains Mono for code), spacing scale (4px base), border-radius tokens, and shadow tokens. Build foundational components: Button, Badge, Card, Code Block, Callout, Comparison Table.
- **Estimated Effort:** 4 days
- **Dependencies:** SITE-001
- **Acceptance Criteria:**
  - Storybook (or Astro-native story pages) renders all components in light and dark mode
  - Components pass axe-core accessibility audit (WCAG 2.1 AA)
  - Design tokens are exported as CSS custom properties consumed by both Astro and Next.js
  - Typography scale is responsive (fluid clamp-based sizing)
- **Best Practices:**
  - Use CSS custom properties (not Tailwind `@apply` in component CSS) for theme values — enables runtime theme switching
  - All interactive components must have `:focus-visible` styles
  - Color contrast ratio >= 4.5:1 for all text on background combinations

### SITE-003: Landing Page — Hero Section
- **Description:** Build the above-the-fold hero section. Layout: split — left side has headline, subheadline, two CTAs ("Get Started Free" + "View on GitHub"); right side has an animated terminal showing ValiForge CLI running a validation (use a typewriter effect with realistic output including colored pass/fail results). Below the fold: trust bar with logos (works with OpenAPI, GraphQL, gRPC).
- **Estimated Effort:** 3 days
- **Dependencies:** SITE-002
- **Acceptance Criteria:**
  - Hero loads with zero layout shift (CLS = 0)
  - Terminal animation starts only when visible (Intersection Observer)
  - LCP element (headline text) renders in < 1.2s on 4G throttle
  - Both CTAs are above the fold on 1280px+ viewports
  - Mobile layout stacks vertically; terminal animation is replaced with a static screenshot on viewports < 768px
- **Best Practices:**
  - Preload the hero font (Inter Bold) via `<link rel="preload">`
  - Use `content-visibility: auto` on below-fold sections
  - Hero image/animation: use CSS animations or lightweight JS, not Lottie (adds 50KB+)
  - Copy guidelines: lead with the outcome ("Find API bugs before your users do"), not the technology

### SITE-004: Landing Page — Features Section
- **Description:** Six-card feature grid showcasing ValiForge's core capabilities: (1) Stateful Multi-Step Validation, (2) Property-Based Test Generation, (3) Breaking Change Detection, (4) SLM-Powered Intelligence, (5) Rust-Speed Performance, (6) CI/CD Native. Each card has an icon (Lucide), title, 2-line description, and a subtle hover animation. Below the grid: a "How It Works" 3-step flow (Install -> Configure -> Validate) with connecting lines.
- **Estimated Effort:** 2 days
- **Dependencies:** SITE-002
- **Acceptance Criteria:**
  - Cards render in a responsive grid (3-col desktop, 2-col tablet, 1-col mobile)
  - All icons are inline SVG (no icon font), tree-shaken from Lucide
  - Each card links to the relevant docs section
  - "How It Works" step flow is accessible (uses ordered list semantics)
- **Best Practices:**
  - Use `<section aria-labelledby="features-heading">` for landmark navigation
  - Lazy-load icons below the fold
  - Card hover effects use `transform` and `opacity` only (GPU-composited, no layout thrash)

### SITE-005: Landing Page — Benchmarks Section
- **Description:** Interactive benchmark comparison: ValiForge vs Schemathesis vs Dredd vs Postman. Display a bar chart showing requests/second, time-to-first-result, and memory usage. Data sourced from reproducible benchmarks committed to the repo (`benchmarks/` directory). Include a "Run it yourself" callout linking to the benchmark script. Use a simple chart library (Chart.js or a hand-rolled SVG bar chart for minimal JS).
- **Estimated Effort:** 3 days
- **Dependencies:** SITE-002
- **Acceptance Criteria:**
  - Benchmark data is loaded from a static JSON file (not hardcoded in HTML)
  - Chart renders without JavaScript as a fallback (`<noscript>` shows a static table)
  - Numbers include methodology footnotes (hardware specs, sample size, confidence intervals)
  - Chart is accessible: data table alternative is screen-reader accessible
- **Best Practices:**
  - Never misrepresent competitor performance — link to the exact benchmark scripts
  - Use `<figure>` + `<figcaption>` for chart semantics
  - SVG charts preferred over canvas for SEO (text is indexable)
  - Keep chart JS < 15KB gzipped

### SITE-006: Landing Page — Pricing Section
- **Description:** Three-tier pricing cards (Community/Team/Business) plus an Enterprise "Contact Us" CTA. Community tier emphasizes "Free forever, open source." Team tier highlights collaboration. Business tier highlights SLM inference and advanced analytics. Include a feature comparison matrix below the cards with tooltips for each feature. Toggle for monthly/annual pricing (annual saves 20%).
- **Estimated Effort:** 2 days
- **Dependencies:** SITE-002
- **Acceptance Criteria:**
  - Monthly/annual toggle updates prices with a smooth number transition animation
  - "Most Popular" badge on Team tier
  - Feature matrix is a responsive table with sticky headers
  - All prices are pulled from a single `pricing.ts` config (not duplicated)
  - CTA buttons route to `/signup?plan=team` etc.
- **Best Practices:**
  - Use `<table>` semantics for the feature matrix (not CSS grid with divs)
  - Price display: show annual price as per-month equivalent ("$23/user/mo billed annually")
  - Include a FAQ section below pricing addressing common objections

### SITE-007: Landing Page — Testimonials & Social Proof
- **Description:** Rotating testimonial carousel (or static grid) from early adopters and beta users. Include name, role, company logo, and a 1-2 sentence quote. Below testimonials: GitHub stars counter (live via GitHub API, cached), npm download count, and "Trusted by X teams" counter.
- **Estimated Effort:** 1.5 days
- **Dependencies:** SITE-002
- **Acceptance Criteria:**
  - Testimonials render as static HTML (no JS carousel — use CSS `scroll-snap` for mobile swipe)
  - GitHub star count is fetched at build time (ISR or Astro server endpoint) and cached for 1 hour
  - Company logos are grayscale, desaturate on hover to color
  - Quotes use `<blockquote>` + `<cite>` semantics
- **Best Practices:**
  - Never fabricate testimonials — use placeholder structure with "Coming soon" until real quotes are available
  - Lazy-load company logo images
  - Star count: fall back to a static number if GitHub API fails at build time

### SITE-008: Landing Page — Footer & CTA Banner
- **Description:** Sticky bottom CTA banner ("Ready to validate your APIs? Start free.") that appears after scrolling past the hero. Site footer with: product links, docs links, company links, GitHub/Twitter/Discord social icons, SOC2/GDPR badges (when applicable), and a newsletter signup (Resend or Buttondown integration).
- **Estimated Effort:** 1 day
- **Dependencies:** SITE-002
- **Acceptance Criteria:**
  - CTA banner uses `position: sticky` with Intersection Observer to show/hide
  - Footer links are organized in a 4-column grid (responsive to 2-col on mobile)
  - Newsletter form validates email client-side and submits to an API endpoint
  - Social icons are SVG, not icon font
- **Best Practices:**
  - Newsletter: double opt-in, GDPR-compliant, clear privacy language
  - Footer: include `aria-label` on navigation sections
  - CTA banner: respect `prefers-reduced-motion` (no slide-in animation)

### SITE-009: SEO Foundation
- **Description:** Implement comprehensive SEO infrastructure: dynamic `<meta>` tags per page (title, description, og:image, twitter:card), structured data (JSON-LD for SoftwareApplication, FAQPage, HowTo), XML sitemap generation, robots.txt, canonical URLs, and Open Graph images auto-generated via `@vercel/og` or Satori for each page/blog post.
- **Estimated Effort:** 2.5 days
- **Dependencies:** SITE-003 through SITE-008
- **Acceptance Criteria:**
  - Every page has unique `<title>` and `<meta name="description">` (no duplicates)
  - OG images generate correctly for all pages (test with Facebook Debugger, Twitter Card Validator)
  - Sitemap includes all pages, blog posts, and docs pages with correct `<lastmod>`
  - Google Search Console validates zero errors after deployment
  - Structured data passes Google Rich Results Test
- **Best Practices:**
  - Target keywords per page: landing = "API validation tool", docs = "API testing documentation", blog = long-tail keywords
  - Use descriptive, keyword-rich URLs (`/docs/getting-started` not `/docs/gs`)
  - Implement `hreflang` tags if internationalization is planned

### SITE-010: Analytics Integration
- **Description:** Integrate Plausible Analytics (self-hosted or cloud) for privacy-first, GDPR-compliant analytics. Track: page views, unique visitors, referral sources, campaign UTM parameters, outbound link clicks (GitHub, docs), CTA clicks (signup, playground), and custom events (playground-used, docs-search-query). No cookies, no consent banner needed.
- **Estimated Effort:** 1 day
- **Dependencies:** SITE-001
- **Acceptance Criteria:**
  - Plausible script loads asynchronously and is < 1KB
  - Custom events fire correctly (verify in Plausible dashboard)
  - UTM parameters are tracked for marketing campaigns
  - Dashboard is accessible to marketing team
  - No performance regression (Lighthouse score unchanged)
- **Best Practices:**
  - Use `data-domain` attribute, not inline script
  - Proxy the Plausible script through your domain to avoid ad-blockers (`/js/script.js` -> Plausible)
  - Define goal funnels: Landing -> Playground -> Signup -> First Validation

### SITE-011: Interactive WASM Playground
- **Description:** Build an in-browser playground where users paste an OpenAPI spec (or select from examples) and see ValiForge validate it live — entirely client-side via WASM. The playground has a split-pane layout: left = Monaco editor (YAML/JSON) with syntax highlighting, right = validation results (pass/fail per endpoint, violations, suggestions). Compile `valiforge-core` to `wasm32-unknown-unknown` via `wasm-pack`, expose a `validate(spec: string) -> ValidationResult` function.
- **Estimated Effort:** 8 days
- **Dependencies:** SITE-001, Rust `valiforge-core` crate (external dependency)
- **Acceptance Criteria:**
  - WASM module loads in < 2 seconds on 4G throttle (target < 500KB gzipped)
  - Validation runs in < 500ms for a 100-endpoint spec
  - Editor supports YAML and JSON with syntax highlighting and error markers
  - Results panel shows: endpoint list with pass/fail badges, expandable violation details, fix suggestions
  - Three example specs are preloaded (Petstore, Stripe-like, GitHub-like)
  - Playground works fully offline once loaded (service worker caches WASM)
  - Graceful degradation: if WASM fails, show a message with CLI install instructions
- **Best Practices:**
  - Use `wasm-opt -Oz` to minimize WASM binary size
  - Lazy-load Monaco editor (it's 2MB+ — only load when playground tab is active)
  - Use a Web Worker to run WASM validation off the main thread (prevent UI jank)
  - Implement debounced validation (300ms after last keystroke)
  - Show a skeleton/shimmer while WASM loads, not a blank screen

### SITE-012: Blog Infrastructure
- **Description:** Set up an MDX-based blog at `/blog`. Features: author profiles, tags, reading time estimate, table of contents (auto-generated from headings), syntax-highlighted code blocks (Shiki), RSS feed, and social sharing meta. Blog posts live in `content/blog/` as `.mdx` files with frontmatter (title, date, author, tags, description, og_image).
- **Estimated Effort:** 2 days
- **Dependencies:** SITE-001, SITE-002
- **Acceptance Criteria:**
  - Blog index page shows paginated post list (10 per page) with excerpts
  - Individual post pages render MDX correctly (custom components: Callout, CodeComparison, Benchmark)
  - RSS feed validates at `feed.xml` (test with feed validator)
  - Code blocks support line highlighting, file names, and copy button
  - Reading time is calculated from word count (avg 200 wpm)
- **Best Practices:**
  - Use Shiki for syntax highlighting (build-time, no client JS)
  - Image optimization: use Astro's `<Image>` component for responsive images with AVIF/WebP
  - Blog post URL format: `/blog/yyyy/slug` (includes year for temporal context)
  - Draft posts (`draft: true` in frontmatter) are excluded from production builds

### SITE-013: Performance Optimization & Lighthouse Audit
- **Description:** Final performance pass on the marketing site. Targets: Lighthouse Performance >= 95, Accessibility >= 95, Best Practices >= 95, SEO >= 95. Tasks: implement resource hints (preconnect, prefetch), optimize critical rendering path, audit bundle sizes, configure CDN caching headers (Vercel/Cloudflare), implement image lazy loading with blur placeholders, minimize third-party script impact.
- **Estimated Effort:** 2 days
- **Dependencies:** All SITE-* tasks
- **Acceptance Criteria:**
  - Lighthouse CI runs on every PR and blocks merge if scores drop below 90
  - Core Web Vitals pass: LCP < 2.5s, FID < 100ms, CLS < 0.1
  - Total JS shipped on landing page < 50KB (excluding playground island)
  - Time to Interactive < 3s on Moto G4 (3G throttle)
  - All images serve AVIF with WebP and JPEG fallbacks
- **Best Practices:**
  - Use `@astrojs/check` for build-time HTML validation
  - Configure `Cache-Control: public, max-age=31536000, immutable` for hashed assets
  - Use `<link rel="preload">` for critical fonts only (max 2 font files)
  - Run WebPageTest in addition to Lighthouse for real-world waterfall analysis

**Section 1 Subtotal: ~35 engineering-days**

---

## Section 2: Documentation Architecture

### DOCS-001: Starlight Documentation Site Setup
- **Description:** Configure Astro Starlight as the documentation engine at `/docs`. Set up: sidebar navigation (auto-generated from filesystem + manual overrides), search (Pagefind — runs at build time, zero-JS search), versioning (v1, v2 via path prefix), dark/light mode, edit-on-GitHub links, and last-updated timestamps (from git commit dates).
- **Estimated Effort:** 2 days
- **Dependencies:** SITE-001
- **Acceptance Criteria:**
  - Docs site builds in < 30 seconds for 100+ pages
  - Search returns results in < 100ms with typo tolerance
  - Sidebar collapses nested sections and highlights current page
  - Version switcher dropdown works (e.g., `/docs/v1/getting-started` vs `/docs/v2/getting-started`)
  - Edit links point to the correct GitHub file path
- **Best Practices:**
  - Use Starlight's built-in components: `<Tabs>`, `<Card>`, `<Steps>`, `<Aside>`
  - Configure `social` links in Starlight config (GitHub, Discord)
  - Use `expressive-code` for code blocks (built into Starlight) — supports diff highlighting, line markers, frame labels

### DOCS-002: Getting Started Guide
- **Description:** Write a "Zero to First Validation in 5 Minutes" guide. Flow: (1) Install ValiForge (`cargo install valiforge` or `brew install valiforge`), (2) Point at an OpenAPI spec (`valiforge init --spec petstore.yaml`), (3) Run validation (`valiforge validate`), (4) Read results, (5) Configure `valiforge.toml` for customization. Include a Petstore example spec bundled with the CLI.
- **Estimated Effort:** 2 days
- **Dependencies:** DOCS-001
- **Acceptance Criteria:**
  - A new user with Rust toolchain can go from zero to seeing validation results in < 5 minutes (timed test)
  - Guide includes OS-specific install instructions (macOS, Linux, Windows) with tabs
  - All code snippets are tested in CI (using `mdx-test` or a custom script that extracts and runs code blocks)
  - Includes a "What's Next?" section linking to deeper guides
- **Best Practices:**
  - Show expected output for every command (users need to verify they're on track)
  - Use `<Steps>` component for numbered walkthrough
  - Include a troubleshooting section for common errors (missing OpenSSL, wrong Rust version)
  - Provide a one-line "quick start" for the impatient: `curl -sSf https://valiforge.dev/install.sh | sh && valiforge validate --spec https://petstore.swagger.io/v2/swagger.json`

### DOCS-003: CLI Reference (Auto-Generated)
- **Description:** Auto-generate CLI reference documentation from the clap command definitions in the Rust source. Build a CI step that runs `valiforge --help` for every subcommand and converts the output into structured Markdown pages. Each command gets its own page with: synopsis, description, arguments, options (with defaults), examples, and related commands.
- **Estimated Effort:** 3 days
- **Dependencies:** DOCS-001, ValiForge CLI crate
- **Acceptance Criteria:**
  - CLI docs are regenerated on every release (CI job)
  - Every flag and subcommand is documented with at least one example
  - Docs include exit codes and their meanings
  - Search indexes CLI commands and flags
  - Docs stay in sync with code automatically (no manual updates)
- **Best Practices:**
  - Use `clap`'s `help_template` to produce structured output parseable by the doc generator
  - Generate both man pages (`man valiforge`) and web docs from the same source
  - Include a "See Also" section on each command page linking to related commands
  - Categorize commands: Core (`validate`, `init`, `check`), Data (`generate`, `fixtures`), Config (`config`, `profile`)

### DOCS-004: Configuration Reference (valiforge.toml)
- **Description:** Comprehensive reference for every configuration option in `valiforge.toml`. Organized by section: `[validation]`, `[generation]`, `[output]`, `[cloud]`, `[ci]`. Each option documents: type, default value, description, example, environment variable override, and which CLI flag corresponds to it.
- **Estimated Effort:** 2 days
- **Dependencies:** DOCS-001
- **Acceptance Criteria:**
  - Every config option has a documented default value
  - Configuration is validated against a JSON Schema (published at `valiforge.dev/schema/valiforge.toml.json`)
  - Docs include a complete example `valiforge.toml` with all options annotated
  - Environment variable mapping is documented (`VALIFORGE_VALIDATION_DEPTH` = `validation.depth`)
- **Best Practices:**
  - Publish the JSON Schema so editors (VS Code, IntelliJ) can provide autocomplete for `valiforge.toml`
  - Use TOML syntax highlighting (expressive-code supports it)
  - Group options by use case ("Minimal CI config", "Full enterprise config") with complete examples

### DOCS-005: CI/CD Integration Guide
- **Description:** Step-by-step guides for integrating ValiForge into CI/CD pipelines. Cover: GitHub Actions (with reusable workflow), GitLab CI, Jenkins, CircleCI, and generic Docker usage. Each guide includes: YAML configuration, caching strategy (cache `~/.valiforge/`), artifact upload (JUnit XML reports), and failure handling (fail-fast vs warn-only modes).
- **Estimated Effort:** 3 days
- **Dependencies:** DOCS-001
- **Acceptance Criteria:**
  - GitHub Actions workflow is copy-paste ready and tested in a reference repo
  - Each CI platform guide includes a "verify it's working" section
  - JUnit XML output is documented for CI report integration
  - Caching reduces subsequent run time by > 50% (documented with benchmarks)
  - Includes "Advanced" section: matrix testing across multiple API versions
- **Best Practices:**
  - Provide a reusable GitHub Action (`valiforge/action@v1`) with configurable inputs
  - Pin CLI version in CI configs (`valiforge@0.5.2`, not `latest`)
  - Show how to use ValiForge with API mocking (WireMock, Prism) for offline CI

### DOCS-006: Test Data Generation Guide
- **Description:** Document ValiForge's property-based test data generation: how it reads schema constraints (min/max, patterns, enums, formats) to produce valid and invalid test fixtures. Cover: generation strategies (random, edge-case, boundary), seed control for reproducibility, custom generators (via plugins), and exporting fixtures as JSON/YAML for use in other tools.
- **Estimated Effort:** 2 days
- **Dependencies:** DOCS-001
- **Acceptance Criteria:**
  - Guide includes visual examples: schema input -> generated test data output
  - Edge case generation is explained (empty strings, max-length strings, boundary integers)
  - Custom generator plugin API is documented with a complete example
  - Reproducibility: same seed produces identical fixtures (documented guarantee)
- **Best Practices:**
  - Include a "recipes" section: "Generate 1000 realistic user profiles", "Fuzz a payment endpoint"
  - Show diff between `--strategy random` and `--strategy edge-case` output
  - Warn about PII in generated data (use faker-style generators, not real data)

### DOCS-007: Breaking Change Detection Guide
- **Description:** Document ValiForge's schema diffing and breaking change detection: how it compares two versions of an API spec, classifies changes (breaking, non-breaking, deprecation), and produces a migration report. Cover: integration with git (diff between branches/tags), severity levels, suppression rules, and the relationship to semantic versioning.
- **Estimated Effort:** 2 days
- **Dependencies:** DOCS-001
- **Acceptance Criteria:**
  - Guide includes examples of each breaking change type (removed endpoint, changed type, narrowed enum)
  - Suppression rules are documented (`# valiforge:ignore breaking-change`)
  - Output formats are documented (text, JSON, JUnit, GitHub annotations)
  - Integration with PR bot is cross-referenced
- **Best Practices:**
  - Include a decision flowchart: "Is this change breaking?" with visual diagram
  - Show real-world before/after spec diffs with ValiForge's output
  - Link to API versioning best practices (external resources)

### DOCS-008: Migration Guides (From Competitors)
- **Description:** Write three migration guides: "Migrating from Postman", "Migrating from Schemathesis", "Migrating from Karate". Each guide: maps concepts (Postman Collection -> ValiForge config), provides a conversion script or CLI command where possible, highlights feature parity and gaps honestly, and includes a "Why Switch?" value proposition.
- **Estimated Effort:** 4 days
- **Dependencies:** DOCS-001
- **Acceptance Criteria:**
  - Each guide is structured: Overview -> Concept Mapping -> Step-by-Step Migration -> Feature Comparison -> FAQ
  - Conversion scripts (if any) are tested and published as `valiforge migrate --from postman collection.json`
  - Feature comparison tables are honest (mark features ValiForge doesn't have yet)
  - Each guide has a "5-minute migration" quick path for the simplest case
- **Best Practices:**
  - Never disparage competitors — focus on ValiForge's unique strengths
  - Include a "What you'll gain" and "What's different" section
  - Provide a Rosetta Stone table: "In Postman you do X, in ValiForge you do Y"

### DOCS-009: API Reference (valiforge-sdk Crate)
- **Description:** Generate and publish Rustdoc API reference for the `valiforge-sdk` crate. This is the programmatic Rust API for embedding ValiForge in custom tooling. Host rendered docs at `/docs/api/rust`. Include usage examples in doc comments, organized by module.
- **Estimated Effort:** 2 days
- **Dependencies:** DOCS-001, `valiforge-sdk` crate
- **Acceptance Criteria:**
  - Every public type, function, and trait has a doc comment with at least one example
  - Examples compile and pass (`cargo test --doc`)
  - Docs are regenerated and published on every crate release
  - Module-level docs provide conceptual overview before diving into types
  - Cross-links between related types work correctly
- **Best Practices:**
  - Use `#![warn(missing_docs)]` to enforce documentation coverage
  - Include `# Examples` and `# Errors` sections in doc comments
  - Use `#[doc(hidden)]` for internal APIs that shouldn't appear in public docs
  - Publish to docs.rs in addition to the ValiForge website

### DOCS-010: Tutorials (Step-by-Step Scenarios)
- **Description:** Create four hands-on tutorials: (1) "Validate a REST API from Scratch" (30 min), (2) "Set Up Breaking Change Detection in GitHub Actions" (20 min), (3) "Generate Test Data for Load Testing" (25 min), (4) "Build a Custom Validation Plugin" (45 min). Each tutorial includes a companion Git repo with start/end states for each step.
- **Estimated Effort:** 5 days
- **Dependencies:** DOCS-001 through DOCS-007
- **Acceptance Criteria:**
  - Each tutorial has a clear learning objective stated upfront
  - Companion repos have branches for each step (`step-1`, `step-2`, etc.)
  - Tutorials are tested end-to-end by a team member who didn't write them
  - Each step shows expected output so users can verify progress
  - Tutorials include "Stretch Goals" for advanced users
- **Best Practices:**
  - Follow the Divio documentation system: tutorials teach by doing (no theory dumps)
  - Time estimates are accurate (tested)
  - Include "Stuck? Check the solution" expandable sections at each step
  - Use real-world APIs (not toy examples) where possible

### DOCS-011: Versioned Documentation System
- **Description:** Implement doc versioning so that each major release (v1, v2) has its own complete doc set. Implement a version switcher in the navigation. Older versions show a banner: "You're reading docs for ValiForge v1. View the latest version." Redirects from unversioned URLs go to the latest version.
- **Estimated Effort:** 2 days
- **Dependencies:** DOCS-001
- **Acceptance Criteria:**
  - Version switcher dropdown is present on every docs page
  - URLs follow pattern: `/docs/v1/getting-started`, `/docs/v2/getting-started`
  - `/docs/getting-started` redirects to `/docs/v2/getting-started` (latest)
  - Old version pages have a "This is outdated" banner with link to current version
  - Sitemap includes all versions with correct canonical URLs (latest version is canonical)
- **Best Practices:**
  - Use Starlight's built-in versioning (or implement via Astro route groups)
  - Only create a new version on major releases (not every minor)
  - Old version docs are frozen (no edits except security fixes)
  - Search defaults to latest version but allows filtering by version

**Section 2 Subtotal: ~29 engineering-days**

---

## Section 3: ValiForge Cloud Dashboard

### CLOUD-001: Next.js 15 App Scaffold
- **Description:** Initialize the Next.js 15 (App Router) project within the monorepo. Configure: TypeScript strict mode, Tailwind v4, shadcn/ui (New York style), path aliases (`@/components`, `@/lib`, `@/hooks`), Zod for runtime validation, TanStack Query v5 for server state, and environment variable schema validation (via `t3-env`). Set up route groups: `(marketing)`, `(auth)`, `(dashboard)`.
- **Estimated Effort:** 2 days
- **Dependencies:** SITE-001
- **Acceptance Criteria:**
  - `pnpm dev` hot-reloads in < 500ms
  - TypeScript strict mode enabled with no `any` types
  - shadcn/ui theme customized with ValiForge brand colors
  - Environment variables validated at build time (missing vars fail the build)
  - Route groups separate marketing layout (no sidebar) from dashboard layout (sidebar + header)
- **Best Practices:**
  - Use `next.config.ts` (TypeScript config) for type-safe configuration
  - Configure `serverExternalPackages` for any Rust/WASM bindings
  - Set `output: 'standalone'` for Docker deployability
  - Use Biome instead of ESLint for faster linting (Next.js 15 supports it)

### CLOUD-002: Authentication System
- **Description:** Implement authentication with Clerk (or Auth.js as fallback). Support: GitHub OAuth (primary — our users are developers), Google OAuth, email/password with email verification. After auth, create a ValiForge user record in Postgres (synced via Clerk webhooks). Implement: protected routes (middleware), session management, user profile page, organization/team creation.
- **Estimated Effort:** 4 days
- **Dependencies:** CLOUD-001, API-001
- **Acceptance Criteria:**
  - GitHub OAuth login works end-to-end (click -> redirect -> callback -> dashboard)
  - Protected routes redirect to `/login` for unauthenticated users
  - JWT tokens are validated on the API server (Axum middleware verifies Clerk JWTs)
  - User creation webhook syncs Clerk user to Postgres `users` table
  - Session persists across page reloads (refresh token rotation)
  - Logout clears all tokens and redirects to marketing site
- **Best Practices:**
  - Use Clerk's `<SignIn>` and `<SignUp>` components for hosted UI (faster to ship, more secure)
  - Implement CSRF protection on all auth endpoints
  - Store sensitive tokens only in HTTP-only cookies (never localStorage)
  - Rate-limit login attempts: 5 per minute per IP

### CLOUD-003: Dashboard Layout Shell
- **Description:** Build the authenticated dashboard layout: left sidebar navigation (collapsible), top header (breadcrumbs, user avatar dropdown, notification bell, dark mode toggle), and main content area. Sidebar items: Overview, Projects, Runs, Schema Explorer, Test Data, Team, Settings, Billing. Use `next/navigation` for active link highlighting. Implement command palette (Cmd+K) for quick navigation.
- **Estimated Effort:** 3 days
- **Dependencies:** CLOUD-001, CLOUD-002
- **Acceptance Criteria:**
  - Sidebar collapses to icons on narrow viewports and via toggle button
  - Command palette (shadcn `<Command>`) searches pages, projects, and recent runs
  - Breadcrumbs auto-generate from route segments
  - Dark mode persists across sessions (stored in cookie, not localStorage — avoids flash)
  - Layout renders as a Server Component (sidebar links are not client components)
  - Mobile: sidebar becomes a slide-out drawer
- **Best Practices:**
  - Use `<nav aria-label="Dashboard navigation">` for accessibility
  - Sidebar state (collapsed/expanded) persists in a cookie (avoids layout shift on reload)
  - Use CSS `grid` for the layout (sidebar + main), not flexbox (avoids scroll issues)
  - Prefetch visible navigation links via `<Link prefetch={true}>`

### CLOUD-004: Overview Dashboard Page
- **Description:** The default authenticated view. Displays: (1) Summary cards — total validations (7d), pass rate, breaking changes detected, active projects. (2) Trend chart — pass/fail over last 30 days (area chart, Recharts or Tremor). (3) Recent runs table — last 10 runs with status, project, duration, violations count, timestamp. (4) Quick actions — "New Validation Run", "Add Project", "Invite Team Member".
- **Estimated Effort:** 4 days
- **Dependencies:** CLOUD-003, API-002
- **Acceptance Criteria:**
  - Dashboard loads in < 1s (server-side data fetch, streaming with Suspense boundaries)
  - Summary cards show change vs previous period ("+12% pass rate" with green/red indicator)
  - Trend chart supports time range selector (7d, 30d, 90d)
  - Recent runs table supports click-to-navigate to run detail
  - Empty state: shows onboarding checklist for new users ("1. Create a project, 2. Run your first validation")
  - Data refreshes every 30 seconds via TanStack Query `refetchInterval`
- **Best Practices:**
  - Use Server Components for initial data fetch, Client Components only for interactive chart
  - Implement loading skeletons (shadcn `<Skeleton>`) for each section independently
  - Chart: use `<Suspense>` boundary so chart loads independently of the summary cards
  - Cache dashboard data aggressively (stale-while-revalidate pattern)

### CLOUD-005: Project List & Project Detail Pages
- **Description:** **Project List** (`/projects`): Card grid showing all projects in the user's organization. Each card: project name, last validation date, pass rate badge, endpoint count. Supports: search, sort (name, last run, pass rate), and "New Project" creation dialog. **Project Detail** (`/projects/[id]`): Tabbed view with (1) Overview — pass rate trend, endpoint coverage, recent runs, (2) Endpoints — list of all validated endpoints with status, (3) Breaking Changes — timeline of detected breaking changes, (4) Settings — project config, webhook URLs, API spec source.
- **Estimated Effort:** 6 days
- **Dependencies:** CLOUD-003, API-003
- **Acceptance Criteria:**
  - Project creation dialog accepts: name, OpenAPI spec URL or file upload, validation schedule (manual/daily/on-push)
  - Project detail tabs use URL-based routing (`/projects/[id]/endpoints`, `/projects/[id]/breaking-changes`)
  - Endpoint list supports filtering by status (pass/fail/skip) and HTTP method (GET/POST/etc.)
  - Breaking changes timeline is a vertical timeline component with severity badges
  - Pagination on project list (server-side, 20 per page)
  - Optimistic UI: project creation shows immediately while API call is in flight
- **Best Practices:**
  - Use dynamic `og:image` for project pages (useful when sharing links in Slack)
  - Implement search with URL params (`?q=petstore`) so searches are shareable/bookmarkable
  - Use `useOptimistic` hook from React 19 for optimistic project creation
  - Project settings: validate spec URL reachability on save

### CLOUD-006: Run Detail Page
- **Description:** Detailed view of a single validation run (`/runs/[id]`). Layout: top summary (duration, total requests, pass/fail/skip counts, started at, triggered by). Main content: expandable accordion of validated endpoints, each showing: HTTP method + path, request sent (headers, body), response received (status, headers, body), violations found (with severity, description, schema path, and suggested fix). Syntax-highlighted request/response bodies (JSON/XML). Export run as PDF or JSON report.
- **Estimated Effort:** 5 days
- **Dependencies:** CLOUD-003, API-004
- **Acceptance Criteria:**
  - Run detail loads in < 2s for a run with 200 endpoints
  - Request/response bodies are syntax-highlighted with collapsible sections
  - Violations link to the relevant schema path in the Schema Explorer
  - Filter endpoints by status (pass/fail), HTTP method, or search by path
  - JSON export includes all request/response data for debugging
  - PDF export generates a clean, printable report (use `@react-pdf/renderer` or server-side Puppeteer)
  - Long-running run shows real-time progress via SSE (endpoints completing live)
- **Best Practices:**
  - Virtualize the endpoint list for runs with 500+ endpoints (use `@tanstack/react-virtual`)
  - Truncate large response bodies (> 10KB) with "Show full response" toggle
  - Use `<details>` HTML element for expandable sections (works without JS)
  - Implement deep linking to specific endpoints (`/runs/[id]#endpoint-get-users`)

### CLOUD-007: Schema Explorer Page
- **Description:** Interactive API schema browser (`/projects/[id]/schema`). Renders the OpenAPI spec as a navigable tree: paths -> operations -> parameters/request body/responses -> schemas. Each node shows: validation status (last run), coverage (how many times tested), and a "Validate Now" button. Schema definitions render as collapsible JSON Schema trees with type annotations and constraints.
- **Estimated Effort:** 5 days
- **Dependencies:** CLOUD-003, API-005
- **Acceptance Criteria:**
  - Schema tree renders the full OpenAPI spec (paths, components/schemas, security schemes)
  - Each endpoint shows its last validation status (green check, red X, gray dash for untested)
  - Schema definitions are clickable and expandable (show properties, types, constraints, examples)
  - Coverage percentage is displayed per endpoint and overall
  - Search within the schema (by path, operation ID, or schema name)
  - "Validate Now" triggers a single-endpoint validation run and updates in real-time
- **Best Practices:**
  - Use a tree component (shadcn `<Tree>` or custom) with keyboard navigation
  - Lazy-load deep schema branches (don't render 1000 properties on page load)
  - Color-code types: string (green), integer (blue), boolean (purple), array (orange), object (gray)
  - Show `example` values from the spec inline

### CLOUD-008: Test Data Viewer Page
- **Description:** Browse and manage generated test fixtures (`/projects/[id]/test-data`). Display: fixture sets (grouped by generation run), individual fixtures with their schema source, generation strategy used, and the fixture data (formatted JSON). Actions: regenerate with different strategy, download as JSON/YAML/CSV, copy to clipboard, and "Use in Postman" export.
- **Estimated Effort:** 3 days
- **Dependencies:** CLOUD-003, API-006
- **Acceptance Criteria:**
  - Fixture list is searchable and filterable by schema, strategy, and date
  - JSON data is displayed with syntax highlighting and collapsible nesting
  - Download supports JSON, YAML, and CSV formats
  - Bulk download as ZIP archive
  - Regenerate action shows a dialog with strategy options (random, edge-case, boundary)
  - Empty state explains how to generate test data via CLI or API
- **Best Practices:**
  - Use a JSON viewer component with expand/collapse all (not just raw text)
  - Large fixture sets (> 1000 items): paginate server-side, don't load all into browser
  - Show fixture data size (KB/MB) in the list view
  - Include a "Copy as cURL" option for each fixture (generates a curl command with the fixture as body)

### CLOUD-009: Team Management Page
- **Description:** Team/organization management (`/settings/team`). Features: invite members by email (sends invitation link), assign roles (Admin, Member, Viewer), remove members, view pending invitations, and manage team-level API keys. Admins can: manage billing, delete projects, manage SSO (enterprise). Members can: create projects, run validations. Viewers can: view dashboards and run history (read-only).
- **Estimated Effort:** 4 days
- **Dependencies:** CLOUD-002, CLOUD-003, API-007
- **Acceptance Criteria:**
  - Invite flow sends email with a unique invitation link (expires in 7 days)
  - Role assignment is enforced both in UI (disable buttons) and API (middleware check)
  - Team member list shows: name, email, role, last active date, status (active/pending)
  - Admin can change roles and remove members (with confirmation dialog)
  - Viewer role cannot access Settings, Billing, or Team Management pages
  - Invitation link works for both existing and new ValiForge users
- **Best Practices:**
  - Use RBAC middleware on both frontend (hide UI elements) and backend (reject API calls)
  - Show role descriptions in the role selector ("Admin: Full access including billing")
  - Implement "Leave team" functionality for non-admin members
  - Audit log entry for every team management action

### CLOUD-010: Settings Page
- **Description:** User and organization settings (`/settings`). Sections: (1) **Profile** — name, avatar, email, timezone. (2) **API Keys** — generate, view (masked), revoke, set expiry. (3) **Webhooks** — add URL, select events (run.completed, breaking-change.detected), test webhook. (4) **Notifications** — email and Slack notification preferences per event type. (5) **Danger Zone** — delete account, transfer ownership.
- **Estimated Effort:** 3 days
- **Dependencies:** CLOUD-003, API-008
- **Acceptance Criteria:**
  - API key generation shows the key once (never again) with a "Copy" button and warning
  - Webhook test sends a sample payload and shows the response code
  - Notification preferences are granular (per event type, per channel)
  - Delete account requires typing "DELETE" to confirm
  - All settings save optimistically with toast confirmation
  - API keys are masked in the UI (`vf_sk_...abc123`)
- **Best Practices:**
  - API keys: generate with a recognizable prefix (`vf_sk_` for secret, `vf_pk_` for publishable)
  - Webhook payloads are documented with JSON schema examples on the settings page
  - Implement rate limiting on API key generation (max 10 active keys per org)
  - Use `<input type="password">` for API key display with a show/hide toggle

### CLOUD-011: Dark Mode Implementation
- **Description:** Implement system-aware dark mode with manual toggle. Uses `next-themes` for theme management. All dashboard and marketing pages support dark mode. Theme preference is stored in a cookie (not localStorage) to avoid the flash of unstyled content on server-rendered pages.
- **Estimated Effort:** 2 days
- **Dependencies:** CLOUD-001, SITE-002
- **Acceptance Criteria:**
  - Theme toggles between light, dark, and system
  - No flash of wrong theme on page load (cookie-based detection + inline script)
  - All components (charts, code blocks, tables, badges) have dark mode variants
  - Dark mode colors meet WCAG 2.1 AA contrast requirements
  - Charts use appropriate color palettes for dark backgrounds
- **Best Practices:**
  - Use CSS custom properties for all colors (not Tailwind dark: variants everywhere)
  - Test dark mode in every PR (add a visual regression test)
  - Avoid pure black (`#000`) — use dark gray (`#0A0A0B`) for less eye strain
  - Ensure focus rings are visible in both modes

### CLOUD-012: Real-Time Run Updates (SSE)
- **Description:** Implement Server-Sent Events for live validation run progress. When a user is viewing a run detail page and the run is in progress, endpoint results stream in as they complete. The overview dashboard's "Recent Runs" table updates when runs complete. Use TanStack Query's `queryClient.setQueryData` to merge SSE updates into the cache.
- **Estimated Effort:** 3 days
- **Dependencies:** CLOUD-004, CLOUD-006, API-009
- **Acceptance Criteria:**
  - Run detail page shows live progress bar (X of Y endpoints validated)
  - Completed endpoints appear in real time without page refresh
  - Dashboard "Recent Runs" updates status when a run finishes
  - SSE reconnects automatically on connection drop (with exponential backoff)
  - Works behind corporate proxies and load balancers (HTTP/2 compatible)
  - Graceful degradation: if SSE fails, falls back to polling every 5 seconds
- **Best Practices:**
  - Use `EventSource` API with a custom hook (`useSSE`)
  - SSE events are typed: `run.progress`, `run.endpoint.completed`, `run.completed`, `run.failed`
  - Include `Last-Event-ID` for resumability after reconnection
  - Server sends periodic heartbeat events (`:keep-alive\n\n` every 30s) to prevent timeout

### CLOUD-013: Responsive Design & Mobile Optimization
- **Description:** Ensure the entire dashboard is usable on tablets (768px-1024px) and functional on mobile (320px-767px). Desktop-first design with responsive breakpoints. Tablet: sidebar collapses, tables gain horizontal scroll. Mobile: sidebar becomes a hamburger menu, cards stack vertically, charts resize, tables switch to card-based layouts for narrow screens.
- **Estimated Effort:** 3 days
- **Dependencies:** All CLOUD-* pages
- **Acceptance Criteria:**
  - No horizontal scrollbar on any viewport width >= 320px
  - Tables are either horizontally scrollable or switch to card layout on mobile
  - Touch targets are >= 44x44px on mobile
  - Charts are legible on mobile (simplified labels, larger touch targets for tooltips)
  - Navigation is fully functional via hamburger menu on mobile
  - Tested on: iPhone SE (375px), iPad (768px), iPad Pro (1024px)
- **Best Practices:**
  - Use CSS container queries (not just media queries) for component-level responsiveness
  - Test with real device emulators (not just browser resize)
  - Prioritize information: on mobile, show the most important metrics first
  - Use `<dialog>` element for modals (native mobile support)

### CLOUD-014: Error Handling & Empty States
- **Description:** Implement comprehensive error handling and empty states across all dashboard pages. Error boundary at the layout level catches unhandled errors. Each page has specific empty states: "No projects yet" (with CTA), "No runs in this project" (with setup guide link), "No team members" (with invite CTA). API errors show user-friendly messages with retry actions.
- **Estimated Effort:** 2 days
- **Dependencies:** All CLOUD-* pages
- **Acceptance Criteria:**
  - Every page has a designed empty state (not just "No data")
  - API errors show a toast notification with the error message and a retry button
  - 404 pages show a helpful message with navigation links
  - Error boundary catches React errors and shows a "Something went wrong" UI with a report button
  - Offline state is detected and shown (banner: "You're offline. Data may be stale.")
  - All error messages are human-readable (no raw error codes)
- **Best Practices:**
  - Use Next.js `error.tsx` files for route-level error boundaries
  - Empty states include illustrations (simple SVGs) and a single clear CTA
  - Log errors to Sentry (CLOUD-015) with user context (anonymized)
  - Distinguish between transient errors (retry) and permanent errors (contact support)

### CLOUD-015: Observability & Error Tracking
- **Description:** Integrate Sentry for error tracking and performance monitoring on the Next.js dashboard. Configure: error boundary integration, performance tracing (page load, API call durations), session replay for debugging, and custom breadcrumbs for user actions. Exclude PII from error reports.
- **Estimated Effort:** 1.5 days
- **Dependencies:** CLOUD-001
- **Acceptance Criteria:**
  - Unhandled errors are reported to Sentry with stack traces and user context
  - Performance transactions are created for every page navigation and API call
  - Session replay captures the last 60 seconds before an error (for debugging)
  - Source maps are uploaded to Sentry on each deployment
  - PII scrubbing rules are configured (no email, no API keys in reports)
  - Alert rules: notify on error spike (> 5x normal rate)
- **Best Practices:**
  - Use `@sentry/nextjs` for automatic instrumentation
  - Set sample rate: 100% for errors, 10% for performance transactions (adjust based on volume)
  - Configure release tracking tied to git commit SHA
  - Use Sentry's `beforeSend` hook to scrub sensitive data

### CLOUD-016: End-to-End Testing
- **Description:** Set up Playwright for E2E testing of critical user flows. Test scenarios: (1) Sign up -> create project -> run validation -> view results, (2) Invite team member -> accept invitation -> view dashboard, (3) Upgrade plan -> verify billing status, (4) Dark mode toggle, (5) Command palette navigation. Run in CI on every PR against a staging environment.
- **Estimated Effort:** 4 days
- **Dependencies:** All CLOUD-* pages
- **Acceptance Criteria:**
  - 5+ critical user flow tests pass reliably (< 1% flake rate)
  - Tests run in CI in < 3 minutes (parallel execution)
  - Visual regression tests for key pages (overview, run detail, schema explorer)
  - Tests run against a seeded database (deterministic data)
  - Accessibility checks included in E2E tests (`@axe-core/playwright`)
  - Test reports are uploaded as CI artifacts with screenshots on failure
- **Best Practices:**
  - Use Playwright's `test.describe` to group related tests
  - Use `page.waitForLoadState('networkidle')` judiciously (prefer waiting for specific elements)
  - Seed test data via API calls in `beforeAll`, not via UI clicks
  - Use `data-testid` attributes for test selectors (not CSS classes)

**Section 3 Subtotal: ~54.5 engineering-days**

---

## Section 4: PR Bot / GitHub Integration

### GHBOT-001: GitHub App Registration & OAuth Flow
- **Description:** Create a GitHub App (not OAuth App — GitHub Apps have finer-grained permissions). Configure: permissions (pull_requests: write, checks: write, contents: read, metadata: read), webhook events (pull_request.opened, pull_request.synchronize, check_suite.requested). Implement the OAuth installation flow: user installs app on their repo -> webhook fires -> store installation ID and access token in database.
- **Estimated Effort:** 3 days
- **Dependencies:** API-001
- **Acceptance Criteria:**
  - GitHub App is registered with correct permissions and events
  - Installation flow works: install on repo -> webhook received -> installation stored in DB
  - Access token refresh works (GitHub App tokens expire every hour)
  - App can be installed on individual repos or entire organizations
  - Uninstall webhook cleans up stored tokens
  - Installation ID is linked to ValiForge Cloud project
- **Best Practices:**
  - Use the `octokit` Rust crate (or `@octokit/app` if using TypeScript worker)
  - Store private key in a secrets manager (not environment variable)
  - Implement webhook signature verification (SHA-256 HMAC)
  - Rate limit handling: respect `X-RateLimit-Remaining` headers

### GHBOT-002: PR Comment with Validation Results
- **Description:** When a PR is opened or updated on a repo with ValiForge installed, run validation against the PR's API spec changes and post a summary comment. Comment format: header with ValiForge branding, pass/fail badge, summary stats (endpoints validated, violations found, breaking changes), expandable details per violation, and a link to the full run in ValiForge Cloud. Update the same comment on subsequent pushes (don't create new comments).
- **Estimated Effort:** 4 days
- **Dependencies:** GHBOT-001, API-004
- **Acceptance Criteria:**
  - Comment is posted within 60 seconds of PR open/push
  - Comment updates on subsequent pushes (uses `comment_id` to update, not create)
  - Markdown renders correctly in GitHub (tables, code blocks, badges)
  - Comment includes a "View Full Report" link to ValiForge Cloud
  - If no API spec changes are detected, comment says "No API schema changes detected"
  - Comment is posted from the ValiForge bot account (not a personal account)
- **Best Practices:**
  - Use GitHub's Markdown collapse (`<details><summary>`) for verbose output
  - Include run duration and ValiForge version in the comment footer
  - Handle large outputs: if > 65536 chars (GitHub comment limit), truncate with link to full report
  - Use emoji sparingly for status: checkmark for pass, X for fail (accessibility: don't rely on color alone)

### GHBOT-003: Inline PR Annotations for Breaking Changes
- **Description:** Use GitHub's Check Runs API to add inline annotations on the PR diff. When ValiForge detects a breaking change (e.g., removed endpoint, changed response type), annotate the exact line in the diff where the change occurs. Annotations include: severity (error/warning), description of the breaking change, and the impact assessment.
- **Estimated Effort:** 3 days
- **Dependencies:** GHBOT-001, API-004
- **Acceptance Criteria:**
  - Annotations appear inline on the correct file and line in the PR diff
  - Severity levels map to GitHub annotation levels (failure, warning, notice)
  - Each annotation has a descriptive message (not just "breaking change detected")
  - Annotations link to the relevant docs page explaining the breaking change type
  - Maximum 50 annotations per run (GitHub limit) — prioritize by severity
- **Best Practices:**
  - Use the Checks API (not the deprecated Status API) for annotations
  - Include the old and new schema snippet in the annotation message for context
  - Group related annotations (e.g., "3 endpoints affected by this schema change")
  - Test with YAML and JSON spec files (line numbers differ)

### GHBOT-004: Status Checks (Pass/Fail)
- **Description:** Report ValiForge validation results as a GitHub Status Check. The check blocks PR merge if validation fails (configurable: can be set to "required" or "informational" in repo settings). Check states: pending (validation running), success (all pass), failure (violations found), action_required (breaking changes detected, needs review). Include a summary and link to the full report.
- **Estimated Effort:** 2 days
- **Dependencies:** GHBOT-001
- **Acceptance Criteria:**
  - Check appears in the PR "Checks" tab with correct status
  - Check name is "ValiForge Validation" (configurable via `.github/valiforge.yml`)
  - Check summary shows: pass/fail counts, duration, link to Cloud report
  - "Required check" setting works: PR cannot merge if ValiForge check fails
  - If validation times out (> 5 minutes), check status is set to "timed_out"
  - Re-running the check (via GitHub UI) triggers a new validation run
- **Best Practices:**
  - Use `checks:write` permission (already configured in GHBOT-001)
  - Set check status to "in_progress" immediately (not "queued") so users see it's running
  - Include timing information: "Completed in 12.3s"
  - Handle race conditions: if a new push arrives while a check is running, cancel the old run

### GHBOT-005: Configuration via `.github/valiforge.yml`
- **Description:** Support repo-level configuration via `.github/valiforge.yml`. Configuration options: spec file path(s), validation profile (strict/moderate/lenient), breaking change sensitivity, ignored paths/endpoints, failure threshold (e.g., "fail if > 5 violations"), environment variables for spec resolution, and custom validation rules.
- **Estimated Effort:** 2 days
- **Dependencies:** GHBOT-001
- **Acceptance Criteria:**
  - Bot reads `.github/valiforge.yml` from the PR's head branch (not base)
  - Invalid YAML produces a clear error comment on the PR
  - Missing config file uses sensible defaults (auto-detect spec file, moderate strictness)
  - Config supports glob patterns for spec files (`specs/**/*.yaml`)
  - Config is validated against a JSON Schema (published for IDE autocompletion)
  - Changes to the config file in a PR take effect for that PR's validation
- **Best Practices:**
  - Provide a `valiforge.yml` JSON Schema for VS Code autocompletion
  - Document every config option with defaults and examples
  - Support environment variable interpolation in config (`spec: ${API_SPEC_URL}`)
  - Include a `valiforge validate --config` command to test the config locally

### GHBOT-006: GitHub App Dashboard Integration
- **Description:** Add a "GitHub Integration" section to the ValiForge Cloud project settings. Features: (1) "Install GitHub App" button (links to GitHub App installation page), (2) Connected repositories list, (3) Per-repo configuration override (UI for `.github/valiforge.yml`), (4) Recent PR checks with status, (5) Disconnect repository.
- **Estimated Effort:** 2 days
- **Dependencies:** GHBOT-001, CLOUD-010
- **Acceptance Criteria:**
  - Install button deep-links to GitHub App installation page with pre-selected repos
  - Connected repos show last check status and date
  - Configuration UI generates valid YAML (download or copy to clipboard)
  - Disconnect removes the installation and stops webhook processing
  - UI shows installation status: "Connected", "Pending", "Error"
- **Best Practices:**
  - Cache GitHub API responses (repos list changes infrequently)
  - Show the GitHub App installation status in the project overview sidebar
  - Link to GitHub's "Installed GitHub Apps" page for users who need to manage permissions
  - Handle the case where the app is uninstalled from GitHub directly (webhook cleanup)

**Section 4 Subtotal: ~16 engineering-days**

---

## Section 5: API Server (Backend-for-Frontend)

### API-001: Axum Server Scaffold
- **Description:** Initialize the Axum API server within the Cargo workspace. Configure: router with versioned API routes (`/api/v1/`), structured logging (tracing + tracing-subscriber), CORS middleware (allow dashboard origin), request ID middleware (UUID per request), graceful shutdown, health check endpoint (`/healthz`), and OpenAPI spec generation via `utoipa`.
- **Estimated Effort:** 3 days
- **Dependencies:** SITE-001
- **Acceptance Criteria:**
  - Server starts in < 500ms and responds to `/healthz` with 200
  - Every request logs: request ID, method, path, status code, duration
  - CORS allows requests from `localhost:3000` (dev) and `app.valiforge.dev` (prod)
  - OpenAPI spec is auto-generated and served at `/api/v1/openapi.json`
  - Graceful shutdown: in-flight requests complete, new requests get 503
  - Request ID is returned in `X-Request-Id` response header
- **Best Practices:**
  - Use `tower-http` for CORS, compression, request ID, and tracing middleware
  - Structure routes as `Router::new().nest("/api/v1", v1_routes())`
  - Use `#[derive(Deserialize)]` with `serde` validation for all request bodies
  - Configure `tower::limit::RateLimitLayer` at the middleware level

### API-002: Database Schema & Migrations
- **Description:** Design and implement the Postgres schema for ValiForge Cloud. Core tables: `organizations`, `users`, `org_memberships` (with role), `projects`, `validation_runs`, `run_endpoints` (individual endpoint results), `violations`, `api_keys`, `webhook_configs`, `invitations`, `audit_logs`. Use `sqlx` for compile-time checked queries. Implement Row-Level Security (RLS) for multi-tenant isolation.
- **Estimated Effort:** 5 days
- **Dependencies:** API-001
- **Acceptance Criteria:**
  - All tables have: `id` (UUID v7), `created_at`, `updated_at` columns
  - Foreign keys enforce referential integrity
  - RLS policies ensure users can only access their organization's data
  - Indexes on: `org_id` (all tables), `project_id` (runs), `created_at` (runs, for time-range queries)
  - Migration files are idempotent and reversible (`up.sql` + `down.sql`)
  - `sqlx` compile-time checking validates all queries against the schema
  - Seed script creates a demo organization with sample data for development
- **Best Practices:**
  - Use UUID v7 (time-sortable) instead of v4 for primary keys
  - Store validation results in JSONB columns (flexible schema for different violation types)
  - Partition `validation_runs` by `created_at` (monthly) for query performance at scale
  - Use `CHECK` constraints for enums (role, status) instead of Postgres ENUM type (easier to migrate)

### API-003: Project CRUD API
- **Description:** RESTful API for project management. Endpoints: `POST /api/v1/projects` (create), `GET /api/v1/projects` (list, paginated), `GET /api/v1/projects/:id` (detail), `PATCH /api/v1/projects/:id` (update), `DELETE /api/v1/projects/:id` (soft delete). Each endpoint validates input with Zod-equivalent Rust validation (via `validator` crate), checks authorization (user must be org member), and returns consistent response envelopes.
- **Estimated Effort:** 3 days
- **Dependencies:** API-001, API-002
- **Acceptance Criteria:**
  - All endpoints require authentication (JWT in `Authorization: Bearer` header)
  - Create: validates required fields (name, spec_url), returns 201 with created project
  - List: supports pagination (`?page=1&per_page=20`), sorting (`?sort=created_at:desc`), filtering (`?status=active`)
  - Detail: returns 404 for non-existent or unauthorized projects (no information leakage)
  - Update: partial update (PATCH semantics), returns 200 with updated project
  - Delete: soft delete (sets `deleted_at`), returns 204
  - All responses follow envelope: `{ "data": {...}, "meta": {...} }`
- **Best Practices:**
  - Use `axum::extract::Path`, `Query`, `Json` extractors for type-safe parameter handling
  - Implement cursor-based pagination (not offset) for stable pagination under concurrent writes
  - Return `ETag` headers for GET responses (enables conditional requests)
  - Log all write operations to `audit_logs` table

### API-004: Validation Run API
- **Description:** API for triggering and viewing validation runs. Endpoints: `POST /api/v1/projects/:id/runs` (trigger), `GET /api/v1/projects/:id/runs` (list), `GET /api/v1/runs/:id` (detail with endpoints and violations), `GET /api/v1/runs/:id/events` (SSE stream for real-time updates). The trigger endpoint: fetches the project's API spec, invokes `valiforge-core::validate()`, stores results, and streams progress via SSE.
- **Estimated Effort:** 5 days
- **Dependencies:** API-001, API-002, API-003, ValiForge core crate
- **Acceptance Criteria:**
  - Trigger endpoint returns 202 (accepted) with a run ID immediately (validation runs async)
  - Run detail includes: summary stats, list of endpoints with results, violations with full detail
  - SSE stream sends events: `run.started`, `endpoint.validated` (per endpoint), `run.completed`
  - List endpoint supports: time range filter, status filter, pagination
  - Concurrent runs on the same project are queued (max 1 active per project)
  - Run timeout: 5 minutes max, after which the run is marked as `timed_out`
- **Best Practices:**
  - Use Tokio `spawn` for async run execution (don't block the request handler)
  - Store run progress in Redis (or Postgres LISTEN/NOTIFY) for SSE fan-out
  - Implement idempotency key on the trigger endpoint (prevent duplicate runs from retry)
  - Return `Retry-After` header on 429 (rate limited) responses

### API-005: Schema & Coverage API
- **Description:** API endpoints for schema exploration and coverage data. Endpoints: `GET /api/v1/projects/:id/schema` (full parsed schema with coverage annotations), `GET /api/v1/projects/:id/coverage` (coverage summary: total endpoints, tested endpoints, percentage), `GET /api/v1/projects/:id/breaking-changes` (list of detected breaking changes across runs).
- **Estimated Effort:** 3 days
- **Dependencies:** API-003, API-004
- **Acceptance Criteria:**
  - Schema endpoint returns the OpenAPI spec parsed into a structured JSON format with coverage annotations per endpoint
  - Coverage endpoint returns: overall percentage, per-method breakdown, untested endpoints list
  - Breaking changes endpoint: paginated, sortable by severity and date, includes the diff for each change
  - Schema is cached (invalidated when a new spec version is detected)
  - Response includes the spec version (hash) for cache busting
- **Best Practices:**
  - Parse and cache the OpenAPI spec on upload/fetch (don't re-parse on every request)
  - Use ETags for schema caching (spec hash as ETag value)
  - Breaking changes: store as a diff (old value, new value, path) for efficient querying
  - Coverage calculation: run as a background job after each validation run

### API-006: Test Data API
- **Description:** API for managing generated test fixtures. Endpoints: `POST /api/v1/projects/:id/fixtures/generate` (trigger generation), `GET /api/v1/projects/:id/fixtures` (list fixture sets), `GET /api/v1/fixtures/:id` (fixture detail), `GET /api/v1/fixtures/:id/download` (download as JSON/YAML/CSV). Generation invokes `valiforge-core::generate()` with the project's schema and specified strategy.
- **Estimated Effort:** 3 days
- **Dependencies:** API-003, ValiForge core crate
- **Acceptance Criteria:**
  - Generate endpoint accepts: strategy (random, edge-case, boundary), count, seed, and target schemas
  - Generated fixtures are stored in Postgres (JSONB) with metadata
  - List endpoint supports: pagination, filter by schema, filter by strategy
  - Download endpoint supports: JSON, YAML, and CSV content types (via `Accept` header)
  - Large fixture sets (> 1000) are generated async and notify via SSE when complete
  - Fixture generation respects schema constraints (minLength, pattern, enum, etc.)
- **Best Practices:**
  - Limit fixture count per generation: max 10,000 per request
  - Store fixture metadata (schema, strategy, seed) for reproducibility
  - Use streaming response for large downloads (don't buffer entire fixture set in memory)
  - Implement S3/GCS storage for fixtures > 10MB (don't store in Postgres)

### API-007: Team & Organization API
- **Description:** API for organization and team management. Endpoints: `POST /api/v1/orgs` (create org), `GET /api/v1/orgs/:id/members` (list members), `POST /api/v1/orgs/:id/invitations` (invite member), `PATCH /api/v1/orgs/:id/members/:user_id` (change role), `DELETE /api/v1/orgs/:id/members/:user_id` (remove member), `POST /api/v1/invitations/:token/accept` (accept invitation).
- **Estimated Effort:** 3 days
- **Dependencies:** API-001, API-002
- **Acceptance Criteria:**
  - Invitation sends email via Resend/SendGrid with a unique token (expires in 7 days)
  - Accept invitation: if user exists, add to org; if new, create user and add to org
  - Role changes are immediate and enforced on next API call
  - Remove member: cannot remove the last admin, cannot remove yourself if you're the only admin
  - List members: includes role, status (active/pending), last active date
  - All team management actions require admin role
- **Best Practices:**
  - Use a transactional email service (Resend) for reliable delivery
  - Invitation tokens: cryptographically random, 32 bytes, URL-safe base64
  - Rate limit invitations: max 20 per hour per org (prevent spam)
  - Implement `transfer_ownership` as a separate, explicitly confirmed action

### API-008: Settings & API Keys API
- **Description:** API for user settings, API keys, and webhooks. Endpoints: `PATCH /api/v1/users/me` (update profile), `POST /api/v1/api-keys` (create), `GET /api/v1/api-keys` (list, keys masked), `DELETE /api/v1/api-keys/:id` (revoke), `POST /api/v1/webhooks` (create), `GET /api/v1/webhooks` (list), `DELETE /api/v1/webhooks/:id` (delete), `POST /api/v1/webhooks/:id/test` (send test event).
- **Estimated Effort:** 3 days
- **Dependencies:** API-001, API-002
- **Acceptance Criteria:**
  - API keys are generated as: `vf_sk_` + 32 random bytes (base62 encoded)
  - Key is returned in full only on creation; list endpoint returns only last 6 chars
  - Key hash (SHA-256) is stored in DB (not plaintext)
  - Authentication via API key works: `Authorization: Bearer vf_sk_...`
  - Webhooks: URL is validated (HTTPS required in production, HTTP allowed in dev)
  - Test webhook sends a sample `run.completed` event and returns the response status
  - API key supports optional expiry date and IP allowlist
- **Best Practices:**
  - Never log API keys (even in debug mode)
  - API key lookup: use a hash index for O(1) lookup of the key hash
  - Webhook delivery: retry 3 times with exponential backoff (1s, 10s, 60s)
  - Webhook payloads include: event type, timestamp, signature (HMAC-SHA256)

### API-009: SSE Event System
- **Description:** Implement a centralized Server-Sent Events system for real-time updates. Architecture: validation engine publishes events to an in-process broadcast channel (Tokio `broadcast`); SSE endpoint subscribes to the channel and filters events by user/org/project permissions. Events: `run.started`, `run.progress`, `endpoint.completed`, `run.completed`, `run.failed`, `breaking-change.detected`.
- **Estimated Effort:** 3 days
- **Dependencies:** API-001, API-004
- **Acceptance Criteria:**
  - SSE endpoint at `GET /api/v1/events` requires authentication
  - Events are filtered by user's organization (no cross-tenant leakage)
  - Supports `Last-Event-ID` for reconnection (replays missed events from a buffer)
  - Heartbeat sent every 30 seconds to keep connection alive
  - Max 1000 concurrent SSE connections per server instance
  - Events are JSON-encoded with `id`, `event`, `data`, and `retry` fields
- **Best Practices:**
  - Use Tokio `broadcast` channel with a buffer of 256 events
  - Implement connection draining on server shutdown (send `close` event)
  - Monitor active SSE connections (metric: `sse_active_connections`)
  - Use `Cache-Control: no-cache` and `Connection: keep-alive` headers

### API-010: Rate Limiting & Security Middleware
- **Description:** Implement rate limiting, input validation, and security middleware. Rate limits: 100 req/min for authenticated users, 20 req/min for unauthenticated, 1000 req/min for API key access. Additional middleware: request body size limit (1MB), SQL injection protection (parameterized queries via sqlx), OWASP security headers (Strict-Transport-Security, X-Content-Type-Options, X-Frame-Options).
- **Estimated Effort:** 2 days
- **Dependencies:** API-001
- **Acceptance Criteria:**
  - Rate limit exceeded returns 429 with `Retry-After` header
  - Rate limits are tracked per user ID (authenticated) or IP (unauthenticated)
  - Request body > 1MB returns 413
  - All security headers are present on every response
  - CORS preflight (`OPTIONS`) requests are not rate-limited
  - Rate limit state is stored in Redis (shared across server instances)
- **Best Practices:**
  - Use `tower::limit::RateLimitLayer` or a custom Redis-based limiter (token bucket algorithm)
  - Return `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset` headers
  - Exempt health check and metrics endpoints from rate limiting
  - Implement IP-based blocking for abuse (configurable blocklist)

### API-011: Multi-Tenant Data Isolation
- **Description:** Ensure strict data isolation between organizations. Implement at two layers: (1) Application layer — every query includes `org_id` in the WHERE clause (enforced via an `OrgScope` extractor that injects the org_id from the JWT), (2) Database layer — Postgres Row-Level Security (RLS) policies as a defense-in-depth measure. Audit: add a CI test that runs queries without org_id and verifies they fail.
- **Estimated Effort:** 2 days
- **Dependencies:** API-002
- **Acceptance Criteria:**
  - No API endpoint can return data from another organization (verified by integration tests)
  - RLS policies are enabled on all tenant-scoped tables
  - A test suite specifically targets multi-tenant isolation (create 2 orgs, verify no cross-access)
  - `OrgScope` extractor is used in every route handler (enforced by code review checklist)
  - Queries without `org_id` filter fail at the RLS level (defense in depth)
- **Best Practices:**
  - Use a `set_config('app.current_org_id', ...)` call at the start of each request (for RLS)
  - Implement an integration test that creates two orgs and attempts cross-access on every endpoint
  - Consider `org_id` in all unique indexes (e.g., project name is unique per org, not globally)
  - Log (and alert on) any RLS policy violation as a potential security incident

### API-012: API Versioning Strategy
- **Description:** Implement URL-based API versioning (`/api/v1/`, `/api/v2/`). Versioning policy: breaking changes only in major versions, additive changes (new fields, new endpoints) are added to the current version. Deprecation: old versions serve a `Sunset` header with the deprecation date. Implement a version negotiation middleware that routes requests to the correct handler set.
- **Estimated Effort:** 1.5 days
- **Dependencies:** API-001
- **Acceptance Criteria:**
  - `/api/v1/` and `/api/v2/` route to separate handler sets
  - Deprecated versions return `Sunset: <date>` and `Deprecation: true` headers
  - API responses include `API-Version: v1` header
  - Unknown version returns 404 with helpful message ("Supported versions: v1, v2")
  - Version routing is implemented as Axum nested routers (not if/else in handlers)
- **Best Practices:**
  - Document the versioning policy in the API reference
  - Maintain both versions for at least 6 months after deprecation
  - Use feature flags internally (not code duplication) to manage version differences
  - Monitor v1 usage to determine safe sunset date

**Section 5 Subtotal: ~36.5 engineering-days**

---

## Section 6: Billing & Subscription Management

### BILL-001: Stripe Integration Setup
- **Description:** Configure Stripe for subscription billing. Set up: Stripe products (Community, Team, Business, Enterprise), prices (monthly and annual for each), tax settings (Stripe Tax for automatic tax calculation), customer portal (for self-service plan management), and webhook endpoint for Stripe events. Use Stripe's Customer Portal for invoice history and payment method management.
- **Estimated Effort:** 3 days
- **Dependencies:** API-001
- **Acceptance Criteria:**
  - Four products created in Stripe with monthly and annual prices
  - Annual prices reflect 20% discount
  - Webhook endpoint receives and verifies Stripe events (signature verification)
  - Customer portal is configured and accessible from the dashboard
  - Stripe Tax is enabled for automatic tax calculation
  - Test mode is used for development; live mode for production (separate API keys)
- **Best Practices:**
  - Use Stripe's idempotency keys for all API calls
  - Store Stripe customer ID in the `organizations` table (not users — billing is per org)
  - Use Stripe CLI for local webhook testing (`stripe listen --forward-to localhost:8080`)
  - Never expose Stripe secret key to the frontend; all Stripe API calls go through the BFF

### BILL-002: Subscription Management API
- **Description:** API endpoints for subscription lifecycle. Endpoints: `POST /api/v1/billing/checkout` (create Stripe Checkout Session for new subscription), `GET /api/v1/billing/subscription` (current plan, status, next billing date), `POST /api/v1/billing/portal` (create Stripe Customer Portal session for plan changes/cancellation), `GET /api/v1/billing/usage` (current period usage: runs, SLM tokens, team size).
- **Estimated Effort:** 3 days
- **Dependencies:** BILL-001, API-001
- **Acceptance Criteria:**
  - Checkout Session redirects user to Stripe-hosted checkout page
  - After successful payment, webhook updates org's plan in database
  - Subscription status is cached and refreshed from Stripe on each billing page load
  - Portal session allows: upgrade, downgrade, cancel, update payment method, view invoices
  - Usage endpoint returns: runs this period, SLM tokens used, seats used vs. limit
  - Downgrade: takes effect at end of billing period (Stripe handles proration)
- **Best Practices:**
  - Use Stripe Checkout (hosted) instead of custom payment form (reduces PCI scope to SAQ-A)
  - Handle the `customer.subscription.updated` webhook to sync plan changes to DB
  - Cache subscription data with a 5-minute TTL (reduce Stripe API calls)
  - Implement grace period: on payment failure, give 7 days before restricting access

### BILL-003: Usage-Based Metering
- **Description:** Implement usage tracking for metered features: validation runs (per run), SLM inference tokens (per 1K tokens), and team seats (per user). Report usage to Stripe Metering API at the end of each billing period (or in real-time for overage protection). Display usage in the dashboard with progress bars showing usage vs. plan limit.
- **Estimated Effort:** 4 days
- **Dependencies:** BILL-001, BILL-002, API-004
- **Acceptance Criteria:**
  - Every validation run increments the usage counter (stored in Redis for speed, persisted to Postgres)
  - SLM token usage is tracked per request and aggregated per billing period
  - Usage dashboard shows: bar chart of daily usage, cumulative usage vs. limit, projected usage at current rate
  - Overage protection: when usage hits 80% of limit, email notification; at 100%, either block or charge overage (configurable per plan)
  - Usage data is reported to Stripe for invoice line items
  - Usage resets at the start of each billing period
- **Best Practices:**
  - Use Redis INCR for atomic usage counting (no race conditions)
  - Batch usage reports to Stripe (every hour, not per event) to reduce API calls
  - Store usage snapshots for historical reporting (don't just rely on Stripe)
  - Implement a usage estimation algorithm: "At this rate, you'll hit your limit in X days"

### BILL-004: Plan Tiers & Feature Gating
- **Description:** Implement feature gating based on subscription plan. Feature matrix: Community (5 projects, 100 runs/month, no SLM, 1 user), Team (unlimited projects, 1000 runs/month, SLM basic, 10 users), Business (unlimited everything, SLM advanced, 50 users, SSO), Enterprise (custom limits, on-prem, SLA). Gate features in both frontend (hide/disable UI) and backend (reject API calls).
- **Estimated Effort:** 3 days
- **Dependencies:** BILL-002
- **Acceptance Criteria:**
  - Feature gates are defined in a central config (not scattered across codebase)
  - Frontend: disabled features show a tooltip "Upgrade to Team plan to unlock"
  - Backend: gated endpoints return 403 with `{ "error": "upgrade_required", "required_plan": "team" }`
  - Plan limits are enforced: Community user creating 6th project gets an error
  - Upgrade CTA is contextual (shown when a gated feature is attempted)
  - Feature flags can be overridden per org (for sales-led trials)
- **Best Practices:**
  - Implement feature gates as middleware (Axum) that checks the org's plan before executing the handler
  - Use a typed feature flag system (not string comparisons): `Feature::SLMInference.requires(Plan::Team)`
  - Cache the org's plan in the JWT claims (avoid DB lookup on every request)
  - Log all feature gate rejections (useful for product analytics: "X% of free users tried SLM")

### BILL-005: Trial Period Implementation
- **Description:** Implement a 14-day free trial for the Team plan. Trial starts when a user creates an organization. During the trial: all Team features are available, usage limits are Team-tier. At trial end: prompt to subscribe (banner + email sequence). If no subscription: downgrade to Community plan. Trial state: `trialing`, `trial_expired`, `active`, `past_due`, `canceled`.
- **Estimated Effort:** 2 days
- **Dependencies:** BILL-002, BILL-004
- **Acceptance Criteria:**
  - New organizations start with a 14-day Team trial (no credit card required)
  - Trial countdown is visible in the dashboard header ("12 days left in trial")
  - At 3 days remaining: email reminder with upgrade CTA
  - At trial end: email notification, dashboard shows upgrade prompt
  - Post-trial: graceful degradation (existing data is preserved, but new Team features are gated)
  - Trial can be extended by admin (for sales-led deals)
- **Best Practices:**
  - Use Stripe's built-in trial period for Checkout Sessions
  - Don't delete or hide data when trial expires (frustrating UX)
  - Show what the user will lose: "Your 3 SLM-powered validations will stop working"
  - A/B test trial length (14 days vs. 30 days) to optimize conversion

### BILL-006: Billing Dashboard Page
- **Description:** Build the billing page in the Cloud dashboard (`/settings/billing`). Sections: (1) Current plan card (name, price, billing cycle, next payment date), (2) Usage meter bars (runs, tokens, seats) with limits, (3) Plan comparison table with upgrade/downgrade buttons, (4) Payment method (last 4 digits, expiry, update link), (5) Invoice history table, (6) "Manage Billing" button (opens Stripe Customer Portal).
- **Estimated Effort:** 3 days
- **Dependencies:** BILL-002, BILL-003, CLOUD-003
- **Acceptance Criteria:**
  - Current plan is prominently displayed with a visual indicator (badge)
  - Usage bars show percentage with color coding (green < 50%, yellow < 80%, red >= 80%)
  - Plan comparison table matches the marketing site pricing section (single source of truth)
  - Upgrade button creates a Checkout Session; downgrade button opens confirmation dialog
  - Invoice history shows: date, amount, status (paid/pending/failed), PDF download link
  - Non-admin users see billing info but cannot make changes
- **Best Practices:**
  - Use Stripe's Customer Portal for payment method updates (reduces PCI scope)
  - Fetch invoice list from Stripe API (not stored locally — Stripe is the source of truth)
  - Show annual savings prominently when on a monthly plan ("Save $X by switching to annual")
  - Implement a "Contact Sales" CTA on the Enterprise card (opens Calendly or form)

**Section 6 Subtotal: ~18 engineering-days**

---

## Section 7: Enterprise Features

### ENT-001: SAML SSO Integration
- **Description:** Implement SAML 2.0 SSO for enterprise customers. Support: IdP-initiated and SP-initiated flows, integration with major IdPs (Okta, Azure AD, OneLogin, Google Workspace). Use a SAML library (`samael` in Rust or integrate with WorkOS/Clerk Enterprise SSO). Configuration: per-org SSO settings (IdP metadata URL, certificate, entity ID). Enforce SSO: when enabled, email/password login is disabled for that org.
- **Estimated Effort:** 6 days
- **Dependencies:** API-001, CLOUD-002
- **Acceptance Criteria:**
  - SP-initiated flow: user visits `/login` -> enters email -> redirected to IdP -> authenticated -> redirected back
  - IdP-initiated flow: user clicks ValiForge tile in IdP dashboard -> authenticated
  - SSO works with Okta, Azure AD, and Google Workspace (tested)
  - When SSO is enabled for an org, non-SSO login is blocked (except for a break-glass admin account)
  - JIT (Just-In-Time) provisioning: new users from the IdP are auto-created in ValiForge
  - SAML assertion is validated: signature, expiry, audience, issuer
- **Best Practices:**
  - Consider WorkOS as a SAML abstraction layer (handles IdP quirks, reduces engineering effort by ~60%)
  - Implement SSO configuration in a separate admin page (not exposed to non-enterprise plans)
  - Store SAML certificates encrypted at rest
  - Test with all major IdPs: each has subtle differences in SAML implementation
  - Implement SCIM provisioning as a follow-up (ENT-002) for user sync

### ENT-002: SCIM User Provisioning
- **Description:** Implement SCIM 2.0 for automated user lifecycle management. When an enterprise customer adds/removes/modifies a user in their IdP, the change is automatically synced to ValiForge. SCIM endpoints: `/scim/v2/Users` (CRUD), `/scim/v2/Groups` (map to ValiForge roles). This eliminates manual user management for large organizations.
- **Estimated Effort:** 4 days
- **Dependencies:** ENT-001
- **Acceptance Criteria:**
  - SCIM endpoints conform to RFC 7643 and RFC 7644
  - User creation via SCIM creates a ValiForge user and adds to the org
  - User deactivation via SCIM removes the user from the org (soft delete)
  - Group mapping: IdP group -> ValiForge role (configurable)
  - SCIM bearer token authentication (per org)
  - Tested with Okta and Azure AD SCIM provisioning
- **Best Practices:**
  - Use WorkOS SCIM if also using WorkOS for SSO (single integration)
  - SCIM endpoints must be idempotent (IdPs may retry)
  - Log all SCIM operations to audit log
  - Rate limit SCIM endpoints (IdPs can be chatty during initial sync)

### ENT-003: Role-Based Access Control (RBAC)
- **Description:** Implement granular RBAC beyond the basic Admin/Member/Viewer model. Roles: Organization Owner (super admin), Project Admin (manage specific projects), Team Member (run validations, view results), Viewer (read-only access), Billing Admin (manage billing only). Permissions are checked at both frontend (UI visibility) and backend (API authorization). Custom roles are available for Enterprise plans.
- **Estimated Effort:** 4 days
- **Dependencies:** API-007, CLOUD-009
- **Acceptance Criteria:**
  - Five built-in roles with documented permission matrices
  - Permissions are granular: `project.create`, `project.delete`, `run.trigger`, `billing.manage`, etc.
  - Frontend: UI elements are hidden/disabled based on user's effective permissions
  - Backend: unauthorized API calls return 403 with specific permission required
  - Custom roles (Enterprise only): admin can create roles with specific permission sets
  - Role assignment is per-org (a user can have different roles in different orgs)
  - Permission checks are cached (not a DB query per check)
- **Best Practices:**
  - Use a permission model (not role model) internally — roles are just permission bundles
  - Implement `can(user, 'project.delete', project)` helper function used across all handlers
  - Cache permissions in the JWT or a short-lived Redis cache (invalidate on role change)
  - Document the permission matrix in the admin docs

### ENT-004: Audit Logging System
- **Description:** Comprehensive audit logging for enterprise compliance. Log: who did what, when, on which resource, from which IP, with what result. Events: login/logout, project CRUD, validation runs triggered, team member changes, role changes, billing changes, API key creation/revocation, SSO configuration changes. Audit logs are immutable (append-only table, no UPDATE/DELETE).
- **Estimated Effort:** 4 days
- **Dependencies:** API-002
- **Acceptance Criteria:**
  - Every write operation creates an audit log entry
  - Audit log entry schema: `id`, `org_id`, `actor_id`, `action`, `resource_type`, `resource_id`, `metadata` (JSONB), `ip_address`, `user_agent`, `created_at`
  - Audit logs are queryable via API: `GET /api/v1/audit-logs?action=project.deleted&actor=user_123&from=2026-01-01`
  - Dashboard page for audit logs with search, filter, and CSV export
  - Audit logs are retained for 1 year (configurable per org for enterprise)
  - Audit log table is append-only (no UPDATE/DELETE permissions for the application DB user)
- **Best Practices:**
  - Implement as Axum middleware (not manually in each handler)
  - Use a separate DB user for audit writes (with only INSERT permission on the audit table)
  - Include `before` and `after` values for update operations (diff tracking)
  - Consider shipping audit logs to an external SIEM (Datadog, Splunk) for enterprise customers

### ENT-005: Data Retention Policies
- **Description:** Implement configurable data retention for enterprise compliance. Default retention: 90 days for validation runs, 30 days for generated test data, 1 year for audit logs. Enterprise customers can: configure custom retention periods, enable permanent retention, and manually trigger data purge. Implement a background job that runs daily to clean up expired data.
- **Estimated Effort:** 3 days
- **Dependencies:** API-002, ENT-004
- **Acceptance Criteria:**
  - Background job runs daily and deletes data older than the retention period
  - Deletion is soft-delete first (30-day grace period), then hard-delete
  - Enterprise customers can configure retention per data type
  - Retention settings are configurable in the dashboard settings page
  - Data export: before deletion, data can be exported (JSON archive)
  - Deletion respects foreign key constraints (delete in correct order)
  - Audit log entry is created for every retention-based deletion
- **Best Practices:**
  - Use Postgres partitioning by date for efficient bulk deletion (`DROP PARTITION` instead of row-by-row DELETE)
  - Run retention jobs during off-peak hours (configurable schedule)
  - Notify the org admin 7 days before a major deletion
  - Implement a "legal hold" flag that prevents deletion (for compliance)

### ENT-006: On-Premises Deployment (Helm Chart)
- **Description:** Package ValiForge Cloud for on-premises deployment via Helm chart. Components: API server (Axum), PostgreSQL (or bring-your-own), Redis (or bring-your-own), Next.js dashboard (containerized). Helm chart supports: custom domain, TLS configuration, resource limits, external database connection, external Redis connection, SSO configuration, and air-gapped installation.
- **Estimated Effort:** 6 days
- **Dependencies:** All API-* and CLOUD-* tasks
- **Acceptance Criteria:**
  - `helm install valiforge-cloud ./charts/valiforge-cloud` deploys a working instance on any Kubernetes 1.27+ cluster
  - Helm values support: custom domain, TLS cert, external Postgres/Redis, resource requests/limits
  - All containers pass Trivy vulnerability scan (zero critical/high CVEs)
  - Health checks and readiness probes are configured for all pods
  - Horizontal Pod Autoscaler (HPA) is configured for the API server
  - Air-gapped installation: all images are available from a private registry
  - Upgrade path: `helm upgrade` preserves data and configuration
  - Documentation: deployment guide with prerequisites, installation, configuration, and troubleshooting
- **Best Practices:**
  - Use multi-stage Docker builds for minimal image sizes (< 100MB for API server)
  - Pin all image tags to digests (not `:latest`)
  - Use Kubernetes secrets (or external secrets operator) for sensitive config
  - Include a `helm test` that validates the installation (health check + sample API call)
  - Provide a `values.production.yaml` example with recommended production settings

### ENT-007: Enterprise Admin Dashboard
- **Description:** Build an enterprise-specific admin section in the dashboard (`/admin`). Features: organization overview (all orgs for self-hosted), user management across orgs, system health dashboard (API latency, DB connections, queue depth), license management (seat count, expiry), SSO/SCIM configuration, and data retention settings. This page is only visible to users with the "System Admin" role.
- **Estimated Effort:** 4 days
- **Dependencies:** ENT-001, ENT-003, ENT-004, CLOUD-003
- **Acceptance Criteria:**
  - Admin dashboard shows: system health metrics, active users, validation runs (24h), storage usage
  - User management: search, view, deactivate, change role across all orgs
  - License info: current plan, seat usage vs. limit, expiry date, renewal CTA
  - SSO config: upload IdP metadata, test connection, enable/disable SSO
  - Data retention: configure retention periods, view upcoming deletions
  - Page is protected by System Admin role (not just org admin)
- **Best Practices:**
  - Use Prometheus metrics exposed at `/metrics` for system health data
  - Implement real-time system health via SSE (not polling)
  - Admin actions require re-authentication (step-up auth)
  - Separate the admin API routes from the regular API routes (`/api/v1/admin/`)

**Section 7 Subtotal: ~31 engineering-days**

---

## Total Effort Summary

| Section | Engineering-Days |
|---------|-----------------|
| 1. Marketing Website | 35.0 |
| 2. Documentation | 29.0 |
| 3. Cloud Dashboard | 54.5 |
| 4. PR Bot / GitHub Integration | 16.0 |
| 5. API Server (BFF) | 36.5 |
| 6. Billing & Subscription | 18.0 |
| 7. Enterprise Features | 31.0 |
| **TOTAL** | **220.0** |

**Note:** Add 25% buffer for integration testing, bug fixes, code review cycles, and unforeseen complexity: **220 * 1.25 = 275 engineering-days (~55 person-weeks)**.

With a team of **3 engineers**, this is approximately **18.3 weeks of parallel work** assuming 100% allocation (unrealistic). At **80% productive allocation** (meetings, on-call, etc.), expect **~23 weeks to GA**.

---

## Critical Path Analysis

The critical path (longest dependency chain determining minimum calendar time):

```
SITE-001 (3d) → API-001 (3d) → API-002 (5d) → API-003 (3d) → API-004 (5d)
    → CLOUD-001 (2d) → CLOUD-002 (4d) → CLOUD-003 (3d) → CLOUD-004 (4d)
    → CLOUD-006 (5d) → CLOUD-012 (3d) → CLOUD-016 (4d)
```

**Critical path duration: ~44 engineering-days (~9 weeks)**

Parallelizable work off the critical path:
- Marketing site (SITE-002 through SITE-013) can proceed in parallel with API development
- Documentation (DOCS-001 through DOCS-011) is fully independent
- GitHub Bot (GHBOT-001 through GHBOT-006) can proceed after API-004
- Billing (BILL-001 through BILL-006) can proceed after API-001
- Enterprise (ENT-001 through ENT-007) can proceed after the core dashboard is complete

---

## Sprint Plan (16 Sprints / 32 Weeks)

Each sprint is 2 weeks. Team: Engineer A (Frontend), Engineer B (Full-Stack), Engineer C (Backend/Infra).

### Phase 1: Foundation (Sprints 1-4, Weeks 1-8)

**Sprint 1 — Scaffolding & Design System**
| Engineer | Tasks | Days |
|----------|-------|------|
| A | SITE-001 (monorepo), SITE-002 (design system) | 7 |
| B | CLOUD-001 (Next.js scaffold), CLOUD-011 (dark mode) | 4 |
| C | API-001 (Axum scaffold), API-002 (DB schema) | 8 |

**Sprint 2 — Marketing Site & Auth**
| Engineer | Tasks | Days |
|----------|-------|------|
| A | SITE-003 (hero), SITE-004 (features), SITE-005 (benchmarks) | 8 |
| B | CLOUD-002 (auth), CLOUD-003 (layout shell) | 7 |
| C | API-003 (project CRUD), API-010 (security middleware) | 5 |

**Sprint 3 — Marketing Site Complete & Dashboard Start**
| Engineer | Tasks | Days |
|----------|-------|------|
| A | SITE-006 (pricing), SITE-007 (testimonials), SITE-008 (footer), SITE-009 (SEO) | 7 |
| B | CLOUD-004 (overview dashboard) | 4 |
| C | API-004 (validation run API), API-009 (SSE system) | 8 |

**Sprint 4 — Docs Foundation & Core Dashboard Pages**
| Engineer | Tasks | Days |
|----------|-------|------|
| A | DOCS-001 (Starlight setup), DOCS-002 (getting started), SITE-010 (analytics) | 5 |
| B | CLOUD-005 (project pages) | 6 |
| C | API-005 (schema/coverage), API-007 (team API) | 6 |

### Phase 2: Core Product (Sprints 5-8, Weeks 9-16)

**Sprint 5 — Dashboard Deep Pages**
| Engineer | Tasks | Days |
|----------|-------|------|
| A | DOCS-003 (CLI ref), DOCS-004 (config ref) | 5 |
| B | CLOUD-006 (run detail), CLOUD-012 (SSE integration) | 8 |
| C | API-006 (test data API), API-008 (settings API) | 6 |

**Sprint 6 — Schema Explorer & Team Management**
| Engineer | Tasks | Days |
|----------|-------|------|
| A | DOCS-005 (CI/CD guide), DOCS-006 (test data guide) | 5 |
| B | CLOUD-007 (schema explorer), CLOUD-008 (test data viewer) | 8 |
| C | API-011 (multi-tenant isolation), API-012 (versioning) | 3.5 |

**Sprint 7 — GitHub Bot & Settings**
| Engineer | Tasks | Days |
|----------|-------|------|
| A | DOCS-007 (breaking changes), DOCS-008 (migration guides) | 6 |
| B | CLOUD-009 (team management), CLOUD-010 (settings) | 7 |
| C | GHBOT-001 (GitHub App), GHBOT-004 (status checks) | 5 |

**Sprint 8 — WASM Playground & GitHub Bot Complete**
| Engineer | Tasks | Days |
|----------|-------|------|
| A | SITE-011 (WASM playground) | 8 |
| B | GHBOT-006 (dashboard integration), CLOUD-014 (error handling) | 4 |
| C | GHBOT-002 (PR comments), GHBOT-003 (annotations), GHBOT-005 (config) | 9 |

**--- MVP / Cloud Beta Release (Week 16) ---**

### Phase 3: Monetization (Sprints 9-12, Weeks 17-24)

**Sprint 9 — Billing Foundation**
| Engineer | Tasks | Days |
|----------|-------|------|
| A | SITE-012 (blog), SITE-013 (performance audit) | 4 |
| B | BILL-006 (billing page) | 3 |
| C | BILL-001 (Stripe setup), BILL-002 (subscription API) | 6 |

**Sprint 10 — Billing Complete & Docs Polish**
| Engineer | Tasks | Days |
|----------|-------|------|
| A | DOCS-009 (API ref), DOCS-010 (tutorials, 2 of 4) | 5 |
| B | BILL-004 (feature gating frontend), BILL-005 (trial) | 5 |
| C | BILL-003 (usage metering) | 4 |

**Sprint 11 — Testing & Polish**
| Engineer | Tasks | Days |
|----------|-------|------|
| A | DOCS-010 (tutorials, 2 of 4), DOCS-011 (versioned docs) | 5 |
| B | CLOUD-013 (responsive design), CLOUD-015 (Sentry) | 4.5 |
| C | CLOUD-016 (E2E tests) | 4 |

**Sprint 12 — Integration Testing & Bug Fixes**
| Engineer | Tasks | Days |
|----------|-------|------|
| A | Integration testing, bug fixes, performance tuning | 10 |
| B | Integration testing, bug fixes, UX polish | 10 |
| C | Load testing, security audit, infrastructure hardening | 10 |

**--- GA Release (Week 24) ---**

### Phase 4: Enterprise (Sprints 13-16, Weeks 25-32)

**Sprint 13 — SSO & RBAC**
| Engineer | Tasks | Days |
|----------|-------|------|
| A | ENT-007 (enterprise admin dashboard) | 4 |
| B | ENT-003 (RBAC frontend) | 4 |
| C | ENT-001 (SAML SSO), ENT-003 (RBAC backend) | 10 |

**Sprint 14 — SCIM & Audit Logs**
| Engineer | Tasks | Days |
|----------|-------|------|
| A | ENT-007 (enterprise admin dashboard, cont.) | 4 |
| B | ENT-004 (audit log UI), ENT-005 (retention UI) | 4 |
| C | ENT-002 (SCIM), ENT-004 (audit log backend) | 8 |

**Sprint 15 — On-Prem & Retention**
| Engineer | Tasks | Days |
|----------|-------|------|
| A | ENT-006 documentation, installation guide testing | 5 |
| B | ENT-005 (retention backend) | 3 |
| C | ENT-006 (Helm chart) | 6 |

**Sprint 16 — Enterprise Polish & GA**
| Engineer | Tasks | Days |
|----------|-------|------|
| A | Enterprise E2E testing, bug fixes | 10 |
| B | Enterprise E2E testing, SSO testing with IdPs | 10 |
| C | Security hardening, penetration testing, enterprise deployment testing | 10 |

**--- Enterprise GA (Week 32) ---**

---

## Key Risks & Mitigations

### Risk 1: WASM Binary Size (SITE-011)
- **Likelihood:** High
- **Impact:** High (playground is a key differentiator)
- **Description:** `valiforge-core` compiled to WASM may exceed 2MB, causing slow load times.
- **Mitigation:** (1) Use `wasm-opt -Oz` and `wasm-snip` to remove dead code. (2) Feature-flag the crate to exclude non-essential modules (SLM inference, test generation) from the WASM build. (3) Use streaming compilation (`WebAssembly.instantiateStreaming`) for faster startup. (4) Set a binary size budget of 500KB gzipped; if exceeded, explore a server-side fallback.

### Risk 2: Stripe Billing Complexity (BILL-001 through BILL-006)
- **Likelihood:** Medium
- **Impact:** High (revenue depends on correct billing)
- **Description:** Usage-based billing with metering is notoriously complex. Edge cases: proration on mid-cycle plan changes, failed payments, subscription pauses, tax calculation, currency conversion.
- **Mitigation:** (1) Start with simple flat-rate pricing (no usage metering) for MVP. (2) Add usage metering in Phase 3 once the core billing works. (3) Extensive testing with Stripe Test Clocks (simulate billing cycles). (4) Hire a billing consultant for the first iteration.

### Risk 3: SAML SSO IdP Variability (ENT-001)
- **Likelihood:** High
- **Impact:** Medium (enterprise feature, but blockers for large deals)
- **Description:** Every IdP implements SAML slightly differently. Okta, Azure AD, OneLogin, and Ping all have quirks.
- **Mitigation:** (1) Use WorkOS or Clerk Enterprise SSO as an abstraction layer. (2) Budget 2 extra days for IdP-specific debugging. (3) Maintain test accounts on all major IdPs. (4) Provide a detailed SSO setup guide per IdP.

### Risk 4: Real-Time SSE Scalability (API-009, CLOUD-012)
- **Likelihood:** Medium
- **Impact:** Medium
- **Description:** SSE connections are long-lived. With 1000 concurrent dashboard users, the server holds 1000 open connections.
- **Mitigation:** (1) Use connection pooling: max 1 SSE connection per browser tab. (2) Implement connection limits per user (max 3). (3) Use a Redis PubSub fan-out pattern so SSE connections are distributed across server instances. (4) Fall back to polling if SSE connections are exhausted.

### Risk 5: Multi-Tenant Data Leakage (API-011)
- **Likelihood:** Low (with proper implementation)
- **Impact:** Critical (security incident, customer trust destruction)
- **Description:** A bug in a query could expose one organization's data to another.
- **Mitigation:** (1) Postgres RLS as defense-in-depth (even if application layer bugs, DB layer blocks). (2) Mandatory `org_id` in every query (enforced by the `OrgScope` extractor). (3) Dedicated integration test suite for multi-tenant isolation. (4) Quarterly security audit of all data-access patterns. (5) Bug bounty program.

### Risk 6: Scope Creep in Documentation (DOCS-*)
- **Likelihood:** High
- **Impact:** Low-Medium (delays, but docs can ship incrementally)
- **Description:** Documentation is unbounded work. Migration guides, tutorials, and reference docs can always be expanded.
- **Mitigation:** (1) Define "minimum viable docs" for each milestone (Getting Started + CLI Reference for MVP). (2) Prioritize by user journey: new user onboarding first, advanced topics later. (3) Track docs coverage metrics (% of CLI commands documented, % of config options documented). (4) Accept that docs are never "done" — budget ongoing effort.

### Risk 7: Next.js 15 App Router Maturity
- **Likelihood:** Medium
- **Impact:** Medium
- **Description:** App Router patterns (Server Components, Server Actions, streaming) are still evolving. Some libraries may not fully support RSC.
- **Mitigation:** (1) Use Client Components as escape hatch for interactive sections. (2) Evaluate Remix or SvelteKit as fallback if critical issues are found in Sprint 1. (3) Pin Next.js to a specific patch version and upgrade cautiously. (4) Monitor the Next.js GitHub issues for App Router regressions.

### Risk 8: GitHub App Rate Limits (GHBOT-*)
- **Likelihood:** Medium
- **Impact:** Medium (degraded experience for high-volume repos)
- **Description:** GitHub App rate limits are 5000 requests/hour per installation. A large org with many repos could exhaust this.
- **Mitigation:** (1) Cache GitHub API responses aggressively (installation tokens, repo metadata). (2) Batch API calls where possible (create check + annotations in minimal calls). (3) Implement request queuing with exponential backoff. (4) Monitor rate limit headers and alert at 80% usage.

---

## Appendix: Architecture Decision Records (ADRs)

### ADR-001: Monorepo vs. Polyrepo
- **Decision:** Monorepo (Turborepo + Cargo workspace)
- **Rationale:** Shared design tokens, atomic PRs across frontend + backend, unified CI, simplified onboarding for new engineers.
- **Trade-offs:** Larger repo size, longer CI times (mitigated by Turborepo caching), potential for coupling.

### ADR-002: Astro for Marketing vs. Next.js for Everything
- **Decision:** Separate Astro (marketing) + Next.js (dashboard)
- **Rationale:** Marketing site benefits from Astro's zero-JS static output (Lighthouse 95+). Dashboard needs React ecosystem (shadcn/ui, TanStack Query, Recharts). Using Next.js for marketing would ship unnecessary JS.
- **Trade-offs:** Two frameworks to maintain, slight learning curve for Astro. Mitigated by the fact that marketing site changes are infrequent after launch.

### ADR-003: Clerk vs. Custom Auth
- **Decision:** Clerk for MVP, evaluate migration to custom for Enterprise
- **Rationale:** Clerk eliminates 15+ days of auth engineering. Provides: hosted UI, GitHub/Google OAuth, JWT verification, org management, Webhook sync. For Enterprise SAML, Clerk's Enterprise SSO or WorkOS can be added.
- **Trade-offs:** Vendor dependency, recurring cost (~$25/month + per-MAU). If Clerk becomes a bottleneck (pricing, features), migration to custom auth is feasible but expensive (~3 weeks).

### ADR-004: SSE vs. WebSocket for Real-Time
- **Decision:** Server-Sent Events
- **Rationale:** All real-time data flows are server-to-client. SSE is simpler (HTTP, auto-reconnect, no handshake), works through corporate proxies, and is natively supported by Axum.
- **Trade-offs:** No bidirectional communication. If future features need client-to-server real-time (e.g., collaborative editing), WebSocket would be needed. Can be added alongside SSE.

### ADR-005: PostgreSQL vs. PlanetScale/Turso
- **Decision:** PostgreSQL 16
- **Rationale:** RLS for multi-tenancy, JSONB for flexible validation results, pgvector for future semantic search, mature tooling (sqlx, pgAdmin), on-prem deployment requirement.
- **Trade-offs:** Requires managing Postgres (or using a managed service like Neon/Supabase). PlanetScale would offer better edge performance but lacks RLS and JSONB flexibility.

---

## Open Questions (To Resolve Before Sprint 1)

1. **WASM Feasibility:** Can `valiforge-core` compile to WASM with acceptable binary size (< 500KB gzipped)? Needs a spike (1-2 days).
2. **Hosting Provider:** Vercel (easy Next.js deployment) vs. Fly.io (Rust-native, closer to metal) vs. GCP Cloud Run (cost-effective, Kubernetes-compatible)?
3. **SLM Inference Hosting:** Where does the SLM (Small Language Model) run? GPU inference on Fly.io/Modal, or partner with an inference provider?
4. **Pricing Validation:** Are the proposed price points ($29/$79/custom) validated with potential customers? Needs customer interviews.
5. **Legal:** Privacy policy, terms of service, DPA (Data Processing Agreement) for enterprise — who drafts these?
6. **Brand:** Is the "electric indigo" brand color finalized? Is the ValiForge logo ready?
7. **Domain:** Is `valiforge.dev` registered? Also need `app.valiforge.dev` for the dashboard.

---

*End of Engineering Plan. Next step: review with the team, resolve open questions, and kick off Sprint 1.*
