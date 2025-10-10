//! Pizzaz MCP Server - Rust implementation
//!
//! This library provides an MCP server that exposes pizza-themed widgets
//! for integration with ChatGPT and other MCP clients.

pub mod handler;
pub mod types;
pub mod widgets;

#[cfg(test)]
mod test_helpers;

use axum::Router;
use tower_http::cors::CorsLayer;

/// Creates the Axum application with all routes and middleware
///
/// This function is public to allow testing without starting an HTTP server.
///
/// # Example
///
/// ```no_run
/// use pizzaz_server_rust::create_app;
/// use tower::ServiceExt;
///
/// #[tokio::main]
/// async fn main() {
///     let app = create_app();
///     // Use app for testing with tower::ServiceExt::oneshot()
/// }
/// ```
pub fn create_app() -> Router {
    use handler::PizzazServerHandler;

    let _handler = PizzazServerHandler::new();

    // TODO: Integrate with rmcp::transport::StreamableHttpService once handler is implemented
    Router::new().layer(CorsLayer::permissive())
}
