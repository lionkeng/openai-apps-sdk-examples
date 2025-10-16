//! Integration tests for pizzaz_server_rust
//!
//! These tests verify the full HTTP request/response cycle using tower::ServiceExt.

use axum::{
    body::Body,
    extract::connect_info::ConnectInfo as AxumConnectInfo,
    http::{header, Method, Request, StatusCode},
};
use http_body_util::BodyExt;
use pizzaz_server_rust::handler::PizzazServerHandler;
use serde_json::{json, Value};
use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Once, OnceLock},
};
use tokio::sync::Mutex as AsyncMutex;
use tower::ServiceExt; // for oneshot()

fn ensure_manifest_loaded() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/widgets.json");
        std::env::set_var("WIDGETS_MANIFEST_PATH", &path);
        std::env::set_var("WIDGETS_REFRESH_TOKEN", "test-refresh-token");
        pizzaz_server_rust::widgets::bootstrap_registry();
    });
}

const ACCEPT_HEADER_VALUE: &str = "application/json, text/event-stream";

/// Helper to create test app
fn create_test_app() -> axum::Router {
    ensure_manifest_loaded();
    pizzaz_server_rust::create_app()
}

fn make_handler() -> PizzazServerHandler {
    ensure_manifest_loaded();
    PizzazServerHandler::new()
}

fn add_connect_info(mut request: Request<Body>, port: u16) -> Request<Body> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    request.extensions_mut().insert(AxumConnectInfo(addr));
    request
}

async fn env_lock() -> tokio::sync::MutexGuard<'static, ()> {
    static ENV_MUTEX: OnceLock<AsyncMutex<()>> = OnceLock::new();
    ENV_MUTEX.get_or_init(|| AsyncMutex::new(())).lock().await
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
    let handler = make_handler();
    let tools = handler.list_widget_tools().await;

    assert_eq!(tools.len(), 5);
    assert!(tools.iter().any(|t| t.name == "pizza-map"));
    assert!(tools.iter().any(|t| t.name == "pizza-carousel"));
}

#[tokio::test]
async fn test_handler_call_tool_returns_structured_content() {
    let handler = make_handler();
    let args = json!({ "pizzaTopping": "mushroom" });

    let result = handler
        .call_widget_tool("pizza-map", args)
        .await
        .expect("tool call succeeds");
    assert_eq!(result.structured_content["pizzaTopping"], json!("mushroom"));
}

#[tokio::test]
async fn test_handler_list_resources_returns_five_resources() {
    let handler = make_handler();
    let resources = handler.list_widget_resources().await;

    assert_eq!(resources.len(), 5);
    assert!(resources
        .iter()
        .any(|r| r.uri == "ui://widget/pizza-map.html"));
}

#[tokio::test]
async fn test_handler_read_resource_returns_html() {
    let handler = make_handler();
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
    let handler = make_handler();
    let templates = handler.list_widget_resource_templates().await;

    assert_eq!(templates.len(), 5);
    assert!(templates
        .iter()
        .any(|t| t.uri_template == "ui://widget/pizza-map.html"));
}

#[tokio::test]
async fn test_widgets_status_endpoint_returns_metadata() {
    let app = create_test_app();
    let request = add_connect_info(
        Request::builder()
            .method(Method::GET)
            .uri("/internal/widgets/status")
            .body(Body::empty())
            .unwrap(),
        4100,
    );

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = parse_response_body(response).await.unwrap();
    assert_eq!(body["registry_initialized"], json!(true));
    assert_eq!(body["widgets_count"], json!(5));
    assert_eq!(body["schema_version"], json!("1.0.0"));
    assert!(body["last_successful_load"].is_string());

    let manifest_path = body["manifest_path"].as_str().unwrap_or_default();
    assert!(
        manifest_path.ends_with("tests/fixtures/widgets.json"),
        "manifest_path should point to fixture, got {}",
        manifest_path
    );
    assert_eq!(body["manifest_exists"], json!(true));
}

#[tokio::test]
async fn test_refresh_endpoint_requires_token() {
    let app = create_test_app();
    let request = add_connect_info(
        Request::builder()
            .method(Method::POST)
            .uri("/internal/widgets/refresh")
            .body(Body::empty())
            .unwrap(),
        4200,
    );

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body = parse_response_body(response).await.unwrap();
    assert_eq!(body["success"], json!(false));
}

#[tokio::test]
async fn test_refresh_endpoint_succeeds_with_valid_token() {
    let _env_guard = env_lock().await;
    std::env::set_var("WIDGETS_REFRESH_RATE_LIMIT", "10/60s");
    let app = create_test_app();
    let request = add_connect_info(
        Request::builder()
            .method(Method::POST)
            .uri("/internal/widgets/refresh")
            .header(header::AUTHORIZATION, "Bearer test-refresh-token")
            .body(Body::empty())
            .unwrap(),
        4300,
    );

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = parse_response_body(response).await.unwrap();
    assert_eq!(body["success"], json!(true));
    assert_eq!(body["widgets_loaded"], json!(5));
    assert_eq!(body["schema_version"], json!("1.0.0"));
}

#[tokio::test]
async fn test_refresh_endpoint_rate_limit() {
    let _env_guard = env_lock().await;
    std::env::set_var("WIDGETS_REFRESH_RATE_LIMIT", "1/60s");
    let app = create_test_app();

    let request1 = add_connect_info(
        Request::builder()
            .method(Method::POST)
            .uri("/internal/widgets/refresh")
            .header(header::AUTHORIZATION, "Bearer test-refresh-token")
            .body(Body::empty())
            .unwrap(),
        4400,
    );

    let response1 = app.clone().oneshot(request1).await.unwrap();
    assert_eq!(response1.status(), StatusCode::OK);

    let request2 = add_connect_info(
        Request::builder()
            .method(Method::POST)
            .uri("/internal/widgets/refresh")
            .header(header::AUTHORIZATION, "Bearer test-refresh-token")
            .body(Body::empty())
            .unwrap(),
        4400,
    );

    let response2 = app.clone().oneshot(request2).await.unwrap();
    assert_eq!(response2.status(), StatusCode::TOO_MANY_REQUESTS);

    let retry_after = response2.headers().get(header::RETRY_AFTER);
    assert!(retry_after.is_some());

    let body = parse_response_body(response2).await.unwrap();
    assert_eq!(body["success"], json!(false));
    assert!(
        body["message"]
            .as_str()
            .unwrap_or_default()
            .contains("Rate limit exceeded"),
        "Expected rate limit message, got {:?}",
        body["message"]
    );

    // Reset to default for other tests
    std::env::set_var("WIDGETS_REFRESH_RATE_LIMIT", "10/60s");
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
