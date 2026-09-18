# ValiForge — Frontend & Cloud Implementation Guide

**For: Senior Frontend/Cloud Engineer**
**Goal: Build the cloud dashboard, marketing site, API server, and billing layer**

---

## 1. Tech Stack

| Layer | Technology | Purpose |
|-------|-----------|---------|
| Marketing/Docs | Astro 5 + Starlight + Tailwind CSS | Static site, docs, blog |
| Dashboard | Next.js 15 (App Router) + TypeScript + shadcn/ui | Cloud management UI |
| API Server | Axum (Rust) + Tower middleware | Backend-for-frontend |
| Database | PostgreSQL 16 + sqlx | Multi-tenant data |
| Auth | Clerk (MVP) → custom OIDC (Enterprise) | Authentication |
| Payments | Stripe Billing + Metering | Subscriptions |
| Real-time | Server-Sent Events (Axum SSE) | Live run updates |
| Hosting | Vercel (frontend) + Cloud Run (API) | Deployment |

---

## 2. Monorepo Structure

```
valiforge/
├── crates/                    # Rust workspace (CLI + core engine)
│   ├── valiforge-cli/
│   ├── valiforge-core/
│   ├── valiforge-cloud/       # Axum API server (NEW)
│   └── ...
├── apps/
│   ├── www/                   # Astro marketing + docs site
│   └── dashboard/             # Next.js cloud dashboard
├── packages/
│   └── ui/                    # Shared design tokens + components
├── turbo.json
├── pnpm-workspace.yaml
└── package.json
```

### `pnpm-workspace.yaml`

```yaml
packages:
  - "apps/*"
  - "packages/*"
```

### Root `package.json`

```json
{
  "name": "valiforge",
  "private": true,
  "scripts": {
    "dev": "turbo dev",
    "build": "turbo build",
    "lint": "turbo lint",
    "typecheck": "turbo typecheck"
  },
  "devDependencies": {
    "turbo": "^2.3",
    "typescript": "^5.7"
  },
  "packageManager": "pnpm@9.15.0"
}
```

### `turbo.json`

```json
{
  "$schema": "https://turbo.build/schema.json",
  "globalDependencies": ["**/.env.*local"],
  "tasks": {
    "build": {
      "dependsOn": ["^build"],
      "outputs": [".next/**", "!.next/cache/**", "dist/**"]
    },
    "dev": {
      "cache": false,
      "persistent": true
    },
    "lint": {},
    "typecheck": {
      "dependsOn": ["^build"]
    }
  }
}
```

---

## 3. Database Schema (PostgreSQL)

### `crates/valiforge-cloud/migrations/001_initial.sql`

```sql
-- Enable extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Organizations (multi-tenant root)
CREATE TABLE organizations (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    plan TEXT NOT NULL DEFAULT 'free' CHECK (plan IN ('free', 'pro', 'team', 'enterprise')),
    stripe_customer_id TEXT,
    stripe_subscription_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_organizations_slug ON organizations(slug);

-- Users
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    external_id TEXT NOT NULL UNIQUE,  -- Clerk user ID
    email TEXT NOT NULL,
    name TEXT,
    role TEXT NOT NULL DEFAULT 'member' CHECK (role IN ('owner', 'admin', 'member', 'viewer')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_users_org ON users(org_id);
CREATE INDEX idx_users_external ON users(external_id);

-- API Keys
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    key_hash TEXT NOT NULL UNIQUE,  -- SHA-256 of the key
    key_prefix TEXT NOT NULL,       -- First 8 chars for display (vf_live_xxxx...)
    scopes TEXT[] NOT NULL DEFAULT '{"validate", "read"}',
    last_used_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_api_keys_org ON api_keys(org_id);
CREATE INDEX idx_api_keys_hash ON api_keys(key_hash);

-- Schemas (uploaded OpenAPI/Proto/GraphQL specs)
CREATE TABLE schemas (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    version TEXT NOT NULL DEFAULT '1.0.0',
    schema_type TEXT NOT NULL CHECK (schema_type IN ('openapi', 'protobuf', 'graphql')),
    content JSONB NOT NULL,
    file_hash TEXT NOT NULL,
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(org_id, name, version)
);

CREATE INDEX idx_schemas_org ON schemas(org_id);

-- Validation Runs
CREATE TABLE validation_runs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    schema_id UUID REFERENCES schemas(id),
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled')),
    target_url TEXT NOT NULL,
    config JSONB NOT NULL DEFAULT '{}',
    summary JSONB,       -- { pass: N, fail: N, skip: N, error: N }
    duration_ms INTEGER,
    triggered_by TEXT,   -- 'api', 'dashboard', 'ci', 'schedule'
    api_key_id UUID REFERENCES api_keys(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX idx_runs_org ON validation_runs(org_id);
CREATE INDEX idx_runs_status ON validation_runs(status);
CREATE INDEX idx_runs_created ON validation_runs(created_at DESC);

-- Validation Results (per-endpoint results within a run)
CREATE TABLE validation_results (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    run_id UUID NOT NULL REFERENCES validation_runs(id) ON DELETE CASCADE,
    method TEXT NOT NULL,
    path TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pass', 'fail', 'skip', 'error')),
    response_status INTEGER,
    latency_ms INTEGER,
    violations JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_results_run ON validation_results(run_id);

-- Usage Records (for metered billing)
CREATE TABLE usage_records (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    metric TEXT NOT NULL,  -- 'validation_run', 'endpoint_check', 'slm_generation'
    quantity INTEGER NOT NULL DEFAULT 1,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_usage_org_metric ON usage_records(org_id, metric, recorded_at);

-- Billing plans reference
CREATE TABLE billing_plans (
    id TEXT PRIMARY KEY,  -- 'free', 'pro', 'team', 'enterprise'
    name TEXT NOT NULL,
    price_monthly_cents INTEGER NOT NULL,
    max_runs_per_month INTEGER,       -- NULL = unlimited
    max_endpoints_per_run INTEGER,
    max_schemas INTEGER,
    max_team_members INTEGER,
    features JSONB NOT NULL DEFAULT '[]'
);

-- Seed billing plans
INSERT INTO billing_plans (id, name, price_monthly_cents, max_runs_per_month, max_endpoints_per_run, max_schemas, max_team_members, features) VALUES
    ('free',       'Free',       0,    50,   20,  3,   1,  '["cli", "json_report"]'),
    ('pro',        'Pro',        2900, 500,  100, 20,  1,  '["cli", "all_reports", "slm", "diff"]'),
    ('team',       'Team',       7900, 5000, 500, 100, 10, '["cli", "all_reports", "slm", "diff", "dashboard", "api"]'),
    ('enterprise', 'Enterprise', 0,    NULL, NULL, NULL, NULL, '["all", "sso", "audit_log", "sla"]');

-- Updated_at trigger
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_organizations_updated
    BEFORE UPDATE ON organizations FOR EACH ROW EXECUTE FUNCTION update_updated_at();
CREATE TRIGGER trg_users_updated
    BEFORE UPDATE ON users FOR EACH ROW EXECUTE FUNCTION update_updated_at();
```

