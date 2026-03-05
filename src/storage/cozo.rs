use std::collections::BTreeMap;
use std::path::Path;

use cozo::{DataValue, DbInstance, NamedRows, ScriptMutability};

use crate::error::{Error, Result};
use crate::schema::edge::GraphEdge;
use crate::schema::edge_props::EdgeProps;
use crate::schema::node::GraphNode;
use crate::schema::node_props::NodeProps;
use crate::schema::{EdgeType, Layer, NodeType};
use crate::types::*;

use super::SCHEMA_MIGRATIONS;

pub struct CozoStorage {
    db: DbInstance,
}

impl CozoStorage {
    /// Open a SQLite-backed database at the given path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = DbInstance::new("sqlite", path.as_ref(), "")
            .map_err(|e| Error::Storage(e.to_string()))?;
        let storage = CozoStorage { db };
        storage.initialize()?;
        Ok(storage)
    }

    /// Create an in-memory database (for testing).
    pub fn open_in_memory() -> Result<Self> {
        let db = DbInstance::new("mem", "", "").map_err(|e| Error::Storage(e.to_string()))?;
        let storage = CozoStorage { db };
        storage.initialize()?;
        Ok(storage)
    }

    fn initialize(&self) -> Result<()> {
        for migration in SCHEMA_MIGRATIONS {
            let result = self
                .db
                .run_script(migration, BTreeMap::new(), ScriptMutability::Mutable);
            if let Err(e) = result {
                let err_str = e.to_string();
                if !err_str.contains("already exists")
                    && !err_str.contains("conflicts with an existing one")
                {
                    return Err(Error::Storage(err_str));
                }
            }
        }
        Ok(())
    }

    /// Execute a mutable CozoDB script.
    pub fn run_script(
        &self,
        script: &str,
        params: BTreeMap<String, DataValue>,
    ) -> Result<NamedRows> {
        self.db
            .run_script(script, params, ScriptMutability::Mutable)
            .map_err(|e| Error::Storage(e.to_string()))
    }

    /// Execute an immutable (read-only) CozoDB script.
    pub fn run_query(
        &self,
        script: &str,
        params: BTreeMap<String, DataValue>,
    ) -> Result<NamedRows> {
        self.db
            .run_script(script, params, ScriptMutability::Immutable)
            .map_err(|e| Error::Storage(e.to_string()))
    }

    /// Insert a node.
    pub fn insert_node(&self, node: &GraphNode) -> Result<()> {
        let props_json = node.props.try_to_json_untagged()?;
        let script = r#"
            ?[uid, node_type, layer, label, summary, created_at, updated_at, version,
              confidence, salience, privacy_level, embedding_ref,
              tombstone_at, tombstone_reason, tombstone_by, props] <- [[
                $uid, $node_type, $layer, $label, $summary, $created_at, $updated_at, $version,
                $confidence, $salience, $privacy_level, $embedding_ref,
                $tombstone_at, $tombstone_reason, $tombstone_by, $props
            ]]
            :put node {
                uid =>
                node_type, layer, label, summary, created_at, updated_at, version,
                confidence, salience, privacy_level, embedding_ref,
                tombstone_at, tombstone_reason, tombstone_by, props
            }
        "#;

        let params = self.node_to_params(node, props_json);
        self.run_script(script, params)?;
        Ok(())
    }

    /// Get a node by UID.
    pub fn get_node(&self, uid: &Uid) -> Result<Option<GraphNode>> {
        let script = r#"
            ?[uid, node_type, layer, label, summary, created_at, updated_at, version,
              confidence, salience, privacy_level, embedding_ref,
              tombstone_at, tombstone_reason, tombstone_by, props] :=
                *node[uid, node_type, layer, label, summary, created_at, updated_at, version,
                      confidence, salience, privacy_level, embedding_ref,
                      tombstone_at, tombstone_reason, tombstone_by, props],
                uid == $uid
        "#;

        let mut params = BTreeMap::new();
        params.insert("uid".into(), str_val(uid.as_str()));

        let result = self.run_query(script, params)?;
        if result.rows.is_empty() {
            return Ok(None);
        }

        self.row_to_node(&result.rows[0]).map(Some)
    }

    /// Insert an edge.
    pub fn insert_edge(&self, edge: &GraphEdge) -> Result<()> {
        let props_json = edge.props.try_to_json_untagged()?;
        let script = r#"
            ?[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
              confidence, weight, tombstone_at, props] <- [[
                $uid, $from_uid, $to_uid, $edge_type, $layer, $created_at, $updated_at, $version,
                $confidence, $weight, $tombstone_at, $props
            ]]
            :put edge {
                uid =>
                from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                confidence, weight, tombstone_at, props
            }
        "#;

        let params = self.edge_to_params(edge, props_json);
        self.run_script(script, params)?;
        Ok(())
    }

    /// Get an edge by UID.
    pub fn get_edge(&self, uid: &Uid) -> Result<Option<GraphEdge>> {
        let script = r#"
            ?[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
              confidence, weight, tombstone_at, props] :=
                *edge[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                      confidence, weight, tombstone_at, props],
                uid == $uid
        "#;

        let mut params = BTreeMap::new();
        params.insert("uid".into(), str_val(uid.as_str()));

        let result = self.run_query(script, params)?;
        if result.rows.is_empty() {
            return Ok(None);
        }

        self.row_to_edge(&result.rows[0]).map(Some)
    }

    /// Query live (non-tombstoned) nodes by type.
    pub fn query_nodes_by_type(
        &self,
        node_type: NodeType,
        include_tombstoned: bool,
    ) -> Result<Vec<GraphNode>> {
        let script = if include_tombstoned {
            r#"
                ?[uid, node_type, layer, label, summary, created_at, updated_at, version,
                  confidence, salience, privacy_level, embedding_ref,
                  tombstone_at, tombstone_reason, tombstone_by, props] :=
                    *node[uid, node_type, layer, label, summary, created_at, updated_at, version,
                          confidence, salience, privacy_level, embedding_ref,
                          tombstone_at, tombstone_reason, tombstone_by, props],
                    node_type == $node_type
            "#
        } else {
            r#"
                ?[uid, node_type, layer, label, summary, created_at, updated_at, version,
                  confidence, salience, privacy_level, embedding_ref,
                  tombstone_at, tombstone_reason, tombstone_by, props] :=
                    *node[uid, node_type, layer, label, summary, created_at, updated_at, version,
                          confidence, salience, privacy_level, embedding_ref,
                          tombstone_at, tombstone_reason, tombstone_by, props],
                    node_type == $node_type,
                    tombstone_at == 0.0
            "#
        };

        let mut params = BTreeMap::new();
        params.insert("node_type".into(), str_val(node_type.as_str()));

        let result = self.run_query(script, params)?;
        result
            .rows
            .iter()
            .map(|row| self.row_to_node(row))
            .collect()
    }

    /// Query edges from a node, optionally filtered by edge type.
    pub fn query_edges_from(
        &self,
        from_uid: &Uid,
        edge_type: Option<EdgeType>,
    ) -> Result<Vec<GraphEdge>> {
        let mut params = BTreeMap::new();
        params.insert("from_uid".into(), str_val(from_uid.as_str()));

        let script = if let Some(et) = edge_type {
            params.insert("edge_type".into(), str_val(et.as_str()));
            r#"
                ?[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                  confidence, weight, tombstone_at, props] :=
                    *edge[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                          confidence, weight, tombstone_at, props],
                    from_uid == $from_uid,
                    edge_type == $edge_type,
                    tombstone_at == 0.0
            "#
        } else {
            r#"
                ?[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                  confidence, weight, tombstone_at, props] :=
                    *edge[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                          confidence, weight, tombstone_at, props],
                    from_uid == $from_uid,
                    tombstone_at == 0.0
            "#
        };

        let result = self.run_query(script, params)?;
        result
            .rows
            .iter()
            .map(|row| self.row_to_edge(row))
            .collect()
    }

    /// Query edges to a node.
    pub fn query_edges_to(
        &self,
        to_uid: &Uid,
        edge_type: Option<EdgeType>,
    ) -> Result<Vec<GraphEdge>> {
        let mut params = BTreeMap::new();
        params.insert("to_uid".into(), str_val(to_uid.as_str()));

        let script = if let Some(et) = edge_type {
            params.insert("edge_type".into(), str_val(et.as_str()));
            r#"
                ?[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                  confidence, weight, tombstone_at, props] :=
                    *edge[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                          confidence, weight, tombstone_at, props],
                    to_uid == $to_uid,
                    edge_type == $edge_type,
                    tombstone_at == 0.0
            "#
        } else {
            r#"
                ?[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                  confidence, weight, tombstone_at, props] :=
                    *edge[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                          confidence, weight, tombstone_at, props],
                    to_uid == $to_uid,
                    tombstone_at == 0.0
            "#
        };

        let result = self.run_query(script, params)?;
        result
            .rows
            .iter()
            .map(|row| self.row_to_edge(row))
            .collect()
    }

    /// Query live nodes by type where a JSON props field equals a given value.
    pub fn query_nodes_by_type_and_prop(
        &self,
        node_type: NodeType,
        prop_field: &str,
        prop_value: &str,
    ) -> Result<Vec<GraphNode>> {
        if !prop_field
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(Error::Validation(format!(
                "Invalid property field name: {}",
                prop_field
            )));
        }

        let script = format!(
            r#"
                ?[uid, node_type, layer, label, summary, created_at, updated_at, version,
                  confidence, salience, privacy_level, embedding_ref,
                  tombstone_at, tombstone_reason, tombstone_by, props] :=
                    *node[uid, node_type, layer, label, summary, created_at, updated_at, version,
                          confidence, salience, privacy_level, embedding_ref,
                          tombstone_at, tombstone_reason, tombstone_by, props],
                    node_type == $node_type,
                    tombstone_at == 0.0,
                    field_val = get(props, '{}', ''),
                    field_val == $prop_value
            "#,
            prop_field
        );

        let mut params = BTreeMap::new();
        params.insert("node_type".into(), str_val(node_type.as_str()));
        params.insert("prop_value".into(), str_val(prop_value));

        let result = self.run_query(&script, params)?;
        result
            .rows
            .iter()
            .map(|row| self.row_to_node(row))
            .collect()
    }

    /// Query live nodes by type where a JSON props field is one of several values.
    pub fn query_nodes_by_type_and_prop_in(
        &self,
        node_type: NodeType,
        prop_field: &str,
        prop_values: &[&str],
    ) -> Result<Vec<GraphNode>> {
        if !prop_field
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(Error::Validation(format!(
                "Invalid property field name: {}",
                prop_field
            )));
        }

        let conditions: Vec<String> = prop_values
            .iter()
            .enumerate()
            .map(|(i, _)| format!("field_val == $pv_{}", i))
            .collect();
        let or_clause = conditions.join(" or ");

        let script = format!(
            r#"
                ?[uid, node_type, layer, label, summary, created_at, updated_at, version,
                  confidence, salience, privacy_level, embedding_ref,
                  tombstone_at, tombstone_reason, tombstone_by, props] :=
                    *node[uid, node_type, layer, label, summary, created_at, updated_at, version,
                          confidence, salience, privacy_level, embedding_ref,
                          tombstone_at, tombstone_reason, tombstone_by, props],
                    node_type == $node_type,
                    tombstone_at == 0.0,
                    field_val = get(props, '{}', ''),
                    ({})
            "#,
            prop_field, or_clause
        );

        let mut params = BTreeMap::new();
        params.insert("node_type".into(), str_val(node_type.as_str()));
        for (i, val) in prop_values.iter().enumerate() {
            params.insert(format!("pv_{}", i), str_val(val));
        }

        let result = self.run_query(&script, params)?;
        result
            .rows
            .iter()
            .map(|row| self.row_to_node(row))
            .collect()
    }

    /// Query live nodes by type where confidence is below a threshold.
    pub fn query_nodes_by_type_below_confidence(
        &self,
        node_type: NodeType,
        threshold: f64,
    ) -> Result<Vec<GraphNode>> {
        let script = r#"
            ?[uid, node_type, layer, label, summary, created_at, updated_at, version,
              confidence, salience, privacy_level, embedding_ref,
              tombstone_at, tombstone_reason, tombstone_by, props] :=
                *node[uid, node_type, layer, label, summary, created_at, updated_at, version,
                      confidence, salience, privacy_level, embedding_ref,
                      tombstone_at, tombstone_reason, tombstone_by, props],
                node_type == $node_type,
                tombstone_at == 0.0,
                confidence < $threshold
            :order confidence
        "#;

        let mut params = BTreeMap::new();
        params.insert("node_type".into(), str_val(node_type.as_str()));
        params.insert("threshold".into(), DataValue::from(threshold));

        let result = self.run_query(script, params)?;
        result
            .rows
            .iter()
            .map(|row| self.row_to_node(row))
            .collect()
    }

    /// Insert a version record for a node.
    pub fn insert_node_version(
        &self,
        node_uid: &Uid,
        version: i64,
        snapshot: serde_json::Value,
        changed_by: &str,
        change_type: &str,
        reason: &str,
    ) -> Result<()> {
        let script = r#"
            ?[node_uid, version, snapshot, changed_by, changed_at, change_type, change_reason] <- [[
                $node_uid, $version, $snapshot, $changed_by, $changed_at, $change_type, $change_reason
            ]]
            :put node_version { node_uid, version => snapshot, changed_by, changed_at, change_type, change_reason }
        "#;

        let mut params = BTreeMap::new();
        params.insert("node_uid".into(), str_val(node_uid.as_str()));
        params.insert("version".into(), DataValue::from(version));
        params.insert("snapshot".into(), DataValue::Json(cozo::JsonData(snapshot)));
        params.insert("changed_by".into(), str_val(changed_by));
        params.insert("changed_at".into(), DataValue::from(now()));
        params.insert("change_type".into(), str_val(change_type));
        params.insert("change_reason".into(), str_val(reason));

        self.run_script(script, params)?;
        Ok(())
    }

    /// Insert a version record for an edge.
    pub fn insert_edge_version(
        &self,
        edge_uid: &Uid,
        version: i64,
        snapshot: serde_json::Value,
        changed_by: &str,
        change_type: &str,
        reason: &str,
    ) -> Result<()> {
        let script = r#"
            ?[edge_uid, version, snapshot, changed_by, changed_at, change_type, change_reason] <- [[
                $edge_uid, $version, $snapshot, $changed_by, $changed_at, $change_type, $change_reason
            ]]
            :put edge_version { edge_uid, version => snapshot, changed_by, changed_at, change_type, change_reason }
        "#;

        let mut params = BTreeMap::new();
        params.insert("edge_uid".into(), str_val(edge_uid.as_str()));
        params.insert("version".into(), DataValue::from(version));
        params.insert("snapshot".into(), DataValue::Json(cozo::JsonData(snapshot)));
        params.insert("changed_by".into(), str_val(changed_by));
        params.insert("changed_at".into(), DataValue::from(now()));
        params.insert("change_type".into(), str_val(change_type));
        params.insert("change_reason".into(), str_val(reason));

        self.run_script(script, params)?;
        Ok(())
    }

    /// Insert a provenance record.
    pub fn insert_provenance(&self, record: &crate::provenance::ProvenanceRecord) -> Result<()> {
        let script = r#"
            ?[node_uid, source_uid, extraction_method, extraction_confidence,
              source_location, text_span, extracted_by, extracted_at] <- [[
                $node_uid, $source_uid, $method, $confidence,
                $location, $text_span, $extracted_by, $extracted_at
            ]]
            :put provenance {
                node_uid, source_uid =>
                extraction_method, extraction_confidence,
                source_location, text_span, extracted_by, extracted_at
            }
        "#;

        let mut params = BTreeMap::new();
        params.insert("node_uid".into(), str_val(record.node_uid.as_str()));
        params.insert("source_uid".into(), str_val(record.source_uid.as_str()));
        params.insert("method".into(), str_val(record.extraction_method.as_str()));
        params.insert(
            "confidence".into(),
            DataValue::from(record.extraction_confidence),
        );
        params.insert("location".into(), str_val(&record.source_location));
        params.insert("text_span".into(), str_val(&record.text_span));
        params.insert("extracted_by".into(), str_val(&record.extracted_by));
        params.insert("extracted_at".into(), DataValue::from(now()));

        self.run_script(script, params)?;
        Ok(())
    }

    /// Insert an alias for entity resolution.
    pub fn insert_alias(
        &self,
        alias_text: &str,
        canonical_uid: &Uid,
        match_score: f64,
    ) -> Result<()> {
        let script = r#"
            ?[alias_text, canonical_uid, match_score, created_at] <- [[
                $alias_text, $canonical_uid, $match_score, $created_at
            ]]
            :put alias { alias_text, canonical_uid => match_score, created_at }
        "#;

        let mut params = BTreeMap::new();
        params.insert("alias_text".into(), str_val(alias_text));
        params.insert("canonical_uid".into(), str_val(canonical_uid.as_str()));
        params.insert("match_score".into(), DataValue::from(match_score));
        params.insert("created_at".into(), DataValue::from(now()));

        self.run_script(script, params)?;
        Ok(())
    }

    /// Resolve an alias to a canonical UID.
    pub fn resolve_alias(&self, alias_text: &str) -> Result<Option<Uid>> {
        let script = r#"
            ?[canonical_uid, match_score] :=
                *alias[alias_text, canonical_uid, match_score, _],
                alias_text == $alias_text
            :order -match_score
            :limit 1
        "#;

        let mut params = BTreeMap::new();
        params.insert("alias_text".into(), str_val(alias_text));

        let result = self.run_query(script, params)?;
        if result.rows.is_empty() {
            return Ok(None);
        }

        let uid_str = extract_string(&result.rows[0][0])?;
        Ok(Some(Uid::from(uid_str.as_str())))
    }

    // ---- Layer / Count / Exists Queries ----

    /// Query all live nodes in a given layer using the indexed `layer` column.
    pub fn query_nodes_by_layer(&self, layer: Layer) -> Result<Vec<GraphNode>> {
        let script = r#"
            ?[uid, node_type, layer, label, summary, created_at, updated_at, version,
              confidence, salience, privacy_level, embedding_ref,
              tombstone_at, tombstone_reason, tombstone_by, props] :=
                *node[uid, node_type, layer, label, summary, created_at, updated_at, version,
                      confidence, salience, privacy_level, embedding_ref,
                      tombstone_at, tombstone_reason, tombstone_by, props],
                layer == $layer,
                tombstone_at == 0.0
        "#;

        let mut params = BTreeMap::new();
        params.insert("layer".into(), str_val(layer.as_str()));

        let result = self.run_query(script, params)?;
        result
            .rows
            .iter()
            .map(|row| self.row_to_node(row))
            .collect()
    }

    /// Count live nodes of a given type.
    pub fn count_nodes_by_type(&self, node_type: NodeType) -> Result<u64> {
        let script = r#"
            ?[count(uid)] :=
                *node{uid, node_type, tombstone_at},
                node_type == $node_type,
                tombstone_at == 0.0
        "#;

        let mut params = BTreeMap::new();
        params.insert("node_type".into(), str_val(node_type.as_str()));

        let result = self.run_query(script, params)?;
        if result.rows.is_empty() {
            return Ok(0);
        }
        Ok(extract_int(&result.rows[0][0])? as u64)
    }

    /// Count live nodes in a given layer.
    pub fn count_nodes_by_layer(&self, layer: Layer) -> Result<u64> {
        let script = r#"
            ?[count(uid)] :=
                *node{uid, layer, tombstone_at},
                layer == $layer,
                tombstone_at == 0.0
        "#;

        let mut params = BTreeMap::new();
        params.insert("layer".into(), str_val(layer.as_str()));

        let result = self.run_query(script, params)?;
        if result.rows.is_empty() {
            return Ok(0);
        }
        Ok(extract_int(&result.rows[0][0])? as u64)
    }

    /// Count live edges of a given type.
    pub fn count_edges_by_type(&self, edge_type: EdgeType) -> Result<u64> {
        let script = r#"
            ?[count(uid)] :=
                *edge{uid, edge_type, tombstone_at},
                edge_type == $edge_type,
                tombstone_at == 0.0
        "#;

        let mut params = BTreeMap::new();
        params.insert("edge_type".into(), str_val(edge_type.as_str()));

        let result = self.run_query(script, params)?;
        if result.rows.is_empty() {
            return Ok(0);
        }
        Ok(extract_int(&result.rows[0][0])? as u64)
    }

    /// Check whether a node exists (live, not tombstoned).
    pub fn node_exists(&self, uid: &Uid) -> Result<bool> {
        let script = r#"
            ?[found] :=
                *node[uid, _, _, _, _, _, _, _, _, _, _, _, tombstone_at, _, _, _],
                uid == $uid,
                tombstone_at == 0.0,
                found = true
        "#;

        let mut params = BTreeMap::new();
        params.insert("uid".into(), str_val(uid.as_str()));

        let result = self.run_query(script, params)?;
        Ok(!result.rows.is_empty())
    }

    // ---- Traversal ----

    /// Traverse reachable nodes from a starting node using optimized 2-query BFS.
    ///
    /// Query 1: Fetch all live edges into an in-memory adjacency list.
    /// BFS: Traverse in-memory (zero DB round-trips during traversal).
    /// Query 2: Batch-fetch node metadata for all reached UIDs.
    pub fn traverse_reachable(
        &self,
        start: &Uid,
        direction: &crate::traversal::Direction,
        edge_types: &Option<Vec<EdgeType>>,
        max_depth: u32,
        weight_threshold: Option<f64>,
    ) -> Result<Vec<crate::traversal::PathStep>> {
        use std::collections::{HashMap, HashSet, VecDeque};

        // Query 1: Fetch all live edges into memory
        let (script, params) = if let Some(threshold) = weight_threshold {
            let mut p = BTreeMap::new();
            p.insert("w_threshold".into(), DataValue::from(threshold));
            (
                r#"
                    ?[from_uid, to_uid, edge_type] :=
                        *edge{from_uid, to_uid, edge_type, weight, tombstone_at},
                        tombstone_at == 0.0,
                        weight >= $w_threshold
                "#
                .to_string(),
                p,
            )
        } else {
            (
                r#"
                    ?[from_uid, to_uid, edge_type] :=
                        *edge{from_uid, to_uid, edge_type, tombstone_at},
                        tombstone_at == 0.0
                "#
                .to_string(),
                BTreeMap::new(),
            )
        };
        let result = self.run_query(&script, params)?;

        // Build adjacency lists
        let mut outgoing: HashMap<String, Vec<(String, String)>> = HashMap::new(); // from -> [(to, edge_type)]
        let mut incoming: HashMap<String, Vec<(String, String)>> = HashMap::new(); // to -> [(from, edge_type)]
        for row in &result.rows {
            let from = extract_string(&row[0])?;
            let to = extract_string(&row[1])?;
            let et = extract_string(&row[2])?;
            outgoing
                .entry(from.clone())
                .or_default()
                .push((to.clone(), et.clone()));
            incoming.entry(to).or_default().push((from, et));
        }

        // BFS on in-memory adjacency list
        let mut visited = HashSet::new();
        let mut queue: VecDeque<(String, u32, Option<String>)> = VecDeque::new();
        // (node_uid_str, edge_type, depth, parent_uid_str)
        let mut reached: Vec<(String, String, u32, Option<String>)> = Vec::new();

        let start_str = start.as_str().to_string();
        queue.push_back((start_str.clone(), 0, None));
        visited.insert(start_str);

        while let Some((current, depth, _parent)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            let mut neighbors: Vec<(&String, &String)> = Vec::new();
            match direction {
                crate::traversal::Direction::Outgoing => {
                    if let Some(edges) = outgoing.get(&current) {
                        for (to, et) in edges {
                            neighbors.push((to, et));
                        }
                    }
                }
                crate::traversal::Direction::Incoming => {
                    if let Some(edges) = incoming.get(&current) {
                        for (from, et) in edges {
                            neighbors.push((from, et));
                        }
                    }
                }
                crate::traversal::Direction::Both => {
                    if let Some(edges) = outgoing.get(&current) {
                        for (to, et) in edges {
                            neighbors.push((to, et));
                        }
                    }
                    if let Some(edges) = incoming.get(&current) {
                        for (from, et) in edges {
                            neighbors.push((from, et));
                        }
                    }
                }
            }

            for (neighbor, et) in neighbors {
                // Filter by edge types if specified
                if let Some(ref types) = edge_types {
                    let parsed = parse_edge_type(et);
                    match parsed {
                        Ok(parsed_et) if types.contains(&parsed_et) => {}
                        _ => continue,
                    }
                }

                if visited.contains(neighbor) {
                    continue;
                }

                visited.insert(neighbor.clone());
                reached.push((
                    neighbor.clone(),
                    et.clone(),
                    depth + 1,
                    Some(current.clone()),
                ));
                queue.push_back((neighbor.clone(), depth + 1, Some(current.clone())));
            }
        }

        if reached.is_empty() {
            return Ok(Vec::new());
        }

        // Query 2: Batch-fetch node metadata for all reached UIDs
        let reached_uids: Vec<&str> = reached.iter().map(|(uid, _, _, _)| uid.as_str()).collect();
        let uid_conditions: Vec<String> = reached_uids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("uid == $ruid_{}", i))
            .collect();

        let script = format!(
            r#"
            ?[uid, label, node_type, tombstone_at] :=
                *node{{uid, label, node_type, tombstone_at}},
                ({})
            "#,
            uid_conditions.join(" or ")
        );

        let mut params = BTreeMap::new();
        for (i, uid) in reached_uids.iter().enumerate() {
            params.insert(format!("ruid_{}", i), str_val(uid));
        }

        let node_result = self.run_query(&script, params)?;
        let mut node_map: HashMap<String, (String, String)> = HashMap::new(); // uid -> (label, node_type)
        for row in &node_result.rows {
            let uid = extract_string(&row[0])?;
            let label = extract_string(&row[1])?;
            let node_type = extract_string(&row[2])?;
            let ts_at = extract_float(&row[3])?;
            if ts_at == 0.0 {
                node_map.insert(uid, (label, node_type));
            }
        }

        // Build PathSteps, filtering out tombstoned/missing nodes and parsing types
        let mut steps: Vec<crate::traversal::PathStep> = Vec::new();
        for (uid, et, depth, parent) in reached {
            if let Some((label, node_type_str)) = node_map.get(&uid) {
                let nt = parse_node_type(node_type_str)?;
                let edge_t = if et.is_empty() {
                    None
                } else {
                    Some(parse_edge_type(&et)?)
                };
                steps.push(crate::traversal::PathStep {
                    node_uid: Uid::from(uid.as_str()),
                    label: label.clone(),
                    node_type: nt,
                    edge_type: edge_t,
                    depth,
                    parent_uid: parent.map(|p| Uid::from(p.as_str())),
                });
            }
        }

        steps.sort_by_key(|s| s.depth);
        Ok(steps)
    }

    /// Extract a subgraph reachable from a starting node.
    pub fn subgraph(
        &self,
        start: &Uid,
        direction: &crate::traversal::Direction,
        edge_types: &Option<Vec<EdgeType>>,
        max_depth: u32,
        weight_threshold: Option<f64>,
    ) -> Result<(Vec<GraphNode>, Vec<GraphEdge>)> {
        // Get reachable node UIDs
        let steps =
            self.traverse_reachable(start, direction, edge_types, max_depth, weight_threshold)?;
        let mut node_uids: Vec<Uid> = steps.iter().map(|s| s.node_uid.clone()).collect();
        node_uids.push(start.clone());

        let mut nodes = Vec::new();
        for uid in &node_uids {
            if let Some(node) = self.get_node(uid)? {
                if node.tombstone_at.is_none() {
                    nodes.push(node);
                }
            }
        }

        // Collect all live edges between the nodes in the subgraph
        let uid_set: std::collections::HashSet<&Uid> = node_uids.iter().collect();
        let mut edges = Vec::new();
        for uid in &node_uids {
            let from_edges = self.query_edges_from(uid, None)?;
            for edge in from_edges {
                if uid_set.contains(&edge.to_uid) {
                    edges.push(edge);
                }
            }
        }

        Ok((nodes, edges))
    }

    // ---- Pagination ----

    /// Query nodes by layer with pagination.
    pub fn query_nodes_by_layer_paginated(
        &self,
        layer: Layer,
        limit: u32,
        offset: u32,
    ) -> Result<(Vec<GraphNode>, bool)> {
        let script = format!(
            r#"
            ?[uid, node_type, layer, label, summary, created_at, updated_at, version,
              confidence, salience, privacy_level, embedding_ref,
              tombstone_at, tombstone_reason, tombstone_by, props] :=
                *node[uid, node_type, layer, label, summary, created_at, updated_at, version,
                      confidence, salience, privacy_level, embedding_ref,
                      tombstone_at, tombstone_reason, tombstone_by, props],
                layer == $layer,
                tombstone_at == 0.0
            :limit {}
            :offset {}
            "#,
            limit + 1,
            offset
        );

        let mut params = BTreeMap::new();
        params.insert("layer".into(), str_val(layer.as_str()));

        let result = self.run_query(&script, params)?;
        let has_more = result.rows.len() > limit as usize;
        let take = if has_more {
            limit as usize
        } else {
            result.rows.len()
        };
        let nodes: Result<Vec<GraphNode>> = result.rows[..take]
            .iter()
            .map(|row| self.row_to_node(row))
            .collect();
        Ok((nodes?, has_more))
    }

    /// Query edges from a node with pagination.
    pub fn query_edges_from_paginated(
        &self,
        from_uid: &Uid,
        edge_type: Option<EdgeType>,
        limit: u32,
        offset: u32,
    ) -> Result<(Vec<GraphEdge>, bool)> {
        let mut params = BTreeMap::new();
        params.insert("from_uid".into(), str_val(from_uid.as_str()));

        let script = if let Some(et) = edge_type {
            params.insert("edge_type".into(), str_val(et.as_str()));
            format!(
                r#"
                ?[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                  confidence, weight, tombstone_at, props] :=
                    *edge[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                          confidence, weight, tombstone_at, props],
                    from_uid == $from_uid,
                    edge_type == $edge_type,
                    tombstone_at == 0.0
                :limit {}
                :offset {}
                "#,
                limit + 1,
                offset
            )
        } else {
            format!(
                r#"
                ?[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                  confidence, weight, tombstone_at, props] :=
                    *edge[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                          confidence, weight, tombstone_at, props],
                    from_uid == $from_uid,
                    tombstone_at == 0.0
                :limit {}
                :offset {}
                "#,
                limit + 1,
                offset
            )
        };

        let result = self.run_query(&script, params)?;
        let has_more = result.rows.len() > limit as usize;
        let take = if has_more {
            limit as usize
        } else {
            result.rows.len()
        };
        let edges: Result<Vec<GraphEdge>> = result.rows[..take]
            .iter()
            .map(|row| self.row_to_edge(row))
            .collect();
        Ok((edges?, has_more))
    }

    /// Query weak claims with pagination.
    pub fn query_nodes_by_type_below_confidence_paginated(
        &self,
        node_type: NodeType,
        threshold: f64,
        limit: u32,
        offset: u32,
    ) -> Result<(Vec<GraphNode>, bool)> {
        let script = format!(
            r#"
            ?[uid, node_type, layer, label, summary, created_at, updated_at, version,
              confidence, salience, privacy_level, embedding_ref,
              tombstone_at, tombstone_reason, tombstone_by, props] :=
                *node[uid, node_type, layer, label, summary, created_at, updated_at, version,
                      confidence, salience, privacy_level, embedding_ref,
                      tombstone_at, tombstone_reason, tombstone_by, props],
                node_type == $node_type,
                tombstone_at == 0.0,
                confidence < $threshold
            :order confidence
            :limit {}
            :offset {}
            "#,
            limit + 1,
            offset
        );

        let mut params = BTreeMap::new();
        params.insert("node_type".into(), str_val(node_type.as_str()));
        params.insert("threshold".into(), DataValue::from(threshold));

        let result = self.run_query(&script, params)?;
        let has_more = result.rows.len() > limit as usize;
        let take = if has_more {
            limit as usize
        } else {
            result.rows.len()
        };
        let nodes: Result<Vec<GraphNode>> = result.rows[..take]
            .iter()
            .map(|row| self.row_to_node(row))
            .collect();
        Ok((nodes?, has_more))
    }

    /// Query active goals with pagination.
    pub fn query_nodes_by_type_and_prop_paginated(
        &self,
        node_type: NodeType,
        prop_field: &str,
        prop_value: &str,
        limit: u32,
        offset: u32,
    ) -> Result<(Vec<GraphNode>, bool)> {
        if !prop_field
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(Error::Validation(format!(
                "Invalid property field name: {}",
                prop_field
            )));
        }

        let script = format!(
            r#"
                ?[uid, node_type, layer, label, summary, created_at, updated_at, version,
                  confidence, salience, privacy_level, embedding_ref,
                  tombstone_at, tombstone_reason, tombstone_by, props] :=
                    *node[uid, node_type, layer, label, summary, created_at, updated_at, version,
                          confidence, salience, privacy_level, embedding_ref,
                          tombstone_at, tombstone_reason, tombstone_by, props],
                    node_type == $node_type,
                    tombstone_at == 0.0,
                    field_val = get(props, '{}', ''),
                    field_val == $prop_value
                :limit {}
                :offset {}
            "#,
            prop_field,
            limit + 1,
            offset
        );

        let mut params = BTreeMap::new();
        params.insert("node_type".into(), str_val(node_type.as_str()));
        params.insert("prop_value".into(), str_val(prop_value));

        let result = self.run_query(&script, params)?;
        let has_more = result.rows.len() > limit as usize;
        let take = if has_more {
            limit as usize
        } else {
            result.rows.len()
        };
        let nodes: Result<Vec<GraphNode>> = result.rows[..take]
            .iter()
            .map(|row| self.row_to_node(row))
            .collect();
        Ok((nodes?, has_more))
    }

    /// Query active goals with pagination, sorted by priority rank in Datalog.
    pub fn query_active_goals_paginated(
        &self,
        limit: u32,
        offset: u32,
    ) -> Result<(Vec<GraphNode>, bool)> {
        let script = format!(
            r#"
                ?[priority_rank, uid, node_type, layer, label, summary, created_at, updated_at, version,
                  confidence, salience, privacy_level, embedding_ref,
                  tombstone_at, tombstone_reason, tombstone_by, props] :=
                    *node[uid, node_type, layer, label, summary, created_at, updated_at, version,
                          confidence, salience, privacy_level, embedding_ref,
                          tombstone_at, tombstone_reason, tombstone_by, props],
                    node_type == 'Goal',
                    tombstone_at == 0.0,
                    status = get(props, 'status', ''),
                    status == 'active',
                    priority = get(props, 'priority', 'low'),
                    priority_rank = if(priority == 'critical', 0, if(priority == 'high', 1, if(priority == 'medium', 2, 3)))
                :order priority_rank
                :limit {}
                :offset {}
            "#,
            limit + 1,
            offset
        );

        let result = self.run_query(&script, BTreeMap::new())?;
        let has_more = result.rows.len() > limit as usize;
        let take = if has_more {
            limit as usize
        } else {
            result.rows.len()
        };
        // Skip the priority_rank column (index 0), node data starts at index 1
        let nodes: Result<Vec<GraphNode>> = result.rows[..take]
            .iter()
            .map(|row| self.row_to_node(&row[1..]))
            .collect();
        Ok((nodes?, has_more))
    }

    /// Query edges to a node with pagination.
    pub fn query_edges_to_paginated(
        &self,
        to_uid: &Uid,
        edge_type: Option<EdgeType>,
        limit: u32,
        offset: u32,
    ) -> Result<(Vec<GraphEdge>, bool)> {
        let mut params = BTreeMap::new();
        params.insert("to_uid".into(), str_val(to_uid.as_str()));

        let script = if let Some(et) = edge_type {
            params.insert("edge_type".into(), str_val(et.as_str()));
            format!(
                r#"
                ?[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                  confidence, weight, tombstone_at, props] :=
                    *edge[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                          confidence, weight, tombstone_at, props],
                    to_uid == $to_uid,
                    edge_type == $edge_type,
                    tombstone_at == 0.0
                :limit {}
                :offset {}
                "#,
                limit + 1,
                offset
            )
        } else {
            format!(
                r#"
                ?[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                  confidence, weight, tombstone_at, props] :=
                    *edge[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                          confidence, weight, tombstone_at, props],
                    to_uid == $to_uid,
                    tombstone_at == 0.0
                :limit {}
                :offset {}
                "#,
                limit + 1,
                offset
            )
        };

        let result = self.run_query(&script, params)?;
        let has_more = result.rows.len() > limit as usize;
        let take = if has_more {
            limit as usize
        } else {
            result.rows.len()
        };
        let edges: Result<Vec<GraphEdge>> = result.rows[..take]
            .iter()
            .map(|row| self.row_to_edge(row))
            .collect();
        Ok((edges?, has_more))
    }

    // ---- Batch Operations ----

    /// Insert multiple nodes in a single batch operation using multi-row inserts.
    pub fn insert_nodes_batch(&self, nodes: &[GraphNode]) -> Result<()> {
        const CHUNK_SIZE: usize = 100;
        for chunk in nodes.chunks(CHUNK_SIZE) {
            let mut rows = Vec::new();
            let mut params = BTreeMap::new();
            for (i, node) in chunk.iter().enumerate() {
                let props_json = node.props.try_to_json_untagged()?;
                let prefix = format!("n{}", i);
                params.insert(format!("{}_uid", prefix), str_val(node.uid.as_str()));
                params.insert(
                    format!("{}_node_type", prefix),
                    str_val(node.node_type.as_str()),
                );
                params.insert(format!("{}_layer", prefix), str_val(node.layer.as_str()));
                params.insert(format!("{}_label", prefix), str_val(&node.label));
                params.insert(format!("{}_summary", prefix), str_val(&node.summary));
                params.insert(
                    format!("{}_created_at", prefix),
                    DataValue::from(node.created_at),
                );
                params.insert(
                    format!("{}_updated_at", prefix),
                    DataValue::from(node.updated_at),
                );
                params.insert(format!("{}_version", prefix), DataValue::from(node.version));
                params.insert(
                    format!("{}_confidence", prefix),
                    DataValue::from(node.confidence.value()),
                );
                params.insert(
                    format!("{}_salience", prefix),
                    DataValue::from(node.salience.value()),
                );
                params.insert(
                    format!("{}_privacy_level", prefix),
                    str_val(node.privacy_level.as_str()),
                );
                params.insert(
                    format!("{}_embedding_ref", prefix),
                    str_val(node.embedding_ref.as_deref().unwrap_or("")),
                );
                params.insert(
                    format!("{}_tombstone_at", prefix),
                    DataValue::from(node.tombstone_at.unwrap_or(0.0)),
                );
                params.insert(
                    format!("{}_tombstone_reason", prefix),
                    str_val(node.tombstone_reason.as_deref().unwrap_or("")),
                );
                params.insert(
                    format!("{}_tombstone_by", prefix),
                    str_val(node.tombstone_by.as_deref().unwrap_or("")),
                );
                params.insert(
                    format!("{}_props", prefix),
                    DataValue::Json(cozo::JsonData(props_json)),
                );
                rows.push(format!(
                    "[${p}_uid, ${p}_node_type, ${p}_layer, ${p}_label, ${p}_summary, ${p}_created_at, ${p}_updated_at, ${p}_version, ${p}_confidence, ${p}_salience, ${p}_privacy_level, ${p}_embedding_ref, ${p}_tombstone_at, ${p}_tombstone_reason, ${p}_tombstone_by, ${p}_props]",
                    p = prefix
                ));
            }
            let script = format!(
                r#"
                ?[uid, node_type, layer, label, summary, created_at, updated_at, version,
                  confidence, salience, privacy_level, embedding_ref,
                  tombstone_at, tombstone_reason, tombstone_by, props] <- [{}]
                :put node {{
                    uid =>
                    node_type, layer, label, summary, created_at, updated_at, version,
                    confidence, salience, privacy_level, embedding_ref,
                    tombstone_at, tombstone_reason, tombstone_by, props
                }}
                "#,
                rows.join(", ")
            );
            self.run_script(&script, params)?;
        }
        Ok(())
    }

    /// Insert multiple edges in a single batch operation using multi-row inserts.
    pub fn insert_edges_batch(&self, edges: &[GraphEdge]) -> Result<()> {
        const CHUNK_SIZE: usize = 100;
        for chunk in edges.chunks(CHUNK_SIZE) {
            let mut rows = Vec::new();
            let mut params = BTreeMap::new();
            for (i, edge) in chunk.iter().enumerate() {
                let props_json = edge.props.try_to_json_untagged()?;
                let prefix = format!("e{}", i);
                params.insert(format!("{}_uid", prefix), str_val(edge.uid.as_str()));
                params.insert(
                    format!("{}_from_uid", prefix),
                    str_val(edge.from_uid.as_str()),
                );
                params.insert(format!("{}_to_uid", prefix), str_val(edge.to_uid.as_str()));
                params.insert(
                    format!("{}_edge_type", prefix),
                    str_val(edge.edge_type.as_str()),
                );
                params.insert(format!("{}_layer", prefix), str_val(edge.layer.as_str()));
                params.insert(
                    format!("{}_created_at", prefix),
                    DataValue::from(edge.created_at),
                );
                params.insert(
                    format!("{}_updated_at", prefix),
                    DataValue::from(edge.updated_at),
                );
                params.insert(format!("{}_version", prefix), DataValue::from(edge.version));
                params.insert(
                    format!("{}_confidence", prefix),
                    DataValue::from(edge.confidence.value()),
                );
                params.insert(format!("{}_weight", prefix), DataValue::from(edge.weight));
                params.insert(
                    format!("{}_tombstone_at", prefix),
                    DataValue::from(edge.tombstone_at.unwrap_or(0.0)),
                );
                params.insert(
                    format!("{}_props", prefix),
                    DataValue::Json(cozo::JsonData(props_json)),
                );
                rows.push(format!(
                    "[${p}_uid, ${p}_from_uid, ${p}_to_uid, ${p}_edge_type, ${p}_layer, ${p}_created_at, ${p}_updated_at, ${p}_version, ${p}_confidence, ${p}_weight, ${p}_tombstone_at, ${p}_props]",
                    p = prefix
                ));
            }
            let script = format!(
                r#"
                ?[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                  confidence, weight, tombstone_at, props] <- [{}]
                :put edge {{
                    uid =>
                    from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                    confidence, weight, tombstone_at, props
                }}
                "#,
                rows.join(", ")
            );
            self.run_script(&script, params)?;
        }
        Ok(())
    }

    /// Validate that all given UIDs exist as live nodes.
    pub fn validate_nodes_exist(&self, uids: &[Uid]) -> Result<Vec<Uid>> {
        let mut missing = Vec::new();
        for uid in uids {
            if !self.node_exists(uid)? {
                missing.push(uid.clone());
            }
        }
        Ok(missing)
    }

    // ---- Version History ----

    /// Get all version records for a node.
    pub fn node_versions(&self, uid: &Uid) -> Result<Vec<crate::query::VersionRecord>> {
        let script = r#"
            ?[version, changed_by, changed_at, change_type, change_reason, snapshot] :=
                *node_version[node_uid, version, snapshot, changed_by, changed_at, change_type, change_reason],
                node_uid == $uid
            :order version
        "#;

        let mut params = BTreeMap::new();
        params.insert("uid".into(), str_val(uid.as_str()));

        let result = self.run_query(script, params)?;
        let mut records = Vec::new();
        for row in &result.rows {
            records.push(crate::query::VersionRecord {
                version: extract_int(&row[0])?,
                changed_by: extract_string(&row[1])?,
                changed_at: extract_float(&row[2])?,
                change_type: extract_string(&row[3])?,
                change_reason: extract_string(&row[4])?,
                snapshot: extract_json(&row[5])?,
            });
        }
        Ok(records)
    }

    /// Get all version records for an edge.
    pub fn edge_versions(&self, uid: &Uid) -> Result<Vec<crate::query::VersionRecord>> {
        let script = r#"
            ?[version, changed_by, changed_at, change_type, change_reason, snapshot] :=
                *edge_version[edge_uid, version, snapshot, changed_by, changed_at, change_type, change_reason],
                edge_uid == $uid
            :order version
        "#;

        let mut params = BTreeMap::new();
        params.insert("uid".into(), str_val(uid.as_str()));

        let result = self.run_query(script, params)?;
        let mut records = Vec::new();
        for row in &result.rows {
            records.push(crate::query::VersionRecord {
                version: extract_int(&row[0])?,
                changed_by: extract_string(&row[1])?,
                changed_at: extract_float(&row[2])?,
                change_type: extract_string(&row[3])?,
                change_reason: extract_string(&row[4])?,
                snapshot: extract_json(&row[5])?,
            });
        }
        Ok(records)
    }

    /// Get a node at a specific version.
    pub fn node_at_version(&self, uid: &Uid, version: i64) -> Result<Option<serde_json::Value>> {
        let script = r#"
            ?[snapshot] :=
                *node_version[node_uid, version, snapshot, _, _, _, _],
                node_uid == $uid,
                version == $version
        "#;

        let mut params = BTreeMap::new();
        params.insert("uid".into(), str_val(uid.as_str()));
        params.insert("version".into(), DataValue::from(version));

        let result = self.run_query(script, params)?;
        if result.rows.is_empty() {
            return Ok(None);
        }
        Ok(Some(extract_json(&result.rows[0][0])?))
    }

    /// Query all live edges connected to a node (either direction).
    pub fn query_edges_connected(&self, uid: &Uid) -> Result<Vec<GraphEdge>> {
        // Query edges going out from this node
        let mut edges = self.query_edges_from(uid, None)?;
        // Query edges coming into this node
        let incoming = self.query_edges_to(uid, None)?;
        edges.extend(incoming);
        Ok(edges)
    }

    // ---- Full-Text Search ----

    /// Full-text search across node labels and summaries.
    pub fn query_fts_search(
        &self,
        query: &str,
        opts: &crate::query::SearchOptions,
    ) -> Result<Vec<crate::query::SearchResult>> {
        let limit = opts.limit.unwrap_or(20);
        let min_score = opts.min_score.unwrap_or(0.0);

        // CozoDB FTS `k` controls retrieval breadth, not just output limit.
        // Low k causes missed results for common terms. Use high k internally,
        // then apply user's limit as a post-filter via :limit clause.
        let fts_k = std::cmp::max(limit as i64, 500);

        let mut params = BTreeMap::new();
        params.insert("q".into(), str_val(query));
        params.insert("k".into(), DataValue::from(fts_k));
        params.insert("min_score".into(), DataValue::from(min_score));

        // Build type/layer filter clauses
        let mut extra_filters = String::new();
        if let Some(ref nt) = opts.node_type {
            params.insert("filter_node_type".into(), str_val(nt.as_str()));
            extra_filters.push_str(",\n                    node_type == $filter_node_type");
        }
        if let Some(ref layer) = opts.layer {
            params.insert("filter_layer".into(), str_val(layer.as_str()));
            extra_filters.push_str(",\n                    layer == $filter_layer");
        }

        let summary_rule = if opts.search_summary {
            r#"
                summary_matches[uid, score] :=
                    ~node:summary_fts{ uid, summary | query: $q, k: $k, bind_score: score },
                    score >= $min_score
                "#
            .to_string()
        } else {
            // Empty rule that never matches
            "summary_matches[uid, score] := uid = '', score = 0.0, false".to_string()
        };

        let script = format!(
            r#"
                label_matches[uid, score] :=
                    ~node:label_fts{{ uid, label | query: $q, k: $k, bind_score: score }},
                    score >= $min_score

                {summary_rule}

                combined[uid, max(score)] := label_matches[uid, score]
                combined[uid, max(score)] := summary_matches[uid, score]

                ?[uid, node_type, layer, label, summary, created_at, updated_at, version,
                  confidence, salience, privacy_level, embedding_ref,
                  tombstone_at, tombstone_reason, tombstone_by, props, score] :=
                    combined[uid, score],
                    *node[uid, node_type, layer, label, summary, created_at, updated_at, version,
                          confidence, salience, privacy_level, embedding_ref,
                          tombstone_at, tombstone_reason, tombstone_by, props],
                    tombstone_at == 0.0{extra_filters}
                :order -score
                :limit {limit}
            "#,
            summary_rule = summary_rule,
            extra_filters = extra_filters,
            limit = limit,
        );

        let result = self.run_query(&script, params)?;
        let mut results = Vec::new();
        for row in &result.rows {
            let node = self.row_to_node(&row[..16])?;
            let score = extract_float(&row[16])?;
            results.push(crate::query::SearchResult { node, score });
        }
        Ok(results)
    }

    // ---- Structured Filtering ----

    /// Query nodes with structured filters.
    pub fn query_nodes_filtered(
        &self,
        filter: &crate::query::NodeFilter,
    ) -> Result<(Vec<GraphNode>, bool)> {
        let limit = filter.limit.unwrap_or(100);
        let offset = filter.offset.unwrap_or(0);

        let mut params = BTreeMap::new();
        let mut conditions = Vec::new();

        if !filter.include_tombstoned {
            conditions.push("tombstone_at == 0.0".to_string());
        }
        if let Some(ref nts) = filter.node_types {
            // node_types takes precedence over node_type
            let or_parts: Vec<String> = nts
                .iter()
                .enumerate()
                .map(|(i, nt)| {
                    params.insert(format!("f_nt_{}", i), str_val(nt.as_str()));
                    format!("node_type == $f_nt_{}", i)
                })
                .collect();
            if !or_parts.is_empty() {
                conditions.push(format!("({})", or_parts.join(" or ")));
            }
        } else if let Some(ref nt) = filter.node_type {
            params.insert("f_node_type".into(), str_val(nt.as_str()));
            conditions.push("node_type == $f_node_type".to_string());
        }
        if let Some(ref layer) = filter.layer {
            params.insert("f_layer".into(), str_val(layer.as_str()));
            conditions.push("layer == $f_layer".to_string());
        }
        if let Some(ref term) = filter.label_contains {
            params.insert("f_label_term".into(), str_val(term));
            conditions.push("str_includes(label, $f_label_term)".to_string());
        }
        if let Some((ref field, ref value)) = filter.prop_equals {
            if !field.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(Error::Validation(format!(
                    "Invalid property field name: {}",
                    field
                )));
            }
            params.insert("f_prop_val".into(), str_val(value));
            conditions.push(format!("get(props, '{}', '') == $f_prop_val", field));
        }
        if let Some((ref field, ref values)) = filter.prop_in {
            if !field.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(Error::Validation(format!(
                    "Invalid property field name: {}",
                    field
                )));
            }
            let or_parts: Vec<String> = values
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    params.insert(format!("f_pv_{}", i), str_val(v));
                    format!("f_field_val == $f_pv_{}", i)
                })
                .collect();
            conditions.push(format!("f_field_val = get(props, '{}', '')", field));
            conditions.push(format!("({})", or_parts.join(" or ")));
        }
        if let Some(min) = filter.confidence_min {
            params.insert("f_conf_min".into(), DataValue::from(min));
            conditions.push("confidence >= $f_conf_min".to_string());
        }
        if let Some(max) = filter.confidence_max {
            params.insert("f_conf_max".into(), DataValue::from(max));
            conditions.push("confidence <= $f_conf_max".to_string());
        }
        if let Some(ts) = filter.created_after {
            params.insert("f_created_after".into(), DataValue::from(ts));
            conditions.push("created_at >= $f_created_after".to_string());
        }
        if let Some(ts) = filter.created_before {
            params.insert("f_created_before".into(), DataValue::from(ts));
            conditions.push("created_at <= $f_created_before".to_string());
        }
        if let Some(min) = filter.salience_min {
            params.insert("f_sal_min".into(), DataValue::from(min));
            conditions.push("salience >= $f_sal_min".to_string());
        }
        if let Some(max) = filter.salience_max {
            params.insert("f_sal_max".into(), DataValue::from(max));
            conditions.push("salience <= $f_sal_max".to_string());
        }
        // PropConditions (AND'd)
        for (i, cond) in filter.prop_conditions.iter().enumerate() {
            if !cond
                .field
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                return Err(Error::Validation(format!(
                    "Invalid property field name: {}",
                    cond.field
                )));
            }
            let var_name = format!("f_pc_{}", i);
            match &cond.op {
                crate::query::PropOp::Equals(val) => {
                    params.insert(var_name.clone(), str_val(val));
                    conditions.push(format!("get(props, '{}', '') == ${}", cond.field, var_name));
                }
                crate::query::PropOp::NotEquals(val) => {
                    params.insert(var_name.clone(), str_val(val));
                    conditions.push(format!("get(props, '{}', '') != ${}", cond.field, var_name));
                }
                crate::query::PropOp::Contains(val) => {
                    params.insert(var_name.clone(), str_val(val));
                    conditions.push(format!(
                        "str_includes(get(props, '{}', ''), ${})",
                        cond.field, var_name
                    ));
                }
                crate::query::PropOp::In(vals) => {
                    let or_parts: Vec<String> = vals
                        .iter()
                        .enumerate()
                        .map(|(j, v)| {
                            let pname = format!("{}_v{}", var_name, j);
                            params.insert(pname.clone(), str_val(v));
                            format!("pc_{}_val == ${}", i, pname)
                        })
                        .collect();
                    conditions.push(format!("pc_{}_val = get(props, '{}', '')", i, cond.field));
                    conditions.push(format!("({})", or_parts.join(" or ")));
                }
                crate::query::PropOp::GreaterThan(val) => {
                    params.insert(var_name.clone(), DataValue::from(*val));
                    conditions.push(format!(
                        "to_float(get(props, '{}', '0')) > ${}",
                        cond.field, var_name
                    ));
                }
                crate::query::PropOp::LessThan(val) => {
                    params.insert(var_name.clone(), DataValue::from(*val));
                    conditions.push(format!(
                        "to_float(get(props, '{}', '0')) < ${}",
                        cond.field, var_name
                    ));
                }
            }
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!(
                ",\n                    {}",
                conditions.join(",\n                    ")
            )
        };

        let script = format!(
            r#"
                ?[uid, node_type, layer, label, summary, created_at, updated_at, version,
                  confidence, salience, privacy_level, embedding_ref,
                  tombstone_at, tombstone_reason, tombstone_by, props] :=
                    *node[uid, node_type, layer, label, summary, created_at, updated_at, version,
                          confidence, salience, privacy_level, embedding_ref,
                          tombstone_at, tombstone_reason, tombstone_by, props]{where_clause}
                :limit {limit}
                :offset {offset}
            "#,
            where_clause = where_clause,
            limit = limit + 1,
            offset = offset,
        );

        let result = self.run_query(&script, params)?;
        let has_more = result.rows.len() > limit as usize;
        let take = if has_more {
            limit as usize
        } else {
            result.rows.len()
        };
        let nodes: Result<Vec<GraphNode>> = result.rows[..take]
            .iter()
            .map(|row| self.row_to_node(row))
            .collect();
        Ok((nodes?, has_more))
    }

    // ---- Purge Operations ----

    /// Hard-delete tombstoned nodes and their associated data.
    pub fn purge_tombstoned(&self, cutoff: Option<f64>) -> Result<crate::query::PurgeResult> {
        // 1. Find tombstoned node UIDs
        let node_script = if let Some(cutoff_ts) = cutoff {
            let mut params = BTreeMap::new();
            params.insert("cutoff".into(), DataValue::from(cutoff_ts));
            let script = r#"
                ?[uid] :=
                    *node{uid, tombstone_at},
                    tombstone_at > 0.0,
                    tombstone_at <= $cutoff
            "#;
            self.run_query(script, params)?
        } else {
            let script = r#"
                ?[uid] :=
                    *node{uid, tombstone_at},
                    tombstone_at > 0.0
            "#;
            self.run_query(script, BTreeMap::new())?
        };

        let node_uids: Vec<String> = node_script
            .rows
            .iter()
            .map(|row| extract_string(&row[0]))
            .collect::<Result<Vec<_>>>()?;

        // 2. Find tombstoned edge UIDs
        let edge_script = if let Some(cutoff_ts) = cutoff {
            let mut params = BTreeMap::new();
            params.insert("cutoff".into(), DataValue::from(cutoff_ts));
            let script = r#"
                ?[uid] :=
                    *edge{uid, tombstone_at},
                    tombstone_at > 0.0,
                    tombstone_at <= $cutoff
            "#;
            self.run_query(script, params)?
        } else {
            let script = r#"
                ?[uid] :=
                    *edge{uid, tombstone_at},
                    tombstone_at > 0.0
            "#;
            self.run_query(script, BTreeMap::new())?
        };

        let edge_uids: Vec<String> = edge_script
            .rows
            .iter()
            .map(|row| extract_string(&row[0]))
            .collect::<Result<Vec<_>>>()?;

        let mut versions_purged = 0usize;

        // 3. Delete node-related data
        for uid in &node_uids {
            let mut params = BTreeMap::new();
            params.insert("uid".into(), str_val(uid));

            // Delete node_version records
            let ver_script = r#"
                ?[node_uid, version] :=
                    *node_version{node_uid, version},
                    node_uid == $uid
                :rm node_version { node_uid, version }
            "#;
            let ver_result = self.run_script(ver_script, params.clone())?;
            versions_purged += ver_result.rows.len();

            // Delete provenance records
            let prov_script = r#"
                ?[node_uid, source_uid] :=
                    *provenance{node_uid, source_uid},
                    node_uid == $uid
                :rm provenance { node_uid, source_uid }
            "#;
            let _ = self.run_script(prov_script, params.clone());

            // Delete alias records
            let alias_script = r#"
                ?[alias_text, canonical_uid] :=
                    *alias{alias_text, canonical_uid},
                    canonical_uid == $uid
                :rm alias { alias_text, canonical_uid }
            "#;
            let _ = self.run_script(alias_script, params.clone());

            // Delete the node itself
            let node_del_script = r#"
                ?[uid] := uid = $uid
                :rm node { uid }
            "#;
            self.run_script(node_del_script, params)?;
        }

        // 4. Delete edge-related data
        for uid in &edge_uids {
            let mut params = BTreeMap::new();
            params.insert("uid".into(), str_val(uid));

            // Delete edge_version records
            let ver_script = r#"
                ?[edge_uid, version] :=
                    *edge_version{edge_uid, version},
                    edge_uid == $uid
                :rm edge_version { edge_uid, version }
            "#;
            let ver_result = self.run_script(ver_script, params.clone())?;
            versions_purged += ver_result.rows.len();

            // Delete the edge itself
            let edge_del_script = r#"
                ?[uid] := uid = $uid
                :rm edge { uid }
            "#;
            self.run_script(edge_del_script, params)?;
        }

        Ok(crate::query::PurgeResult {
            nodes_purged: node_uids.len(),
            edges_purged: edge_uids.len(),
            versions_purged,
        })
    }

    // ---- Export / Import ----

    /// Export all relations as JSON.
    pub fn export_all(&self) -> Result<BTreeMap<String, serde_json::Value>> {
        let relation_names = [
            "node",
            "edge",
            "node_version",
            "edge_version",
            "provenance",
            "alias",
        ];
        let mut result = BTreeMap::new();

        for name in &relation_names {
            let export = self
                .db
                .export_relations([name].into_iter())
                .map_err(|e| Error::Storage(e.to_string()))?;

            if let Some(named_rows) = export.get(*name) {
                let rows_json = named_rows_to_json(named_rows);
                result.insert(name.to_string(), rows_json);
            }
        }

        Ok(result)
    }

    /// Import relations from a JSON snapshot.
    pub fn import_snapshot(
        &self,
        relations: &BTreeMap<String, serde_json::Value>,
    ) -> Result<usize> {
        let mut data: BTreeMap<String, NamedRows> = BTreeMap::new();
        let mut count = 0;

        for (name, json_val) in relations {
            let named_rows = json_to_named_rows(json_val)?;
            data.insert(name.clone(), named_rows);
            count += 1;
        }

        self.db
            .import_relations(data)
            .map_err(|e| Error::Storage(e.to_string()))?;

        Ok(count)
    }

    /// Backup the database to a file path.
    pub fn backup(&self, path: &std::path::Path) -> Result<()> {
        self.db
            .backup_db(path.to_str().unwrap_or("backup.db"))
            .map_err(|e| Error::Storage(e.to_string()))
    }

    /// Restore the database from a file path.
    pub fn restore(&self, path: &std::path::Path) -> Result<()> {
        self.db
            .restore_backup(path.to_str().unwrap_or("backup.db"))
            .map_err(|e| Error::Storage(e.to_string()))
    }

    // ---- Entity Resolution (extended) ----

    /// Get all aliases for a canonical UID, ordered by score descending.
    pub fn aliases_for(&self, canonical_uid: &Uid) -> Result<Vec<(String, f64)>> {
        let script = r#"
            ?[alias_text, match_score] :=
                *alias{alias_text, canonical_uid, match_score},
                canonical_uid == $uid
            :order -match_score
        "#;
        let mut params = BTreeMap::new();
        params.insert("uid".into(), str_val(canonical_uid.as_str()));

        let result = self.run_query(script, params)?;
        let mut aliases = Vec::new();
        for row in &result.rows {
            aliases.push((extract_string(&row[0])?, extract_float(&row[1])?));
        }
        Ok(aliases)
    }

    /// Retarget all edges from/to merge_uid to keep_uid.
    pub fn retarget_edges(&self, merge_uid: &Uid, keep_uid: &Uid) -> Result<usize> {
        // Fetch all live edges where merge_uid is from or to
        let script = r#"
            ?[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
              confidence, weight, tombstone_at, props] :=
                *edge[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                      confidence, weight, tombstone_at, props],
                (from_uid == $merge_uid or to_uid == $merge_uid),
                tombstone_at == 0.0
        "#;
        let mut params = BTreeMap::new();
        params.insert("merge_uid".into(), str_val(merge_uid.as_str()));

        let result = self.run_query(script, params)?;
        let mut count = 0;

        for row in &result.rows {
            let mut edge = self.row_to_edge(row)?;
            if edge.from_uid == *merge_uid {
                edge.from_uid = keep_uid.clone();
            }
            if edge.to_uid == *merge_uid {
                edge.to_uid = keep_uid.clone();
            }
            self.insert_edge(&edge)?;
            count += 1;
        }

        Ok(count)
    }

    /// Retarget aliases from merge_uid to keep_uid.
    pub fn retarget_aliases(&self, merge_uid: &Uid, keep_uid: &Uid) -> Result<usize> {
        let script = r#"
            ?[alias_text, canonical_uid, match_score, created_at] :=
                *alias[alias_text, canonical_uid, match_score, created_at],
                canonical_uid == $merge_uid
        "#;
        let mut params = BTreeMap::new();
        params.insert("merge_uid".into(), str_val(merge_uid.as_str()));

        let result = self.run_query(script, params)?;
        let mut count = 0;

        for row in &result.rows {
            let alias_text = extract_string(&row[0])?;
            let match_score = extract_float(&row[2])?;

            // Remove old alias
            let rm_script = r#"
                ?[alias_text, canonical_uid] <- [[$alias_text, $merge_uid]]
                :rm alias { alias_text, canonical_uid }
            "#;
            let mut rm_params = BTreeMap::new();
            rm_params.insert("alias_text".into(), str_val(&alias_text));
            rm_params.insert("merge_uid".into(), str_val(merge_uid.as_str()));
            let _ = self.run_script(rm_script, rm_params);

            // Re-insert with new canonical_uid
            self.insert_alias(&alias_text, keep_uid, match_score)?;
            count += 1;
        }

        Ok(count)
    }

    // ---- Stats ----

    /// Gather graph-wide statistics.
    pub fn query_stats(&self, embedding_dim: Option<usize>) -> Result<crate::query::GraphStats> {
        use std::collections::BTreeMap;

        // Total and live nodes
        let r = self.run_query("?[count(uid)] := *node{uid}", BTreeMap::new())?;
        let total_nodes = extract_int(&r.rows[0][0])? as u64;
        let r = self.run_query(
            "?[count(uid)] := *node{uid, tombstone_at}, tombstone_at == 0.0",
            BTreeMap::new(),
        )?;
        let live_nodes = extract_int(&r.rows[0][0])? as u64;

        // Total and live edges
        let r = self.run_query("?[count(uid)] := *edge{uid}", BTreeMap::new())?;
        let total_edges = extract_int(&r.rows[0][0])? as u64;
        let r = self.run_query(
            "?[count(uid)] := *edge{uid, tombstone_at}, tombstone_at == 0.0",
            BTreeMap::new(),
        )?;
        let live_edges = extract_int(&r.rows[0][0])? as u64;

        // Nodes by type (live only)
        let r = self.run_query(
            "?[node_type, count(uid)] := *node{uid, node_type, tombstone_at}, tombstone_at == 0.0",
            BTreeMap::new(),
        )?;
        let mut nodes_by_type = BTreeMap::new();
        for row in &r.rows {
            nodes_by_type.insert(extract_string(&row[0])?, extract_int(&row[1])? as u64);
        }

        // Nodes by layer (live only)
        let r = self.run_query(
            "?[layer, count(uid)] := *node{uid, layer, tombstone_at}, tombstone_at == 0.0",
            BTreeMap::new(),
        )?;
        let mut nodes_by_layer = BTreeMap::new();
        for row in &r.rows {
            nodes_by_layer.insert(extract_string(&row[0])?, extract_int(&row[1])? as u64);
        }

        // Edges by type (live only)
        let r = self.run_query(
            "?[edge_type, count(uid)] := *edge{uid, edge_type, tombstone_at}, tombstone_at == 0.0",
            BTreeMap::new(),
        )?;
        let mut edges_by_type = BTreeMap::new();
        for row in &r.rows {
            edges_by_type.insert(extract_string(&row[0])?, extract_int(&row[1])? as u64);
        }

        // Total versions (node + edge)
        let r = self.run_query(
            "?[count(node_uid)] := *node_version{node_uid}",
            BTreeMap::new(),
        )?;
        let node_versions = extract_int(&r.rows[0][0])? as u64;
        let r = self.run_query(
            "?[count(edge_uid)] := *edge_version{edge_uid}",
            BTreeMap::new(),
        )?;
        let edge_versions = extract_int(&r.rows[0][0])? as u64;
        let total_versions = node_versions + edge_versions;

        // Total aliases
        let r = self.run_query(
            "?[count(alias_text)] := *alias{alias_text}",
            BTreeMap::new(),
        )?;
        let total_aliases = extract_int(&r.rows[0][0])? as u64;

        // Embedding count
        let embedding_count = self.count_embeddings()?;

        Ok(crate::query::GraphStats {
            total_nodes,
            total_edges,
            live_nodes,
            live_edges,
            nodes_by_type,
            nodes_by_layer,
            edges_by_type,
            tombstoned_nodes: total_nodes - live_nodes,
            tombstoned_edges: total_edges - live_edges,
            total_versions,
            total_aliases,
            embedding_count,
            embedding_dimension: embedding_dim,
        })
    }

    // ---- Embedding Operations ----

    /// Create the embedding relation and HNSW index.
    pub fn create_embedding_schema(&self, dimension: usize) -> Result<()> {
        let create_script = format!(
            ":create node_embedding {{ uid: String => embedding: <F32; {}> }}",
            dimension
        );
        self.run_script(&create_script, BTreeMap::new())?;

        let hnsw_script = format!(
            "::hnsw create node_embedding:semantic_idx {{ dim: {}, dtype: F32, fields: [embedding], distance: Cosine, m: 50, ef_construction: 200 }}",
            dimension
        );
        self.run_script(&hnsw_script, BTreeMap::new())?;

        // Store dimension in mg_meta
        let mut params = BTreeMap::new();
        params.insert("key".into(), str_val("embedding_dimension"));
        params.insert("value".into(), str_val(&dimension.to_string()));
        self.run_script(
            r#"?[key, value] <- [[$key, $value]] :put mg_meta { key => value }"#,
            params,
        )?;

        Ok(())
    }

    /// Drop the embedding schema (relation + HNSW index + metadata).
    /// Used when reconfiguring to a different dimension.
    pub fn drop_embedding_schema(&self) -> Result<()> {
        // Drop HNSW index first (ignore errors if not exists)
        let _ = self.run_script("::hnsw drop node_embedding:semantic_idx", BTreeMap::new());
        // Drop the relation (ignore errors if not exists)
        let _ = self.run_script("::remove node_embedding", BTreeMap::new());
        // Clear the dimension metadata
        let mut params = BTreeMap::new();
        params.insert("key".into(), str_val("embedding_dimension"));
        let _ = self.run_script(r#"?[key] <- [[$key]] :rm mg_meta { key }"#, params);
        Ok(())
    }

    /// Read embedding dimension from mg_meta.
    pub fn get_embedding_dimension(&self) -> Result<Option<usize>> {
        let mut params = BTreeMap::new();
        params.insert("key".into(), str_val("embedding_dimension"));
        let result = self.run_query(r#"?[value] := *mg_meta{key, value}, key == $key"#, params)?;
        if result.rows.is_empty() {
            return Ok(None);
        }
        let dim_str = extract_string(&result.rows[0][0])?;
        dim_str
            .parse::<usize>()
            .map(Some)
            .map_err(|e| Error::Storage(e.to_string()))
    }

    /// Upsert an embedding vector for a node.
    pub fn upsert_embedding(&self, uid: &Uid, embedding: &[f32]) -> Result<()> {
        let vec_str: String = embedding
            .iter()
            .map(|v| format!("{}", v))
            .collect::<Vec<_>>()
            .join(", ");
        let script = format!(
            r#"?[uid, embedding] <- [[$uid, vec([{}])]]
            :put node_embedding {{ uid => embedding }}"#,
            vec_str
        );
        let mut params = BTreeMap::new();
        params.insert("uid".into(), str_val(uid.as_str()));
        self.run_script(&script, params)?;

        Ok(())
    }

    /// Get embedding for a node.
    pub fn get_embedding(&self, uid: &Uid) -> Result<Option<Vec<f32>>> {
        let script = r#"
            ?[embedding] := *node_embedding{uid, embedding}, uid == $uid
        "#;
        let mut params = BTreeMap::new();
        params.insert("uid".into(), str_val(uid.as_str()));

        let result = self.run_query(script, params);
        match result {
            Ok(r) => {
                if r.rows.is_empty() {
                    return Ok(None);
                }
                match &r.rows[0][0] {
                    DataValue::List(items) => {
                        let vec: Vec<f32> = items
                            .iter()
                            .map(|v| match v {
                                DataValue::Num(n) => match n {
                                    cozo::Num::Float(f) => *f as f32,
                                    cozo::Num::Int(i) => *i as f32,
                                },
                                _ => 0.0,
                            })
                            .collect();
                        Ok(Some(vec))
                    }
                    DataValue::Vec(v) => {
                        use cozo::Vector;
                        match v {
                            Vector::F32(arr) => Ok(Some(arr.to_vec())),
                            Vector::F64(arr) => Ok(Some(arr.iter().map(|x| *x as f32).collect())),
                        }
                    }
                    _ => Ok(None),
                }
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("not found")
                    || msg.contains("does not exist")
                    || msg.contains("Cannot find")
                {
                    Ok(None)
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Delete embedding for a node.
    pub fn delete_embedding(&self, uid: &Uid) -> Result<()> {
        let script = r#"
            ?[uid] <- [[$uid]]
            :rm node_embedding { uid }
        "#;
        let mut params = BTreeMap::new();
        params.insert("uid".into(), str_val(uid.as_str()));
        self.run_script(script, params)?;
        Ok(())
    }

    /// Semantic search using HNSW index.
    pub fn semantic_search_raw(
        &self,
        query_vec: &[f32],
        k: usize,
        ef: usize,
    ) -> Result<Vec<(Uid, f64)>> {
        let vec_str: String = query_vec
            .iter()
            .map(|v| format!("{}", v))
            .collect::<Vec<_>>()
            .join(", ");
        let script = format!(
            r#"?[uid, dist] := ~node_embedding:semantic_idx{{ uid | query: vec([{vec}]), k: {k}, ef: {ef}, bind_distance: dist }}"#,
            vec = vec_str,
            k = k,
            ef = ef,
        );
        let result = self.run_query(&script, BTreeMap::new())?;
        let mut results = Vec::new();
        for row in &result.rows {
            let uid = Uid::from(extract_string(&row[0])?.as_str());
            let dist = extract_float(&row[1])?;
            results.push((uid, dist));
        }
        Ok(results)
    }

    /// Count embeddings (returns 0 if relation doesn't exist).
    pub fn count_embeddings(&self) -> Result<u64> {
        let result = self.run_query("?[count(uid)] := *node_embedding{uid}", BTreeMap::new());
        match result {
            Ok(r) => {
                if r.rows.is_empty() {
                    return Ok(0);
                }
                Ok(extract_int(&r.rows[0][0])? as u64)
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("not found")
                    || msg.contains("does not exist")
                    || msg.contains("Cannot find")
                {
                    Ok(0)
                } else {
                    Err(e)
                }
            }
        }
    }

    // ---- Decay Operations ----

    /// Apply exponential salience decay to all live nodes.
    /// Returns (uid, old_salience, new_salience) for changed nodes.
    pub fn apply_salience_decay(
        &self,
        half_life_secs: f64,
        current_time: f64,
    ) -> Result<Vec<(String, f64, f64)>> {
        // Step 1: Read all live nodes with their salience and updated_at
        let script = r#"
            ?[uid, salience, updated_at] :=
                *node{uid, salience, updated_at, tombstone_at},
                tombstone_at == 0.0
        "#;
        let result = self.run_query(script, BTreeMap::new())?;

        let mut changed = Vec::new();
        for row in &result.rows {
            let uid_str = extract_string(&row[0])?;
            let old_salience = extract_float(&row[1])?;
            let updated_at = extract_float(&row[2])?;

            let elapsed = current_time - updated_at;
            if elapsed <= 0.0 {
                continue;
            }

            let decay_factor = (0.5_f64).powf(elapsed / half_life_secs);
            let new_salience = (old_salience * decay_factor).clamp(0.0, 1.0);

            if (new_salience - old_salience).abs() < 1e-9 {
                continue;
            }

            changed.push((uid_str, old_salience, new_salience));
        }

        // Step 2: Batch-read all changed nodes and batch-write updated salience values
        if !changed.is_empty() {
            // Build a lookup map for new salience values
            let salience_map: std::collections::HashMap<&str, f64> = changed
                .iter()
                .map(|(uid, _old, new_sal)| (uid.as_str(), *new_sal))
                .collect();

            // Batch-read full node rows in chunks
            let uid_strs: Vec<&str> = changed.iter().map(|(u, _, _)| u.as_str()).collect();
            let mut nodes_to_write: Vec<GraphNode> = Vec::with_capacity(changed.len());

            for chunk in uid_strs.chunks(100) {
                let or_parts: Vec<String> = chunk
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("uid == $br_{}", i))
                    .collect();
                let read_script = format!(
                    r#"
                    ?[uid, node_type, layer, label, summary, created_at, updated_at, version,
                      confidence, salience, privacy_level, embedding_ref,
                      tombstone_at, tombstone_reason, tombstone_by, props] :=
                        *node[uid, node_type, layer, label, summary, created_at, updated_at, version,
                              confidence, salience, privacy_level, embedding_ref,
                              tombstone_at, tombstone_reason, tombstone_by, props],
                        ({})
                    "#,
                    or_parts.join(" or ")
                );
                let mut params = BTreeMap::new();
                for (i, uid_str) in chunk.iter().enumerate() {
                    params.insert(format!("br_{}", i), str_val(uid_str));
                }
                let r = self.run_query(&read_script, params)?;
                for row in &r.rows {
                    let mut node = self.row_to_node(row)?;
                    if let Some(&new_sal) = salience_map.get(node.uid.as_str()) {
                        node.salience = crate::types::Salience::new(new_sal).unwrap_or_default();
                        node.updated_at = current_time;
                        nodes_to_write.push(node);
                    }
                }
            }

            // Batch-write all updated nodes
            self.insert_nodes_batch(&nodes_to_write)?;
        }

        Ok(changed)
    }

    /// Query nodes with salience below a threshold and created before a cutoff time.
    pub fn query_low_salience_old_nodes(
        &self,
        min_salience: f64,
        created_before: f64,
    ) -> Result<Vec<Uid>> {
        let script = r#"
            ?[uid] :=
                *node{uid, salience, created_at, tombstone_at},
                tombstone_at == 0.0,
                salience < $min_salience,
                created_at < $created_before
        "#;
        let mut params = BTreeMap::new();
        params.insert("min_salience".into(), DataValue::from(min_salience));
        params.insert("created_before".into(), DataValue::from(created_before));

        let result = self.run_query(script, params)?;
        let mut uids = Vec::new();
        for row in &result.rows {
            uids.push(Uid::from(extract_string(&row[0])?.as_str()));
        }
        Ok(uids)
    }

    // ---- Typed Export ----

    /// Export all live nodes.
    pub fn export_all_live_nodes(&self) -> Result<Vec<GraphNode>> {
        let script = r#"
            ?[uid, node_type, layer, label, summary, created_at, updated_at, version,
              confidence, salience, privacy_level, embedding_ref,
              tombstone_at, tombstone_reason, tombstone_by, props] :=
                *node[uid, node_type, layer, label, summary, created_at, updated_at, version,
                      confidence, salience, privacy_level, embedding_ref,
                      tombstone_at, tombstone_reason, tombstone_by, props],
                tombstone_at == 0.0
        "#;
        let result = self.run_query(script, BTreeMap::new())?;
        result
            .rows
            .iter()
            .map(|row| self.row_to_node(row))
            .collect()
    }

    /// Export all live edges.
    pub fn export_all_live_edges(&self) -> Result<Vec<GraphEdge>> {
        let script = r#"
            ?[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
              confidence, weight, tombstone_at, props] :=
                *edge[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                      confidence, weight, tombstone_at, props],
                tombstone_at == 0.0
        "#;
        let result = self.run_query(script, BTreeMap::new())?;
        result
            .rows
            .iter()
            .map(|row| self.row_to_edge(row))
            .collect()
    }

    /// Export all embeddings. Gracefully handles missing `node_embedding` relation.
    pub fn export_all_embeddings(&self) -> Result<Vec<(Uid, Vec<f32>)>> {
        let result = self.run_query(
            "?[uid, embedding] := *node_embedding{uid, embedding}",
            BTreeMap::new(),
        );
        match result {
            Ok(r) => {
                let mut embeddings = Vec::new();
                for row in &r.rows {
                    let uid = Uid::from(extract_string(&row[0])?.as_str());
                    let vec = match &row[1] {
                        DataValue::List(items) => items
                            .iter()
                            .map(|v| match v {
                                DataValue::Num(n) => match n {
                                    cozo::Num::Float(f) => *f as f32,
                                    cozo::Num::Int(i) => *i as f32,
                                },
                                _ => 0.0,
                            })
                            .collect(),
                        DataValue::Vec(v) => {
                            use cozo::Vector;
                            match v {
                                Vector::F32(arr) => arr.to_vec(),
                                Vector::F64(arr) => arr.iter().map(|x| *x as f32).collect(),
                            }
                        }
                        _ => continue,
                    };
                    embeddings.push((uid, vec));
                }
                Ok(embeddings)
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("not found")
                    || msg.contains("does not exist")
                    || msg.contains("Cannot find")
                {
                    Ok(Vec::new())
                } else {
                    Err(e)
                }
            }
        }
    }

    // ---- Connected-to query helper ----

    /// Get UIDs of all nodes connected to a given node (either direction).
    pub fn query_connected_uids(&self, uid: &Uid) -> Result<Vec<Uid>> {
        let script = r#"
            connected[to_uid] := *edge{from_uid, to_uid, tombstone_at}, from_uid == $uid, tombstone_at == 0.0
            connected[from_uid] := *edge{from_uid, to_uid, tombstone_at}, to_uid == $uid, tombstone_at == 0.0
            ?[uid] := connected[uid]
        "#;
        let mut params = BTreeMap::new();
        params.insert("uid".into(), str_val(uid.as_str()));
        let result = self.run_query(script, params)?;
        let mut uids = Vec::new();
        for row in &result.rows {
            uids.push(Uid::from(extract_string(&row[0])?.as_str()));
        }
        Ok(uids)
    }

    /// Fuzzy resolve alias text using substring matching.
    pub fn fuzzy_resolve_alias(&self, text: &str, limit: u32) -> Result<Vec<(Uid, f64)>> {
        let script = format!(
            r#"
                ?[canonical_uid, match_score] :=
                    *alias{{alias_text, canonical_uid, match_score}},
                    str_includes(alias_text, $text)
                :order -match_score
                :limit {}
            "#,
            limit
        );
        let mut params = BTreeMap::new();
        params.insert("text".into(), str_val(text));

        let result = self.run_query(&script, params)?;
        let mut results = Vec::new();
        for row in &result.rows {
            let uid = Uid::from(extract_string(&row[0])?.as_str());
            let score = extract_float(&row[1])?;
            results.push((uid, score));
        }
        Ok(results)
    }

    // ---- Internal helpers ----

    fn node_to_params(
        &self,
        node: &GraphNode,
        props_json: serde_json::Value,
    ) -> BTreeMap<String, DataValue> {
        let mut p = BTreeMap::new();
        p.insert("uid".into(), str_val(node.uid.as_str()));
        p.insert("node_type".into(), str_val(node.node_type.as_str()));
        p.insert("layer".into(), str_val(node.layer.as_str()));
        p.insert("label".into(), str_val(&node.label));
        p.insert("summary".into(), str_val(&node.summary));
        p.insert("created_at".into(), DataValue::from(node.created_at));
        p.insert("updated_at".into(), DataValue::from(node.updated_at));
        p.insert("version".into(), DataValue::from(node.version));
        p.insert(
            "confidence".into(),
            DataValue::from(node.confidence.value()),
        );
        p.insert("salience".into(), DataValue::from(node.salience.value()));
        p.insert("privacy_level".into(), str_val(node.privacy_level.as_str()));
        p.insert(
            "embedding_ref".into(),
            str_val(node.embedding_ref.as_deref().unwrap_or("")),
        );
        p.insert(
            "tombstone_at".into(),
            DataValue::from(node.tombstone_at.unwrap_or(0.0)),
        );
        p.insert(
            "tombstone_reason".into(),
            str_val(node.tombstone_reason.as_deref().unwrap_or("")),
        );
        p.insert(
            "tombstone_by".into(),
            str_val(node.tombstone_by.as_deref().unwrap_or("")),
        );
        p.insert("props".into(), DataValue::Json(cozo::JsonData(props_json)));
        p
    }

    fn edge_to_params(
        &self,
        edge: &GraphEdge,
        props_json: serde_json::Value,
    ) -> BTreeMap<String, DataValue> {
        let mut p = BTreeMap::new();
        p.insert("uid".into(), str_val(edge.uid.as_str()));
        p.insert("from_uid".into(), str_val(edge.from_uid.as_str()));
        p.insert("to_uid".into(), str_val(edge.to_uid.as_str()));
        p.insert("edge_type".into(), str_val(edge.edge_type.as_str()));
        p.insert("layer".into(), str_val(edge.layer.as_str()));
        p.insert("created_at".into(), DataValue::from(edge.created_at));
        p.insert("updated_at".into(), DataValue::from(edge.updated_at));
        p.insert("version".into(), DataValue::from(edge.version));
        p.insert(
            "confidence".into(),
            DataValue::from(edge.confidence.value()),
        );
        p.insert("weight".into(), DataValue::from(edge.weight));
        p.insert(
            "tombstone_at".into(),
            DataValue::from(edge.tombstone_at.unwrap_or(0.0)),
        );
        p.insert("props".into(), DataValue::Json(cozo::JsonData(props_json)));
        p
    }

    fn row_to_node(&self, row: &[DataValue]) -> Result<GraphNode> {
        if row.len() < 16 {
            return Err(Error::Storage(format!(
                "Invalid node row length: expected 16 columns, got {}",
                row.len()
            )));
        }
        let uid = Uid::from(extract_string(&row[0])?.as_str());
        let node_type_str = extract_string(&row[1])?;
        let node_type = parse_node_type(&node_type_str)?;
        let layer_str = extract_string(&row[2])?;
        let layer = parse_layer(&layer_str)?;
        let label = extract_string(&row[3])?;
        let summary = extract_string(&row[4])?;
        let created_at = extract_float(&row[5])?;
        let updated_at = extract_float(&row[6])?;
        let version = extract_int(&row[7])?;
        let confidence = Confidence::new(extract_float(&row[8])?).unwrap_or_default();
        let salience = Salience::new(extract_float(&row[9])?).unwrap_or_default();
        let privacy_str = extract_string(&row[10])?;
        let privacy_level = parse_privacy_level(&privacy_str)?;
        let emb_str = extract_string(&row[11])?;
        let embedding_ref = if emb_str.is_empty() {
            None
        } else {
            Some(emb_str)
        };
        let ts_at = extract_float(&row[12])?;
        let tombstone_at = if ts_at == 0.0 { None } else { Some(ts_at) };
        let ts_reason = extract_string(&row[13])?;
        let tombstone_reason = if ts_reason.is_empty() {
            None
        } else {
            Some(ts_reason)
        };
        let ts_by = extract_string(&row[14])?;
        let tombstone_by = if ts_by.is_empty() { None } else { Some(ts_by) };
        let props_json = extract_json(&row[15])?;
        let props = NodeProps::from_json(&node_type, &props_json)?;
        // For custom types, use the layer from the deserialized props
        let layer = if node_type.is_custom() {
            props.layer()
        } else {
            layer
        };

        Ok(GraphNode {
            uid,
            node_type,
            layer,
            label,
            summary,
            created_at,
            updated_at,
            version,
            confidence,
            salience,
            privacy_level,
            embedding_ref,
            tombstone_at,
            tombstone_reason,
            tombstone_by,
            props,
        })
    }

    fn row_to_edge(&self, row: &[DataValue]) -> Result<GraphEdge> {
        if row.len() < 12 {
            return Err(Error::Storage(format!(
                "Invalid edge row length: expected 12 columns, got {}",
                row.len()
            )));
        }
        let uid = Uid::from(extract_string(&row[0])?.as_str());
        let from_uid = Uid::from(extract_string(&row[1])?.as_str());
        let to_uid = Uid::from(extract_string(&row[2])?.as_str());
        let edge_type_str = extract_string(&row[3])?;
        let edge_type = parse_edge_type(&edge_type_str)?;
        let layer_str = extract_string(&row[4])?;
        let layer = parse_layer(&layer_str)?;
        let created_at = extract_float(&row[5])?;
        let updated_at = extract_float(&row[6])?;
        let version = extract_int(&row[7])?;
        let confidence = Confidence::new(extract_float(&row[8])?).unwrap_or_default();
        let weight = extract_float(&row[9])?;
        let ts_at = extract_float(&row[10])?;
        let tombstone_at = if ts_at == 0.0 { None } else { Some(ts_at) };
        let props_json = extract_json(&row[11])?;
        let props = EdgeProps::from_json(&edge_type, &props_json)?;

        Ok(GraphEdge {
            uid,
            from_uid,
            to_uid,
            edge_type,
            layer,
            created_at,
            updated_at,
            version,
            confidence,
            weight,
            tombstone_at,
            props,
        })
    }

    // ---- v0.5: get_edge_between ----

    /// Get live edges between two nodes, optionally filtered by edge type.
    pub fn query_edges_between(
        &self,
        from_uid: &Uid,
        to_uid: &Uid,
        edge_type: Option<EdgeType>,
    ) -> Result<Vec<GraphEdge>> {
        let script = if edge_type.is_some() {
            r#"
                ?[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                  confidence, weight, tombstone_at, props] :=
                    *edge[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                          confidence, weight, tombstone_at, props],
                    from_uid == $from_uid,
                    to_uid == $to_uid,
                    edge_type == $edge_type,
                    tombstone_at == 0.0
            "#
        } else {
            r#"
                ?[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                  confidence, weight, tombstone_at, props] :=
                    *edge[uid, from_uid, to_uid, edge_type, layer, created_at, updated_at, version,
                          confidence, weight, tombstone_at, props],
                    from_uid == $from_uid,
                    to_uid == $to_uid,
                    tombstone_at == 0.0
            "#
        };

        let mut params = BTreeMap::new();
        params.insert("from_uid".into(), str_val(from_uid.as_str()));
        params.insert("to_uid".into(), str_val(to_uid.as_str()));
        if let Some(et) = edge_type {
            params.insert("edge_type".into(), str_val(et.as_str()));
        }

        let result = self.run_query(script, params)?;
        result
            .rows
            .iter()
            .map(|row| self.row_to_edge(row))
            .collect()
    }

    // ---- v0.5: list_nodes (paginated) ----

    /// List all live nodes with pagination.
    pub fn query_all_live_nodes_paginated(
        &self,
        limit: u32,
        offset: u32,
    ) -> Result<(Vec<GraphNode>, bool)> {
        let effective_limit = limit as usize + 1;
        let script = r#"
            ?[uid, node_type, layer, label, summary, created_at, updated_at, version,
              confidence, salience, privacy_level, embedding_ref,
              tombstone_at, tombstone_reason, tombstone_by, props] :=
                *node[uid, node_type, layer, label, summary, created_at, updated_at, version,
                      confidence, salience, privacy_level, embedding_ref,
                      tombstone_at, tombstone_reason, tombstone_by, props],
                tombstone_at == 0.0
            :limit $limit
            :offset $offset
        "#;

        let mut params = BTreeMap::new();
        params.insert("limit".into(), DataValue::from(effective_limit as i64));
        params.insert("offset".into(), DataValue::from(offset as i64));

        let result = self.run_query(script, params)?;
        let has_more = result.rows.len() > limit as usize;
        let rows = if has_more {
            &result.rows[..limit as usize]
        } else {
            &result.rows
        };
        let nodes: Vec<GraphNode> = rows
            .iter()
            .map(|row| self.row_to_node(row))
            .collect::<Result<_>>()?;
        Ok((nodes, has_more))
    }

    // ---- v0.6: Multi-Agent ----

    /// Query all live nodes created by a specific agent (version 1, changed_by == agent_id).
    pub fn query_nodes_by_agent(&self, agent_id: &str) -> Result<Vec<GraphNode>> {
        let script = r#"
            ?[uid, node_type, layer, label, summary, created_at, updated_at, version,
              confidence, salience, privacy_level, embedding_ref,
              tombstone_at, tombstone_reason, tombstone_by, props] :=
                *node_version{node_uid, version: ver, changed_by},
                ver == 1,
                changed_by == $agent_id,
                *node[node_uid, node_type, layer, label, summary, created_at, updated_at, version,
                      confidence, salience, privacy_level, embedding_ref,
                      tombstone_at, tombstone_reason, tombstone_by, props],
                tombstone_at == 0.0,
                uid = node_uid
        "#;

        let mut params = BTreeMap::new();
        params.insert("agent_id".into(), str_val(agent_id));

        let result = self.run_query(script, params)?;
        result
            .rows
            .iter()
            .map(|row| self.row_to_node(row))
            .collect()
    }

    // ---- v0.5: clear ----

    /// Delete all data from all relations. Destructive operation for testing/reset.
    pub fn clear_all(&self) -> Result<()> {
        // Each relation needs its key columns specified for :rm
        let clear_scripts = [
            "?[uid] := *edge{uid}\n:rm edge {uid}",
            "?[uid] := *node{uid}\n:rm node {uid}",
            "?[node_uid, version] := *node_version{node_uid, version}\n:rm node_version {node_uid, version}",
            "?[edge_uid, version] := *edge_version{edge_uid, version}\n:rm edge_version {edge_uid, version}",
            "?[node_uid, source_uid] := *provenance{node_uid, source_uid}\n:rm provenance {node_uid, source_uid}",
            "?[alias_text, canonical_uid] := *alias{alias_text, canonical_uid}\n:rm alias {alias_text, canonical_uid}",
            "?[key] := *mg_meta{key}\n:rm mg_meta {key}",
        ];
        for script in &clear_scripts {
            self.run_script(script, BTreeMap::new())?;
        }
        // Also try to clear embeddings if they exist
        let _ = self.run_script(
            "?[uid] := *node_embedding{uid}\n:rm node_embedding {uid}",
            BTreeMap::new(),
        );
        Ok(())
    }
}

// ---- Free-standing helpers ----

fn str_val(s: &str) -> DataValue {
    DataValue::Str(smartstring::SmartString::from(s))
}

pub(crate) fn extract_string(val: &DataValue) -> Result<String> {
    match val {
        DataValue::Str(s) => Ok(s.to_string()),
        DataValue::Null => Ok(String::new()),
        other => Err(Error::Storage(format!("Expected string, got {:?}", other))),
    }
}

fn extract_float(val: &DataValue) -> Result<f64> {
    match val {
        DataValue::Num(n) => Ok(match n {
            cozo::Num::Int(i) => *i as f64,
            cozo::Num::Float(f) => *f,
        }),
        DataValue::Null => Ok(0.0),
        other => Err(Error::Storage(format!("Expected number, got {:?}", other))),
    }
}

fn extract_int(val: &DataValue) -> Result<i64> {
    match val {
        DataValue::Num(n) => Ok(match n {
            cozo::Num::Int(i) => *i,
            cozo::Num::Float(f) => *f as i64,
        }),
        DataValue::Null => Ok(0),
        other => Err(Error::Storage(format!("Expected integer, got {:?}", other))),
    }
}

fn extract_json(val: &DataValue) -> Result<serde_json::Value> {
    match val {
        DataValue::Json(j) => Ok(j.0.clone()),
        DataValue::Null => Ok(serde_json::Value::Object(Default::default())),
        other => Err(Error::Storage(format!("Expected JSON, got {:?}", other))),
    }
}

fn parse_node_type(s: &str) -> Result<NodeType> {
    match s {
        "Source" => Ok(NodeType::Source),
        "Snippet" => Ok(NodeType::Snippet),
        "Entity" => Ok(NodeType::Entity),
        "Observation" => Ok(NodeType::Observation),
        "Claim" => Ok(NodeType::Claim),
        "Evidence" => Ok(NodeType::Evidence),
        "Warrant" => Ok(NodeType::Warrant),
        "Argument" => Ok(NodeType::Argument),
        "Hypothesis" => Ok(NodeType::Hypothesis),
        "Theory" => Ok(NodeType::Theory),
        "Paradigm" => Ok(NodeType::Paradigm),
        "Anomaly" => Ok(NodeType::Anomaly),
        "Method" => Ok(NodeType::Method),
        "Experiment" => Ok(NodeType::Experiment),
        "Concept" => Ok(NodeType::Concept),
        "Assumption" => Ok(NodeType::Assumption),
        "Question" => Ok(NodeType::Question),
        "OpenQuestion" => Ok(NodeType::OpenQuestion),
        "Analogy" => Ok(NodeType::Analogy),
        "Pattern" => Ok(NodeType::Pattern),
        "Mechanism" => Ok(NodeType::Mechanism),
        "Model" => Ok(NodeType::Model),
        "ModelEvaluation" => Ok(NodeType::ModelEvaluation),
        "InferenceChain" => Ok(NodeType::InferenceChain),
        "SensitivityAnalysis" => Ok(NodeType::SensitivityAnalysis),
        "ReasoningStrategy" => Ok(NodeType::ReasoningStrategy),
        "Theorem" => Ok(NodeType::Theorem),
        "Equation" => Ok(NodeType::Equation),
        "Goal" => Ok(NodeType::Goal),
        "Project" => Ok(NodeType::Project),
        "Decision" => Ok(NodeType::Decision),
        "Option" => Ok(NodeType::Option),
        "Constraint" => Ok(NodeType::Constraint),
        "Milestone" => Ok(NodeType::Milestone),
        "Affordance" => Ok(NodeType::Affordance),
        "Flow" => Ok(NodeType::Flow),
        "FlowStep" => Ok(NodeType::FlowStep),
        "Control" => Ok(NodeType::Control),
        "RiskAssessment" => Ok(NodeType::RiskAssessment),
        "Session" => Ok(NodeType::Session),
        "Trace" => Ok(NodeType::Trace),
        "Summary" => Ok(NodeType::Summary),
        "Preference" => Ok(NodeType::Preference),
        "MemoryPolicy" => Ok(NodeType::MemoryPolicy),
        "Agent" => Ok(NodeType::Agent),
        "Task" => Ok(NodeType::Task),
        "Plan" => Ok(NodeType::Plan),
        "PlanStep" => Ok(NodeType::PlanStep),
        "Approval" => Ok(NodeType::Approval),
        "Policy" => Ok(NodeType::Policy),
        "Execution" => Ok(NodeType::Execution),
        "SafetyBudget" => Ok(NodeType::SafetyBudget),
        other => Ok(NodeType::Custom(other.to_string())),
    }
}

fn parse_edge_type(s: &str) -> Result<EdgeType> {
    match s {
        "EXTRACTED_FROM" => Ok(EdgeType::ExtractedFrom),
        "PART_OF" => Ok(EdgeType::PartOf),
        "HAS_PART" => Ok(EdgeType::HasPart),
        "INSTANCE_OF" => Ok(EdgeType::InstanceOf),
        "CONTAINS" => Ok(EdgeType::Contains),
        "SUPPORTS" => Ok(EdgeType::Supports),
        "REFUTES" => Ok(EdgeType::Refutes),
        "JUSTIFIES" => Ok(EdgeType::Justifies),
        "HAS_PREMISE" => Ok(EdgeType::HasPremise),
        "HAS_CONCLUSION" => Ok(EdgeType::HasConclusion),
        "HAS_WARRANT" => Ok(EdgeType::HasWarrant),
        "REBUTS" => Ok(EdgeType::Rebuts),
        "ASSUMES" => Ok(EdgeType::Assumes),
        "TESTS" => Ok(EdgeType::Tests),
        "PRODUCES" => Ok(EdgeType::Produces),
        "USES_METHOD" => Ok(EdgeType::UsesMethod),
        "ADDRESSES" => Ok(EdgeType::Addresses),
        "GENERATES" => Ok(EdgeType::Generates),
        "EXTENDS" => Ok(EdgeType::Extends),
        "SUPERSEDES" => Ok(EdgeType::Supersedes),
        "CONTRADICTS" => Ok(EdgeType::Contradicts),
        "ANOMALOUS_TO" => Ok(EdgeType::AnomalousTo),
        "ANALOGOUS_TO" => Ok(EdgeType::AnalogousTo),
        "INSTANTIATES" => Ok(EdgeType::Instantiates),
        "TRANSFERS_TO" => Ok(EdgeType::TransfersTo),
        "EVALUATES" => Ok(EdgeType::Evaluates),
        "OUTPERFORMS" => Ok(EdgeType::Outperforms),
        "FAILS_ON" => Ok(EdgeType::FailsOn),
        "HAS_CHAIN_STEP" => Ok(EdgeType::HasChainStep),
        "PROPAGATES_UNCERTAINTY_TO" => Ok(EdgeType::PropagatesUncertaintyTo),
        "SENSITIVE_TO" => Ok(EdgeType::SensitiveTo),
        "ROBUST_ACROSS" => Ok(EdgeType::RobustAcross),
        "DESCRIBES" => Ok(EdgeType::Describes),
        "DERIVED_FROM" => Ok(EdgeType::DerivedFrom),
        "RELIES_ON" => Ok(EdgeType::ReliesOn),
        "PROVEN_BY" => Ok(EdgeType::ProvenBy),
        "PROPOSED_BY" => Ok(EdgeType::ProposedBy),
        "AUTHORED_BY" => Ok(EdgeType::AuthoredBy),
        "CITED_BY" => Ok(EdgeType::CitedBy),
        "BELIEVED_BY" => Ok(EdgeType::BelievedBy),
        "CONSENSUS_IN" => Ok(EdgeType::ConsensusIn),
        "DECOMPOSES_INTO" => Ok(EdgeType::DecomposesInto),
        "MOTIVATED_BY" => Ok(EdgeType::MotivatedBy),
        "HAS_OPTION" => Ok(EdgeType::HasOption),
        "DECIDED_ON" => Ok(EdgeType::DecidedOn),
        "CONSTRAINED_BY" => Ok(EdgeType::ConstrainedBy),
        "BLOCKS" => Ok(EdgeType::Blocks),
        "INFORMS" => Ok(EdgeType::Informs),
        "RELEVANT_TO" => Ok(EdgeType::RelevantTo),
        "DEPENDS_ON" => Ok(EdgeType::DependsOn),
        "AVAILABLE_ON" => Ok(EdgeType::AvailableOn),
        "COMPOSED_OF" => Ok(EdgeType::ComposedOf),
        "STEP_USES" => Ok(EdgeType::StepUses),
        "RISK_ASSESSED_BY" => Ok(EdgeType::RiskAssessedBy),
        "CONTROLS" => Ok(EdgeType::Controls),
        "CAPTURED_IN" => Ok(EdgeType::CapturedIn),
        "TRACE_ENTRY" => Ok(EdgeType::TraceEntry),
        "SUMMARIZES" => Ok(EdgeType::Summarizes),
        "RECALLS" => Ok(EdgeType::Recalls),
        "GOVERNED_BY" => Ok(EdgeType::GovernedBy),
        "ASSIGNED_TO" => Ok(EdgeType::AssignedTo),
        "PLANNED_BY" => Ok(EdgeType::PlannedBy),
        "HAS_STEP" => Ok(EdgeType::HasStep),
        "TARGETS" => Ok(EdgeType::Targets),
        "REQUIRES_APPROVAL" => Ok(EdgeType::RequiresApproval),
        "EXECUTED_BY" => Ok(EdgeType::ExecutedBy),
        "EXECUTION_OF" => Ok(EdgeType::ExecutionOf),
        "PRODUCES_NODE" => Ok(EdgeType::ProducesNode),
        "GOVERNED_BY_POLICY" => Ok(EdgeType::GovernedByPolicy),
        "BUDGET_FOR" => Ok(EdgeType::BudgetFor),
        "WORKS_FOR" => Ok(EdgeType::WorksFor),
        "AFFILIATED_WITH" => Ok(EdgeType::AffiliatedWith),
        "ABOUT" => Ok(EdgeType::About),
        "KNOWN_BY" => Ok(EdgeType::KnownBy),
        other => Ok(EdgeType::Custom(other.to_string())),
    }
}

fn parse_layer(s: &str) -> Result<Layer> {
    match s {
        "reality" => Ok(Layer::Reality),
        "epistemic" => Ok(Layer::Epistemic),
        "intent" => Ok(Layer::Intent),
        "action" => Ok(Layer::Action),
        "memory" => Ok(Layer::Memory),
        "agent" => Ok(Layer::Agent),
        _ => Err(Error::Storage(format!("Invalid layer: {}", s))),
    }
}

fn parse_privacy_level(s: &str) -> Result<PrivacyLevel> {
    match s {
        "private" | "" => Ok(PrivacyLevel::Private),
        "shared" => Ok(PrivacyLevel::Shared),
        "public" => Ok(PrivacyLevel::Public),
        _ => Err(Error::Storage(format!("Invalid privacy level: {}", s))),
    }
}

/// Convert a DataValue to a serde_json::Value for export.
fn datavalue_to_json(dv: &DataValue) -> serde_json::Value {
    match dv {
        DataValue::Null => serde_json::Value::Null,
        DataValue::Bool(b) => serde_json::Value::Bool(*b),
        DataValue::Num(n) => match n {
            cozo::Num::Int(i) => serde_json::json!(*i),
            cozo::Num::Float(f) => serde_json::json!(*f),
        },
        DataValue::Str(s) => serde_json::Value::String(s.to_string()),
        DataValue::Json(j) => j.0.clone(),
        DataValue::List(items) => {
            serde_json::Value::Array(items.iter().map(datavalue_to_json).collect())
        }
        DataValue::Bytes(b) => serde_json::json!({ "__bytes__": base64_encode(b) }),
        _ => serde_json::Value::String(format!("{:?}", dv)),
    }
}

/// Convert a serde_json::Value back to a DataValue for import.
fn json_to_datavalue(val: &serde_json::Value) -> DataValue {
    match val {
        serde_json::Value::Null => DataValue::Null,
        serde_json::Value::Bool(b) => DataValue::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                DataValue::from(i)
            } else if let Some(f) = n.as_f64() {
                DataValue::from(f)
            } else {
                DataValue::Null
            }
        }
        serde_json::Value::String(s) => str_val(s),
        serde_json::Value::Array(items) => {
            DataValue::List(items.iter().map(json_to_datavalue).collect())
        }
        serde_json::Value::Object(map) => {
            if let Some(b64) = map.get("__bytes__") {
                if let Some(s) = b64.as_str() {
                    if let Ok(bytes) = base64_decode(s) {
                        return DataValue::Bytes(bytes);
                    }
                }
            }
            DataValue::Json(cozo::JsonData(serde_json::Value::Object(map.clone())))
        }
    }
}

/// Convert NamedRows to JSON for export.
fn named_rows_to_json(named_rows: &NamedRows) -> serde_json::Value {
    let headers: Vec<serde_json::Value> = named_rows
        .headers
        .iter()
        .map(|h| serde_json::Value::String(h.clone()))
        .collect();

    let rows: Vec<serde_json::Value> = named_rows
        .rows
        .iter()
        .map(|row| serde_json::Value::Array(row.iter().map(datavalue_to_json).collect()))
        .collect();

    serde_json::json!({
        "headers": headers,
        "rows": rows,
    })
}

/// Convert JSON back to NamedRows for import.
fn json_to_named_rows(val: &serde_json::Value) -> Result<NamedRows> {
    let headers = val
        .get("headers")
        .and_then(|h| h.as_array())
        .ok_or_else(|| Error::Storage("Missing 'headers' in snapshot".into()))?
        .iter()
        .map(|v| v.as_str().unwrap_or("").to_string())
        .collect();

    let rows = val
        .get("rows")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::Storage("Missing 'rows' in snapshot".into()))?
        .iter()
        .map(|row| {
            row.as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(json_to_datavalue)
                .collect()
        })
        .collect();

    Ok(NamedRows {
        headers,
        rows,
        next: None,
    })
}

fn base64_encode(bytes: &[u8]) -> String {
    // Simple base64 encoding without external dep
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((n >> 18) & 63) as usize] as char);
        result.push(CHARS[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((n >> 6) & 63) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(n & 63) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn base64_decode(s: &str) -> std::result::Result<Vec<u8>, ()> {
    fn char_val(c: u8) -> std::result::Result<u32, ()> {
        match c {
            b'A'..=b'Z' => Ok((c - b'A') as u32),
            b'a'..=b'z' => Ok((c - b'a' + 26) as u32),
            b'0'..=b'9' => Ok((c - b'0' + 52) as u32),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err(()),
        }
    }
    let bytes = s.as_bytes();
    let mut result = Vec::new();
    for chunk in bytes.chunks(4) {
        if chunk.len() < 2 {
            break;
        }
        let a = char_val(chunk[0])?;
        let b = char_val(chunk[1])?;
        result.push(((a << 2) | (b >> 4)) as u8);
        if chunk.len() > 2 && chunk[2] != b'=' {
            let c = char_val(chunk[2])?;
            result.push((((b & 0xf) << 4) | (c >> 2)) as u8);
            if chunk.len() > 3 && chunk[3] != b'=' {
                let d = char_val(chunk[3])?;
                result.push((((c & 0x3) << 6) | d) as u8);
            }
        }
    }
    Ok(result)
}
