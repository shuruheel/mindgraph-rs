use mindgraph::*;
#[allow(unused_imports)]
use std::sync::Arc;

// ---- Helpers ----

fn mem_graph() -> MindGraph {
    MindGraph::open_in_memory().expect("should create in-memory graph")
}

fn make_source_node() -> CreateNode {
    CreateNode::new(
        "Wikipedia: Rust",
        NodeProps::Source(SourceProps {
            source_type: "web_page".into(),
            uri: "https://en.wikipedia.org/wiki/Rust_(programming_language)".into(),
            title: "Rust (programming language)".into(),
            ..Default::default()
        }),
    )
}

fn make_entity_node(name: &str) -> CreateNode {
    CreateNode::new(
        name,
        NodeProps::Entity(EntityProps {
            entity_type: "programming_language".into(),
            canonical_name: name.into(),
            description: Some(format!("{} programming language", name)),
            ..Default::default()
        }),
    )
}

fn make_claim_node(text: &str, conf: f64) -> CreateNode {
    CreateNode::new(
        text,
        NodeProps::Claim(ClaimProps {
            content: text.into(),
            claim_type: Some("factual".into()),
            ..Default::default()
        }),
    )
    .confidence(Confidence::new(conf).unwrap())
}

fn make_evidence_node(text: &str) -> CreateNode {
    CreateNode::new(
        text,
        NodeProps::Evidence(EvidenceProps {
            description: text.into(),
            evidence_type: Some("empirical".into()),
            ..Default::default()
        }),
    )
}

// ---- Node CRUD ----

#[test]
fn test_add_and_get_node() {
    let g = mem_graph();
    let node = g.add_node(make_source_node()).unwrap();

    assert_eq!(node.label, "Wikipedia: Rust");
    assert_eq!(node.node_type, NodeType::Source);
    assert_eq!(node.layer, Layer::Reality);
    assert_eq!(node.version, 1);

    let fetched = g.get_node(&node.uid).unwrap().expect("node should exist");
    assert_eq!(fetched.uid, node.uid);
    assert_eq!(fetched.label, "Wikipedia: Rust");
}

#[test]
fn test_get_node_not_found() {
    let g = mem_graph();
    let uid = Uid::new();
    let result = g.get_node(&uid).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_get_live_node_not_found() {
    let g = mem_graph();
    let uid = Uid::new();
    let err = g.get_live_node(&uid).unwrap_err();
    assert!(matches!(err, Error::NodeNotFound(_)));
}

#[test]
fn test_update_node() {
    let g = mem_graph();
    let node = g.add_node(make_entity_node("Rust")).unwrap();

    let updated = g
        .update_node(
            &node.uid,
            Some("Rust Language".into()),
            Some("A systems programming language".into()),
            None,
            None,
            None,
            "test",
            "renamed",
        )
        .unwrap();

    assert_eq!(updated.label, "Rust Language");
    assert_eq!(updated.summary, "A systems programming language");
    assert_eq!(updated.version, 2);
    assert!(updated.updated_at >= node.updated_at);
}

#[test]
fn test_update_node_type_mismatch() {
    let g = mem_graph();
    let node = g.add_node(make_entity_node("Rust")).unwrap();

    let wrong_props = NodeProps::Source(SourceProps::default());
    let err = g
        .update_node(&node.uid, None, None, None, None, Some(wrong_props), "test", "bad")
        .unwrap_err();
    assert!(matches!(err, Error::TypeMismatch { .. }));
}

// ---- Edge CRUD ----

#[test]
fn test_add_and_get_edge() {
    let g = mem_graph();
    let source = g.add_node(make_source_node()).unwrap();
    let entity = g.add_node(make_entity_node("Rust")).unwrap();

    let edge = g
        .add_edge(CreateEdge::new(
            entity.uid.clone(),
            source.uid.clone(),
            EdgeProps::ExtractedFrom {
                location: Some("paragraph 1".into()),
                method: Some("llm".into()),
                confidence: Some(0.9),
            },
        ))
        .unwrap();

    assert_eq!(edge.edge_type, EdgeType::ExtractedFrom);
    assert_eq!(edge.from_uid, entity.uid);
    assert_eq!(edge.to_uid, source.uid);

    let fetched = g.get_edge(&edge.uid).unwrap().expect("edge should exist");
    assert_eq!(fetched.uid, edge.uid);
}

#[test]
fn test_edges_from() {
    let g = mem_graph();
    let entity = g.add_node(make_entity_node("Rust")).unwrap();
    let source = g.add_node(make_source_node()).unwrap();

    g.add_edge(CreateEdge::new(
        entity.uid.clone(),
        source.uid.clone(),
        EdgeProps::ExtractedFrom {
            location: None,
            method: None,
            confidence: None,
        },
    ))
    .unwrap();

    let edges = g.edges_from(&entity.uid, None).unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].edge_type, EdgeType::ExtractedFrom);

    // Filter by type
    let edges = g
        .edges_from(&entity.uid, Some(EdgeType::Supports))
        .unwrap();
    assert_eq!(edges.len(), 0);
}

#[test]
fn test_edges_to() {
    let g = mem_graph();
    let entity = g.add_node(make_entity_node("Rust")).unwrap();
    let source = g.add_node(make_source_node()).unwrap();

    g.add_edge(CreateEdge::new(
        entity.uid.clone(),
        source.uid.clone(),
        EdgeProps::ExtractedFrom {
            location: None,
            method: None,
            confidence: None,
        },
    ))
    .unwrap();

    let edges = g.edges_to(&source.uid, None).unwrap();
    assert_eq!(edges.len(), 1);
}

// ---- Tombstone ----

#[test]
fn test_tombstone_and_restore() {
    let g = mem_graph();
    let node = g.add_node(make_entity_node("Rust")).unwrap();

    g.tombstone(&node.uid, "obsolete", "test").unwrap();

    let err = g.get_live_node(&node.uid).unwrap_err();
    assert!(matches!(err, Error::Tombstoned(_)));

    // Can still get the raw node
    let raw = g.get_node(&node.uid).unwrap().unwrap();
    assert!(raw.tombstone_at.is_some());
    assert_eq!(raw.tombstone_reason.as_deref(), Some("obsolete"));

    // Restore
    g.restore(&node.uid).unwrap();
    let restored = g.get_live_node(&node.uid).unwrap();
    assert!(restored.tombstone_at.is_none());
}

// ---- Entity Resolution ----

#[test]
fn test_alias_resolution() {
    let g = mem_graph();
    let entity = g.add_node(make_entity_node("Rust")).unwrap();

    g.add_alias("rust", &entity.uid, 1.0).unwrap();
    g.add_alias("rust-lang", &entity.uid, 0.9).unwrap();
    g.add_alias("Rust programming language", &entity.uid, 0.8)
        .unwrap();

    let resolved = g.resolve_alias("rust").unwrap();
    assert_eq!(resolved, Some(entity.uid.clone()));

    let resolved = g.resolve_alias("rust-lang").unwrap();
    assert_eq!(resolved, Some(entity.uid));

    let resolved = g.resolve_alias("nonexistent").unwrap();
    assert!(resolved.is_none());
}

// ---- Provenance ----

#[test]
fn test_add_provenance() {
    let g = mem_graph();
    let source = g.add_node(make_source_node()).unwrap();
    let claim = g.add_node(make_claim_node("Rust is memory safe", 0.95)).unwrap();

    let record = ProvenanceRecord {
        node_uid: claim.uid.clone(),
        source_uid: source.uid.clone(),
        extraction_method: ExtractionMethod::Llm,
        extraction_confidence: 0.9,
        source_location: "paragraph 3".into(),
        text_span: "Rust guarantees memory safety".into(),
        extracted_by: "gpt-4".into(),
        extracted_at: now(),
    };

    g.add_provenance(&record).unwrap();
}

// ---- Type System ----

#[test]
fn test_confidence_validation() {
    assert!(Confidence::new(0.0).is_ok());
    assert!(Confidence::new(0.5).is_ok());
    assert!(Confidence::new(1.0).is_ok());
    assert!(Confidence::new(-0.1).is_err());
    assert!(Confidence::new(1.1).is_err());
}

#[test]
fn test_salience_validation() {
    assert!(Salience::new(0.0).is_ok());
    assert!(Salience::new(0.5).is_ok());
    assert!(Salience::new(1.0).is_ok());
    assert!(Salience::new(-0.1).is_err());
    assert!(Salience::new(1.1).is_err());
}

#[test]
fn test_node_type_layer_mapping() {
    assert_eq!(NodeType::Source.layer(), Layer::Reality);
    assert_eq!(NodeType::Claim.layer(), Layer::Epistemic);
    assert_eq!(NodeType::Goal.layer(), Layer::Intent);
    assert_eq!(NodeType::Flow.layer(), Layer::Action);
    assert_eq!(NodeType::Session.layer(), Layer::Memory);
    assert_eq!(NodeType::Agent.layer(), Layer::Agent);
}

#[test]
fn test_epistemic_requires_provenance() {
    assert!(NodeType::Claim.requires_provenance());
    assert!(NodeType::Evidence.requires_provenance());
    assert!(NodeType::Hypothesis.requires_provenance());
    assert!(!NodeType::Source.requires_provenance());
    assert!(!NodeType::Goal.requires_provenance());
    assert!(!NodeType::Agent.requires_provenance());
}

// ---- Query Patterns ----

#[test]
fn test_active_goals() {
    let g = mem_graph();

    g.add_node(CreateNode::new(
        "Ship v1.0",
        NodeProps::Goal(GoalProps {
            status: Some("active".into()),
            priority: Some("high".into()),
            ..Default::default()
        }),
    ))
    .unwrap();

    g.add_node(CreateNode::new(
        "Old goal",
        NodeProps::Goal(GoalProps {
            status: Some("completed".into()),
            priority: Some("low".into()),
            ..Default::default()
        }),
    ))
    .unwrap();

    g.add_node(CreateNode::new(
        "Critical bug",
        NodeProps::Goal(GoalProps {
            status: Some("active".into()),
            priority: Some("critical".into()),
            ..Default::default()
        }),
    ))
    .unwrap();

    let goals = g.active_goals().unwrap();
    assert_eq!(goals.len(), 2);
    // Critical should come first
    assert_eq!(goals[0].label, "Critical bug");
    assert_eq!(goals[1].label, "Ship v1.0");
}

#[test]
fn test_weak_claims() {
    let g = mem_graph();

    g.add_node(make_claim_node("Strong claim", 0.95)).unwrap();
    g.add_node(make_claim_node("Weak claim", 0.3)).unwrap();
    g.add_node(make_claim_node("Very weak claim", 0.1)).unwrap();

    let weak = g.weak_claims(0.5).unwrap();
    assert_eq!(weak.len(), 2);
    // Sorted by confidence ascending
    assert_eq!(weak[0].label, "Very weak claim");
    assert_eq!(weak[1].label, "Weak claim");
}

