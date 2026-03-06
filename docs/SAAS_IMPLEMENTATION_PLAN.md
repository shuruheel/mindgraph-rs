# MindGraph Cloud — SaaS Implementation Plan

## Vision

Turn MindGraph Server into a multi-tenant SaaS at **mindgraph.cloud** where users sign up, get an API key, and get their own isolated MindGraph database. MVP exposes 28 endpoints (18 cognitive layer + 10 essential CRUD) for building agents with comprehensive cognitive memory. Future dashboard serves as mission control: multi-agent cockpit, graph explorer, and human-in-the-loop approval workflows.

## Decisions

- **Domain**: mindgraph.cloud
- **Auth**: Email/password + GitHub OAuth + Google OAuth (all via Supabase Auth)
- **Multi-tenancy**: Org-based — multiple users can share one graph
- **Embeddings**: Proxied through our OpenAI key with per-plan limits (no BYOK in MVP)
- **Repo**: `mindgraph-cloud` is a **private repo** (separate from the open-source `mindgraph-rs`)
- **MVP scope**: 18 cognitive endpoints + `/retrieve` + `/traverse` + `/evolve` + 7 essential CRUD endpoints
- **MCP**: Not supported (removed from codebase)

## Architecture Overview

```
                    ┌─────────────────────────────────────────┐
                    │          api.mindgraph.cloud             │
                    │          (mindgraph-cloud bin)            │
                    ├─────────────────────────────────────────┤
                    │                                         │
  Agents ──────►   │  Axum Router                            │
  (API keys)       │    │                                    │
                    │    ├─► Auth Middleware                  │
  Dashboard ───►   │    │     (Supabase JWT or API key)      │
  (JWT)            │    │                                    │
                    │    ├─► Tenant Resolution                │
                    │    │     (API key → org_id)             │
                    │    │                                    │
                    │    ├─► Usage + Rate Limit Middleware    │
                    │    │                                    │
                    │    ├─► Graph Pool                       │
                    │    │     (org_id → AsyncMindGraph)      │
                    │    │                                    │
                    │    ├─► Graph API (28 endpoints)         │
                    │    │     (reused from mindgraph-server) │
                    │    │                                    │
                    │    └─► Management API                   │
                    │          (accounts, keys, billing)      │
                    │                                         │
                    ├─────────────────────────────────────────┤
                    │  Supabase (Postgres + Auth)             │
                    │    ├─ auth.users (email/GitHub/Google)  │
                    │    ├─ orgs (tenants)                    │
                    │    ├─ org_members (user ↔ org mapping)  │
                    │    ├─ api_keys                          │
                    │    └─ usage_events                      │
                    │                                         │
                    │  Stripe                                 │
                    │    ├─ Customers (per org)               │
                    │    ├─ Subscriptions                     │
                    │    └─ Metered billing                   │
                    │                                         │
                    │  Fly.io (deployment)                    │
                    │    ├─ Single performance-2x machine     │
                    │    └─ Attached volume: /data/orgs/      │
                    │         ├─ {org_id}/mindgraph.db        │
                    │         └─ ...                          │
                    │                                         │
                    │  Vercel (dashboard)                     │
                    │    └─ app.mindgraph.cloud               │
                    └─────────────────────────────────────────┘
```

## Key Architectural Decisions

### 1. One CozoDB per org (not shared Postgres)
Each org gets their own CozoDB file. This provides:
- **Complete data isolation** — no row-level security bugs, no cross-tenant leaks
- **Independent scaling** — heavy orgs don't slow others
- **Simple migration** — export/import a single file
- **Performance** — CozoDB's embedded design is fast; no network hop to a shared DB
- **Future-proof** — can move individual org DBs to dedicated hosts

Supabase Postgres stores only control-plane data (accounts, API keys, billing, usage). Graph data stays in CozoDB.

### 2. Handler reuse via library extraction
Extract the existing `mindgraph-server` handlers into a shared library (`mindgraph-server/src/lib.rs`) that both the standalone server and the cloud version import. The cloud version adds tenant resolution, billing middleware, and the management API on top.

