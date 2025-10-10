//! Pizzaz MCP Server - Rust implementation
//!
//! This library provides an MCP server that exposes pizza-themed widgets
//! for integration with ChatGPT and other MCP clients.

pub mod handler;
pub mod types;
pub mod widgets;

#[cfg(test)]
mod test_helpers;

use axum::{
    http::{header, Request, Response},
    routing::any_service,
    Router,
};
use bytes::Bytes;
use futures::future::BoxFuture;
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use rmcp::transport::{
    streamable_http_server::session::local::LocalSessionManager, StreamableHttpServerConfig,
    StreamableHttpService,
};
use serde_json::Value;
use std::{convert::Infallible, sync::Arc};
use tower::Service;
use tower_http::cors::CorsLayer;

type McpResponse = Response<BoxBody<Bytes, Infallible>>;

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
    let session_manager = Arc::new(LocalSessionManager::default());
    let config = StreamableHttpServerConfig::default();
    let streamable_service = StreamableHttpService::new(
        || Ok(handler::PizzazServerHandler::new()),
        session_manager,
        config,
    );

    let augmented_service = MetaAugmentService::new(streamable_service);

    Router::new()
        .route("/mcp", any_service(augmented_service))
        .layer(CorsLayer::permissive())
}

/// Wraps an MCP HTTP service and injects widget metadata into JSON responses.
#[derive(Clone)]
struct MetaAugmentService<S> {
    inner: S,
}

impl<S> MetaAugmentService<S>
where
    S: Service<Request<axum::body::Body>, Response = McpResponse, Error = std::convert::Infallible>
        + Clone,
    S::Future: Send + 'static,
{
    fn new(service: S) -> Self {
        Self { inner: service }
    }
}

impl<S> Service<Request<axum::body::Body>> for MetaAugmentService<S>
where
    S: Service<Request<axum::body::Body>, Response = McpResponse, Error = std::convert::Infallible>
        + Clone,
    S::Future: Send + 'static,
{
    type Response = McpResponse;
    type Error = std::convert::Infallible;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<axum::body::Body>) -> Self::Future {
        let future = self.inner.call(request);
        Box::pin(async move {
            let response = future.await?;
            if !should_augment(&response) {
                return Ok(response);
            }

            let (mut parts, body) = response.into_parts();
            let collected = match body.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(_) => Bytes::new(),
            };

            let mut json: Value = match serde_json::from_slice(&collected) {
                Ok(value) => value,
                Err(_) => {
                    let body = Full::new(collected).boxed();
                    return Ok(Response::from_parts(parts, body));
                }
            };

            augment_widget_metadata(&mut json);

            let serialized = match serde_json::to_vec(&json) {
                Ok(bytes) => bytes,
                Err(_) => collected.to_vec(),
            };

            if let Ok(value) = header::HeaderValue::from_str(&serialized.len().to_string()) {
                parts.headers.insert(header::CONTENT_LENGTH, value);
            }

            let body = Full::new(Bytes::from(serialized)).boxed();
            Ok(Response::from_parts(parts, body))
        })
    }
}

fn should_augment(response: &McpResponse) -> bool {
    response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|mime| mime.contains("application/json"))
        .unwrap_or(false)
}

fn augment_widget_metadata(payload: &mut Value) {
    let Some(result) = payload.get_mut("result") else {
        return;
    };

    if let Some(tools) = result.get_mut("tools").and_then(Value::as_array_mut) {
        for tool in tools {
            if let Some(object) = tool.as_object_mut() {
                if let Some(name) = object.get("name").and_then(Value::as_str) {
                    if let Some(widget) = crate::widgets::get_widget_by_id(name) {
                        object
                            .entry("_meta".to_string())
                            .or_insert_with(|| widget.meta());
                    }
                }
            }
        }
    }

    if let Some(resources) = result.get_mut("resources").and_then(Value::as_array_mut) {
        for resource in resources {
            if let Some(object) = resource.as_object_mut() {
                if let Some(uri) = object.get("uri").and_then(Value::as_str) {
                    if let Some(widget) = crate::widgets::get_widget_by_uri(uri) {
                        object
                            .entry("_meta".to_string())
                            .or_insert_with(|| widget.meta());
                    }
                }
            }
        }
    }

    if let Some(templates) = result
        .get_mut("resourceTemplates")
        .and_then(Value::as_array_mut)
    {
        for template in templates {
            if let Some(object) = template.as_object_mut() {
                if let Some(uri) = object.get("uriTemplate").and_then(Value::as_str) {
                    if let Some(widget) = crate::widgets::get_widget_by_uri(uri) {
                        object
                            .entry("_meta".to_string())
                            .or_insert_with(|| widget.meta());
                    }
                }
            }
        }
    }
}