#[test]
fn test_nodes_in_layer() {
    let g = mem_graph();

    g.add_node(make_source_node()).unwrap();
    g.add_node(make_entity_node("Rust")).unwrap();
    g.add_node(make_claim_node("Rust is fast", 0.9)).unwrap();

    let reality = g.nodes_in_layer(Layer::Reality).unwrap();
    assert_eq!(reality.len(), 2);

    let epistemic = g.nodes_in_layer(Layer::Epistemic).unwrap();
    assert_eq!(epistemic.len(), 1);
}

// ---- Node Props Serialization ----

#[test]
fn test_node_props_roundtrip() {
    let props = NodeProps::Entity(EntityProps {
        entity_type: "person".into(),
        canonical_name: "Ada Lovelace".into(),
        description: Some("First programmer".into()),
        ..Default::default()
    });

    let json = props.to_json();
    assert_eq!(json["entity_type"], "person");
    assert_eq!(json["canonical_name"], "Ada Lovelace");

    let restored = NodeProps::from_json(&NodeType::Entity, &json).unwrap();
    assert_eq!(restored.node_type(), NodeType::Entity);
}

#[test]
fn test_edge_props_roundtrip() {
    let props = EdgeProps::Supports {
        strength: Some(0.8),
        support_type: Some("empirical".into()),
    };

    let json = props.to_json();
    assert_eq!(json["strength"], 0.8);

    let restored = EdgeProps::from_json(&EdgeType::Supports, &json).unwrap();
    assert_eq!(restored.edge_type(), EdgeType::Supports);
}

#[test]
fn test_empty_edge_props_roundtrip() {
    let props = EdgeProps::InstanceOf {};
    let json = props.to_json();
    let restored = EdgeProps::from_json(&EdgeType::InstanceOf, &json).unwrap();
    assert_eq!(restored.edge_type(), EdgeType::InstanceOf);
}

// ---- Multiple Edges Between Same Nodes ----

#[test]
fn test_multiple_edge_types_between_nodes() {
    let g = mem_graph();
    let claim1 = g.add_node(make_claim_node("Claim A", 0.9)).unwrap();
    let claim2 = g.add_node(make_claim_node("Claim B", 0.8)).unwrap();

    g.add_edge(CreateEdge::new(
        claim1.uid.clone(),
        claim2.uid.clone(),
        EdgeProps::Supports {
            strength: Some(0.7),
            support_type: None,
        },
    ))
    .unwrap();

    g.add_edge(CreateEdge::new(
        claim1.uid.clone(),
        claim2.uid.clone(),
        EdgeProps::Contradicts {
            description: Some("conflicting evidence".into()),
            contradiction_type: None,
            resolution_status: Some("unresolved".into()),
            proposed_resolution: None,
        },
    ))
    .unwrap();

    let edges = g.edges_from(&claim1.uid, None).unwrap();
    assert_eq!(edges.len(), 2);
}

// ---- Uid ----

#[test]
fn test_uid_uniqueness() {
    let a = Uid::new();
    let b = Uid::new();
    assert_ne!(a, b);
}

#[test]
fn test_uid_from_str() {
    let uid = Uid::from("test-id-123");
    assert_eq!(uid.as_str(), "test-id-123");
    assert_eq!(uid.to_string(), "test-id-123");
}

#[test]
fn test_uid_parse_from_str() {
    let uid: Uid = "test-id-456".parse().unwrap();
    assert_eq!(uid.as_str(), "test-id-456");
}

// ---- Edge to Tombstoned Node ----

#[test]
fn test_add_edge_to_tombstoned_node_fails() {
    let g = mem_graph();
    let entity = g.add_node(make_entity_node("Rust")).unwrap();
    let source = g.add_node(make_source_node()).unwrap();

    g.tombstone(&entity.uid, "removed", "test").unwrap();

    let err = g
        .add_edge(CreateEdge::new(
            entity.uid.clone(),
            source.uid.clone(),
            EdgeProps::InstanceOf {},
        ))
        .unwrap_err();
    assert!(matches!(err, Error::Tombstoned(_)));
}

#[test]
fn test_add_edge_to_tombstoned_target_fails() {
    let g = mem_graph();
    let entity = g.add_node(make_entity_node("Rust")).unwrap();
    let source = g.add_node(make_source_node()).unwrap();

    g.tombstone(&source.uid, "removed", "test").unwrap();

    let err = g
        .add_edge(CreateEdge::new(
            entity.uid.clone(),
            source.uid.clone(),
            EdgeProps::InstanceOf {},
        ))
        .unwrap_err();
    assert!(matches!(err, Error::Tombstoned(_)));
}

#[test]
fn test_add_edge_to_nonexistent_target_fails() {
    let g = mem_graph();
    let entity = g.add_node(make_entity_node("Rust")).unwrap();
    let fake_uid = Uid::new();

    let err = g
        .add_edge(CreateEdge::new(
            entity.uid.clone(),
            fake_uid,
            EdgeProps::InstanceOf {},
        ))
        .unwrap_err();
    assert!(matches!(err, Error::NodeNotFound(_)));
}

// ---- All Node Types Can Be Created ----

#[test]
fn test_create_node_all_layers() {
    let g = mem_graph();

    // Reality
    g.add_node(CreateNode::new("src", NodeProps::Source(SourceProps::default()))).unwrap();
    g.add_node(CreateNode::new("snip", NodeProps::Snippet(SnippetProps::default()))).unwrap();
    g.add_node(CreateNode::new("ent", NodeProps::Entity(EntityProps::default()))).unwrap();
    g.add_node(CreateNode::new("obs", NodeProps::Observation(ObservationProps::default()))).unwrap();

    // Epistemic
    g.add_node(CreateNode::new("claim", NodeProps::Claim(ClaimProps::default()))).unwrap();
    g.add_node(CreateNode::new("evidence", NodeProps::Evidence(EvidenceProps::default()))).unwrap();
    g.add_node(CreateNode::new("concept", NodeProps::Concept(ConceptProps::default()))).unwrap();
    g.add_node(CreateNode::new("hypothesis", NodeProps::Hypothesis(HypothesisProps::default()))).unwrap();

    // Intent
    g.add_node(CreateNode::new("goal", NodeProps::Goal(GoalProps::default()))).unwrap();
    g.add_node(CreateNode::new("project", NodeProps::Project(ProjectProps::default()))).unwrap();
    g.add_node(CreateNode::new("decision", NodeProps::Decision(DecisionProps::default()))).unwrap();

    // Action
    g.add_node(CreateNode::new("flow", NodeProps::Flow(FlowProps::default()))).unwrap();
    g.add_node(CreateNode::new("affordance", NodeProps::Affordance(AffordanceProps::default()))).unwrap();

    // Memory
    g.add_node(CreateNode::new("session", NodeProps::Session(SessionProps::default()))).unwrap();
    g.add_node(CreateNode::new("pref", NodeProps::Preference(PreferenceProps::default()))).unwrap();

    // Agent
    g.add_node(CreateNode::new("agent", NodeProps::Agent(AgentProps::default()))).unwrap();
    g.add_node(CreateNode::new("task", NodeProps::Task(TaskProps::default()))).unwrap();
    g.add_node(CreateNode::new("plan", NodeProps::Plan(PlanProps::default()))).unwrap();
}

// ---- Open Questions and Decisions ----

#[test]
fn test_open_decisions() {
    let g = mem_graph();

    g.add_node(CreateNode::new(
        "Choose DB",
        NodeProps::Decision(DecisionProps {
            status: Some("open".into()),
            ..Default::default()
        }),
    ))
    .unwrap();

    g.add_node(CreateNode::new(
        "Resolved decision",
        NodeProps::Decision(DecisionProps {
            status: Some("resolved".into()),
            ..Default::default()
        }),
    ))
    .unwrap();

    let open = g.open_decisions().unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].label, "Choose DB");
}

#[test]
fn test_open_questions() {
    let g = mem_graph();

    g.add_node(CreateNode::new(
        "How to scale?",
        NodeProps::OpenQuestion(OpenQuestionProps {
            status: Some("open".into()),
            ..Default::default()
        }),
    ))
    .unwrap();

    g.add_node(CreateNode::new(
        "Answered question",
        NodeProps::OpenQuestion(OpenQuestionProps {
            status: Some("answered".into()),
            ..Default::default()
        }),
    ))
    .unwrap();

    let open = g.open_questions().unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].label, "How to scale?");
}

// ---- Edge Update ----

#[test]
fn test_update_edge() {
    let g = mem_graph();
    let entity = g.add_node(make_entity_node("Rust")).unwrap();
    let source = g.add_node(make_source_node()).unwrap();

    let edge = g
        .add_edge(CreateEdge::new(
            entity.uid.clone(),
            source.uid.clone(),
            EdgeProps::Supports {
                strength: Some(0.5),
                support_type: None,
            },
        ))
        .unwrap();

    assert_eq!(edge.version, 1);

    let updated = g
        .update_edge(
            &edge.uid,
            None,
            Some(0.9),
            Some(EdgeProps::Supports {
                strength: Some(0.8),
                support_type: Some("empirical".into()),
            }),
            "test",
            "refined strength",
        )
        .unwrap();

    assert_eq!(updated.version, 2);
    assert!((updated.weight - 0.9).abs() < f64::EPSILON);
}

#[test]
fn test_update_edge_type_mismatch() {
    let g = mem_graph();
    let entity = g.add_node(make_entity_node("Rust")).unwrap();
    let source = g.add_node(make_source_node()).unwrap();

    let edge = g
        .add_edge(CreateEdge::new(
            entity.uid.clone(),
            source.uid.clone(),
            EdgeProps::Supports {
                strength: Some(0.5),
                support_type: None,
            },
        ))
        .unwrap();

    let err = g
        .update_edge(
            &edge.uid,
            None,
            None,
            Some(EdgeProps::Refutes {
                strength: None,
                refutation_type: None,
            }),
            "test",
            "bad",
        )
        .unwrap_err();
    assert!(matches!(err, Error::TypeMismatch { .. }));
}

// ---- Edge Tombstone ----

#[test]
fn test_tombstone_and_restore_edge() {
    let g = mem_graph();
    let entity = g.add_node(make_entity_node("Rust")).unwrap();
    let source = g.add_node(make_source_node()).unwrap();

    let edge = g
        .add_edge(CreateEdge::new(
            entity.uid.clone(),
            source.uid.clone(),
            EdgeProps::InstanceOf {},
        ))
        .unwrap();

    g.tombstone_edge(&edge.uid, "duplicate", "test").unwrap();

    // Should not appear in live queries
    let edges = g.edges_from(&entity.uid, None).unwrap();
    assert_eq!(edges.len(), 0);

    // get_live_edge should fail
    let err = g.get_live_edge(&edge.uid).unwrap_err();
    assert!(matches!(err, Error::Tombstoned(_)));

    // Raw get should still work
    let raw = g.get_edge(&edge.uid).unwrap().unwrap();
    assert!(raw.tombstone_at.is_some());

    // Restore
    g.restore_edge(&edge.uid).unwrap();
    let restored = g.get_live_edge(&edge.uid).unwrap();
    assert!(restored.tombstone_at.is_none());

    // Should reappear in live queries
    let edges = g.edges_from(&entity.uid, None).unwrap();
    assert_eq!(edges.len(), 1);
}

// ---- Persistence (SQLite) ----