---

## 4. Axum API Server (`crates/valiforge-cloud/`)

### `crates/valiforge-cloud/Cargo.toml`

```toml
[package]
name = "valiforge-cloud"
version.workspace = true
edition.workspace = true

[dependencies]
valiforge-core = { path = "../valiforge-core" }
valiforge-schema = { path = "../valiforge-schema" }
valiforge-report = { path = "../valiforge-report" }

tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
anyhow = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }

axum = { version = "0.8", features = ["macros"] }
axum-extra = { version = "0.10", features = ["typed-header"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace", "compression-gzip", "limit"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "uuid", "chrono", "json"] }
jsonwebtoken = "9"
sha2 = "0.10"
hex = "0.4"
rand = "0.8"
```

### `crates/valiforge-cloud/src/main.rs`

```rust
use anyhow::Result;
use axum::{Router, middleware};
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tower_http::cors::{CorsLayer, Any};
use tower_http::trace::TraceLayer;
use tower_http::compression::CompressionLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod auth;
mod error;
mod handlers;
mod middleware as mw;
mod models;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub jwt_secret: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "valiforge_cloud=info,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost/valiforge".into());

    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let state = AppState {
        db: pool,
        jwt_secret: std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "dev-secret-change-in-prod".into()),
    };

    let app = Router::new()
        .nest("/api/v1", api_routes())
        .with_state(state)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("ValiForge Cloud API listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn api_routes() -> Router<AppState> {
    Router::new()
        // Public
        .route("/health", axum::routing::get(handlers::health))
        // Authenticated
        .route("/validate", axum::routing::post(handlers::validate::create_run))
        .route("/runs", axum::routing::get(handlers::runs::list_runs))
        .route("/runs/{id}", axum::routing::get(handlers::runs::get_run))
        .route("/runs/{id}/results", axum::routing::get(handlers::runs::get_run_results))
        .route("/schemas", axum::routing::post(handlers::schemas::upload_schema))
        .route("/schemas", axum::routing::get(handlers::schemas::list_schemas))
        .route("/api-keys", axum::routing::post(handlers::api_keys::create_key))
        .route("/api-keys", axum::routing::get(handlers::api_keys::list_keys))
        .route("/usage", axum::routing::get(handlers::usage::get_usage))
}
```

### `crates/valiforge-cloud/src/error.rs`

```rust
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    BadRequest(String),
    Unauthorized,
    Forbidden,
    RateLimited,
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "Invalid or missing API key".into()),
            Self::Forbidden => (StatusCode::FORBIDDEN, "Insufficient permissions".into()),
            Self::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded".into()),
            Self::Internal(msg) => {
                tracing::error!("Internal error: {msg}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".into())
            }
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        Self::Internal(e.to_string())
    }
}
```

### `crates/valiforge-cloud/src/auth.rs`

```rust
use crate::error::ApiError;
use crate::AppState;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub org_id: Uuid,
    pub api_key_id: Uuid,
    pub scopes: Vec<String>,
}

/// Extract and validate API key from Authorization header
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Allow health check without auth
    if request.uri().path() == "/api/v1/health" {
        return Ok(next.run(request).await);
    }

    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;

    let api_key = auth_header
        .strip_prefix("Bearer ")
        .ok_or(ApiError::Unauthorized)?;

    // Hash the key and look up in database
    let key_hash = hex::encode(Sha256::digest(api_key.as_bytes()));

    let row = sqlx::query_as::<_, (Uuid, Uuid, Vec<String>)>(
        "SELECT id, org_id, scopes FROM api_keys WHERE key_hash = $1 AND (expires_at IS NULL OR expires_at > now())"
    )
    .bind(&key_hash)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::from)?
    .ok_or(ApiError::Unauthorized)?;

    // Update last_used_at
    let _ = sqlx::query("UPDATE api_keys SET last_used_at = now() WHERE id = $1")
        .bind(row.0)
        .execute(&state.db)
        .await;

    let ctx = AuthContext {
        org_id: row.1,
        api_key_id: row.0,
        scopes: row.2,
    };

    request.extensions_mut().insert(ctx);
    Ok(next.run(request).await)
}
```

### `crates/valiforge-cloud/src/handlers/validate.rs`