### 3. Lazy graph pool
Org graphs are loaded on first request and cached in a `DashMap<OrgId, AsyncMindGraph>`. An LRU eviction policy unloads idle orgs to control memory. CozoDB opens fast (~10ms) so cold starts are invisible.

### 4. Org-based multi-tenancy
Users belong to orgs. Multiple users (and their agents) can share one org's graph. This enables:
- Team collaboration on a shared knowledge graph
- Multiple agents from different team members writing to the same graph
- Future RBAC (admin/member/viewer roles)

---

## MVP Endpoint Scope (28 endpoints)

### Cognitive Layer (18 endpoints — from handlers.rs)
| Endpoint | Purpose |
|---|---|
| `POST /reality/ingest` | Source/Snippet/Observation creation |
| `POST /reality/entity` | Entity CRUD, alias, resolve, merge |
| `POST /epistemic/argument` | Claim + Evidence + Warrant + Argument |
| `POST /epistemic/inquiry` | Hypothesis/Theory/Paradigm/Anomaly/Question |
| `POST /epistemic/structure` | Concept/Pattern/Mechanism/Model/Analogy |
| `POST /intent/commitment` | Goal/Project/Milestone |
| `POST /intent/deliberation` | Decision lifecycle |
| `POST /action/procedure` | Flow/FlowStep/Affordance/Control |
| `POST /action/risk` | RiskAssessment |
| `POST /memory/session` | Session open/trace/close |
| `POST /memory/distill` | Summary creation |
| `POST /memory/config` | Preference/MemoryPolicy |
| `POST /agent/plan` | Task/Plan/PlanStep |
| `POST /agent/governance` | Policy/SafetyBudget/Approval |
| `POST /agent/execution` | Execution lifecycle + Agent registration |
| `POST /retrieve` | Multi-mode search and query |
| `POST /traverse` | Chain/neighborhood/path/subgraph |
| `POST /evolve` | Update/tombstone/restore/decay/history |

### Essential CRUD (10 endpoints — from main.rs)
| Endpoint | Why needed |
|---|---|
| `GET /health` | Infrastructure |
| `GET /stats` | Diagnostics, dashboard |
| `GET /node/{uid}` | Fundamental read — agents constantly need this |
| `GET /edges` | Inspect relationships by from/to UID |
| `POST /batch` | Performance for bulk operations |
| `POST /embeddings/configure` | Set up embedding dimension |
| `POST /embeddings/search-text` | Semantic search (proxied through our key) |
| `PUT /node/{uid}/embedding` | Set embedding vector |
| `GET /node/{uid}/embedding` | Get embedding vector |
| `DELETE /node/{uid}/embedding` | Delete embedding vector |

---

## Phase 0: Preparation & Refactoring

**Goal**: Make the existing codebase ready for multi-tenancy without breaking standalone mode.

### 0.1 Extract shared handler library
- Create `mindgraph-server/src/lib.rs` that exports the router builder, all handlers, helpers, and `AppState`
- `mindgraph-server/src/main.rs` becomes a thin binary that calls the lib
- This lets `mindgraph-cloud` depend on `mindgraph-server` as a library and mount the same routes

### 0.2 Make AppState generic over graph resolution
- Current: `AppState.graph` is a single `AsyncMindGraph`
- New: Introduce a `GraphProvider` trait:
  ```rust
  #[async_trait]
  pub trait GraphProvider: Send + Sync + 'static {
      async fn get_graph(&self, tenant_id: &str) -> Result<Arc<AsyncMindGraph>>;
  }
  ```
- Standalone mode: `SingleGraphProvider` always returns the same graph
- Cloud mode: `TenantGraphPool` resolves org_id → CozoDB file → cached AsyncMindGraph
- Middleware resolves the graph and sets it as a request extension before handlers run

### 0.3 Add tenant context to request flow
- Add `TenantContext` request extension (org_id, plan tier, rate limit info)
- Auth middleware sets this before handlers run
- Standalone mode: hardcoded "local" tenant, no plan limits

---

## Phase 1: Control Plane — Supabase + API Keys

**Goal**: Users can sign up, create orgs, invite members, create API keys, and authenticate.

### 1.1 Supabase schema