#[test]
fn test_sqlite_persistence() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");

    let uid;
    {
        let g = MindGraph::open(&path).unwrap();
        let node = g.add_node(make_entity_node("Rust")).unwrap();
        uid = node.uid;
    }

    // Reopen
    {
        let g = MindGraph::open(&path).unwrap();
        let node = g.get_node(&uid).unwrap().expect("should persist");
        assert_eq!(node.label, "Rust");
    }
}

// ==== Phase 1: Count / Exists Tests ====

#[test]
fn test_count_nodes_by_type() {
    let g = mem_graph();
    g.add_node(make_claim_node("Claim 1", 0.9)).unwrap();
    g.add_node(make_claim_node("Claim 2", 0.8)).unwrap();
    g.add_node(make_entity_node("Rust")).unwrap();

    assert_eq!(g.count_nodes(NodeType::Claim).unwrap(), 2);
    assert_eq!(g.count_nodes(NodeType::Entity).unwrap(), 1);
    assert_eq!(g.count_nodes(NodeType::Source).unwrap(), 0);
}

#[test]
fn test_count_nodes_in_layer() {
    let g = mem_graph();
    g.add_node(make_source_node()).unwrap();
    g.add_node(make_entity_node("Rust")).unwrap();
    g.add_node(make_claim_node("Rust is fast", 0.9)).unwrap();

    assert_eq!(g.count_nodes_in_layer(Layer::Reality).unwrap(), 2);
    assert_eq!(g.count_nodes_in_layer(Layer::Epistemic).unwrap(), 1);
    assert_eq!(g.count_nodes_in_layer(Layer::Intent).unwrap(), 0);
}

#[test]
fn test_count_edges_by_type() {
    let g = mem_graph();
    let entity = g.add_node(make_entity_node("Rust")).unwrap();
    let source = g.add_node(make_source_node()).unwrap();

    g.add_edge(CreateEdge::new(
        entity.uid.clone(),
        source.uid.clone(),
        EdgeProps::ExtractedFrom {
            location: None,
            method: None,
            confidence: None,
        },
    ))
    .unwrap();

    assert_eq!(g.count_edges(EdgeType::ExtractedFrom).unwrap(), 1);
    assert_eq!(g.count_edges(EdgeType::Supports).unwrap(), 0);
}

#[test]
fn test_node_exists() {
    let g = mem_graph();
    let node = g.add_node(make_entity_node("Rust")).unwrap();

    assert!(g.node_exists(&node.uid).unwrap());
    assert!(!g.node_exists(&Uid::new()).unwrap());

    // Tombstoned nodes should not exist
    g.tombstone(&node.uid, "gone", "test").unwrap();
    assert!(!g.node_exists(&node.uid).unwrap());
}

// ==== Phase 2: Update Builder Tests ====

#[test]
fn test_node_update_builder() {
    let g = mem_graph();
    let node = g.add_node(make_claim_node("Original claim", 0.5)).unwrap();

    let updated = g
        .update(&node.uid)
        .label("Updated claim")
        .confidence(Confidence::new(0.9).unwrap())
        .changed_by("agent-1")
        .reason("new evidence found")
        .apply()
        .unwrap();

    assert_eq!(updated.label, "Updated claim");
    assert!((updated.confidence.value() - 0.9).abs() < f64::EPSILON);
    assert_eq!(updated.version, 2);
}

#[test]
fn test_node_update_builder_partial() {
    let g = mem_graph();
    let node = g.add_node(make_entity_node("Rust")).unwrap();

    // Only update label, leave everything else
    let updated = g
        .update(&node.uid)
        .label("Rust Language")
        .apply()
        .unwrap();

    assert_eq!(updated.label, "Rust Language");
    assert_eq!(updated.version, 2);
}

#[test]
fn test_edge_update_builder() {
    let g = mem_graph();
    let claim1 = g.add_node(make_claim_node("A", 0.9)).unwrap();
    let claim2 = g.add_node(make_claim_node("B", 0.8)).unwrap();

    let edge = g
        .add_edge(CreateEdge::new(
            claim1.uid.clone(),
            claim2.uid.clone(),
            EdgeProps::Supports {
                strength: Some(0.5),
                support_type: None,
            },
        ))
        .unwrap();

    let updated = g
        .update_edge_builder(&edge.uid)
        .weight(0.95)
        .changed_by("agent-2")
        .reason("re-evaluated")
        .apply()
        .unwrap();

    assert!((updated.weight - 0.95).abs() < f64::EPSILON);
    assert_eq!(updated.version, 2);
}

// ==== Phase 3: Traversal Tests ====

#[test]
fn test_reasoning_chain_traversal() {
    let g = mem_graph();

    // Build: Evidence --(SUPPORTS)--> Claim, Evidence --(EXTRACTED_FROM)--> Source
    let claim = g.add_node(make_claim_node("Rust is memory safe", 0.9)).unwrap();
    let evidence = g.add_node(make_evidence_node("Borrow checker prevents dangling pointers")).unwrap();
    let source = g.add_node(make_source_node()).unwrap();

    g.add_edge(CreateEdge::new(
        evidence.uid.clone(),
        claim.uid.clone(),
        EdgeProps::Supports {
            strength: Some(0.8),
            support_type: Some("empirical".into()),
        },
    ))
    .unwrap();

    g.add_edge(CreateEdge::new(
        evidence.uid.clone(),
        source.uid.clone(),
        EdgeProps::ExtractedFrom {
            location: Some("section 3".into()),
            method: Some("llm".into()),
            confidence: Some(0.95),
        },
    ))
    .unwrap();

    let chain = g.reasoning_chain(&claim.uid, 5).unwrap();
    assert!(chain.len() >= 3);
    // First step should be the starting claim at depth 0
    assert_eq!(chain[0].depth, 0);
    assert_eq!(chain[0].node_uid, claim.uid);
    // Second step should be the evidence at depth 1
    assert_eq!(chain[1].depth, 1);
    // Third step should be the source at depth 2
    assert_eq!(chain[2].depth, 2);
}

#[test]
fn test_neighborhood() {
    let g = mem_graph();
    let center = g.add_node(make_entity_node("Rust")).unwrap();
    let related1 = g.add_node(make_claim_node("Fast", 0.9)).unwrap();
    let related2 = g.add_node(make_source_node()).unwrap();

    g.add_edge(CreateEdge::new(
        center.uid.clone(),
        related1.uid.clone(),
        EdgeProps::InstanceOf {},
    ))
    .unwrap();

    g.add_edge(CreateEdge::new(
        related2.uid.clone(),
        center.uid.clone(),
        EdgeProps::Contains {},
    ))
    .unwrap();

    let neighbors = g.neighborhood(&center.uid, 1).unwrap();
    assert_eq!(neighbors.len(), 2);
}

#[test]
fn test_reachable_with_edge_type_filter() {
    let g = mem_graph();
    let claim = g.add_node(make_claim_node("Main claim", 0.9)).unwrap();
    let support = g.add_node(make_evidence_node("Supporting evidence")).unwrap();
    let unrelated = g.add_node(make_entity_node("Unrelated")).unwrap();

    g.add_edge(CreateEdge::new(
        claim.uid.clone(),
        support.uid.clone(),
        EdgeProps::Supports {
            strength: Some(0.8),
            support_type: None,
        },
    ))
    .unwrap();

    g.add_edge(CreateEdge::new(
        claim.uid.clone(),
        unrelated.uid.clone(),
        EdgeProps::InstanceOf {},
    ))
    .unwrap();

    // Only follow SUPPORTS edges
    let opts = TraversalOptions {
        direction: Direction::Outgoing,
        edge_types: Some(vec![EdgeType::Supports]),
        max_depth: 5,
        weight_threshold: None,
    };

    let steps = g.reachable(&claim.uid, &opts).unwrap();
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].node_uid, support.uid);
}

#[test]
fn test_find_path() {
    let g = mem_graph();
    let a = g.add_node(make_claim_node("A", 0.9)).unwrap();
    let b = g.add_node(make_claim_node("B", 0.8)).unwrap();
    let c = g.add_node(make_claim_node("C", 0.7)).unwrap();

    g.add_edge(CreateEdge::new(
        a.uid.clone(),
        b.uid.clone(),
        EdgeProps::Supports { strength: Some(0.5), support_type: None },
    ))
    .unwrap();

    g.add_edge(CreateEdge::new(
        b.uid.clone(),
        c.uid.clone(),
        EdgeProps::Supports { strength: Some(0.5), support_type: None },
    ))
    .unwrap();

    let opts = TraversalOptions {
        direction: Direction::Outgoing,
        edge_types: None,
        max_depth: 5,
        weight_threshold: None,
    };

    let path = g.find_path(&a.uid, &c.uid, &opts).unwrap();
    assert!(path.is_some());
    let path = path.unwrap();
    assert!(path.iter().any(|s| s.node_uid == c.uid));

    // No path in reverse direction with outgoing-only
    let no_path = g.find_path(&c.uid, &a.uid, &opts).unwrap();
    assert!(no_path.is_none());
}

#[test]
fn test_find_path_ignores_branches() {
    let g = mem_graph();
    let a = g.add_node(make_claim_node("A", 0.9)).unwrap();
    let b = g.add_node(make_claim_node("B", 0.8)).unwrap();
    let c = g.add_node(make_claim_node("C", 0.7)).unwrap();
    let d = g.add_node(make_claim_node("D", 0.6)).unwrap();

    // A→B→C and A→D (branch)
    g.add_edge(CreateEdge::new(
        a.uid.clone(), b.uid.clone(),
        EdgeProps::Supports { strength: Some(0.5), support_type: None },
    )).unwrap();
    g.add_edge(CreateEdge::new(
        b.uid.clone(), c.uid.clone(),
        EdgeProps::Supports { strength: Some(0.5), support_type: None },
    )).unwrap();
    g.add_edge(CreateEdge::new(
        a.uid.clone(), d.uid.clone(),
        EdgeProps::Supports { strength: Some(0.5), support_type: None },
    )).unwrap();

    let opts = TraversalOptions {
        direction: Direction::Outgoing,
        edge_types: None,
        max_depth: 5,
        weight_threshold: None,
    };

    let path = g.find_path(&a.uid, &c.uid, &opts).unwrap().unwrap();
    // Should contain only B and C (the actual path), not D
    assert_eq!(path.len(), 2);
    assert_eq!(path[0].node_uid, b.uid);
    assert_eq!(path[1].node_uid, c.uid);
    // D should not be in the path
    assert!(!path.iter().any(|s| s.node_uid == d.uid));
}

#[test]
fn test_subgraph() {
    let g = mem_graph();
    let a = g.add_node(make_claim_node("A", 0.9)).unwrap();
    let b = g.add_node(make_claim_node("B", 0.8)).unwrap();
    let c = g.add_node(make_claim_node("C", 0.7)).unwrap();

    g.add_edge(CreateEdge::new(
        a.uid.clone(),
        b.uid.clone(),
        EdgeProps::Supports { strength: Some(0.5), support_type: None },
    ))
    .unwrap();

    g.add_edge(CreateEdge::new(
        b.uid.clone(),
        c.uid.clone(),
        EdgeProps::Supports { strength: Some(0.5), support_type: None },
    ))
    .unwrap();

    let opts = TraversalOptions {
        direction: Direction::Outgoing,
        edge_types: None,
        max_depth: 5,
        weight_threshold: None,
    };

    let (nodes, edges) = g.subgraph(&a.uid, &opts).unwrap();
    assert_eq!(nodes.len(), 3); // a, b, c
    assert_eq!(edges.len(), 2);
}

