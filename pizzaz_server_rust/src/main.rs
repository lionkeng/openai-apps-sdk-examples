//! Pizzaz MCP Server - Binary entry point

use std::net::SocketAddr;
use tokio::signal;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing subscriber
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pizzaz_server_rust=info,tower_http=debug,rmcp=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Parse port from environment or use default
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8000);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    info!("<U Pizzaz MCP Server starting...");
    info!("   Listening on: http://{}", addr);
    info!("   HTTP endpoint: POST http://{}/mcp", addr);
    info!(
        "   SSE stream: GET http://{}/mcp (with Session-Id header)",
        addr
    );
    info!("   Press Ctrl+C to stop");

    // Create TCP listener
    let listener = tokio::net::TcpListener::bind(addr).await?;

    // Create app
    let app = pizzaz_server_rust::create_app();

    // Start server with graceful shutdown
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    info!("Server shut down gracefully");
    Ok(())
}

/// Handles Ctrl+C signal for graceful shutdown
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            warn!("Received Ctrl+C, shutting down...");
        },
        _ = terminate => {
            warn!("Received SIGTERM, shutting down...");
        },
    }
}