```sql
-- Organizations (one graph per org)
create table public.orgs (
    id uuid primary key default gen_random_uuid(),
    name text not null,
    slug text not null unique,                    -- URL-friendly name
    plan text not null default 'free',            -- free | pro | enterprise
    stripe_customer_id text,
    stripe_subscription_id text,
    db_path text,                                  -- e.g., "orgs/{id}/mindgraph.db"
    settings jsonb not null default '{}',          -- embedding_model, distance_metric, etc.
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

-- Org membership (users ↔ orgs, many-to-many)
create table public.org_members (
    id uuid primary key default gen_random_uuid(),
    org_id uuid references public.orgs(id) on delete cascade,
    user_id uuid references auth.users(id) on delete cascade,
    role text not null default 'member',           -- owner | admin | member
    invited_by uuid references auth.users(id),
    created_at timestamptz not null default now(),
    unique(org_id, user_id)
);
create index idx_org_members_user on public.org_members(user_id);

-- API keys (multiple per org)
create table public.api_keys (
    id uuid primary key default gen_random_uuid(),
    org_id uuid references public.orgs(id) on delete cascade,
    created_by uuid references auth.users(id),
    key_hash text not null unique,                 -- SHA-256 of the key
    key_prefix text not null,                      -- first 8 chars for display
    name text not null default 'default',
    scopes text[] not null default '{"*"}',        -- future: fine-grained permissions
    last_used_at timestamptz,
    expires_at timestamptz,
    created_at timestamptz not null default now()
);
create index idx_api_keys_hash on public.api_keys(key_hash);
create index idx_api_keys_org on public.api_keys(org_id);

-- Usage tracking (for billing & rate limiting)
create table public.usage_events (
    id bigint generated always as identity primary key,
    org_id uuid references public.orgs(id),
    event_type text not null,                      -- 'api_call', 'node_created', 'embedding_call'
    count int not null default 1,
    metadata jsonb,
    created_at timestamptz not null default now()
);
create index idx_usage_org_time on public.usage_events(org_id, created_at desc);

-- Agent registrations (for dashboard multi-agent view)
create table public.agent_registrations (
    id uuid primary key default gen_random_uuid(),
    org_id uuid references public.orgs(id) on delete cascade,
    agent_name text not null,
    description text,
    status text not null default 'active',         -- active | paused | revoked
    api_key_id uuid references public.api_keys(id),
    config jsonb not null default '{}',
    created_at timestamptz not null default now(),
    unique(org_id, agent_name)
);
```

### 1.2 Auth providers
Enable in Supabase dashboard:
- Email/password (default)
- GitHub OAuth
- Google OAuth

### 1.3 API key format & validation
- Key format: `mg_live_{32 random alphanumeric}` (e.g., `mg_live_a1b2c3d4...`)
- Store SHA-256 hash in `api_keys.key_hash`
- Key displayed once on creation, never again
- Auth flow: `Authorization: Bearer mg_live_...` → SHA-256 → lookup in `api_keys` → get `org_id`

### 1.4 Auth middleware (cloud mode)
```
Request
  → Extract Bearer token
  → If mg_live_* prefix: hash → lookup api_keys → org_id → TenantContext
  → If JWT: validate Supabase JWT → user_id → lookup org_members → org_id → TenantContext
  → Set TenantContext + AsyncMindGraph as request extensions
  → Continue to handlers
```

### 1.5 Signup flow
1. User signs up via Supabase Auth (email/GitHub/Google)
2. Supabase trigger creates a default org: `"{user.email}'s org"`
3. Adds user as `owner` in `org_members`
4. Provisions CozoDB file at `/data/orgs/{org_id}/mindgraph.db`
5. Generates default API key, returns it in the signup response

### 1.6 Management API endpoints
```
POST   /v1/auth/signup              — Create account (Supabase Auth)
POST   /v1/auth/login               — Login (Supabase Auth)
POST   /v1/auth/refresh             — Refresh token

GET    /v1/account                  — Get account + orgs
PATCH  /v1/account                  — Update profile

POST   /v1/orgs                     — Create new org
GET    /v1/orgs                     — List user's orgs
PATCH  /v1/orgs/{id}               — Update org settings
POST   /v1/orgs/{id}/members       — Invite member
DELETE /v1/orgs/{id}/members/{uid} — Remove member
GET    /v1/orgs/{id}/members       — List members

POST   /v1/api-keys                 — Create API key (scoped to an org)
GET    /v1/api-keys                 — List API keys
DELETE /v1/api-keys/{id}            — Revoke API key

GET    /v1/usage                    — Get usage stats
GET    /v1/usage/current-period     — Current billing period usage
```