```rust
use crate::auth::AuthContext;
use crate::error::ApiError;
use crate::AppState;
use axum::extract::State;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreateRunRequest {
    pub schema_id: Option<Uuid>,
    pub schema_url: Option<String>,
    pub target_url: String,
    pub config: Option<RunConfig>,
}

#[derive(Deserialize, Default)]
pub struct RunConfig {
    pub fail_on: Option<String>,
    pub timeout_secs: Option<u64>,
    pub max_concurrency: Option<usize>,
    pub include_datagen: Option<bool>,
}

#[derive(Serialize)]
pub struct CreateRunResponse {
    pub id: Uuid,
    pub status: String,
    pub message: String,
}

pub async fn create_run(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreateRunRequest>,
) -> Result<Json<CreateRunResponse>, ApiError> {
    // Check plan limits
    let usage_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM validation_runs WHERE org_id = $1 AND created_at > date_trunc('month', now())"
    )
    .bind(auth.org_id)
    .fetch_one(&state.db)
    .await?;

    let plan: (Option<i32>,) = sqlx::query_as(
        "SELECT bp.max_runs_per_month FROM organizations o JOIN billing_plans bp ON o.plan = bp.id WHERE o.id = $1"
    )
    .bind(auth.org_id)
    .fetch_one(&state.db)
    .await?;

    if let Some(max) = plan.0 {
        if usage_count.0 >= i64::from(max) {
            return Err(ApiError::RateLimited);
        }
    }

    // Create the run record
    let run_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO validation_runs (id, org_id, schema_id, target_url, config, triggered_by, api_key_id)
         VALUES ($1, $2, $3, $4, $5, 'api', $6)"
    )
    .bind(run_id)
    .bind(auth.org_id)
    .bind(req.schema_id)
    .bind(&req.target_url)
    .bind(serde_json::to_value(&req.config).unwrap_or_default())
    .bind(auth.api_key_id)
    .execute(&state.db)
    .await?;

    // Record usage
    sqlx::query(
        "INSERT INTO usage_records (org_id, metric, quantity) VALUES ($1, 'validation_run', 1)"
    )
    .bind(auth.org_id)
    .execute(&state.db)
    .await?;

    // Spawn background validation task
    let db = state.db.clone();
    tokio::spawn(async move {
        if let Err(e) = execute_validation(db, run_id, req.target_url).await {
            tracing::error!("Validation run {run_id} failed: {e}");
        }
    });

    Ok(Json(CreateRunResponse {
        id: run_id,
        status: "pending".into(),
        message: "Validation run started".into(),
    }))
}

async fn execute_validation(
    db: sqlx::PgPool,
    run_id: Uuid,
    target_url: String,
) -> anyhow::Result<()> {
    // Update status to running
    sqlx::query("UPDATE validation_runs SET status = 'running' WHERE id = $1")
        .bind(run_id)
        .execute(&db)
        .await?;

    let start = std::time::Instant::now();

    // TODO: Load schema, run valiforge-core validation engine, store results
    // This is where valiforge-core::ValidationEngine gets called

    let duration = start.elapsed();

    // Update status to completed
    sqlx::query(
        "UPDATE validation_runs SET status = 'completed', duration_ms = $1, completed_at = now() WHERE id = $2"
    )
    .bind(duration.as_millis() as i32)
    .bind(run_id)
    .execute(&db)
    .await?;

    Ok(())
}
```

### `crates/valiforge-cloud/src/handlers/runs.rs`

```rust
use crate::auth::AuthContext;
use crate::error::ApiError;
use crate::AppState;
use axum::extract::{Path, Query, State};
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ListParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub status: Option<String>,
}

#[derive(Serialize)]
pub struct RunSummary {
    pub id: Uuid,
    pub status: String,
    pub target_url: String,
    pub summary: Option<serde_json::Value>,
    pub duration_ms: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
}

pub async fn list_runs(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Query(params): Query<ListParams>,
) -> Result<Json<PaginatedResponse<RunSummary>>, ApiError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).min(100);
    let offset = (page - 1) * per_page;

    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM validation_runs WHERE org_id = $1"
    )
    .bind(auth.org_id)
    .fetch_one(&state.db)
    .await?;

    let runs: Vec<RunSummary> = sqlx::query_as::<_, (Uuid, String, String, Option<serde_json::Value>, Option<i32>, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>)>(
        "SELECT id, status, target_url, summary, duration_ms, created_at, completed_at
         FROM validation_runs
         WHERE org_id = $1
         ORDER BY created_at DESC
         LIMIT $2 OFFSET $3"
    )
    .bind(auth.org_id)
    .bind(per_page as i64)
    .bind(offset as i64)
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(|r| RunSummary {
        id: r.0,
        status: r.1,
        target_url: r.2,
        summary: r.3,
        duration_ms: r.4,
        created_at: r.5,
        completed_at: r.6,
    })
    .collect();

    Ok(Json(PaginatedResponse {
        data: runs,
        total: total.0,
        page,
        per_page,
    }))
}

pub async fn get_run(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<RunSummary>, ApiError> {
    let run = sqlx::query_as::<_, (Uuid, String, String, Option<serde_json::Value>, Option<i32>, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>)>(
        "SELECT id, status, target_url, summary, duration_ms, created_at, completed_at
         FROM validation_runs
         WHERE id = $1 AND org_id = $2"
    )
    .bind(id)
    .bind(auth.org_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("Run {id} not found")))?;

    Ok(Json(RunSummary {
        id: run.0,
        status: run.1,
        target_url: run.2,
        summary: run.3,
        duration_ms: run.4,
        created_at: run.5,
        completed_at: run.6,
    }))
}

pub async fn get_run_results(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    // Verify run belongs to org
    let _run = sqlx::query("SELECT 1 FROM validation_runs WHERE id = $1 AND org_id = $2")
        .bind(id)
        .bind(auth.org_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Run {id} not found")))?;

    let results: Vec<(serde_json::Value,)> = sqlx::query_as(
        "SELECT jsonb_build_object(
            'id', id, 'method', method, 'path', path, 'status', status,
            'response_status', response_status, 'latency_ms', latency_ms,
            'violations', violations
         )
         FROM validation_results WHERE run_id = $1 ORDER BY path, method"
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(results.into_iter().map(|r| r.0).collect()))
}
```

### `crates/valiforge-cloud/src/handlers/api_keys.rs`

