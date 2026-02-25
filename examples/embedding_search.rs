//! Embedding search with a mock provider.

use mindgraph::*;

/// A mock embedding provider for demonstration.
struct MockEmbeddings;

impl EmbeddingProvider for MockEmbeddings {
    fn dimension(&self) -> usize {
        4
    }

    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Simple hash-based mock embeddings
        let hash = text.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32));
        Ok(vec![
            (hash % 100) as f32 / 100.0,
            ((hash >> 8) % 100) as f32 / 100.0,
            ((hash >> 16) % 100) as f32 / 100.0,
            ((hash >> 24) % 100) as f32 / 100.0,
        ])
    }
}

fn main() -> Result<()> {
    let graph = MindGraph::open_in_memory()?;

    // Configure embeddings
    graph.configure_embeddings(4)?;
    println!("Embedding dimension: {:?}", graph.embedding_dimension());

    // Add some nodes
    let n1 = graph.add_claim("Rust is fast", "Rust compiles to native code", 0.9)?;
    let n2 = graph.add_claim("Rust is safe", "Rust prevents memory errors", 0.95)?;
    let n3 = graph.add_claim("Python is easy", "Python has simple syntax", 0.8)?;
    let n4 = graph.add_observation("Benchmark results", "Rust outperforms Python by 10x")?;

    // Embed all nodes
    let provider = MockEmbeddings;
    let uids = vec![
        n1.uid.clone(),
        n2.uid.clone(),
        n3.uid.clone(),
        n4.uid.clone(),
    ];
    let count = graph.embed_nodes(&uids, &provider)?;
    println!("Embedded {} nodes", count);

    // Semantic search
    let query_vec = provider.embed("Rust performance")?;
    let results = graph.semantic_search(&query_vec, 3)?;
    println!("Semantic search results:");
    for (node, distance) in &results {
        println!("  {}: distance={:.4}", node.label, distance);
    }

    // Text-based semantic search
    let results = graph.semantic_search_text("memory safety", 2, &provider)?;
    println!("Text search results:");
    for (node, distance) in &results {
        println!("  {}: distance={:.4}", node.label, distance);
    }

    Ok(())
}