---

## Phase 2: Tenant Graph Pool & Data Isolation

**Goal**: Each org gets their own CozoDB, loaded on demand.

### 2.1 TenantGraphPool

```rust
pub struct TenantGraphPool {
    graphs: DashMap<String, (Arc<AsyncMindGraph>, Instant)>,  // value + last_used
    data_dir: PathBuf,
    max_loaded: usize,
}

impl TenantGraphPool {
    pub async fn get(&self, org_id: &str) -> Result<Arc<AsyncMindGraph>> {
        if let Some(mut entry) = self.graphs.get_mut(org_id) {
            entry.1 = Instant::now(); // touch LRU
            return Ok(entry.0.clone());
        }
        let path = self.data_dir.join(org_id).join("mindgraph.db");
        fs::create_dir_all(path.parent().unwrap()).await?;
        let graph = Arc::new(AsyncMindGraph::open(&path).await?);
        self.graphs.insert(org_id.to_string(), (graph.clone(), Instant::now()));
        self.maybe_evict().await;
        Ok(graph)
    }

    async fn maybe_evict(&self) {
        while self.graphs.len() > self.max_loaded {
            // Find and remove least recently used
            if let Some(oldest) = self.graphs.iter()
                .min_by_key(|e| e.value().1)
                .map(|e| e.key().clone())
            {
                self.graphs.remove(&oldest);
            }
        }
    }
}
```

### 2.2 Embedding proxy
- Platform OpenAI key stored as env var `OPENAI_API_KEY`
- All embedding calls go through our key
- Usage tracked per-org in `usage_events` with `event_type = 'embedding_call'`
- Rate limited per plan tier

---

## Phase 3: Billing with Stripe

**Goal**: Free tier with limits, paid tiers with metered billing.

### 3.1 Plan tiers

| Feature | Free | Pro ($29/mo) | Enterprise (custom) |
|---|---|---|---|
| Nodes | 1,000 | 100,000 | Unlimited |
| API calls/month | 10,000 | 1,000,000 | Unlimited |
| Org members | 1 | 10 | Unlimited |
| Agents | 2 | 20 | Unlimited |
| Embedding calls | 100/mo | 10,000/mo | Unlimited |
| DB size | 50 MB | 5 GB | Custom |
| Data retention | 30 days | Unlimited | Unlimited |
| Support | Community | Email | Dedicated |
| Human-in-loop seats | 1 | 5 | Custom |

### 3.2 Stripe integration
- On org creation: create Stripe Customer (linked to org, not user)
- On plan upgrade: create Stripe Subscription
- Metered billing for overages
- Webhook handler: `invoice.paid`, `invoice.payment_failed`, `customer.subscription.updated`, `customer.subscription.deleted`

### 3.3 Usage tracking middleware
Usage events batched in-memory and flushed to Supabase Postgres every 10s or 100 events.

### 3.4 Billing API endpoints
```
GET    /v1/billing/plans             — List available plans
GET    /v1/billing/subscription      — Get current subscription
POST   /v1/billing/subscribe         — Subscribe to a plan
POST   /v1/billing/portal            — Get Stripe Customer Portal URL
POST   /v1/webhooks/stripe           — Stripe webhook handler
```

---

## Phase 4: Rate Limiting & Quotas

- **Per-org** rate limits (not per-key)
- **Token bucket** in-memory + periodic Postgres sync
- Response headers: `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`, `X-Plan`
- Soft limits at 80%, hard limits at 100% + 10% grace for 24h

---

## Phase 5: Deployment

### 5.1 Infrastructure

