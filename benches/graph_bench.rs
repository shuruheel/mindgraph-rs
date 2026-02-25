use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mindgraph::*;

fn bench_add_node(c: &mut Criterion) {
    let graph = MindGraph::open_in_memory().unwrap();
    c.bench_function("add_node", |b| {
        b.iter(|| {
            graph
                .add_claim(
                    black_box("Test claim"),
                    black_box("Test content"),
                    black_box(0.9),
                )
                .unwrap()
        })
    });
}

fn bench_add_node_batch(c: &mut Criterion) {
    let graph = MindGraph::open_in_memory().unwrap();
    c.bench_function("add_nodes_batch_100", |b| {
        b.iter(|| {
            let creates: Vec<CreateNode> = (0..100)
                .map(|i| {
                    CreateNode::new(
                        format!("Node {}", i),
                        NodeProps::Claim(ClaimProps {
                            content: format!("Content {}", i),
                            ..Default::default()
                        }),
                    )
                })
                .collect();
            graph.add_nodes_batch(black_box(creates)).unwrap()
        })
    });
}

fn bench_get_node(c: &mut Criterion) {
    let graph = MindGraph::open_in_memory().unwrap();
    let node = graph.add_claim("Benchmark", "content", 0.9).unwrap();
    let uid = node.uid.clone();
    c.bench_function("get_node", |b| {
        b.iter(|| graph.get_node(black_box(&uid)).unwrap())
    });
}

fn bench_add_edge(c: &mut Criterion) {
    let graph = MindGraph::open_in_memory().unwrap();
    let n1 = graph.add_entity("Entity A", "test").unwrap();
    let n2 = graph.add_entity("Entity B", "test").unwrap();
    c.bench_function("add_edge", |b| {
        b.iter(|| {
            graph
                .add_edge(CreateEdge::new(
                    n1.uid.clone(),
                    n2.uid.clone(),
                    black_box(EdgeProps::Supports {
                        strength: Some(0.8),
                        support_type: None,
                    }),
                ))
                .unwrap()
        })
    });
}

fn bench_fts_search(c: &mut Criterion) {
    let graph = MindGraph::open_in_memory().unwrap();
    for i in 0..100 {
        graph
            .add_claim(
                &format!("Claim about Rust safety feature {}", i),
                "content",
                0.9,
            )
            .unwrap();
    }
    c.bench_function("fts_search", |b| {
        b.iter(|| {
            graph
                .search(black_box("safety"), &SearchOptions::new())
                .unwrap()
        })
    });
}

fn bench_find_nodes(c: &mut Criterion) {
    let graph = MindGraph::open_in_memory().unwrap();
    for i in 0..100 {
        graph
            .add_claim(&format!("Claim {}", i), "content", (i as f64) / 100.0)
            .unwrap();
    }
    c.bench_function("find_nodes_filtered", |b| {
        b.iter(|| {
            graph
                .find_nodes(black_box(
                    &NodeFilter::new()
                        .node_type(NodeType::Claim)
                        .confidence_range(0.5, 1.0),
                ))
                .unwrap()
        })
    });
}

fn bench_traversal(c: &mut Criterion) {
    let graph = MindGraph::open_in_memory().unwrap();
    // Build a chain: n0 -> n1 -> n2 -> ... -> n9
    let mut uids = Vec::new();
    for i in 0..10 {
        let n = graph.add_entity(&format!("Node {}", i), "test").unwrap();
        uids.push(n.uid);
    }
    for i in 0..9 {
        graph
            .add_link(&uids[i], &uids[i + 1], EdgeType::DependsOn)
            .unwrap();
    }
    let start = uids[0].clone();
    c.bench_function("traversal_depth_10", |b| {
        b.iter(|| {
            graph
                .reachable(black_box(&start), &TraversalOptions::default())
                .unwrap()
        })
    });
}

fn bench_stats(c: &mut Criterion) {
    let graph = MindGraph::open_in_memory().unwrap();
    for i in 0..50 {
        graph
            .add_claim(&format!("Claim {}", i), "content", 0.9)
            .unwrap();
    }
    c.bench_function("stats", |b| b.iter(|| graph.stats().unwrap()));
}

fn bench_export_import(c: &mut Criterion) {
    let graph = MindGraph::open_in_memory().unwrap();
    for i in 0..20 {
        graph.add_entity(&format!("Entity {}", i), "test").unwrap();
    }
    c.bench_function("export_typed", |b| b.iter(|| graph.export_typed().unwrap()));
}

criterion_group!(
    benches,
    bench_add_node,
    bench_add_node_batch,
    bench_get_node,
    bench_add_edge,
    bench_fts_search,
    bench_find_nodes,
    bench_traversal,
    bench_stats,
    bench_export_import,
);
criterion_main!(benches);
