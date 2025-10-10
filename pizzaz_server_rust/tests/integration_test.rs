//! Integration tests for pizzaz_server_rust
//!
//! These tests verify the full HTTP request/response cycle using tower::ServiceExt.

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use http_body_util::BodyExt;
use pizzaz_server_rust::handler::PizzazServerHandler;
use serde_json::{json, Value};
use tower::ServiceExt; // for oneshot()

const ACCEPT_HEADER_VALUE: &str = "application/json, text/event-stream";

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
    if body.is_empty() {
        return Ok(Value::Null);
    }

    if let Ok(json) = serde_json::from_slice(&body) {
        return Ok(json);
    }

    // Attempt to parse Server-Sent Event payloads ("data: {json}\n\n")
    let text = String::from_utf8_lossy(&body);
    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            let trimmed = data.trim();
            if !trimmed.is_empty() {
                return serde_json::from_str(trimmed)
                    .map_err(|e| format!("Failed to parse JSON: {}", e));
            }
        }
    }

    Err("Failed to parse JSON: unsupported response format".to_string())
}

// ============================================================================
// Handler Direct Tests
// ============================================================================

#[tokio::test]
async fn test_handler_list_tools_returns_five_tools() {
    let handler = PizzazServerHandler::new();
    let tools = handler.list_widget_tools().await;

    assert_eq!(tools.len(), 5);
    assert!(tools.iter().any(|t| t.name == "pizza-map"));
    assert!(tools.iter().any(|t| t.name == "pizza-carousel"));
}

#[tokio::test]
async fn test_handler_call_tool_returns_structured_content() {
    let handler = PizzazServerHandler::new();
    let args = json!({ "pizzaTopping": "mushroom" });

    let result = handler
        .call_widget_tool("pizza-map", args)
        .await
        .expect("tool call succeeds");
    assert_eq!(result.structured_content["pizzaTopping"], json!("mushroom"));
}

#[tokio::test]
async fn test_handler_list_resources_returns_five_resources() {
    let handler = PizzazServerHandler::new();
    let resources = handler.list_widget_resources().await;

    assert_eq!(resources.len(), 5);
    assert!(resources
        .iter()
        .any(|r| r.uri == "ui://widget/pizza-map.html"));
}

#[tokio::test]
async fn test_handler_read_resource_returns_html() {
    let handler = PizzazServerHandler::new();
    let result = handler
        .read_widget_resource("ui://widget/pizza-map.html")
        .await;

    assert!(result.is_ok());
    let content = result.unwrap();
    assert_eq!(content.mime_type, "text/html+skybridge");
    assert!(content.text.contains("pizzaz"));
}

#[tokio::test]
async fn test_handler_list_resource_templates_returns_five_templates() {
    let handler = PizzazServerHandler::new();
    let templates = handler.list_widget_resource_templates().await;

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
                .uri("/mcp")
                .header(header::ACCEPT, ACCEPT_HEADER_VALUE)
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
                .method(Method::POST)
                .uri("/mcp")
                .header(header::ACCEPT, ACCEPT_HEADER_VALUE)
                .header(header::ORIGIN, "https://chatgpt.com")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&build_jsonrpc_request("ping", json!({}), 1)).unwrap(),
                ))
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
                    .method(Method::OPTIONS)
                    .uri("/mcp")
                    .header(header::ACCEPT, ACCEPT_HEADER_VALUE)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}

#[tokio::test]
async fn test_missing_accept_header_returns_406() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/mcp")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&build_jsonrpc_request("tools/list", json!({}), 1)).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_ACCEPTABLE);
}
