#![cfg(feature = "async")]

use mindgraph::*;

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

#[tokio::test]
async fn test_async_add_and_get_node() {
    let g = AsyncMindGraph::open_in_memory().await.unwrap();

    let node = g.add_node(make_entity_node("Rust")).await.unwrap();
    assert_eq!(node.label, "Rust");
    assert_eq!(node.node_type, NodeType::Entity);

    let fetched = g.get_node(node.uid.clone()).await.unwrap().unwrap();
    assert_eq!(fetched.uid, node.uid);
    assert_eq!(fetched.label, "Rust");

    assert_eq!(g.count_nodes(NodeType::Entity).await.unwrap(), 1);
    assert!(g.node_exists(node.uid).await.unwrap());
}

#[tokio::test]
async fn test_async_concurrent_writes() {
    let g = AsyncMindGraph::open_in_memory().await.unwrap();

    let mut handles = Vec::new();
    for i in 0..10 {
        let g = g.clone();
        handles.push(tokio::spawn(async move {
            g.add_node(make_entity_node(&format!("Lang {}", i)))
                .await
                .unwrap()
        }));
    }

    let mut nodes = Vec::new();
    for handle in handles {
        nodes.push(handle.await.unwrap());
    }

    assert_eq!(nodes.len(), 10);
    assert_eq!(g.count_nodes(NodeType::Entity).await.unwrap(), 10);

    // All nodes should be individually readable
    for node in &nodes {
        assert!(g.node_exists(node.uid.clone()).await.unwrap());
    }
}

// ==== v0.6: WatchStream ====

#[tokio::test]
async fn test_async_watch_stream() {
    let g = AsyncMindGraph::open_in_memory().await.unwrap();
    let mut stream = g.watch(EventFilter::new().event_kinds(vec![EventKind::NodeAdded]));

    // Add a node and check we get the event
    let node = g.add_entity("Test".into(), "test".into()).await.unwrap();

    let event = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        stream.recv(),
    ).await.unwrap().unwrap();

    assert_eq!(event.kind(), EventKind::NodeAdded);
    if let GraphEvent::NodeAdded(n) = &event {
        assert_eq!(n.uid, node.uid);
    } else {
        panic!("Expected NodeAdded event");
    }
}

// ==== v0.6: AsyncAgentHandle ====

#[tokio::test]
async fn test_async_agent_handle() {
    let g = AsyncMindGraph::open_in_memory().await.unwrap();
    let alice = g.agent("alice");

    assert_eq!(alice.agent_id(), "alice");

    let node = alice.add_entity("Test Entity".into(), "test".into()).await.unwrap();
    assert_eq!(node.label, "Test Entity");

    let my = alice.my_nodes().await.unwrap();
    assert_eq!(my.len(), 1);
    assert_eq!(my[0].uid, node.uid);
}

// ==== v0.6: Custom types in async ====

#[tokio::test]
async fn test_async_custom_node() {
    use serde::{Serialize, Deserialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Widget {
        color: String,
    }
    impl CustomNodeType for Widget {
        fn type_name() -> &'static str { "Widget" }
        fn layer() -> Layer { Layer::Reality }
    }

    let g = AsyncMindGraph::open_in_memory().await.unwrap();
    let node = g.add_custom_node("red widget".into(), Widget { color: "red".into() }).await.unwrap();
    assert_eq!(node.node_type, NodeType::Custom("Widget".into()));
    let w: Widget = node.custom_props().unwrap();
    assert_eq!(w.color, "red");
}