```rust
use crate::auth::AuthContext;
use crate::error::ApiError;
use crate::AppState;
use axum::extract::State;
use axum::{Extension, Json};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreateKeyRequest {
    pub name: String,
    pub scopes: Option<Vec<String>>,
    pub expires_in_days: Option<u32>,
}

#[derive(Serialize)]
pub struct CreateKeyResponse {
    pub id: Uuid,
    pub key: String,  // Only returned once!
    pub prefix: String,
    pub name: String,
    pub scopes: Vec<String>,
}

pub async fn create_key(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreateKeyRequest>,
) -> Result<Json<CreateKeyResponse>, ApiError> {
    // Generate a secure random API key
    let key_bytes: [u8; 32] = rand::thread_rng().gen();
    let raw_key = format!("vf_live_{}", hex::encode(key_bytes));
    let prefix = raw_key[..16].to_string();
    let key_hash = hex::encode(Sha256::digest(raw_key.as_bytes()));

    let scopes = req.scopes.unwrap_or_else(|| vec!["validate".into(), "read".into()]);
    let id = Uuid::new_v4();

    let expires_at = req.expires_in_days.map(|days| {
        chrono::Utc::now() + chrono::Duration::days(i64::from(days))
    });

    sqlx::query(
        "INSERT INTO api_keys (id, org_id, name, key_hash, key_prefix, scopes, expires_at, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, NULL)"
    )
    .bind(id)
    .bind(auth.org_id)
    .bind(&req.name)
    .bind(&key_hash)
    .bind(&prefix)
    .bind(&scopes)
    .bind(expires_at)
    .execute(&state.db)
    .await?;

    Ok(Json(CreateKeyResponse {
        id,
        key: raw_key,
        prefix,
        name: req.name,
        scopes,
    }))
}

#[derive(Serialize)]
pub struct KeyInfo {
    pub id: Uuid,
    pub name: String,
    pub prefix: String,
    pub scopes: Vec<String>,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_keys(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Vec<KeyInfo>>, ApiError> {
    let keys: Vec<KeyInfo> = sqlx::query_as::<_, (Uuid, String, String, Vec<String>, Option<chrono::DateTime<chrono::Utc>>, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, name, key_prefix, scopes, last_used_at, created_at
         FROM api_keys WHERE org_id = $1 ORDER BY created_at DESC"
    )
    .bind(auth.org_id)
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(|r| KeyInfo {
        id: r.0,
        name: r.1,
        prefix: r.2,
        scopes: r.3,
        last_used_at: r.4,
        created_at: r.5,
    })
    .collect();

    Ok(Json(keys))
}
```

---

## 5. Next.js Dashboard (`apps/dashboard/`)

### `apps/dashboard/package.json`

```json
{
  "name": "@valiforge/dashboard",
  "version": "0.1.0",
  "private": true,
  "scripts": {
    "dev": "next dev --port 3000",
    "build": "next build",
    "start": "next start",
    "lint": "next lint",
    "typecheck": "tsc --noEmit"
  },
  "dependencies": {
    "next": "^15.1",
    "react": "^19.0",
    "react-dom": "^19.0",
    "@clerk/nextjs": "^6.0",
    "recharts": "^2.15",
    "lucide-react": "^0.468",
    "date-fns": "^4.1",
    "zod": "^3.24",
    "class-variance-authority": "^0.7",
    "clsx": "^2.1",
    "tailwind-merge": "^2.6"
  },
  "devDependencies": {
    "@types/node": "^22",
    "@types/react": "^19",
    "typescript": "^5.7",
    "tailwindcss": "^4.0",
    "@tailwindcss/postcss": "^4.0"
  }
}
```

### `apps/dashboard/src/app/layout.tsx`

```tsx
import type { Metadata } from "next";
import { ClerkProvider } from "@clerk/nextjs";
import "./globals.css";

export const metadata: Metadata = {
  title: "ValiForge Cloud",
  description: "API Validation Dashboard",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <ClerkProvider>
      <html lang="en" className="dark">
        <body className="min-h-screen bg-gray-950 text-gray-100 antialiased">
          {children}
        </body>
      </html>
    </ClerkProvider>
  );
}
```

### `apps/dashboard/src/app/dashboard/layout.tsx`

```tsx
import { redirect } from "next/navigation";
import { auth } from "@clerk/nextjs/server";
import { Sidebar } from "@/components/sidebar";

export default async function DashboardLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const { userId } = await auth();
  if (!userId) redirect("/sign-in");

  return (
    <div className="flex h-screen">
      <Sidebar />
      <main className="flex-1 overflow-y-auto p-8">{children}</main>
    </div>
  );
}
```

### `apps/dashboard/src/components/sidebar.tsx`

```tsx
"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import {
  LayoutDashboard,
  Play,
  FileCode,
  Settings,
  CreditCard,
  Key,
} from "lucide-react";

const navItems = [
  { href: "/dashboard", label: "Overview", icon: LayoutDashboard },
  { href: "/dashboard/runs", label: "Validation Runs", icon: Play },
  { href: "/dashboard/schemas", label: "Schemas", icon: FileCode },
  { href: "/dashboard/api-keys", label: "API Keys", icon: Key },
  { href: "/dashboard/billing", label: "Billing", icon: CreditCard },
  { href: "/dashboard/settings", label: "Settings", icon: Settings },
];

export function Sidebar() {
  const pathname = usePathname();

  return (
    <aside className="flex w-64 flex-col border-r border-gray-800 bg-gray-900">
      <div className="flex h-16 items-center gap-2 border-b border-gray-800 px-6">
        <div className="h-8 w-8 rounded-lg bg-teal-500" />
        <span className="text-lg font-bold">ValiForge</span>
      </div>

      <nav className="flex-1 space-y-1 p-4">
        {navItems.map((item) => {
          const isActive =
            pathname === item.href ||
            (item.href !== "/dashboard" && pathname.startsWith(item.href));
          return (
            <Link
              key={item.href}
              href={item.href}
              className={`flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors ${
                isActive
                  ? "bg-teal-500/10 text-teal-400"
                  : "text-gray-400 hover:bg-gray-800 hover:text-gray-200"
              }`}
            >
              <item.icon className="h-4 w-4" />
              {item.label}
            </Link>
          );
        })}
      </nav>
    </aside>
  );
}
```

### `apps/dashboard/src/app/dashboard/page.tsx`

```tsx
import { StatsCards } from "@/components/stats-cards";
import { RecentRuns } from "@/components/recent-runs";
import { UsageChart } from "@/components/usage-chart";

export default function DashboardPage() {
  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-2xl font-bold">Dashboard</h1>
        <p className="text-gray-400">Overview of your API validation activity</p>
      </div>

      <StatsCards />

      <div className="grid grid-cols-1 gap-8 lg:grid-cols-2">
        <UsageChart />
        <RecentRuns />
      </div>
    </div>
  );
}
```

### `apps/dashboard/src/components/stats-cards.tsx`

