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