```
Fly.io Machine (performance-2x, 4 vCPU / 8GB, ~$62/mo)
  ├── mindgraph-cloud binary
  └── /data/orgs/ (Fly Volume, 100GB NVMe SSD)

Supabase (Pro, $25/mo)
  ├── Auth (email + GitHub + Google)
  ├── Postgres (orgs, api_keys, usage_events, org_members)
  └── Realtime (future dashboard)

Vercel (free tier)
  └── app.mindgraph.cloud (Next.js dashboard)

DNS:
  api.mindgraph.cloud  → Fly.io
  app.mindgraph.cloud  → Vercel
  mindgraph.cloud      → Landing page (Vercel or separate)
```

### 5.2 Backup
- Daily cron: tar tenant data dir → upload to S3/R2
- CozoDB files are single files — trivial to backup

### 5.3 Monitoring
- `tracing` structured logging (already in place)
- Prometheus `/metrics` endpoint
- Fly.io built-in metrics + alerts

---

## Phase 6: Dashboard (Next.js)

### Tech: Next.js App Router + Tailwind + shadcn/ui + Supabase client

### MVP pages
```
/login                     — Supabase Auth UI (email/GitHub/Google)
/signup                    — Supabase Auth UI
/dashboard                 — Overview: node count, API calls, agents, plan
/dashboard/api-keys        — Create/revoke API keys
/dashboard/org/settings    — Org name, members, invites
/dashboard/billing         — Plan, usage, Stripe portal link
/dashboard/quickstart      — Code snippets, curl examples
```

### Future (not MVP)
- Graph Explorer, Agent Cockpit, Human-in-the-Loop, Memory Timeline, Analytics

---

## Phase 7: SDK & Developer Experience

- **Python SDK**: `pip install mindgraph` — typed client
- **TypeScript SDK**: `npm install @mindgraph/client` — typed client
- Both auto-generated from OpenAPI spec (`utoipa` crate → `/v1/openapi.json`)
- Documentation site with API reference, quickstart, cognitive layer cookbook

---

## Workspace Structure

```
mindgraph-rs/                          (public repo — open source)
  Cargo.toml
  src/                                 — mindgraph library crate
  mindgraph-server/
    Cargo.toml
    src/
      lib.rs                           — shared handlers, router builder (NEW)
      main.rs                          — standalone binary entry point
      handlers.rs                      — cognitive layer handlers

mindgraph-cloud/                       (private repo)
  Cargo.toml
  src/
    main.rs                            — entry point, config, startup
    auth.rs                            — Supabase JWT + API key middleware
    tenant.rs                          — TenantGraphPool, provisioning
    billing.rs                         — Stripe integration
    usage.rs                           — Usage tracking, rate limiting
    management.rs                      — Account/org/key management endpoints
  migrations/                          — Supabase SQL migrations
  Dockerfile                           — Fly.io deployment
  fly.toml

dashboard/                             (private repo or monorepo with cloud)
  package.json
  app/                                 — Next.js App Router
  ...
```

---

## Phase 0.5: Library Hardening (DONE)

Lessons from real-world usage with OpenClaw. Library-level fixes in `mindgraph` and `mindgraph-server` that apply to both standalone and cloud.

See `docs/DESIGN_LESSONS_ANALYSIS.md` for the full analysis.

### 0.5.1 Write reliability — no silent failures (DONE)
- `NodeProps::validate_patch(node_type, patch)` validates props_patch keys against known fields, returns unknown field names
- `NodeProps::known_fields_for_type(node_type)` discovers valid fields by serializing a default instance
- Server-side validation in `/evolve` handler returns 422 with unknown field names and lists valid fields
- Integration tests: `test_validate_patch_known_fields`, `test_validate_patch_claim_fields`

### 0.5.2 Full-content FTS via `node_search` relation (DONE)
- Separate `node_search { uid => search_text }` relation with FTS index (avoids breaking 18+ positional `*node[...]` queries)
- `NodeProps::search_text()` extracts text from 35+ string fields and 43+ Vec<String> fields across all 52 node types
- `CozoStorage::upsert_search_text()` called on node create/update, batch insert
- `query_fts_search()` unions results from `node_search:search_text_fts` with existing label/summary FTS
- `purge_tombstoned()` and `export_all()` updated for new relation
- Integration tests: `test_fts_searches_props_content`, `test_search_text_extraction`