```tsx
import { CheckCircle, XCircle, Clock, Zap } from "lucide-react";

interface StatCard {
  label: string;
  value: string;
  change: string;
  trend: "up" | "down";
  icon: React.ElementType;
  color: string;
}

const stats: StatCard[] = [
  { label: "Total Runs", value: "1,234", change: "+12%", trend: "up", icon: Zap, color: "text-blue-400" },
  { label: "Pass Rate", value: "94.2%", change: "+2.1%", trend: "up", icon: CheckCircle, color: "text-green-400" },
  { label: "Failures", value: "72", change: "-8%", trend: "down", icon: XCircle, color: "text-red-400" },
  { label: "Avg Duration", value: "1.8s", change: "-15%", trend: "down", icon: Clock, color: "text-yellow-400" },
];

export function StatsCards() {
  return (
    <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
      {stats.map((stat) => (
        <div
          key={stat.label}
          className="rounded-xl border border-gray-800 bg-gray-900 p-6"
        >
          <div className="flex items-center justify-between">
            <span className="text-sm text-gray-400">{stat.label}</span>
            <stat.icon className={`h-5 w-5 ${stat.color}`} />
          </div>
          <div className="mt-2 text-3xl font-bold">{stat.value}</div>
          <div className={`mt-1 text-sm ${
            stat.trend === "up" ? "text-green-400" : "text-red-400"
          }`}>
            {stat.change} from last month
          </div>
        </div>
      ))}
    </div>
  );
}
```

### `apps/dashboard/src/components/recent-runs.tsx`

```tsx
import Link from "next/link";

interface Run {
  id: string;
  target: string;
  status: "pass" | "fail" | "running";
  endpoints: number;
  duration: string;
  time: string;
}

const recentRuns: Run[] = [
  { id: "run_1", target: "api.example.com", status: "pass", endpoints: 24, duration: "1.2s", time: "2 min ago" },
  { id: "run_2", target: "staging.myapp.io", status: "fail", endpoints: 48, duration: "3.8s", time: "15 min ago" },
  { id: "run_3", target: "localhost:8080", status: "pass", endpoints: 12, duration: "0.6s", time: "1 hour ago" },
];

const statusColors = {
  pass: "bg-green-500/10 text-green-400",
  fail: "bg-red-500/10 text-red-400",
  running: "bg-yellow-500/10 text-yellow-400",
};

export function RecentRuns() {
  return (
    <div className="rounded-xl border border-gray-800 bg-gray-900 p-6">
      <h2 className="text-lg font-semibold">Recent Runs</h2>
      <div className="mt-4 space-y-3">
        {recentRuns.map((run) => (
          <Link
            key={run.id}
            href={`/dashboard/runs/${run.id}`}
            className="flex items-center justify-between rounded-lg border border-gray-800 p-4 transition-colors hover:border-gray-700"
          >
            <div>
              <div className="font-medium">{run.target}</div>
              <div className="text-sm text-gray-400">
                {run.endpoints} endpoints &middot; {run.duration}
              </div>
            </div>
            <div className="flex items-center gap-3">
              <span className={`rounded-full px-2.5 py-0.5 text-xs font-medium ${statusColors[run.status]}`}>
                {run.status}
              </span>
              <span className="text-sm text-gray-500">{run.time}</span>
            </div>
          </Link>
        ))}
      </div>
    </div>
  );
}
```

### `apps/dashboard/src/components/usage-chart.tsx`

```tsx
"use client";

import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from "recharts";

const data = [
  { date: "Mon", runs: 45 },
  { date: "Tue", runs: 62 },
  { date: "Wed", runs: 78 },
  { date: "Thu", runs: 55 },
  { date: "Fri", runs: 91 },
  { date: "Sat", runs: 23 },
  { date: "Sun", runs: 18 },
];

export function UsageChart() {
  return (
    <div className="rounded-xl border border-gray-800 bg-gray-900 p-6">
      <h2 className="text-lg font-semibold">Validation Runs This Week</h2>
      <div className="mt-4 h-64">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={data}>
            <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
            <XAxis dataKey="date" stroke="#9CA3AF" fontSize={12} />
            <YAxis stroke="#9CA3AF" fontSize={12} />
            <Tooltip
              contentStyle={{
                backgroundColor: "#1F2937",
                border: "1px solid #374151",
                borderRadius: "8px",
              }}
            />
            <Area
              type="monotone"
              dataKey="runs"
              stroke="#00D4AA"
              fill="#00D4AA"
              fillOpacity={0.1}
              strokeWidth={2}
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
```

### `apps/dashboard/src/lib/api-client.ts`

```typescript
const API_BASE = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/api/v1";

class ValiForgeApiClient {
  private apiKey: string;

  constructor(apiKey: string) {
    this.apiKey = apiKey;
  }

  private async fetch<T>(path: string, options?: RequestInit): Promise<T> {
    const res = await fetch(`${API_BASE}${path}`, {
      ...options,
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${this.apiKey}`,
        ...options?.headers,
      },
    });

    if (!res.ok) {
      const body = await res.json().catch(() => ({}));
      throw new ApiError(res.status, body.error || res.statusText);
    }

    return res.json();
  }

  // Validation runs
  async createRun(params: {
    targetUrl: string;
    schemaId?: string;
  }) {
    return this.fetch<{ id: string; status: string }>("/validate", {
      method: "POST",
      body: JSON.stringify({
        target_url: params.targetUrl,
        schema_id: params.schemaId,
      }),
    });
  }

  async listRuns(params?: { page?: number; perPage?: number; status?: string }) {
    const query = new URLSearchParams();
    if (params?.page) query.set("page", String(params.page));
    if (params?.perPage) query.set("per_page", String(params.perPage));
    if (params?.status) query.set("status", params.status);
    return this.fetch<PaginatedResponse<RunSummary>>(`/runs?${query}`);
  }

  async getRun(id: string) {
    return this.fetch<RunSummary>(`/runs/${id}`);
  }

  async getRunResults(id: string) {
    return this.fetch<EndpointResult[]>(`/runs/${id}/results`);
  }

  // Schemas
  async uploadSchema(params: { name: string; content: object; schemaType: string }) {
    return this.fetch<{ id: string }>("/schemas", {
      method: "POST",
      body: JSON.stringify(params),
    });
  }

  async listSchemas() {
    return this.fetch<Schema[]>("/schemas");
  }

  // API Keys
  async createApiKey(params: { name: string; scopes?: string[] }) {
    return this.fetch<{ id: string; key: string }>("/api-keys", {
      method: "POST",
      body: JSON.stringify(params),
    });
  }

  // Usage
  async getUsage() {
    return this.fetch<UsageStats>("/usage");
  }
}