// ==== Phase 4: Pagination Tests ====

#[test]
fn test_nodes_in_layer_paginated() {
    let g = mem_graph();
    for i in 0..5 {
        g.add_node(make_entity_node(&format!("Entity {}", i))).unwrap();
    }

    let page1 = g.nodes_in_layer_paginated(Layer::Reality, Pagination { limit: 2, offset: 0 }).unwrap();
    assert_eq!(page1.items.len(), 2);
    assert!(page1.has_more);

    let page2 = g.nodes_in_layer_paginated(Layer::Reality, Pagination { limit: 2, offset: 2 }).unwrap();
    assert_eq!(page2.items.len(), 2);
    assert!(page2.has_more);

    let page3 = g.nodes_in_layer_paginated(Layer::Reality, Pagination { limit: 2, offset: 4 }).unwrap();
    assert_eq!(page3.items.len(), 1);
    assert!(!page3.has_more);
}

#[test]
fn test_pagination_first() {
    let g = mem_graph();
    for i in 0..10 {
        g.add_node(make_claim_node(&format!("Claim {}", i), 0.1 * i as f64)).unwrap();
    }

    let page = g.weak_claims_paginated(0.5, Pagination::first(3)).unwrap();
    assert_eq!(page.items.len(), 3);
    assert!(page.has_more);
    assert_eq!(page.offset, 0);
}

#[test]
fn test_edges_from_paginated() {
    let g = mem_graph();
    let entity = g.add_node(make_entity_node("Hub")).unwrap();

    for i in 0..5 {
        let target = g.add_node(make_claim_node(&format!("Target {}", i), 0.9)).unwrap();
        g.add_edge(CreateEdge::new(
            entity.uid.clone(),
            target.uid.clone(),
            EdgeProps::InstanceOf {},
        ))
        .unwrap();
    }

    let page1 = g.edges_from_paginated(&entity.uid, None, Pagination { limit: 2, offset: 0 }).unwrap();
    assert_eq!(page1.items.len(), 2);
    assert!(page1.has_more);

    let page2 = g.edges_from_paginated(&entity.uid, None, Pagination { limit: 10, offset: 0 }).unwrap();
    assert_eq!(page2.items.len(), 5);
    assert!(!page2.has_more);
}

#[test]
fn test_edges_to_paginated() {
    let g = mem_graph();
    let target = g.add_node(make_claim_node("Target", 0.9)).unwrap();

    for i in 0..5 {
        let source = g.add_node(make_claim_node(&format!("Source {}", i), 0.8)).unwrap();
        g.add_edge(CreateEdge::new(
            source.uid.clone(),
            target.uid.clone(),
            EdgeProps::Supports { strength: Some(0.5), support_type: None },
        ))
        .unwrap();
    }

    let page1 = g.edges_to_paginated(&target.uid, None, Pagination { limit: 2, offset: 0 }).unwrap();
    assert_eq!(page1.items.len(), 2);
    assert!(page1.has_more);

    let page2 = g.edges_to_paginated(&target.uid, None, Pagination { limit: 10, offset: 0 }).unwrap();
    assert_eq!(page2.items.len(), 5);
    assert!(!page2.has_more);
}

// ==== Phase 5: Batch Operations Tests ====

#[test]
fn test_batch_add_nodes() {
    let g = mem_graph();

    let creates: Vec<CreateNode> = (0..50)
        .map(|i| make_entity_node(&format!("Entity {}", i)))
        .collect();

    let nodes = g.add_nodes_batch(creates).unwrap();
    assert_eq!(nodes.len(), 50);

    // Verify all exist
    for node in &nodes {
        assert!(g.node_exists(&node.uid).unwrap());
    }

    assert_eq!(g.count_nodes(NodeType::Entity).unwrap(), 50);
}

#[test]
fn test_batch_add_edges() {
    let g = mem_graph();

    // Create nodes first
    let creates: Vec<CreateNode> = (0..10)
        .map(|i| make_claim_node(&format!("Claim {}", i), 0.5 + i as f64 * 0.05))
        .collect();
    let nodes = g.add_nodes_batch(creates).unwrap();

    // Create edges in a chain: 0->1->2->...->9
    let edge_creates: Vec<CreateEdge> = nodes
        .windows(2)
        .map(|pair| {
            CreateEdge::new(
                pair[0].uid.clone(),
                pair[1].uid.clone(),
                EdgeProps::Supports {
                    strength: Some(0.7),
                    support_type: None,
                },
            )
        })
        .collect();

    let edges = g.add_edges_batch(edge_creates).unwrap();
    assert_eq!(edges.len(), 9);
}

// ==== Phase 6: Tombstone Cascade + Version History Tests ====

#[test]
fn test_tombstone_cascade() {
    let g = mem_graph();
    let center = g.add_node(make_entity_node("Center")).unwrap();
    let left = g.add_node(make_claim_node("Left", 0.9)).unwrap();
    let right = g.add_node(make_claim_node("Right", 0.8)).unwrap();

    g.add_edge(CreateEdge::new(
        center.uid.clone(),
        left.uid.clone(),
        EdgeProps::InstanceOf {},
    ))
    .unwrap();

    g.add_edge(CreateEdge::new(
        right.uid.clone(),
        center.uid.clone(),
        EdgeProps::Supports {
            strength: Some(0.5),
            support_type: None,
        },
    ))
    .unwrap();

    let result = g.tombstone_cascade(&center.uid, "cleanup", "test").unwrap();
    assert_eq!(result.edges_tombstoned, 2);

    // Center should be tombstoned
    assert!(!g.node_exists(&center.uid).unwrap());

    // Edges should be tombstoned
    let edges_from_center = g.edges_from(&center.uid, None).unwrap();
    assert_eq!(edges_from_center.len(), 0);

    // Left and right should still be alive
    assert!(g.node_exists(&left.uid).unwrap());
    assert!(g.node_exists(&right.uid).unwrap());
}

#[test]
fn test_node_version_history() {
    let g = mem_graph();
    let node = g.add_node(make_entity_node("Rust")).unwrap();

    g.update(&node.uid)
        .label("Rust Language")
        .changed_by("user-1")
        .reason("renamed")
        .apply()
        .unwrap();

    g.update(&node.uid)
        .summary("A systems programming language")
        .changed_by("user-2")
        .reason("added summary")
        .apply()
        .unwrap();

    let history = g.node_history(&node.uid).unwrap();
    assert_eq!(history.len(), 3); // create + 2 updates
    assert_eq!(history[0].version, 1);
    assert_eq!(history[0].change_type, "create");
    assert_eq!(history[1].version, 2);
    assert_eq!(history[1].change_type, "update");
    assert_eq!(history[1].changed_by, "user-1");
    assert_eq!(history[2].version, 3);
    assert_eq!(history[2].change_type, "update");
    assert_eq!(history[2].changed_by, "user-2");
}

#[test]
fn test_node_at_version() {
    let g = mem_graph();
    let node = g.add_node(make_entity_node("Rust")).unwrap();

    g.update(&node.uid)
        .label("Rust Language")
        .changed_by("user")
        .apply()
        .unwrap();

    // Get version 1 snapshot
    let v1 = g.node_at_version(&node.uid, 1).unwrap().unwrap();
    assert_eq!(v1["label"], "Rust");

    // Get version 2 snapshot
    let v2 = g.node_at_version(&node.uid, 2).unwrap().unwrap();
    assert_eq!(v2["label"], "Rust Language");

    // Non-existent version
    let v99 = g.node_at_version(&node.uid, 99).unwrap();
    assert!(v99.is_none());
}

#[test]
fn test_edge_version_history() {
    let g = mem_graph();
    let a = g.add_node(make_claim_node("A", 0.9)).unwrap();
    let b = g.add_node(make_claim_node("B", 0.8)).unwrap();

    let edge = g
        .add_edge(CreateEdge::new(
            a.uid.clone(),
            b.uid.clone(),
            EdgeProps::Supports {
                strength: Some(0.5),
                support_type: None,
            },
        ))
        .unwrap();

    g.update_edge_builder(&edge.uid)
        .weight(0.9)
        .changed_by("agent")
        .reason("re-evaluated")
        .apply()
        .unwrap();

    let history = g.edge_history(&edge.uid).unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].change_type, "create");
    assert_eq!(history[1].change_type, "update");
}

// ==== Phase 7: Thread Safety Tests ====

#[test]
fn test_send_sync_across_threads() {
    let g = Arc::new(MindGraph::open_in_memory().unwrap());

    let g1 = g.clone();
    let handle1 = std::thread::spawn(move || {
        g1.add_node(make_entity_node("Thread1")).unwrap()
    });

    let g2 = g.clone();
    let handle2 = std::thread::spawn(move || {
        g2.add_node(make_entity_node("Thread2")).unwrap()
    });

    let node1 = handle1.join().unwrap();
    let node2 = handle2.join().unwrap();

    // Both nodes should be readable from the main thread
    assert!(g.node_exists(&node1.uid).unwrap());
    assert!(g.node_exists(&node2.uid).unwrap());
    assert_eq!(g.count_nodes(NodeType::Entity).unwrap(), 2);
}

// ==== Phase 8: Full-Text Search Tests ====

#[test]
fn test_fts_search_basic() {
    let g = mem_graph();
    g.add_node(make_claim_node("Rust is memory safe", 0.9)).unwrap();
    g.add_node(make_claim_node("Python is interpreted", 0.8)).unwrap();

    let opts = SearchOptions::new();
    let results = g.search("memory safe", &opts).unwrap();
    assert!(!results.is_empty());
    assert!(results[0].node.label.contains("memory safe"));
}

#[test]
fn test_fts_search_with_type_filter() {
    let g = mem_graph();
    g.add_node(make_claim_node("Rust is memory safe", 0.9)).unwrap();
    g.add_node(make_entity_node("Rust")).unwrap();

    let mut opts = SearchOptions::new();
    opts.node_type = Some(NodeType::Claim);
    let results = g.search("rust", &opts).unwrap();
    for r in &results {
        assert_eq!(r.node.node_type, NodeType::Claim);
    }
}