### 0.5.3 Entity dedup at creation time (DONE)
- `MindGraph::find_or_create_entity(label, entity_type)` returns `(GraphNode, bool)`:
  1. Exact alias resolution via `resolve_alias()`
  2. Case-insensitive label search among live Entity nodes
  3. If no match, create new entity + register label as alias
- Available on `MindGraph`, `AsyncMindGraph`, `AgentHandle`, `AsyncAgentHandle`
- Integration test: `test_find_or_create_entity_dedup`

### 0.5.4 Hybrid retrieval (BM25 + vector ranked fusion) (DONE)
- `MindGraph::hybrid_search(query, query_vec, limit, opts)` with Reciprocal Rank Fusion (k=60)
- Falls back to FTS-only when no embedding provider is configured
- Available on `MindGraph` and `AsyncMindGraph`
- Integration test: `test_hybrid_search_fts_fallback`

### 0.5.5 Follows edge type (DONE)
- Added `EdgeType::Follows` with `"FOLLOWS"` storage string
- Added `EdgeProps::Follows {}` variant with `default_for()` and `from_json()` support
- Parser updated to handle `"FOLLOWS"` from storage
- Session-anchored auto-edges and `/timeline` endpoint deferred to cloud phase

### 0.5.6 Journal node type for narrative (DONE)
- Added `NodeType::Journal` (Memory layer) with `JournalProps { content, session_uid, journal_type, tags }`
- `NodeProps::Journal(JournalProps)` variant with full ser/de support
- Journal content indexed via `search_text()` extraction
- Parser updated to handle `"Journal"` from storage
- Auto-entity extraction from journal prose deferred

---

## Implementation Order

| Order | Phase | Deliverable |
|---|---|---|
| 1 | Phase 0 | Extract handlers into lib.rs, GraphProvider trait (DONE) |
| 2 | Phase 0.5 | Library hardening: write validation, full-content FTS, entity dedup, hybrid search, Follows edge, Journal type (DONE) |
| 3 | Phase 1 | Supabase schema, auth, org support, API keys, management endpoints (DONE) |
| 4 | Phase 2 | TenantGraphPool, data isolation, graph proxy (DONE) |
| 5 | Phase 3 | Stripe billing, plan tiers, usage tracking (DONE — MVP) |
| 6 | Phase 4 | Rate limiting, quota headers (DONE — in-memory) |
| 7 | Phase 5 | Fly.io deployment config, Dockerfile (DONE — config ready) |
| 8 | Phase 6 | Dashboard MVP |
| 9 | Phase 7 | SDKs, OpenAPI, docs |

---

## Risk Mitigation

| Risk | Mitigation |
|---|---|
| CozoDB file corruption | WAL mode; daily backups to S3/R2 |
| Too many orgs in memory | LRU eviction; CozoDB opens in ~10ms |
| Noisy neighbor (CPU) | Per-org request concurrency limit via tower |
| Data breach | Separate CozoDB files; no shared DB = no cross-tenant leaks |
| Stripe webhook missed | Idempotent handlers; periodic reconciliation |
| Embedding abuse | Per-plan limits; usage tracked per-org; hard cap enforced |
| Single machine limit | Shard by org_id prefix to multiple Fly machines |
| Silent write failures | Server-side field validation; 422 errors; return resulting state |
| Entity duplication | find_or_create_entity with alias matching at creation time |
| Poor retrieval quality | Hybrid BM25+vector fusion; full-content FTS index |

---

## What This Plan Intentionally Defers

- **Multi-region** — single region first, replicate later
- **Custom domains** — per-org vanity URLs (enterprise)
- **WebSocket/SSE** — real-time agent streaming (dashboard v2)
- **Graph Explorer UI** — visual graph browser (dashboard v2)
- **Human-in-the-Loop UI** — approval workflows (dashboard v2)
- **BYOK embeddings** — users bring their own OpenAI key
- **Self-hosted** — `mindgraph-server` standalone already serves this
- **Investigation node type** — model as Plans with epistemic PlanSteps instead
- **Narrative layer** — Journal node type covers 80%; full narrative layer deferred
- **`#[serde(deny_unknown_fields)]`** — library breaking change deferred to next major version; Cloud validates server-side