class ApiError extends Error {
  constructor(public status: number, message: string) {
    super(message);
    this.name = "ApiError";
  }
}

// Types
interface PaginatedResponse<T> {
  data: T[];
  total: number;
  page: number;
  per_page: number;
}

interface RunSummary {
  id: string;
  status: string;
  target_url: string;
  summary: { pass: number; fail: number; skip: number; error: number } | null;
  duration_ms: number | null;
  created_at: string;
  completed_at: string | null;
}

interface EndpointResult {
  id: string;
  method: string;
  path: string;
  status: string;
  response_status: number;
  latency_ms: number;
  violations: Violation[];
}

interface Violation {
  rule: string;
  message: string;
  severity: string;
  path: string;
}

interface Schema {
  id: string;
  name: string;
  version: string;
  schema_type: string;
  created_at: string;
}

interface UsageStats {
  current_month: { runs: number; endpoints: number };
  plan_limits: { max_runs: number; max_endpoints: number };
}

export { ValiForgeApiClient, ApiError };
export type { RunSummary, EndpointResult, Violation, Schema, PaginatedResponse };
```

---

## 6. Astro Marketing Site (`apps/www/`)

### `apps/www/astro.config.mjs`

```javascript
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import tailwind from "@astrojs/tailwind";

export default defineConfig({
  site: "https://valiforge.dev",
  integrations: [
    starlight({
      title: "ValiForge",
      description: "Headless API validation engine for AI workloads",
      social: {
        github: "https://github.com/valiforge/valiforge",
        discord: "https://discord.gg/valiforge",
      },
      sidebar: [
        {
          label: "Getting Started",
          items: [
            { label: "Installation", slug: "guides/installation" },
            { label: "Quick Start", slug: "guides/quickstart" },
            { label: "Configuration", slug: "guides/configuration" },
          ],
        },
        {
          label: "Guides",
          items: [
            { label: "OpenAPI Validation", slug: "guides/openapi" },
            { label: "CI/CD Integration", slug: "guides/ci-cd" },
            { label: "Test Data Generation", slug: "guides/datagen" },
            { label: "Schema Diff", slug: "guides/diff" },
          ],
        },
        {
          label: "Reference",
          items: [
            { label: "CLI Reference", slug: "reference/cli" },
            { label: "Config File", slug: "reference/config" },
            { label: "Report Formats", slug: "reference/reports" },
          ],
        },
      ],
      customCss: ["./src/styles/custom.css"],
    }),
    tailwind({ applyBaseStyles: false }),
  ],
});
```

### `apps/www/src/pages/index.astro` (Landing Page)

```astro
---
import Layout from "../layouts/Landing.astro";
---

<Layout title="ValiForge — API Validation Engine">
  <!-- Hero -->
  <section class="relative overflow-hidden bg-gray-950 px-6 py-24 sm:py-32">
    <div class="mx-auto max-w-4xl text-center">
      <div class="mb-6 inline-flex items-center gap-2 rounded-full border border-teal-500/20 bg-teal-500/10 px-4 py-1.5 text-sm text-teal-400">
        <span>v0.1.0 — Now in Public Beta</span>
      </div>
      <h1 class="text-5xl font-bold tracking-tight text-white sm:text-7xl">
        Validate APIs at
        <span class="bg-gradient-to-r from-teal-400 to-cyan-400 bg-clip-text text-transparent">
          Rust Speed
        </span>
      </h1>
      <p class="mt-6 text-xl text-gray-400 max-w-2xl mx-auto">
        Headless, API-first validation engine for AI-generated code.
        Catch breaking changes, generate test data with local AI, and ship with confidence.
      </p>
      <div class="mt-10 flex flex-col items-center gap-4 sm:flex-row sm:justify-center">
        <div class="rounded-lg bg-gray-900 border border-gray-800 px-6 py-3 font-mono text-sm text-gray-300">
          curl -fsSL https://valiforge.dev/install.sh | sh
        </div>
        <a href="/docs" class="rounded-lg bg-teal-500 px-6 py-3 font-semibold text-gray-950 hover:bg-teal-400 transition-colors">
          Read the Docs →
        </a>
      </div>
    </div>
  </section>

  <!-- Features -->
  <section class="bg-gray-950 px-6 py-24 border-t border-gray-900">
    <div class="mx-auto max-w-6xl">
      <h2 class="text-center text-3xl font-bold text-white">Why ValiForge?</h2>
      <div class="mt-16 grid grid-cols-1 gap-8 md:grid-cols-3">
        <div class="rounded-xl border border-gray-800 bg-gray-900/50 p-8">
          <div class="mb-4 text-3xl">⚡</div>
          <h3 class="text-xl font-semibold text-white">Rust-Fast</h3>
          <p class="mt-2 text-gray-400">
            50x faster than Node-based tools. Validate 500 endpoints in under 2 seconds.
            Single binary, zero runtime dependencies.
          </p>
        </div>
        <div class="rounded-xl border border-gray-800 bg-gray-900/50 p-8">
          <div class="mb-4 text-3xl">🤖</div>
          <h3 class="text-xl font-semibold text-white">AI-Powered Test Data</h3>
          <p class="mt-2 text-gray-400">
            Generate realistic test payloads using local SLMs (Phi-4-mini).
            No data leaves your machine. Domain-aware for fintech, healthcare, e-commerce.
          </p>
        </div>
        <div class="rounded-xl border border-gray-800 bg-gray-900/50 p-8">
          <div class="mb-4 text-3xl">🔄</div>
          <h3 class="text-xl font-semibold text-white">CI/CD Native</h3>
          <p class="mt-2 text-gray-400">
            GitHub Action, GitLab CI, Jenkins. SARIF for GitHub Code Scanning.
            JUnit XML for any CI system. Fail builds on breaking changes.
          </p>
        </div>
      </div>
    </div>
  </section>

  <!-- Pricing -->
  <section class="bg-gray-950 px-6 py-24 border-t border-gray-900" id="pricing">
    <div class="mx-auto max-w-6xl">
      <h2 class="text-center text-3xl font-bold text-white">Simple Pricing</h2>
      <p class="mt-4 text-center text-gray-400">
        Open source CLI is free forever. Cloud features for teams.
      </p>
      <div class="mt-16 grid grid-cols-1 gap-8 md:grid-cols-4">
        <!-- Free -->
        <div class="rounded-xl border border-gray-800 bg-gray-900 p-8">
          <h3 class="text-lg font-semibold">Free</h3>
          <div class="mt-4 text-4xl font-bold">$0</div>
          <p class="mt-1 text-sm text-gray-400">forever</p>
          <ul class="mt-6 space-y-3 text-sm text-gray-300">
            <li>✓ CLI (all features)</li>
            <li>✓ 50 runs/month</li>
            <li>✓ JSON & terminal reports</li>
            <li>✓ Community support</li>
          </ul>
        </div>
        <!-- Pro -->
        <div class="rounded-xl border-2 border-teal-500 bg-gray-900 p-8 relative">
          <div class="absolute -top-3 left-1/2 -translate-x-1/2 rounded-full bg-teal-500 px-3 py-0.5 text-xs font-semibold text-gray-950">
            Popular
          </div>
          <h3 class="text-lg font-semibold">Pro</h3>
          <div class="mt-4 text-4xl font-bold">$29</div>
          <p class="mt-1 text-sm text-gray-400">/month</p>
          <ul class="mt-6 space-y-3 text-sm text-gray-300">
            <li>✓ Everything in Free</li>
            <li>✓ 500 runs/month</li>
            <li>✓ All report formats</li>
            <li>✓ SLM test generation</li>
            <li>✓ Schema diff</li>
          </ul>
        </div>
        <!-- Team -->
        <div class="rounded-xl border border-gray-800 bg-gray-900 p-8">
          <h3 class="text-lg font-semibold">Team</h3>
          <div class="mt-4 text-4xl font-bold">$79</div>
          <p class="mt-1 text-sm text-gray-400">/month per seat</p>
          <ul class="mt-6 space-y-3 text-sm text-gray-300">
            <li>✓ Everything in Pro</li>
            <li>✓ 5,000 runs/month</li>
            <li>✓ Cloud dashboard</li>
            <li>✓ Team management</li>
            <li>✓ API access</li>
          </ul>
        </div>
        <!-- Enterprise -->
        <div class="rounded-xl border border-gray-800 bg-gray-900 p-8">
          <h3 class="text-lg font-semibold">Enterprise</h3>
          <div class="mt-4 text-4xl font-bold">Custom</div>
          <p class="mt-1 text-sm text-gray-400">contact us</p>
          <ul class="mt-6 space-y-3 text-sm text-gray-300">
            <li>✓ Everything in Team</li>
            <li>✓ Unlimited runs</li>
            <li>✓ SSO / SAML</li>
            <li>✓ Audit log</li>
            <li>✓ SLA & support</li>
          </ul>
        </div>
      </div>
    </div>
  </section>
