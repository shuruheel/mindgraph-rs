# Design Lessons Analysis — Critical Review

Input: Real-world usage of mindgraph-server with OpenClaw over multiple days.

## 1. Retrieval Architecture

### Problem as stated
FTS indexes only `label` + `summary`. Semantic scores 0.3-0.5. Two modes run independently.

### Analysis
**Confirmed.** Looking at `storage/migrations.rs:95-96`, only two FTS indexes exist:
- `node:label_fts` (extracts `label`)
- `node:summary_fts` (extracts `summary`)

Meanwhile, the actual content lives in `props.content` (Claims, Observations, Snippets), `props.description` (Entities, Hypotheses), `props.principle` (Warrants), etc. These are invisible to FTS. A search for "Aaron Goh" won't find an Observation whose `props.content` mentions Aaron Goh — only if his name is in the label or summary.

### Proposed fix — critical assessment

**Full-content FTS**: CozoDB FTS only works on columns in the relation, not on JSON fields inside `props`. Options:
1. Add a `search_text` column to the `node` relation — a denormalized concatenation of label + summary + key props fields, rebuilt on every write. FTS indexes this column.
2. External search index (e.g., tantivy). Overkill for v1.
3. Flatten key content fields into top-level columns.

**Recommendation**: Option 1 — add a `search_text: String` column. On node create/update, the server concatenates `label + "\n" + summary + "\n" + props.content/description/principle/etc.` into this field. One FTS index replaces two. This is a schema migration but backward-compatible (defaults to empty string for existing nodes, backfillable).

**Hybrid retrieval with ranked fusion**: Correct approach. The implementation is:
1. Run FTS query → get `(uid, bm25_score)` pairs
2. Run HNSW vector query → get `(uid, 1-distance)` pairs
3. Reciprocal Rank Fusion (RRF): `score = sum(1 / (k + rank_i))` across both lists
4. Deduplicate by uid, return top-k

This should be a new `hybrid_search()` method on `MindGraph`, not a server-layer concern.