#[test]
fn test_fts_search_no_results() {
    let g = mem_graph();
    g.add_node(make_claim_node("Rust is memory safe", 0.9)).unwrap();

    let opts = SearchOptions::new();
    let results = g.search("quantum physics entanglement", &opts).unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_fts_search_excludes_tombstoned() {
    let g = mem_graph();
    let node = g.add_node(make_claim_node("Rust is memory safe", 0.9)).unwrap();
    g.tombstone(&node.uid, "obsolete", "test").unwrap();

    let opts = SearchOptions::new();
    let results = g.search("memory safe", &opts).unwrap();
    assert!(results.is_empty());
}

// ==== Phase 9: Structured Filter Tests ====

#[test]
fn test_find_nodes_by_type() {
    let g = mem_graph();
    g.add_node(make_claim_node("Claim A", 0.9)).unwrap();
    g.add_node(make_claim_node("Claim B", 0.8)).unwrap();
    g.add_node(make_entity_node("Entity A")).unwrap();

    let filter = NodeFilter::new().node_type(NodeType::Claim);
    let nodes = g.find_nodes(&filter).unwrap();
    assert_eq!(nodes.len(), 2);
    for n in &nodes {
        assert_eq!(n.node_type, NodeType::Claim);
    }
}

#[test]
fn test_find_nodes_label_contains() {
    let g = mem_graph();
    g.add_node(make_claim_node("Rust is fast", 0.9)).unwrap();
    g.add_node(make_claim_node("Python is slow", 0.8)).unwrap();
    g.add_node(make_claim_node("Rust is safe", 0.7)).unwrap();

    let filter = NodeFilter::new().label_contains("Rust");
    let nodes = g.find_nodes(&filter).unwrap();
    assert_eq!(nodes.len(), 2);
    for n in &nodes {
        assert!(n.label.contains("Rust"));
    }
}

#[test]
fn test_find_nodes_confidence_range() {
    let g = mem_graph();
    g.add_node(make_claim_node("Low conf", 0.2)).unwrap();
    g.add_node(make_claim_node("Mid conf", 0.5)).unwrap();
    g.add_node(make_claim_node("High conf", 0.9)).unwrap();

    let filter = NodeFilter::new().confidence_range(0.3, 0.7);
    let nodes = g.find_nodes(&filter).unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].label, "Mid conf");
}

#[test]
fn test_find_nodes_combined_filters() {
    let g = mem_graph();
    g.add_node(CreateNode::new(
        "Active Goal",
        NodeProps::Goal(GoalProps {
            status: Some("active".into()),
            ..Default::default()
        }),
    )).unwrap();
    g.add_node(CreateNode::new(
        "Completed Goal",
        NodeProps::Goal(GoalProps {
            status: Some("completed".into()),
            ..Default::default()
        }),
    )).unwrap();
    g.add_node(make_claim_node("Not a goal", 0.9)).unwrap();

    let filter = NodeFilter::new()
        .node_type(NodeType::Goal)
        .prop_equals("status", "active");
    let nodes = g.find_nodes(&filter).unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].label, "Active Goal");
}

// ==== Phase 10: Data Lifecycle Tests ====

#[test]
fn test_purge_tombstoned_all() {
    let g = mem_graph();
    let node = g.add_node(make_entity_node("To purge")).unwrap();
    g.tombstone(&node.uid, "cleanup", "test").unwrap();

    let result = g.purge_tombstoned(None).unwrap();
    assert_eq!(result.nodes_purged, 1);

    // Should be completely gone
    let fetched = g.get_node(&node.uid).unwrap();
    assert!(fetched.is_none());
}

#[test]
fn test_purge_respects_cutoff() {
    let g = mem_graph();
    let node = g.add_node(make_entity_node("To purge")).unwrap();
    g.tombstone(&node.uid, "cleanup", "test").unwrap();

    // Cutoff far in the past — should purge nothing
    let result = g.purge_tombstoned(Some(1.0)).unwrap();
    assert_eq!(result.nodes_purged, 0);

    // Node should still exist
    let fetched = g.get_node(&node.uid).unwrap();
    assert!(fetched.is_some());
}

#[test]
fn test_purge_cleans_aliases_and_provenance() {
    let g = mem_graph();
    let source = g.add_node(make_source_node()).unwrap();
    let node = g.add_node(make_claim_node("To purge", 0.9)).unwrap();

    g.add_alias("purge-alias", &node.uid, 1.0).unwrap();
    g.add_provenance(&ProvenanceRecord {
        node_uid: node.uid.clone(),
        source_uid: source.uid.clone(),
        extraction_method: ExtractionMethod::Llm,
        extraction_confidence: 0.9,
        source_location: "p1".into(),
        text_span: "span".into(),
        extracted_by: "test".into(),
        extracted_at: now(),
    }).unwrap();

    g.tombstone(&node.uid, "cleanup", "test").unwrap();
    let result = g.purge_tombstoned(None).unwrap();
    assert_eq!(result.nodes_purged, 1);

    // Alias should be gone
    let resolved = g.resolve_alias("purge-alias").unwrap();
    assert!(resolved.is_none());
}

#[test]
fn test_export_import_roundtrip() {
    let g1 = mem_graph();
    g1.add_node(make_entity_node("Rust")).unwrap();
    g1.add_node(make_claim_node("Rust is fast", 0.9)).unwrap();

    let snapshot = g1.export().unwrap();
    assert!(!snapshot.relations.is_empty());
    assert!(!snapshot.mindgraph_version.is_empty());

    // Import into a fresh graph
    let g2 = mem_graph();
    let import_result = g2.import(&snapshot).unwrap();
    assert!(import_result.relations_imported > 0);

    // Verify data
    assert_eq!(g2.count_nodes(NodeType::Entity).unwrap(), 1);
    assert_eq!(g2.count_nodes(NodeType::Claim).unwrap(), 1);
}

#[test]
fn test_snapshot_serialization() {
    let g = mem_graph();
    g.add_node(make_entity_node("Rust")).unwrap();

    let snapshot = g.export().unwrap();
    let json_str = serde_json::to_string(&snapshot).unwrap();
    let restored: GraphSnapshot = serde_json::from_str(&json_str).unwrap();
    assert_eq!(restored.relations.len(), snapshot.relations.len());
}

// ==== Phase 11: Type Safety & Ergonomics Tests ====

#[test]
fn test_into_shared() {
    let g = MindGraph::open_in_memory().unwrap().into_shared();

    let g1 = g.clone();
    let handle = std::thread::spawn(move || {
        g1.add_node(make_entity_node("Thread")).unwrap()
    });

    let node = handle.join().unwrap();
    assert!(g.node_exists(&node.uid).unwrap());
}

#[test]
fn test_default_agent() {
    let g = mem_graph();
    assert_eq!(g.default_agent(), "system");

    g.set_default_agent("agent-x");
    assert_eq!(g.default_agent(), "agent-x");

    // Update via builder without setting changed_by — should use default_agent
    let node = g.add_node(make_entity_node("Test")).unwrap();
    g.update(&node.uid)
        .label("Updated")
        .reason("testing default agent")
        .apply()
        .unwrap();

    let history = g.node_history(&node.uid).unwrap();
    assert_eq!(history[1].changed_by, "agent-x");
}

#[test]
fn test_traversal_weight_threshold() {
    let g = mem_graph();
    let a = g.add_node(make_claim_node("A", 0.9)).unwrap();
    let b = g.add_node(make_claim_node("B", 0.8)).unwrap();
    let c = g.add_node(make_claim_node("C", 0.7)).unwrap();

    // High weight edge A->B
    g.add_edge(
        CreateEdge::new(
            a.uid.clone(), b.uid.clone(),
            EdgeProps::Supports { strength: Some(0.9), support_type: None },
        ).weight(0.9),
    ).unwrap();

    // Low weight edge A->C
    g.add_edge(
        CreateEdge::new(
            a.uid.clone(), c.uid.clone(),
            EdgeProps::Supports { strength: Some(0.1), support_type: None },
        ).weight(0.1),
    ).unwrap();

    // With high threshold, only B should be reachable
    let opts = TraversalOptions {
        direction: Direction::Outgoing,
        edge_types: None,
        max_depth: 5,
        weight_threshold: Some(0.5),
    };
    let steps = g.reachable(&a.uid, &opts).unwrap();
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].node_uid, b.uid);

    // With no threshold, both should be reachable
    let opts_all = TraversalOptions {
        direction: Direction::Outgoing,
        edge_types: None,
        max_depth: 5,
        weight_threshold: None,
    };
    let steps_all = g.reachable(&a.uid, &opts_all).unwrap();
    assert_eq!(steps_all.len(), 2);
}

#[test]
fn test_pathstep_types() {
    let g = mem_graph();
    let claim = g.add_node(make_claim_node("Test claim", 0.9)).unwrap();
    let evidence = g.add_node(make_evidence_node("Test evidence")).unwrap();

    g.add_edge(CreateEdge::new(
        evidence.uid.clone(),
        claim.uid.clone(),
        EdgeProps::Supports { strength: Some(0.8), support_type: None },
    )).unwrap();

    let chain = g.reasoning_chain(&claim.uid, 3).unwrap();
    // Start node has NodeType enum and edge_type is None
    assert_eq!(chain[0].node_type, NodeType::Claim);
    assert!(chain[0].edge_type.is_none());
    // Second step has enum types
    assert_eq!(chain[1].node_type, NodeType::Evidence);
    assert_eq!(chain[1].edge_type, Some(EdgeType::Supports));
}

// ==== Phase 12: Entity Resolution & Batch Tests ====

#[test]
fn test_aliases_for() {
    let g = mem_graph();
    let entity = g.add_node(make_entity_node("Rust")).unwrap();

    g.add_alias("rust", &entity.uid, 1.0).unwrap();
    g.add_alias("rust-lang", &entity.uid, 0.9).unwrap();

    let aliases = g.aliases_for(&entity.uid).unwrap();
    assert_eq!(aliases.len(), 2);
    // Should be sorted by score descending
    assert_eq!(aliases[0].0, "rust");
    assert!((aliases[0].1 - 1.0).abs() < f64::EPSILON);
    assert_eq!(aliases[1].0, "rust-lang");
}

#[test]
fn test_merge_entities() {
    let g = mem_graph();
    let keep = g.add_node(make_entity_node("Rust")).unwrap();
    let merge = g.add_node(make_entity_node("rust-lang")).unwrap();
    let other = g.add_node(make_claim_node("Claim about rust-lang", 0.9)).unwrap();

    // Edge from merge node
    g.add_edge(CreateEdge::new(
        merge.uid.clone(),
        other.uid.clone(),
        EdgeProps::InstanceOf {},
    )).unwrap();

    // Alias on merge node
    g.add_alias("rust-lang", &merge.uid, 0.9).unwrap();

    let result = g.merge_entities(&keep.uid, &merge.uid, "duplicate", "test").unwrap();
    assert_eq!(result.edges_retargeted, 1);
    assert_eq!(result.aliases_merged, 1);

    // Merge node should be tombstoned
    assert!(!g.node_exists(&merge.uid).unwrap());

    // Edge should now point from keep node
    let edges = g.edges_from(&keep.uid, None).unwrap();
    assert!(!edges.is_empty());

    // Merged node label should be alias of keep
    let aliases = g.aliases_for(&keep.uid).unwrap();
    assert!(aliases.iter().any(|(text, _)| text == "rust-lang"));
}

#[test]
fn test_fuzzy_resolve() {
    let g = mem_graph();
    let entity = g.add_node(make_entity_node("Rust")).unwrap();
    g.add_alias("Rust programming language", &entity.uid, 0.9).unwrap();

    let results = g.fuzzy_resolve("programming", 10).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].0, entity.uid);
}