</Layout>
```

---

## 7. Stripe Billing Integration

### `crates/valiforge-cloud/src/handlers/billing.rs`

```rust
use crate::auth::AuthContext;
use crate::error::ApiError;
use crate::AppState;
use axum::extract::State;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CreateCheckoutRequest {
    pub plan: String,  // "pro" | "team"
    pub success_url: String,
    pub cancel_url: String,
}

#[derive(Serialize)]
pub struct CheckoutResponse {
    pub checkout_url: String,
}

/// Create a Stripe Checkout session for plan upgrade
pub async fn create_checkout(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreateCheckoutRequest>,
) -> Result<Json<CheckoutResponse>, ApiError> {
    let stripe_key = std::env::var("STRIPE_SECRET_KEY")
        .map_err(|_| ApiError::Internal("Stripe not configured".into()))?;

    // Map plan to Stripe Price ID
    let price_id = match req.plan.as_str() {
        "pro" => std::env::var("STRIPE_PRICE_PRO").unwrap_or_default(),
        "team" => std::env::var("STRIPE_PRICE_TEAM").unwrap_or_default(),
        _ => return Err(ApiError::BadRequest("Invalid plan".into())),
    };

    // Get or create Stripe customer
    let org: (Option<String>,) = sqlx::query_as(
        "SELECT stripe_customer_id FROM organizations WHERE id = $1"
    )
    .bind(auth.org_id)
    .fetch_one(&state.db)
    .await?;

    let customer_id = match org.0 {
        Some(id) => id,
        None => {
            // Create new Stripe customer via API
            let client = reqwest::Client::new();
            let resp: serde_json::Value = client
                .post("https://api.stripe.com/v1/customers")
                .header("Authorization", format!("Bearer {stripe_key}"))
                .form(&[("metadata[org_id]", auth.org_id.to_string())])
                .send()
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?
                .json()
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;

            let cid = resp["id"]
                .as_str()
                .ok_or_else(|| ApiError::Internal("Failed to create customer".into()))?
                .to_string();

            sqlx::query("UPDATE organizations SET stripe_customer_id = $1 WHERE id = $2")
                .bind(&cid)
                .bind(auth.org_id)
                .execute(&state.db)
                .await?;

            cid
        }
    };

    // Create Checkout session
    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .post("https://api.stripe.com/v1/checkout/sessions")
        .header("Authorization", format!("Bearer {stripe_key}"))
        .form(&[
            ("customer", customer_id.as_str()),
            ("mode", "subscription"),
            ("line_items[0][price]", &price_id),
            ("line_items[0][quantity]", "1"),
            ("success_url", &req.success_url),
            ("cancel_url", &req.cancel_url),
        ])
        .send()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .json()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let url = resp["url"]
        .as_str()
        .ok_or_else(|| ApiError::Internal("No checkout URL".into()))?;

    Ok(Json(CheckoutResponse {
        checkout_url: url.to_string(),
    }))
}

