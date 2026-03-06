use std::sync::Arc;

use mindgraph::AsyncMindGraph;
use mindgraph_server::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,mindgraph_server=debug".parse().unwrap()),
        )
        .init();

    let db_path = std::env::var("MINDGRAPH_DB_PATH").unwrap_or_else(|_| "mindgraph.db".into());
    let token = std::env::var("MINDGRAPH_TOKEN").ok();
    let port: u16 = std::env::var("MINDGRAPH_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(18790);

    let graph = if db_path == ":memory:" {
        AsyncMindGraph::open_in_memory()
            .await
            .expect("failed to open in-memory graph")
    } else {
        AsyncMindGraph::open(&db_path)
            .await
            .expect("failed to open graph database")
    };

    if let Ok(agent) = std::env::var("MINDGRAPH_DEFAULT_AGENT") {
        graph.set_default_agent(agent).await;
    }

    let embedding_model = std::env::var("MINDGRAPH_EMBEDDING_MODEL")
        .unwrap_or_else(|_| "text-embedding-3-small".into());
    let distance_metric =
        std::env::var("MINDGRAPH_DISTANCE_METRIC").unwrap_or_else(|_| "cosine".into());

    let state = Arc::new(AppState {
        graph,
        token,
        embedding_model,
        distance_metric,
    });

    let bind_addr = std::env::var("MINDGRAPH_BIND").unwrap_or_else(|_| "127.0.0.1".into());
    let listener = tokio::net::TcpListener::bind((bind_addr.as_str(), port))
        .await
        .expect("failed to bind");
    tracing::info!("mindgraph-server listening on {bind_addr}:{port}");

    let router = mindgraph_server::app(state);

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received");
}