#[test]
fn test_batch_apply() {
    let g = mem_graph();

    let ops = vec![
        GraphOp::AddNode(Box::new(make_entity_node("Batch Entity 1"))),
        GraphOp::AddNode(Box::new(make_entity_node("Batch Entity 2"))),
    ];

    let result = g.batch_apply(ops).unwrap();
    assert_eq!(result.nodes_added, 2);
    assert_eq!(g.count_nodes(NodeType::Entity).unwrap(), 2);
}

#[test]
fn test_batch_apply_with_tombstone() {
    let g = mem_graph();
    let node = g.add_node(make_entity_node("To tombstone")).unwrap();

    let ops = vec![
        GraphOp::AddNode(Box::new(make_entity_node("New node"))),
        GraphOp::Tombstone {
            uid: node.uid.clone(),
            reason: "batch cleanup".into(),
            by: "test".into(),
        },
    ];

    let result = g.batch_apply(ops).unwrap();
    assert_eq!(result.nodes_added, 1);
    assert_eq!(result.nodes_tombstoned, 1);
    assert!(!g.node_exists(&node.uid).unwrap());
}

// ==== v0.4 Phase 1: Stats + Convenience ====

#[test]
fn test_graph_stats_empty() {
    let g = mem_graph();
    let stats = g.stats().unwrap();
    assert_eq!(stats.total_nodes, 0);
    assert_eq!(stats.total_edges, 0);
    assert_eq!(stats.live_nodes, 0);
    assert_eq!(stats.live_edges, 0);
    assert!(stats.nodes_by_type.is_empty());
    assert!(stats.nodes_by_layer.is_empty());
    assert!(stats.edges_by_type.is_empty());
    assert_eq!(stats.tombstoned_nodes, 0);
    assert_eq!(stats.tombstoned_edges, 0);
    assert_eq!(stats.total_aliases, 0);
    assert_eq!(stats.embedding_count, 0);
    assert!(stats.embedding_dimension.is_none());
}

#[test]
fn test_graph_stats_populated() {
    let g = mem_graph();
    let c1 = g.add_claim("Claim 1", "content 1", 0.9).unwrap();
    let c2 = g.add_claim("Claim 2", "content 2", 0.8).unwrap();
    let e1 = g.add_entity("Entity 1", "person").unwrap();
    g.add_link(&c1.uid, &c2.uid, EdgeType::Supports).unwrap();

    let stats = g.stats().unwrap();
    assert_eq!(stats.live_nodes, 3);
    assert_eq!(stats.live_edges, 1);
    assert_eq!(stats.nodes_by_type.get("Claim"), Some(&2));
    assert_eq!(stats.nodes_by_type.get("Entity"), Some(&1));
    assert_eq!(stats.nodes_by_layer.get("epistemic"), Some(&2));
    assert_eq!(stats.nodes_by_layer.get("reality"), Some(&1));
    assert_eq!(stats.edges_by_type.get("SUPPORTS"), Some(&1));

    // Tombstone one and verify
    g.tombstone(&e1.uid, "test", "test").unwrap();
    let stats2 = g.stats().unwrap();
    assert_eq!(stats2.tombstoned_nodes, 1);
    assert_eq!(stats2.live_nodes, 2);
}

#[test]
fn test_convenience_constructors() {
    let g = mem_graph();

    let claim = g.add_claim("Test claim", "claim content", 0.85).unwrap();
    assert_eq!(claim.node_type, NodeType::Claim);
    assert!((claim.confidence.value() - 0.85).abs() < 0.001);

    let entity = g.add_entity("Test entity", "person").unwrap();
    assert_eq!(entity.node_type, NodeType::Entity);

    let goal = g.add_goal("Test goal", "high").unwrap();
    assert_eq!(goal.node_type, NodeType::Goal);

    let obs = g.add_observation("Test obs", "observed something").unwrap();
    assert_eq!(obs.node_type, NodeType::Observation);

    let mem = g.add_memory("Test session", "some context").unwrap();
    assert_eq!(mem.node_type, NodeType::Session);
}

#[test]
fn test_add_link() {
    let g = mem_graph();
    let c1 = g.add_claim("A", "a", 0.9).unwrap();
    let c2 = g.add_claim("B", "b", 0.8).unwrap();
    let edge = g.add_link(&c1.uid, &c2.uid, EdgeType::Supports).unwrap();
    assert_eq!(edge.edge_type, EdgeType::Supports);
}

// ==== v0.4 Phase 2: Embeddings ====

fn small_vec(seed: f32) -> Vec<f32> {
    vec![seed, seed * 0.5, seed * 0.25, seed * 0.125]
}

#[test]
fn test_configure_embeddings() {
    let g = mem_graph();
    assert!(g.embedding_dimension().is_none());
    g.configure_embeddings(4).unwrap();
    assert_eq!(g.embedding_dimension(), Some(4));

    // Idempotent with same dimension
    g.configure_embeddings(4).unwrap();

    // Error on different dimension
    let err = g.configure_embeddings(8);
    assert!(err.is_err());
}

#[test]
fn test_set_and_get_embedding() {
    let g = mem_graph();
    g.configure_embeddings(4).unwrap();
    let node = g.add_claim("Test", "content", 0.9).unwrap();

    let vec = small_vec(1.0);
    g.set_embedding(&node.uid, &vec).unwrap();

    let retrieved = g.get_embedding(&node.uid).unwrap().unwrap();
    assert_eq!(retrieved.len(), 4);
    for (a, b) in retrieved.iter().zip(vec.iter()) {
        assert!((a - b).abs() < 1e-5);
    }
}

#[test]
fn test_embedding_not_configured_error() {
    let g = mem_graph();
    let node = g.add_claim("Test", "content", 0.9).unwrap();
    let err = g.set_embedding(&node.uid, &[1.0, 2.0]);
    assert!(err.is_err());
    assert!(err.unwrap_err().to_string().contains("not configured"));
}

#[test]
fn test_embedding_dimension_mismatch() {
    let g = mem_graph();
    g.configure_embeddings(4).unwrap();
    let node = g.add_claim("Test", "content", 0.9).unwrap();
    let err = g.set_embedding(&node.uid, &[1.0, 2.0, 3.0]);
    assert!(err.is_err());
    assert!(err.unwrap_err().to_string().contains("dimension mismatch"));
}

#[test]
fn test_semantic_search_basic() {
    let g = mem_graph();
    g.configure_embeddings(4).unwrap();

    let n1 = g.add_claim("Apple", "fruit", 0.9).unwrap();
    let n2 = g.add_claim("Banana", "fruit", 0.9).unwrap();
    let n3 = g.add_claim("Cherry", "fruit", 0.9).unwrap();
    let n4 = g.add_claim("Dog", "animal", 0.9).unwrap();
    let n5 = g.add_claim("Eagle", "bird", 0.9).unwrap();

    g.set_embedding(&n1.uid, &small_vec(1.0)).unwrap();
    g.set_embedding(&n2.uid, &small_vec(1.1)).unwrap();
    g.set_embedding(&n3.uid, &small_vec(1.2)).unwrap();
    g.set_embedding(&n4.uid, &small_vec(5.0)).unwrap();
    g.set_embedding(&n5.uid, &small_vec(5.1)).unwrap();

    let results = g.semantic_search(&small_vec(1.05), 3).unwrap();
    assert!(!results.is_empty());
    assert!(results.len() <= 3);
}

#[test]
fn test_semantic_search_excludes_tombstoned() {
    let g = mem_graph();
    g.configure_embeddings(4).unwrap();

    let n1 = g.add_claim("Live", "content", 0.9).unwrap();
    let n2 = g.add_claim("Dead", "content", 0.9).unwrap();

    g.set_embedding(&n1.uid, &small_vec(1.0)).unwrap();
    g.set_embedding(&n2.uid, &small_vec(1.1)).unwrap();

    g.tombstone(&n2.uid, "test", "test").unwrap();

    let results = g.semantic_search(&small_vec(1.0), 10).unwrap();
    assert!(results.iter().all(|(n, _)| n.uid != n2.uid));
}

#[test]
fn test_delete_embedding() {
    let g = mem_graph();
    g.configure_embeddings(4).unwrap();
    let node = g.add_claim("Test", "content", 0.9).unwrap();

    g.set_embedding(&node.uid, &small_vec(1.0)).unwrap();
    assert!(g.get_embedding(&node.uid).unwrap().is_some());

    g.delete_embedding(&node.uid).unwrap();
    assert!(g.get_embedding(&node.uid).unwrap().is_none());
}

#[test]
fn test_embedding_dimension_persisted() {
    let g = mem_graph();
    g.configure_embeddings(4).unwrap();
    // Verify the dimension is stored in _meta by checking it's readable
    let dim = g.storage().get_embedding_dimension().unwrap();
    assert_eq!(dim, Some(4));
}

// ==== v0.4 Phase 3: Decay ====

#[test]
fn test_decay_salience() {
    let g = mem_graph();
    let n1 = g.add_claim("Test 1", "content", 0.9).unwrap();
    let n2 = g.add_claim("Test 2", "content", 0.9).unwrap();

    // Force a small time gap by sleeping
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Apply decay with a very short half-life to see an effect
    let result = g.decay_salience(0.01).unwrap();
    assert!(result.nodes_decayed > 0);

    // Verify salience decreased
    let updated = g.get_live_node(&n1.uid).unwrap();
    assert!(updated.salience.value() < 0.5); // default salience is 0.5
    let updated2 = g.get_live_node(&n2.uid).unwrap();
    assert!(updated2.salience.value() < 0.5);
}

#[test]
fn test_auto_tombstone() {
    let g = mem_graph();
    // Create nodes and set very low salience
    let n = g.add_node(
        make_claim_node("Low salience", 0.5)
            .salience(Salience::new(0.01).unwrap()),
    ).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    // Auto-tombstone: require salience < 0.05 and min_age of 0 seconds
    let count = g.auto_tombstone(0.05, 0.0).unwrap();
    assert_eq!(count, 1);
    assert!(g.get_node(&n.uid).unwrap().unwrap().tombstone_at.is_some());
}

#[test]
fn test_decay_does_not_affect_tombstoned() {
    let g = mem_graph();
    let n = g.add_claim("Test", "content", 0.9).unwrap();
    let original_salience = n.salience.value();

    g.tombstone(&n.uid, "test", "test").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));

    g.decay_salience(0.001).unwrap();

    // Tombstoned node should not have been changed by decay (decay only affects live nodes)
    let node = g.get_node(&n.uid).unwrap().unwrap();
    // The node's salience should remain roughly the same (tombstoned nodes are skipped)
    assert!((node.salience.value() - original_salience).abs() < 0.01);
}

// ==== v0.4 Phase 4: Query Composition ====

#[test]
fn test_filter_created_after_before() {
    let g = mem_graph();
    let before_time = mindgraph::now();
    std::thread::sleep(std::time::Duration::from_millis(10));

    let _n1 = g.add_claim("After", "content", 0.9).unwrap();
    let after_time = mindgraph::now();

    let filter = NodeFilter::new()
        .created_after(before_time)
        .created_before(after_time + 1.0);
    let results = g.find_nodes(&filter).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].label, "After");

    // Filter with time before creation should return nothing
    let filter2 = NodeFilter::new().created_before(before_time);
    let results2 = g.find_nodes(&filter2).unwrap();
    assert!(results2.is_empty());
}

