//! Agent memory: sessions, preferences, summaries, salience decay.

use mindgraph::*;

fn main() -> Result<()> {
    let graph = MindGraph::open_in_memory()?;
    graph.set_default_agent("agent-1");

    // Record a session
    let session = graph.add_session("Planning meeting", "Discussed Q2 roadmap and priorities")?;
    println!(
        "Session: {} (salience={:.2})",
        session.label,
        session.salience.value()
    );

    // Record preferences
    let pref1 = graph.add_preference("Code style", "indent", "4 spaces")?;
    let pref2 = graph.add_preference("Language", "language", "Rust")?;
    println!("Preferences recorded: {}, {}", pref1.label, pref2.label);

    // Create a summary
    let summary = graph.add_summary("Q2 Roadmap", "Focus on performance and API stability")?;
    graph.add_link(&summary.uid, &session.uid, EdgeType::Summarizes)?;

    // Add a goal
    let goal = graph.add_goal("Ship v1.0", "high")?;
    println!("Goal: {} (active)", goal.label);

    // Check active goals
    let goals = graph.active_goals()?;
    println!("Active goals: {}", goals.len());

    // Decay salience (simulating time passing)
    let decay = graph.decay_salience(3600.0)?;
    println!("Decay result: {}", decay);

    // Query memory layer
    let memories = graph.nodes_in_layer(Layer::Memory)?;
    println!("Memory nodes: {}", memories.len());

    // Version history
    let history = graph.node_history(&session.uid)?;
    println!("Session versions: {}", history.len());

    // Export and reimport
    let snapshot = graph.export_typed()?;
    println!(
        "Exported {} nodes, {} edges",
        snapshot.nodes.len(),
        snapshot.edges.len()
    );

    Ok(())
}
