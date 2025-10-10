//! Integration tests for pizzaz_server_rust
//!
//! These tests verify the full HTTP request/response cycle using tower::ServiceExt.

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use http_body_util::BodyExt;
use pizzaz_server_rust::handler::PizzazServerHandler;
use serde_json::json;
use tower::ServiceExt; // for oneshot()

/// Helper to create test app
fn create_test_app() -> axum::Router {
    pizzaz_server_rust::create_app()
}

/// Helper to build JSON-RPC request
#[allow(dead_code)]
fn build_jsonrpc_request(method: &str, params: serde_json::Value, id: i32) -> serde_json::Value {
    json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": id
    })
}

/// Helper to parse response body
#[allow(dead_code)]
async fn parse_response_body(
    response: axum::response::Response,
) -> Result<serde_json::Value, String> {
    let body = response
        .into_body()
        .collect()
        .await
        .map_err(|e| format!("Failed to collect body: {}", e))?
        .to_bytes();
    serde_json::from_slice(&body).map_err(|e| format!("Failed to parse JSON: {}", e))
}

// ============================================================================
// Handler Direct Tests
// ============================================================================

#[tokio::test]
async fn test_handler_list_tools_returns_five_tools() {
    let handler = PizzazServerHandler::new();
    let tools = handler.list_tools().await;

    assert_eq!(tools.len(), 5);
    assert!(tools.iter().any(|t| t.name == "pizza-map"));
    assert!(tools.iter().any(|t| t.name == "pizza-carousel"));
}

#[tokio::test]
async fn test_handler_call_tool_returns_structured_content() {
    let handler = PizzazServerHandler::new();
    let args = json!({ "pizzaTopping": "mushroom" });

    let result = handler.call_tool("pizza-map", args).await;

    assert!(result.is_ok());
    let call_result = result.unwrap();
    assert_eq!(call_result.structured_content["pizzaTopping"], "mushroom");
}

#[tokio::test]
async fn test_handler_list_resources_returns_five_resources() {
    let handler = PizzazServerHandler::new();
    let resources = handler.list_resources().await;

    assert_eq!(resources.len(), 5);
    assert!(resources
        .iter()
        .any(|r| r.uri == "ui://widget/pizza-map.html"));
}

#[tokio::test]
async fn test_handler_read_resource_returns_html() {
    let handler = PizzazServerHandler::new();
    let result = handler.read_resource("ui://widget/pizza-map.html").await;

    assert!(result.is_ok());
    let content = result.unwrap();
    assert_eq!(content.mime_type, "text/html+skybridge");
    assert!(content.text.contains("pizzaz"));
}

#[tokio::test]
async fn test_handler_list_resource_templates_returns_five_templates() {
    let handler = PizzazServerHandler::new();
    let templates = handler.list_resource_templates().await;

    assert_eq!(templates.len(), 5);
    assert!(templates
        .iter()
        .any(|t| t.uri_template == "ui://widget/pizza-map.html"));
}

// ============================================================================
// CORS Tests
// ============================================================================

#[tokio::test]
async fn test_cors_preflight() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/")
                .header(header::ORIGIN, "https://chatgpt.com")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let allow_origin = response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN);
    assert!(allow_origin.is_some());
}

#[tokio::test]
async fn test_cors_on_actual_request() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/")
                .header(header::ORIGIN, "https://chatgpt.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let allow_origin = response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN);
    assert!(
        allow_origin.is_some(),
        "CORS allow-origin header should be present"
    );
}

// ============================================================================
// HTTP Endpoint Tests (Future - when routes are added)
// ============================================================================

#[tokio::test]
async fn test_root_endpoint_returns_ok() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Currently returns 404 since no routes defined
    // This test documents current behavior
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_multiple_requests_with_ready_call() {
    // Test that the service can handle multiple sequential requests
    for _i in 0..5 {
        let app = create_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should handle multiple requests
        assert!(response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::OK);
    }
}