#[test]
fn test_filter_prop_conditions() {
    let g = mem_graph();
    g.add_claim("Alpha", "alpha content", 0.9).unwrap();
    g.add_claim("Beta", "beta content", 0.8).unwrap();

    let filter = NodeFilter::new()
        .prop_condition("content", PropOp::Equals("alpha content".into()));
    let results = g.find_nodes(&filter).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].label, "Alpha");
}

#[test]
fn test_filter_connected_to() {
    let g = mem_graph();
    let c1 = g.add_claim("Hub", "hub", 0.9).unwrap();
    let c2 = g.add_claim("Connected", "conn", 0.9).unwrap();
    let _c3 = g.add_claim("Isolated", "iso", 0.9).unwrap();
    g.add_link(&c1.uid, &c2.uid, EdgeType::Supports).unwrap();

    let filter = NodeFilter::new()
        .node_type(NodeType::Claim)
        .connected_to(c1.uid.clone());
    let results = g.find_nodes(&filter).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].uid, c2.uid);
}

#[test]
fn test_filter_or_composition() {
    let g = mem_graph();
    g.add_claim("Claim A", "content a", 0.9).unwrap();
    g.add_entity("Entity B", "person").unwrap();
    g.add_goal("Goal C", "high").unwrap();

    let filter = NodeFilter::new()
        .node_type(NodeType::Claim)
        .or(vec![
            NodeFilter::new().node_type(NodeType::Entity),
        ]);
    let results = g.find_nodes(&filter).unwrap();
    assert_eq!(results.len(), 2); // Claim + Entity, not Goal
}

// ==== v0.4 Phase 5: Events ====

#[test]
fn test_on_change_node_added() {
    let g = mem_graph();
    let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let events_clone = events.clone();

    g.on_change(move |event| {
        events_clone.lock().unwrap().push(format!("{:?}", std::mem::discriminant(event)));
    });

    g.add_claim("Test", "content", 0.9).unwrap();

    let captured = events.lock().unwrap();
    assert_eq!(captured.len(), 1);
}

#[test]
fn test_on_change_multiple_events() {
    let g = mem_graph();
    let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count_clone = count.clone();

    g.on_change(move |_event| {
        count_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    });

    let n = g.add_claim("Test", "content", 0.9).unwrap(); // +1 NodeAdded
    g.update(&n.uid).label("Updated").apply().unwrap(); // +1 NodeUpdated
    g.tombstone(&n.uid, "done", "test").unwrap(); // +1 NodeTombstoned

    assert_eq!(count.load(std::sync::atomic::Ordering::Relaxed), 3);
}

#[test]
fn test_unsubscribe() {
    let g = mem_graph();
    let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count_clone = count.clone();

    let sub_id = g.on_change(move |_event| {
        count_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    });

    g.add_claim("Test 1", "content", 0.9).unwrap();
    assert_eq!(count.load(std::sync::atomic::Ordering::Relaxed), 1);

    g.unsubscribe(sub_id);
    g.add_claim("Test 2", "content", 0.9).unwrap();
    assert_eq!(count.load(std::sync::atomic::Ordering::Relaxed), 1); // no change
}

// ==== v0.4 Phase 6: Typed Export + Validated Batch ====

#[test]
fn test_export_typed() {
    let g = mem_graph();
    let c1 = g.add_claim("A", "a", 0.9).unwrap();
    let c2 = g.add_claim("B", "b", 0.9).unwrap();
    g.add_link(&c1.uid, &c2.uid, EdgeType::Supports).unwrap();

    let snapshot = g.export_typed().unwrap();
    assert_eq!(snapshot.nodes.len(), 2);
    assert_eq!(snapshot.edges.len(), 1);
    assert!(!snapshot.mindgraph_version.is_empty());
}

#[test]
fn test_import_typed() {
    let g1 = mem_graph();
    let c1 = g1.add_claim("A", "a", 0.9).unwrap();
    let c2 = g1.add_claim("B", "b", 0.9).unwrap();
    g1.add_link(&c1.uid, &c2.uid, EdgeType::Supports).unwrap();

    let snapshot = g1.export_typed().unwrap();

    // Import into a fresh graph
    let g2 = mem_graph();
    let result = g2.import_typed(&snapshot).unwrap();
    assert_eq!(result.nodes_imported, 2);
    assert_eq!(result.edges_imported, 1);
    assert_eq!(result.nodes_skipped, 0);

    // Re-import should skip all
    let result2 = g2.import_typed(&snapshot).unwrap();
    assert_eq!(result2.nodes_skipped, 2);
    assert_eq!(result2.edges_skipped, 1);
}

#[test]
fn test_validated_batch_success() {
    let g = mem_graph();
    let existing = g.add_claim("Existing", "ex", 0.9).unwrap();

    let ops = vec![
        GraphOp::AddNode(Box::new(make_claim_node("New claim", 0.8))),
        GraphOp::Tombstone {
            uid: existing.uid.clone(),
            reason: "cleanup".into(),
            by: "test".into(),
        },
    ];

    let batch = g.validate_batch(ops).unwrap();
    let result = g.apply_validated_batch(batch).unwrap();
    assert_eq!(result.nodes_added, 1);
    assert_eq!(result.nodes_tombstoned, 1);
}

#[test]
fn test_validated_batch_fails_on_missing_node() {
    let g = mem_graph();
    let fake_uid = Uid::new();

    let ops = vec![
        GraphOp::AddEdge(Box::new(CreateEdge::new(
            fake_uid.clone(),
            Uid::new(),
            EdgeProps::Supports { strength: None, support_type: None },
        ))),
    ];

    let result = g.validate_batch(ops);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

// ==== v0.4.1 Tests ====

// ---- Issue 9: Display impls ----

#[test]
fn test_display_graph_event() {
    let g = mem_graph();
    let node = g.add_claim("Test claim", "content", 0.9).unwrap();
    let event = GraphEvent::NodeAdded(Box::new(node.clone()));
    let s = format!("{}", event);
    assert!(s.starts_with("NodeAdded("));
    assert!(s.contains("Test claim"));

    let event2 = GraphEvent::NodeUpdated { uid: node.uid.clone(), version: 2 };
    let s2 = format!("{}", event2);
    assert!(s2.starts_with("NodeUpdated("));
    assert!(s2.contains("v2"));

    let event3 = GraphEvent::NodeTombstoned(node.uid.clone());
    let s3 = format!("{}", event3);
    assert!(s3.starts_with("NodeTombstoned("));

    let edge = g.add_claim("B", "b", 0.8).unwrap();
    let e = g.add_link(&node.uid, &edge.uid, EdgeType::Supports).unwrap();
    let event4 = GraphEvent::EdgeAdded(Box::new(e.clone()));
    let s4 = format!("{}", event4);
    assert!(s4.starts_with("EdgeAdded("));
    assert!(s4.contains("SUPPORTS"));

    let event5 = GraphEvent::EdgeTombstoned {
        uid: e.uid.clone(),
        from_uid: node.uid.clone(),
        to_uid: edge.uid.clone(),
        edge_type: EdgeType::Supports,
    };
    let s5 = format!("{}", event5);
    assert!(s5.starts_with("EdgeTombstoned("));
    assert!(s5.contains("SUPPORTS"));
}

#[test]
fn test_display_graph_stats() {
    let g = mem_graph();
    g.add_claim("A", "a", 0.9).unwrap();
    let stats = g.stats().unwrap();
    let s = format!("{}", stats);
    assert!(s.starts_with("GraphStats {"));
    assert!(s.contains("nodes:"));
    assert!(s.contains("edges:"));
}

#[test]
fn test_display_decay_result() {
    let result = DecayResult { nodes_decayed: 10, below_threshold: 2 };
    let s = format!("{}", result);
    assert_eq!(s, "DecayResult { decayed: 10, below_threshold: 2 }");
}

#[test]
fn test_display_batch_result() {
    let result = BatchResult {
        nodes_added: 3, edges_added: 2, nodes_tombstoned: 1, edges_tombstoned: 0,
    };
    let s = format!("{}", result);
    assert_eq!(s, "BatchResult { +3 nodes, +2 edges, -1 nodes, -0 edges }");
}

// ---- Issue 7: Rich EdgeTombstoned event ----

#[test]
fn test_edge_tombstoned_event_rich() {
    let g = mem_graph();
    let events: Arc<std::sync::Mutex<Vec<String>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let events_clone = events.clone();

    g.on_change(move |event| {
        events_clone.lock().unwrap().push(format!("{:?}", event));
    });

    let c1 = g.add_claim("A", "a", 0.9).unwrap();
    let c2 = g.add_claim("B", "b", 0.8).unwrap();
    let edge = g.add_link(&c1.uid, &c2.uid, EdgeType::Supports).unwrap();
    g.tombstone_edge(&edge.uid, "test", "tester").unwrap();

    let evts = events.lock().unwrap();
    let last = evts.last().unwrap();
    assert!(last.contains("EdgeTombstoned"));
    assert!(last.contains("from_uid"));
    assert!(last.contains("to_uid"));
    assert!(last.contains("edge_type: Supports"));
}

// ---- Issue 6: Multi-type NodeFilter ----

#[test]
fn test_find_nodes_multi_type() {
    let g = mem_graph();
    g.add_claim("Claim A", "a", 0.9).unwrap();
    g.add_entity("Entity B", "person").unwrap();
    g.add_goal("Goal C", "high").unwrap();

    let results = g.find_nodes(&NodeFilter::new().node_types(vec![NodeType::Claim, NodeType::Entity])).unwrap();
    assert_eq!(results.len(), 2);
    let types: Vec<NodeType> = results.iter().map(|n| n.node_type).collect();
    assert!(types.contains(&NodeType::Claim));
    assert!(types.contains(&NodeType::Entity));
}

#[test]
fn test_find_nodes_multi_type_with_single_still_works() {
    let g = mem_graph();
    g.add_claim("Claim A", "a", 0.9).unwrap();
    g.add_entity("Entity B", "person").unwrap();

    // Single node_type still works
    let results = g.find_nodes(&NodeFilter::new().node_type(NodeType::Claim)).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].node_type, NodeType::Claim);

    // node_types takes precedence over node_type
    let filter = NodeFilter {
        node_type: Some(NodeType::Claim),
        node_types: Some(vec![NodeType::Entity]),
        ..Default::default()
    };
    let results2 = g.find_nodes(&filter).unwrap();
    assert_eq!(results2.len(), 1);
    assert_eq!(results2[0].node_type, NodeType::Entity);
}

// ---- Issue 5: Batch salience decay ----

#[test]
fn test_decay_salience_many_nodes() {
    let g = mem_graph();
    // Create 200 nodes
    let creates: Vec<CreateNode> = (0..200)
        .map(|i| {
            CreateNode::new(
                format!("Node {}", i),
                NodeProps::Claim(ClaimProps {
                    content: format!("Content {}", i),
                    ..Default::default()
                }),
            )
            .salience(Salience::new(0.8).unwrap())
        })
        .collect();
    g.add_nodes_batch(creates).unwrap();

    // Give some elapsed time by using a large half-life
    let result = g.decay_salience(1.0).unwrap();
    assert!(result.nodes_decayed > 0);
}