**Canonical node resolution in retrieval**: This is the entity dedup problem (lesson #2), not a retrieval problem per se. If dedup works, retrieval doesn't need to know about it. If dedup doesn't work, retrieval can't fix it. Don't mix these concerns.

**0.75+ semantic scores**: Score targets aren't meaningful as an absolute — they depend on the embedding model, distance metric, and corpus. A better target: "for a direct named-entity query, the correct entity appears in the top-3 results."

---

## 2. Deduplication & Entity Resolution

### Problem as stated
5 nodes for "Aaron Goh" (Entity, Observations, Snippet, Claim). No enforcement at creation time.

### Analysis
This is a real problem but the proposed "upsert semantics on Entity nodes" needs careful scoping. The 5 nodes are *different types* — an Entity, Observations, and a Claim all referencing the same person. They *should* exist as separate nodes. The issue is:
1. The Entity should be canonical
2. The Observations/Claims should be *linked to* the Entity, not competing with it in search
3. Retrieval should prefer the canonical Entity when the query is about the person

### Proposed fix — critical assessment

**Entity dedup at creation time**: Yes, but only for Entity nodes. When `add_entity("Aaron Goh", "person")` is called, check if an Entity with matching label or alias exists. If yes, return the existing one (or merge). Don't apply this logic to Observations/Claims — those are inherently multiple.

**upsert semantics**: Dangerous if applied broadly. An Entity can be upserted. A Claim should not — "Aaron is smart" and "Aaron is kind" are two Claims, not an upsert. The upsert should be type-specific:
- Entity: upsert by canonical_name or alias match
- Preference: upsert by key
- MemoryPolicy: upsert by policy_name
- Everything else: always create new

**Implementation**: Add a `find_or_create_entity()` method that:
1. Fuzzy-searches aliases for the label
2. If match ≥ 0.8, return existing entity UID
3. If no match, create new entity + register label as alias
4. Optionally merge if multiple candidates found

This should be in the library (`graph.rs`), called by the `/reality/entity` handler.

---

## 3. Schema Gaps

### Analysis

**Investigation node type**: This is a valid need. The current `Inquiry` type has no step tracking. But adding a whole new node type is heavyweight. Alternative: use `Hypothesis → Evidence → Claim` chains linked by `FOLLOWS` edges, with the Hypothesis having a `status` field that tracks progress. An Investigation could be modeled as a `Plan` with `PlanStep` children (that type already exists).

**Recommendation**: Don't add a new Investigation type in v1. Instead:
- Add a `FOLLOWS` / `NEXT` edge type (cheap)
- Add a `status` field to HypothesisProps (or use the existing `status` on PlanProps)
- Model investigations as Plans with epistemic PlanSteps

**Longer labels**: The 60-char limit isn't enforced in the library — it's presumably a UI/client constraint. Labels can be any length in the schema (`label: String`). If clients are truncating, that's a client issue, not a schema gap. However, a `short_label` field for display purposes is reasonable.

**Revision semantics**: The existing `SUPERSEDES` edge type handles this. When a belief changes, create a new node, link old → new with SUPERSEDES, tombstone the old. The `/evolve` endpoint already supports this via `tombstone` action. The missing piece is: default retrieval should exclude superseded nodes. This is a filter issue, not a schema issue.

**Recommendation for Cloud v1**: Add `FOLLOWS` edge type. Add `short_label` to node schema. Don't add new node types.

---

## 4. Temporality & Sequence

### Analysis

**Session nodes as timeline anchors**: Sessions already exist and link to traces via `TRACE_ENTRY` edges. The problem is that not all nodes created during a session are auto-linked to it. The `/memory/session` handler handles this for traces, but ad-hoc nodes created via other endpoints aren't captured.

**Recommendation**: When a session is active, all nodes created by the same agent should get a `CAPTURED_IN` edge to the active session automatically. This requires:
1. Agent-scoped "active session" state (stored in the graph as a meta key or on the AgentHandle)
2. `add_node()` checks if the agent has an active session and auto-links

**Timeline API**: A `GET /timeline` endpoint that queries nodes by `created_at` range, grouped by session, is straightforward. This is a server endpoint, not a library change.

**NEXT / PRECEDED_BY edges**: `FOLLOWS` covers this (proposed in #3). Don't add two edge types for the same relationship — one directional edge type is sufficient.

**Narrative node type**: This is the markdown complement (lesson #6). Don't model it as a node type — model it as a separate storage layer (see #6).

---

## 5. propsPatch / Write Reliability

### Root cause analysis

Looking at the code in `handlers.rs:2577-2584`:
```rust
if let (Some(base_map), Some(patch_obj)) = (base.as_object_mut(), patch.as_object()) {
    for (k, v) in patch_obj {
        base_map.insert(k.clone(), v.clone());
    }
}
let rebuilt = NodeProps::from_json(&node_type, &base).map_err(map_err_500)?;
```

The merge itself works. BUT: `NodeProps::from_json` calls `serde_json::from_value` on props structs that all use `#[serde(default)]`. This means:
1. Unknown field names are **silently ignored** (serde default behavior)
2. Wrong types are silently replaced with defaults
3. The response returns the updated node, but if the caller doesn't check it, they don't notice

So `props_patch: {"contnet": "new text"}` (note typo) would succeed with 200, return the node, but `content` would remain unchanged. The "description" field on a Claim (which has `content`, not `description`) would be silently dropped.

### Fix

**Strict deserialization**: Add `#[serde(deny_unknown_fields)]` to props structs. This would break backward compatibility for any client sending extra fields, but it's correct. Alternative: validate in `from_json` and return warnings, not errors.

**For Cloud v1**: The safer approach:
1. Add `#[serde(deny_unknown_fields)]` to all props structs
2. Return 422 with field details when deserialization fails
3. Always return the resulting node state in the response (already done — the issue was the caller not checking)

**Important nuance**: This is a library-level fix (`mindgraph` crate), not just a server fix. Changing `#[serde(default)]` to include `#[serde(deny_unknown_fields)]` is a breaking change for the library.

**Recommendation for Cloud v1**: Add server-side validation in the `/evolve` handler that compares the patch keys against the known fields for the node type. Return 422 with unknown field names. Don't change the library serde attributes yet (that's a major version bump).

---

## 6. The Markdown Complement

### Analysis

This is an honest and accurate assessment. Graphs are terrible at narrative. The key insight is correct: **the graph is the index and reasoning layer, not the storage layer for everything.**

### Recommendation for Cloud v1

Don't build a full narrative layer yet. Instead:
1. Add a `Journal` node type — stores markdown/prose as `content`, tagged with `session_uid` and `created_at`
2. The ingest endpoint can accept prose, extract named entities, create Entity nodes + `MENTIONED_IN` edges
3. FTS indexes the Journal content via the `search_text` column (lesson #1)
4. This gives users a place to store narrative without forcing it into structured nodes

This is simpler than a full "narrative layer" and achieves 80% of the value.

---

## Priority for Cloud v1 — Revised

| Priority | Item | Where it lands |
|---|---|---|
| 1 | Write reliability (deny_unknown_fields, 422 errors) | Library + server fix |
| 2 | Full-content FTS via `search_text` column | Library schema migration |
| 3 | Entity dedup at creation time | Library + server handler |
| 4 | Hybrid retrieval (BM25 + vector fusion) | Library method |
| 5 | Session-anchored temporality | Library + server handler |
| 6 | Timeline API | Server endpoint |
| 7 | Journal node type for narrative | Schema + handler |

I moved write reliability to #1 because silent data loss is worse than bad search. Users can work around bad search; they can't recover from writes they thought succeeded.
