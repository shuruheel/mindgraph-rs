//! Basic usage: create graph, add nodes/edges, query, traverse.

use mindgraph::*;

fn main() -> Result<()> {
    let graph = MindGraph::open_in_memory()?;

    // Add a claim
    let claim = graph.add_claim("Rust is memory safe", "Rust prevents use-after-free bugs", 0.95)?;
    println!("Added claim: {} ({})", claim.label, claim.uid);

    // Add supporting evidence
    let evidence = graph.add_node(
        CreateNode::new("Borrow checker", NodeProps::Evidence(EvidenceProps {
            description: "The borrow checker prevents dangling pointers at compile time".into(),
            evidence_type: Some("empirical".into()),
            ..Default::default()
        }))
    )?;
    println!("Added evidence: {}", evidence.label);

    // Add entity
    let entity = graph.add_entity("Rust language", "programming_language")?;

    // Connect with edges
    graph.add_edge(CreateEdge::new(
        evidence.uid.clone(),
        claim.uid.clone(),
        EdgeProps::Supports { strength: Some(0.9), support_type: Some("empirical".into()) },
    ))?;

    graph.add_link(&claim.uid, &entity.uid, EdgeType::Describes)?;

    // Update using builder pattern
    graph.update(&claim.uid)
        .confidence(Confidence::new(0.99)?)
        .changed_by("demo")
        .reason("strong supporting evidence")
        .apply()?;

    // Search
    let results = graph.search("memory", &SearchOptions::new())?;
    println!("Search results for 'memory': {}", results.len());

    // Structured filter
    let claims = graph.find_nodes(
        &NodeFilter::new().node_type(NodeType::Claim)
    )?;
    println!("Found {} claim(s)", claims.len());

    // Traverse reasoning chain
    let chain = graph.reasoning_chain(&claim.uid, 5)?;
    for step in &chain {
        println!("  depth {}: {} ({:?})", step.depth, step.label, step.node_type);
    }

    // Stats
    let stats = graph.stats()?;
    println!("Graph stats: {}", stats);

    // List all nodes
    let page = graph.list_nodes(Pagination::first(10))?;
    println!("Listed {} nodes, has_more={}", page.items.len(), page.has_more);

    Ok(())
}