// ---- Issue 1: validate_batch cross-reference ----

#[test]
fn test_validate_batch_cross_reference() {
    let g = mem_graph();
    let node_uid = Uid::new();

    let ops = vec![
        GraphOp::AddNode(Box::new(
            CreateNode::new("Cross-ref node", NodeProps::Claim(ClaimProps {
                content: "test".into(),
                ..Default::default()
            }))
            .with_uid(node_uid.clone()),
        )),
        GraphOp::AddEdge(Box::new(CreateEdge::new(
            node_uid.clone(),
            node_uid.clone(), // self-referencing for simplicity
            EdgeProps::Supports { strength: None, support_type: None },
        ))),
    ];

    let batch = g.validate_batch(ops).unwrap();
    let result = g.apply_validated_batch(batch).unwrap();
    assert_eq!(result.nodes_added, 1);
    assert_eq!(result.edges_added, 1);

    // Verify the node got the pre-assigned UID
    let node = g.get_node(&node_uid).unwrap();
    assert!(node.is_some());
    assert_eq!(node.unwrap().label, "Cross-ref node");
}

#[test]
fn test_create_node_with_uid() {
    let g = mem_graph();
    let uid = Uid::new();
    let node = g.add_node(
        CreateNode::new("Pre-UID node", NodeProps::Entity(EntityProps {
            entity_type: "test".into(),
            ..Default::default()
        }))
        .with_uid(uid.clone()),
    ).unwrap();

    assert_eq!(node.uid, uid);
    let fetched = g.get_node(&uid).unwrap().unwrap();
    assert_eq!(fetched.label, "Pre-UID node");
}

// ---- Issue 2: Memory constructors ----

#[test]
fn test_add_session() {
    let g = mem_graph();
    let node = g.add_session("Morning session", "Reviewing PRs").unwrap();
    assert_eq!(node.node_type, NodeType::Session);
    assert_eq!(node.layer, Layer::Memory);
    assert_eq!(node.summary, "Reviewing PRs");
}

#[test]
fn test_add_preference() {
    let g = mem_graph();
    let node = g.add_preference("Dark mode preference", "theme", "dark").unwrap();
    assert_eq!(node.node_type, NodeType::Preference);
    assert_eq!(node.layer, Layer::Memory);
    match &node.props {
        NodeProps::Preference(p) => {
            assert_eq!(p.key, "theme");
            assert_eq!(p.value, "dark");
        }
        _ => panic!("Expected Preference props"),
    }
}

#[test]
fn test_add_summary() {
    let g = mem_graph();
    let node = g.add_summary("Session summary", "Discussed architecture decisions").unwrap();
    assert_eq!(node.node_type, NodeType::Summary);
    assert_eq!(node.layer, Layer::Memory);
    assert_eq!(node.summary, "Discussed architecture decisions");
}

// ---- Issue 4: semantic_search tombstone compensation ----

#[test]
fn test_semantic_search_enough_results_with_tombstoned() {
    let g = mem_graph();
    g.configure_embeddings(3).unwrap();

    // Create 10 nodes with embeddings
    let mut uids = Vec::new();
    for i in 0..10 {
        let node = g.add_claim(&format!("Claim {}", i), &format!("content {}", i), 0.9).unwrap();
        let emb = vec![i as f32 * 0.1, 0.5, 0.5];
        g.set_embedding(&node.uid, &emb).unwrap();
        uids.push(node.uid);
    }

    // Tombstone 5 of them
    for uid in &uids[0..5] {
        g.tombstone(uid, "test", "tester").unwrap();
    }

    // Search for k=3 — should get exactly 3 live nodes
    let query_vec = vec![0.5, 0.5, 0.5];
    let results = g.semantic_search(&query_vec, 3).unwrap();
    assert_eq!(results.len(), 3);
    // All results should be live
    for (node, _dist) in &results {
        assert!(node.tombstone_at.is_none());
    }
}

// ---- Issue 3: embed_nodes batch ----

struct TestEmbedder;
impl EmbeddingProvider for TestEmbedder {
    fn dimension(&self) -> usize { 3 }
    fn embed(&self, text: &str) -> mindgraph::Result<Vec<f32>> {
        Ok(vec![text.len() as f32 * 0.01, 0.5, 0.5])
    }
}

#[test]
fn test_embed_nodes_batch() {
    let g = mem_graph();
    g.configure_embeddings(3).unwrap();

    let c1 = g.add_claim("Alpha", "first", 0.9).unwrap();
    let c2 = g.add_claim("Beta", "second", 0.8).unwrap();

    let provider = TestEmbedder;
    let count = g.embed_nodes(&[c1.uid.clone(), c2.uid.clone()], &provider).unwrap();
    assert_eq!(count, 2);

    // Verify embeddings were stored
    let emb1 = g.get_embedding(&c1.uid).unwrap().unwrap();
    assert_eq!(emb1.len(), 3);
    let emb2 = g.get_embedding(&c2.uid).unwrap().unwrap();
    assert_eq!(emb2.len(), 3);
}

#[test]
fn test_embed_nodes_skips_tombstoned() {
    let g = mem_graph();
    g.configure_embeddings(3).unwrap();

    let c1 = g.add_claim("Live", "content", 0.9).unwrap();
    let c2 = g.add_claim("Dead", "content", 0.8).unwrap();
    g.tombstone(&c2.uid, "done", "test").unwrap();

    let provider = TestEmbedder;
    let count = g.embed_nodes(&[c1.uid.clone(), c2.uid.clone()], &provider).unwrap();
    assert_eq!(count, 1); // Only the live node
}

// ---- Issue 10: TypedSnapshot embeddings ----

#[test]
fn test_typed_export_includes_embeddings() {
    let g = mem_graph();
    g.configure_embeddings(3).unwrap();

    let c1 = g.add_claim("A", "a", 0.9).unwrap();
    g.set_embedding(&c1.uid, &[0.1, 0.2, 0.3]).unwrap();

    let snapshot = g.export_typed().unwrap();
    assert_eq!(snapshot.nodes.len(), 1);
    assert_eq!(snapshot.embeddings.len(), 1);
    assert_eq!(snapshot.embeddings[0].0, c1.uid);
    assert_eq!(snapshot.embeddings[0].1.len(), 3);
}

#[test]
fn test_typed_import_restores_embeddings() {
    let g1 = mem_graph();
    g1.configure_embeddings(3).unwrap();

    let c1 = g1.add_claim("A", "a", 0.9).unwrap();
    g1.set_embedding(&c1.uid, &[0.1, 0.2, 0.3]).unwrap();

    let snapshot = g1.export_typed().unwrap();

    // Import into a fresh graph with same embedding dimension
    let g2 = mem_graph();
    g2.configure_embeddings(3).unwrap();

    let result = g2.import_typed(&snapshot).unwrap();
    assert_eq!(result.nodes_imported, 1);
    assert_eq!(result.embeddings_imported, 1);

    // Verify embedding was restored
    let emb = g2.get_embedding(&c1.uid).unwrap().unwrap();
    assert_eq!(emb.len(), 3);
    assert!((emb[0] - 0.1).abs() < 0.001);
}

// ==== Phase v0.5: New Methods ====

#[test]
fn test_get_edge_between() {
    let g = mem_graph();
    let n1 = g.add_node(make_entity_node("Rust")).unwrap();
    let n2 = g.add_node(make_entity_node("Python")).unwrap();

    // No edges yet
    let edges = g.get_edge_between(&n1.uid, &n2.uid, None).unwrap();
    assert!(edges.is_empty());

    // Add an edge
    let edge = g.add_link(&n1.uid, &n2.uid, EdgeType::RelevantTo).unwrap();

    // Find edge without type filter
    let edges = g.get_edge_between(&n1.uid, &n2.uid, None).unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].uid, edge.uid);

    // Find edge with matching type filter
    let edges = g.get_edge_between(&n1.uid, &n2.uid, Some(EdgeType::RelevantTo)).unwrap();
    assert_eq!(edges.len(), 1);

    // Find edge with non-matching type filter
    let edges = g.get_edge_between(&n1.uid, &n2.uid, Some(EdgeType::Supports)).unwrap();
    assert!(edges.is_empty());

    // Direction matters
    let edges = g.get_edge_between(&n2.uid, &n1.uid, None).unwrap();
    assert!(edges.is_empty());
}

#[test]
fn test_list_nodes() {
    let g = mem_graph();

    // Empty graph
    let page = g.list_nodes(Pagination::first(10)).unwrap();
    assert!(page.items.is_empty());
    assert!(!page.has_more);

    // Add some nodes
    g.add_node(make_entity_node("Rust")).unwrap();
    g.add_node(make_entity_node("Python")).unwrap();
    g.add_node(make_entity_node("Go")).unwrap();

    // List all
    let page = g.list_nodes(Pagination::first(10)).unwrap();
    assert_eq!(page.items.len(), 3);
    assert!(!page.has_more);

    // Paginate
    let page = g.list_nodes(Pagination { limit: 2, offset: 0 }).unwrap();
    assert_eq!(page.items.len(), 2);
    assert!(page.has_more);

    let page = g.list_nodes(Pagination { limit: 2, offset: 2 }).unwrap();
    assert_eq!(page.items.len(), 1);
    assert!(!page.has_more);
}

#[test]
fn test_clear() {
    let g = mem_graph();

    // Add some data
    let n1 = g.add_node(make_entity_node("Rust")).unwrap();
    let n2 = g.add_node(make_entity_node("Python")).unwrap();
    g.add_link(&n1.uid, &n2.uid, EdgeType::RelevantTo).unwrap();

    // Verify data exists
    let stats = g.stats().unwrap();
    assert_eq!(stats.live_nodes, 2);
    assert_eq!(stats.live_edges, 1);

    // Clear
    g.clear().unwrap();

    // Verify empty
    let page = g.list_nodes(Pagination::first(100)).unwrap();
    assert!(page.items.is_empty());
}

#[test]
fn test_pagination_default() {
    let p = Pagination::default();
    assert_eq!(p.limit, 100);
    assert_eq!(p.offset, 0);
}

#[test]
fn test_display_impls() {
    use std::fmt::Write;

    let mut s = String::new();

    let pr = PurgeResult { nodes_purged: 1, edges_purged: 2, versions_purged: 3 };
    write!(s, "{}", pr).unwrap();
    assert!(s.contains("nodes: 1"));
    s.clear();

    let mr = MergeResult { edges_retargeted: 4, aliases_merged: 5 };
    write!(s, "{}", mr).unwrap();
    assert!(s.contains("4"));
    s.clear();

    let ir = ImportResult { relations_imported: 6 };
    write!(s, "{}", ir).unwrap();
    assert!(s.contains("6"));
    s.clear();

    let tr = TombstoneResult { edges_tombstoned: 7 };
    write!(s, "{}", tr).unwrap();
    assert!(s.contains("7"));
    s.clear();

    let tir = TypedImportResult {
        nodes_imported: 1, edges_imported: 2,
        nodes_skipped: 3, edges_skipped: 4,
        embeddings_imported: 5,
    };
    write!(s, "{}", tir).unwrap();
    assert!(s.contains("nodes: +1"));
}
