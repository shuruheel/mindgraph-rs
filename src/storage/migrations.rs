/// CozoDB schema creation scripts, run sequentially on initialization.
pub(crate) const SCHEMA_MIGRATIONS: &[&str] = &[
    // Core node relation
    r#":create node {
        uid: String
        =>
        node_type: String,
        layer: String,
        label: String,
        summary: String default '',
        created_at: Float,
        updated_at: Float,
        version: Int default 1,
        confidence: Float default 1.0,
        salience: Float default 0.5,
        privacy_level: String default 'private',
        embedding_ref: String default '',
        tombstone_at: Float default 0.0,
        tombstone_reason: String default '',
        tombstone_by: String default '',
        props: Json default {}
    }"#,
    // Core edge relation
    r#":create edge {
        uid: String
        =>
        from_uid: String,
        to_uid: String,
        edge_type: String,
        layer: String,
        created_at: Float,
        updated_at: Float,
        version: Int default 1,
        confidence: Float default 1.0,
        weight: Float default 0.5,
        tombstone_at: Float default 0.0,
        props: Json default {}
    }"#,
    // Node version history (append-only)
    r#":create node_version {
        node_uid: String,
        version: Int
        =>
        snapshot: Json,
        changed_by: String,
        changed_at: Float,
        change_type: String,
        change_reason: String default ''
    }"#,
    // Edge version history (append-only)
    r#":create edge_version {
        edge_uid: String,
        version: Int
        =>
        snapshot: Json,
        changed_by: String,
        changed_at: Float,
        change_type: String,
        change_reason: String default ''
    }"#,
    // Provenance tracking
    r#":create provenance {
        node_uid: String,
        source_uid: String
        =>
        extraction_method: String default 'unknown',
        extraction_confidence: Float default 1.0,
        source_location: String default '',
        text_span: String default '',
        extracted_by: String default '',
        extracted_at: Float
    }"#,
    // Entity alias table for dedup
    r#":create alias {
        alias_text: String,
        canonical_uid: String
        =>
        match_score: Float default 1.0,
        created_at: Float
    }"#,
    // Key-value metadata store
    r#":create mg_meta { key: String => value: String }"#,
    // Indices for edge traversal
    "::index create edge:from_idx {from_uid, edge_type}",
    "::index create edge:to_idx {to_uid, edge_type}",
    "::index create edge:type_idx {edge_type}",
    // Indices for node lookup
    "::index create node:type_idx {node_type}",
    "::index create node:layer_idx {layer}",
    // Index for provenance queries
    "::index create provenance:source_idx {source_uid}",
    // Index for alias resolution
    "::index create alias:canonical_idx {canonical_uid}",
    // Full-text search indices on node label and summary
    "::fts create node:label_fts { extractor: label, tokenizer: Simple, filters: [Lowercase] }",
    "::fts create node:summary_fts { extractor: summary, tokenizer: Simple, filters: [Lowercase] }",
    // Full-content search relation — denormalized text extracted from props
    r#":create node_search {
        uid: String
        =>
        search_text: String default ''
    }"#,
    "::fts create node_search:search_text_fts { extractor: search_text, tokenizer: Simple, filters: [Lowercase] }",
];