/// Stripe webhook handler for subscription events
pub async fn stripe_webhook(
    State(state): State<AppState>,
    body: String,
) -> Result<(), ApiError> {
    let event: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let event_type = event["type"].as_str().unwrap_or("");

    match event_type {
        "checkout.session.completed" => {
            let customer_id = event["data"]["object"]["customer"]
                .as_str()
                .unwrap_or("");
            let subscription_id = event["data"]["object"]["subscription"]
                .as_str()
                .unwrap_or("");

            sqlx::query(
                "UPDATE organizations SET stripe_subscription_id = $1, plan = 'pro' WHERE stripe_customer_id = $2"
            )
            .bind(subscription_id)
            .bind(customer_id)
            .execute(&state.db)
            .await?;
        }
        "customer.subscription.deleted" => {
            let customer_id = event["data"]["object"]["customer"]
                .as_str()
                .unwrap_or("");

            sqlx::query(
                "UPDATE organizations SET plan = 'free', stripe_subscription_id = NULL WHERE stripe_customer_id = $1"
            )
            .bind(customer_id)
            .execute(&state.db)
            .await?;
        }
        "customer.subscription.updated" => {
            // Handle plan changes, cancellations, etc.
            tracing::info!("Subscription updated for event: {event_type}");
        }
        _ => {
            tracing::debug!("Unhandled Stripe event: {event_type}");
        }
    }

    Ok(())
}
```

---

## 8. TypeScript SDK

### `packages/sdk/src/index.ts`

```typescript
export class ValiForgeClient {
  private baseUrl: string;
  private apiKey: string;

  constructor(apiKey: string, baseUrl = "https://api.valiforge.dev/v1") {
    this.apiKey = apiKey;
    this.baseUrl = baseUrl;
  }

  async validate(params: {
    schemaUrl?: string;
    schemaId?: string;
    targetUrl: string;
    failOn?: "error" | "warning" | "info";
  }): Promise<ValidationRun> {
    return this.post("/validate", {
      schema_url: params.schemaUrl,
      schema_id: params.schemaId,
      target_url: params.targetUrl,
      config: { fail_on: params.failOn },
    });
  }

  async getRun(id: string): Promise<ValidationRun> {
    return this.get(`/runs/${id}`);
  }

  async waitForRun(id: string, timeoutMs = 60000): Promise<ValidationRun> {
    const start = Date.now();
    while (Date.now() - start < timeoutMs) {
      const run = await this.getRun(id);
      if (run.status === "completed" || run.status === "failed") return run;
      await new Promise((r) => setTimeout(r, 1000));
    }
    throw new Error(`Run ${id} did not complete within ${timeoutMs}ms`);
  }

  async listRuns(params?: {
    page?: number;
    perPage?: number;
  }): Promise<PaginatedResponse<ValidationRun>> {
    const query = new URLSearchParams();
    if (params?.page) query.set("page", String(params.page));
    if (params?.perPage) query.set("per_page", String(params.perPage));
    return this.get(`/runs?${query}`);
  }

  private async get<T>(path: string): Promise<T> {
    const res = await fetch(`${this.baseUrl}${path}`, {
      headers: { Authorization: `Bearer ${this.apiKey}` },
    });
    if (!res.ok) throw new ValiForgeError(res.status, await res.text());
    return res.json();
  }

  private async post<T>(path: string, body: unknown): Promise<T> {
    const res = await fetch(`${this.baseUrl}${path}`, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${this.apiKey}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify(body),
    });
    if (!res.ok) throw new ValiForgeError(res.status, await res.text());
    return res.json();
  }
}

export class ValiForgeError extends Error {
  constructor(public status: number, message: string) {
    super(message);
    this.name = "ValiForgeError";
  }
}

export interface ValidationRun {
  id: string;
  status: "pending" | "running" | "completed" | "failed";
  target_url: string;
  summary: { pass: number; fail: number; skip: number; error: number } | null;
  duration_ms: number | null;
  created_at: string;
  completed_at: string | null;
}

export interface PaginatedResponse<T> {
  data: T[];
  total: number;
  page: number;
  per_page: number;
}
```

---

## 9. Environment Configuration

### `.env.example`

```bash
# Database
DATABASE_URL=postgres://postgres:postgres@localhost:5432/valiforge

# Auth (Clerk)
CLERK_SECRET_KEY=sk_test_xxxx
NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY=pk_test_xxxx
NEXT_PUBLIC_CLERK_SIGN_IN_URL=/sign-in
NEXT_PUBLIC_CLERK_SIGN_UP_URL=/sign-up

# Stripe
STRIPE_SECRET_KEY=sk_test_xxxx
STRIPE_WEBHOOK_SECRET=whsec_xxxx
STRIPE_PRICE_PRO=price_xxxx
STRIPE_PRICE_TEAM=price_xxxx

# API
JWT_SECRET=change-me-in-production
NEXT_PUBLIC_API_URL=http://localhost:8080/api/v1
RUST_LOG=valiforge_cloud=info

# Optional: SLM Model (for server-side generation)
VALIFORGE_SLM_MODEL_PATH=/models/phi-4-mini-q4.gguf
```

### `docker-compose.yml` (Local Development)

```yaml
version: "3.8"

services:
  postgres:
    image: postgres:16-alpine
    ports:
      - "5432:5432"
    environment:
      POSTGRES_DB: valiforge
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: postgres
    volumes:
      - pgdata:/var/lib/postgresql/data

  api:
    build:
      context: .
      dockerfile: crates/valiforge-cloud/Dockerfile
    ports:
      - "8080:8080"
    environment:
      DATABASE_URL: postgres://postgres:postgres@postgres:5432/valiforge
      RUST_LOG: valiforge_cloud=debug
    depends_on:
      - postgres

volumes:
  pgdata:
```

---

## 10. Deployment

### `apps/dashboard/vercel.json`

```json
{
  "framework": "nextjs",
  "buildCommand": "pnpm turbo build --filter=@valiforge/dashboard",
  "outputDirectory": "apps/dashboard/.next",
  "installCommand": "pnpm install"
}
```

### Cloud Run Deployment (`crates/valiforge-cloud/Dockerfile`)

```dockerfile
# Build stage
FROM rust:1.75-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN cargo build --release --package valiforge-cloud

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
RUN useradd -r -s /bin/false valiforge
COPY --from=builder /app/target/release/valiforge-cloud /usr/local/bin/
USER valiforge
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s CMD curl -f http://localhost:8080/api/v1/health || exit 1
CMD ["valiforge-cloud"]
```
