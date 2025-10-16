//! Pizzaz MCP Server - Rust implementation
//!
//! This library provides an MCP server that exposes pizza-themed widgets
//! for integration with ChatGPT and other MCP clients.

pub mod handler;
pub mod types;
pub mod widgets;
pub mod widgets_manifest;

#[cfg(test)]
mod test_helpers;

use async_stream::stream;
use axum::{
    extract::ConnectInfo,
    http::{header, HeaderMap, HeaderValue, Request, Response, StatusCode},
    response::IntoResponse,
    routing::{any_service, get, post},
    Extension, Json, Router,
};
use bytes::Bytes;
use futures::{future::BoxFuture, StreamExt};
use http_body::Frame;
use http_body_util::{combinators::BoxBody, BodyExt, Full, StreamBody};
use rmcp::transport::{
    streamable_http_server::session::local::LocalSessionManager, StreamableHttpServerConfig,
    StreamableHttpService,
};
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::HashMap,
    convert::Infallible,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;
use time::{format_description::well_known::Iso8601, OffsetDateTime};
use tokio::sync::Mutex;
use tower::Service;
use tower_http::cors::CorsLayer;

type McpResponse = Response<BoxBody<Bytes, Infallible>>;

#[derive(Clone)]
struct AppState {
    refresh: RefreshState,
}

#[derive(Clone)]
struct RefreshState {
    token: Option<Arc<Vec<u8>>>,
    rate_limiter: Arc<Mutex<RateLimiter>>,
}

impl RefreshState {
    fn from_config(config: &RefreshConfig) -> Self {
        let token = config
            .token
            .as_ref()
            .map(|value| Arc::new(value.as_bytes().to_vec()));

        Self {
            token,
            rate_limiter: Arc::new(Mutex::new(RateLimiter::new(
                config.rate_limit.max_requests,
                config.rate_limit.window,
            ))),
        }
    }

    fn is_enabled(&self) -> bool {
        self.token.is_some()
    }

    fn token_bytes(&self) -> Option<&[u8]> {
        self.token.as_deref().map(|vec| vec.as_slice())
    }
}

struct RateLimiter {
    limit: u64,
    window: Duration,
    buckets: HashMap<IpAddr, RateLimitBucket>,
}

impl RateLimiter {
    fn new(limit: u64, window: Duration) -> Self {
        Self {
            limit,
            window,
            buckets: HashMap::new(),
        }
    }

    fn check(&mut self, ip: IpAddr, now: Instant) -> Result<(), RateLimitRejection> {
        if self.buckets.len() > 1000 {
            self.cleanup_expired(now);
        }

        let entry = self.buckets.entry(ip).or_insert_with(|| RateLimitBucket {
            window_start: now,
            count: 0,
        });

        if now.duration_since(entry.window_start) >= self.window {
            entry.window_start = now;
            entry.count = 0;
        }

        if entry.count < self.limit {
            entry.count += 1;
            return Ok(());
        }

        let elapsed = now.duration_since(entry.window_start);
        let remaining = self
            .window
            .checked_sub(elapsed)
            .unwrap_or_else(|| Duration::from_secs(0));

        Err(RateLimitRejection {
            retry_after: if remaining.is_zero() {
                Duration::from_secs(1)
            } else {
                remaining
            },
        })
    }

    fn cleanup_expired(&mut self, now: Instant) {
        self.buckets
            .retain(|_, bucket| now.duration_since(bucket.window_start) < self.window * 2);
    }
}

struct RateLimitBucket {
    window_start: Instant,
    count: u64,
}

struct RateLimitRejection {
    retry_after: Duration,
}

struct RefreshConfig {
    token: Option<String>,
    rate_limit: RateLimitConfig,
}

struct RateLimitConfig {
    max_requests: u64,
    window: Duration,
}

impl RefreshConfig {
    fn from_env() -> Self {
        let token = std::env::var("WIDGETS_REFRESH_TOKEN")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let rate_limit = parse_rate_limit_config(
            std::env::var("WIDGETS_REFRESH_RATE_LIMIT")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
        );

        Self { token, rate_limit }
    }
}

fn parse_rate_limit_config(raw: Option<String>) -> RateLimitConfig {
    let default = RateLimitConfig {
        max_requests: 10,
        window: Duration::from_secs(60),
    };

    let Some(raw) = raw else {
        return default;
    };

    let (count_str, window_str) =
        match raw.split_once('/') {
            Some(parts) => parts,
            None => {
                tracing::warn!(
                "Invalid WIDGETS_REFRESH_RATE_LIMIT value '{}'; falling back to default {} per {}s",
                raw, default.max_requests, default.window.as_secs()
            );
                return default;
            }
        };

    let max_requests = match count_str.parse::<u64>() {
        Ok(value) if value > 0 => value,
        _ => {
            tracing::warn!(
                "Invalid rate limit count '{}' in '{}'; using default {}",
                count_str,
                raw,
                default.max_requests
            );
            return default;
        }
    };

    let (magnitude_str, unit) = window_str.split_at(window_str.len().saturating_sub(1));
    let magnitude = match magnitude_str.parse::<u64>() {
        Ok(value) if value > 0 => value,
        _ => {
            tracing::warn!(
                "Invalid rate limit window '{}' in '{}'; using default {}s",
                window_str,
                raw,
                default.window.as_secs()
            );
            return default;
        }
    };

    let window = match unit {
        "s" | "S" => Duration::from_secs(magnitude),
        "m" | "M" => Duration::from_secs(magnitude * 60),
        _ => {
            tracing::warn!(
                "Unsupported rate limit unit '{}' in '{}'; using default window {}s",
                unit,
                raw,
                default.window.as_secs()
            );
            return default;
        }
    };

    RateLimitConfig {
        max_requests,
        window,
    }
}

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
    widgets::bootstrap_registry();

    let refresh_config = RefreshConfig::from_env();
    let refresh_state = RefreshState::from_config(&refresh_config);

    if refresh_state.is_enabled() {
        tracing::info!(
            max_requests = refresh_config.rate_limit.max_requests,
            window_seconds = refresh_config.rate_limit.window.as_secs(),
            "Widgets refresh endpoint enabled"
        );
    } else {
        tracing::info!("Widgets refresh endpoint disabled; set WIDGETS_REFRESH_TOKEN to enable");
    }

    let session_manager = Arc::new(LocalSessionManager::default());
    let config = StreamableHttpServerConfig::default();
    // Wrap the core MCP handler with the streamable transport so each request gets its own session.
    let streamable_service = StreamableHttpService::new(
        || Ok(handler::PizzazServerHandler::new()),
        session_manager,
        config,
    );

    // Add a response decorator that ensures widget metadata is present on all outgoing messages.
    let augmented_service = MetaAugmentService::new(streamable_service);

    let app_state = AppState {
        refresh: refresh_state,
    };

    Router::new()
        .route("/mcp", any_service(augmented_service))
        .route("/internal/widgets/refresh", post(refresh_widgets_handler))
        .route("/internal/widgets/status", get(widgets_status_handler))
        .layer(Extension(app_state))
        .layer(CorsLayer::permissive())
}

/// Wraps an MCP HTTP service and injects widget metadata into JSON and SSE responses.
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
    /// Constructs a new service wrapper that augments outgoing MCP messages with widget metadata.
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

    /// Propagates readiness checks to the wrapped service.
    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    /// Calls the wrapped service and conditionally augments JSON or SSE responses with widget metadata.
    fn call(&mut self, request: Request<axum::body::Body>) -> Self::Future {
        let future = self.inner.call(request);
        Box::pin(async move {
            let response = future.await?;
            // Only attempt augmentation if the response advertises a supported content type.
            let Some(kind) = classify_response(&response) else {
                if let Some(content_type) = response
                    .headers()
                    .get(header::CONTENT_TYPE)
                    .and_then(|value| value.to_str().ok())
                {
                    tracing::trace!(
                        "MetaAugmentService: skipping response with unsupported content-type {content_type}"
                    );
                } else {
                    tracing::trace!(
                        "MetaAugmentService: skipping response without content-type header"
                    );
                }
                return Ok(response);
            };

            let (mut parts, body) = response.into_parts();

            match kind {
                ResponseContentType::Json => {
                    let collected = match body.collect().await {
                        Ok(collected) => collected.to_bytes(),
                        Err(_) => Bytes::new(),
                    };

                    parts.headers.remove(header::TRANSFER_ENCODING);
                    parts.headers.remove(header::CONTENT_LENGTH);

                    tracing::trace!("MetaAugmentService: attempting JSON augmentation");
                    let mut json: Value = match serde_json::from_slice(&collected) {
                        Ok(value) => value,
                        Err(_) => {
                            // If the body is not valid JSON we fall back to the original bytes untouched.
                            tracing::debug!(
                                "MetaAugmentService: unable to parse JSON body; bypassing augmentation"
                            );
                            let body = Full::new(collected).boxed();
                            return Ok(Response::from_parts(parts, body));
                        }
                    };

                    augment_widget_metadata(&mut json);

                    let serialized = match serde_json::to_vec(&json) {
                        Ok(bytes) => bytes,
                        Err(err) => {
                            tracing::debug!(
                                "MetaAugmentService: failed to serialize augmented JSON: {err}"
                            );
                            collected.to_vec()
                        }
                    };

                    if let Ok(value) = header::HeaderValue::from_str(&serialized.len().to_string())
                    {
                        parts.headers.insert(header::CONTENT_LENGTH, value);
                    } else {
                        tracing::debug!(
                            "MetaAugmentService: unable to set content-length after JSON augmentation"
                        );
                    }

                    let body = Full::new(Bytes::from(serialized)).boxed();
                    Ok(Response::from_parts(parts, body))
                }
                ResponseContentType::Sse => {
                    parts.headers.remove(header::CONTENT_LENGTH);
                    tracing::trace!("MetaAugmentService: enabling streaming SSE augmentation");

                    let mut data_stream = body.into_data_stream();
                    // Buffer incomplete SSE events so we can rewrite each event atomically once its full content arrives.
                    let stream = stream! {
                        let mut buffer = String::new();
                        while let Some(chunk_result) = data_stream.next().await {
                            let chunk = match chunk_result {
                                Ok(chunk) => chunk,
                                Err(err) => {
                                    tracing::debug!("MetaAugmentService: error reading SSE chunk: {err}");
                                    continue;
                                }
                            };

                            let normalized_chunk = match std::str::from_utf8(&chunk) {
                                Ok(text) => text.replace("\r\n", "\n"),
                                Err(_) => {
                                    tracing::debug!("MetaAugmentService: encountered non UTF-8 SSE chunk; flushing buffer");
                                    if !buffer.is_empty() {
                                        let leftover = std::mem::take(&mut buffer);
                                        yield Ok::<Frame<Bytes>, Infallible>(Frame::data(Bytes::from(leftover)));
                                    }
                                    yield Ok::<Frame<Bytes>, Infallible>(Frame::data(chunk));
                                    continue;
                                }
                            };

                            buffer.push_str(&normalized_chunk);

                            while let Some(event) = drain_complete_event(&mut buffer) {
                                let (frame, event_changed) = frame_from_event(event);
                                if event_changed {
                                    tracing::trace!("MetaAugmentService: augmented SSE event");
                                }
                                yield Ok::<Frame<Bytes>, Infallible>(frame);
                            }
                        }

                        if !buffer.is_empty() {
                            let (frame, event_changed) = frame_from_event(std::mem::take(&mut buffer));
                            if event_changed {
                                tracing::trace!("MetaAugmentService: augmented trailing SSE event");
                            }
                            yield Ok::<Frame<Bytes>, Infallible>(frame);
                        }
                    };

                    let response_body = http_body_util::BodyExt::boxed(StreamBody::new(stream));
                    Ok(Response::from_parts(parts, response_body))
                }
            }
        })
    }
}

async fn refresh_widgets_handler(
    Extension(state): Extension<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !state.refresh.is_enabled() {
        return StatusCode::NOT_FOUND.into_response();
    }

    let Some(expected) = state.refresh.token_bytes() else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let Some(provided) = extract_bearer_token(&headers) else {
        return unauthorized_response("Missing or invalid bearer token");
    };

    if expected.len() != provided.as_bytes().len()
        || expected.ct_eq(provided.as_bytes()).unwrap_u8() == 0
    {
        tracing::warn!(ip = %addr.ip(), "Invalid widgets refresh token provided");
        return unauthorized_response("Missing or invalid bearer token");
    }

    let ip = addr.ip();
    let now = Instant::now();
    let mut limiter = state.refresh.rate_limiter.lock().await;
    if let Err(rejection) = limiter.check(ip, now) {
        drop(limiter);
        let retry_seconds = rejection.retry_after.as_secs().max(1);
        tracing::warn!(ip = %ip, retry_after = retry_seconds, "Widgets refresh rate limit exceeded");

        let metadata = widgets::registry_metadata();
        let response = RefreshResponse {
            success: false,
            widgets_loaded: widgets::get_all_widgets().len(),
            schema_version: metadata.schema_version.clone(),
            manifest_timestamp: format_optional_timestamp(metadata.manifest_generated_at),
            message: Some(format!(
                "Rate limit exceeded. Retry after {} seconds.",
                retry_seconds
            )),
        };

        let mut http_response = build_refresh_response(StatusCode::TOO_MANY_REQUESTS, response);
        if let Ok(value) = HeaderValue::from_str(&retry_seconds.to_string()) {
            http_response
                .headers_mut()
                .insert(header::RETRY_AFTER, value);
        }
        return http_response;
    }
    drop(limiter);

    match widgets::reload_registry() {
        Ok(outcome) => {
            let response = RefreshResponse {
                success: true,
                widgets_loaded: outcome.widget_count,
                schema_version: outcome.schema_version,
                manifest_timestamp: format_optional_timestamp(outcome.manifest_timestamp),
                message: None,
            };
            build_refresh_response(StatusCode::OK, response)
        }
        Err(widgets::LoadError::NotFound { path }) => {
            let metadata = widgets::registry_metadata();
            let message = if !metadata.registry_initialized {
                "Manifest has never been successfully loaded".to_string()
            } else {
                format!("Manifest not found at {}", path.display())
            };
            tracing::warn!(manifest = %path.display(), "{}", message);

            let response = RefreshResponse {
                success: false,
                widgets_loaded: widgets::get_all_widgets().len(),
                schema_version: metadata.schema_version.clone(),
                manifest_timestamp: format_optional_timestamp(metadata.manifest_generated_at),
                message: Some(message),
            };
            build_refresh_response(StatusCode::SERVICE_UNAVAILABLE, response)
        }
        Err(widgets::LoadError::Validation { path, error }) => {
            tracing::error!(
                manifest = %path.display(),
                error = %error,
                "Widget manifest refresh failed"
            );
            let metadata = widgets::registry_metadata();
            let response = RefreshResponse {
                success: false,
                widgets_loaded: widgets::get_all_widgets().len(),
                schema_version: metadata.schema_version.clone(),
                manifest_timestamp: format_optional_timestamp(metadata.manifest_generated_at),
                message: Some(error.to_string()),
            };
            build_refresh_response(StatusCode::BAD_REQUEST, response)
        }
    }
}

#[derive(Serialize)]
struct RefreshResponse {
    success: bool,
    widgets_loaded: usize,
    schema_version: Option<String>,
    manifest_timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Serialize)]
struct StatusResponse {
    registry_initialized: bool,
    widgets_count: usize,
    schema_version: Option<String>,
    last_successful_load: Option<String>,
    manifest_path: String,
    manifest_exists: bool,
}

async fn widgets_status_handler(Extension(_state): Extension<AppState>) -> impl IntoResponse {
    let metadata = widgets::registry_metadata();
    let response = StatusResponse {
        registry_initialized: metadata.registry_initialized,
        widgets_count: widgets::get_all_widgets().len(),
        schema_version: metadata.schema_version.clone(),
        last_successful_load: format_optional_timestamp(metadata.last_successful_load),
        manifest_path: metadata.manifest_path.display().to_string(),
        manifest_exists: metadata.manifest_exists,
    };

    Json(response)
}

fn unauthorized_response(message: &str) -> axum::response::Response {
    let metadata = widgets::registry_metadata();
    let payload = RefreshResponse {
        success: false,
        widgets_loaded: widgets::get_all_widgets().len(),
        schema_version: metadata.schema_version.clone(),
        manifest_timestamp: format_optional_timestamp(metadata.manifest_generated_at),
        message: Some(message.to_string()),
    };
    let mut response = build_refresh_response(StatusCode::UNAUTHORIZED, payload);
    response.headers_mut().insert(
        header::WWW_AUTHENTICATE,
        HeaderValue::from_static("Bearer realm=\"widgets-refresh\""),
    );
    response
}

fn build_refresh_response(
    status: StatusCode,
    payload: RefreshResponse,
) -> axum::response::Response {
    let mut response = Json(payload).into_response();
    *response.status_mut() = status;
    response
}

fn format_optional_timestamp(value: Option<OffsetDateTime>) -> Option<String> {
    value.and_then(|timestamp| timestamp.format(&Iso8601::DEFAULT).ok())
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(header::AUTHORIZATION)?;
    let value = value.to_str().ok()?.trim();
    let mut parts = value.splitn(2, ' ');
    let scheme = parts.next()?.to_ascii_lowercase();
    if scheme != "bearer" {
        return None;
    }
    let token = parts.next()?.trim();
    if token.is_empty() {
        return None;
    }
    Some(token)
}

/// Identifies whether a response body is JSON or server-sent events based on the `Content-Type` header.
fn classify_response(response: &McpResponse) -> Option<ResponseContentType> {
    response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|mime| {
            if mime.contains("application/json") {
                Some(ResponseContentType::Json)
            } else if mime.contains("text/event-stream") {
                Some(ResponseContentType::Sse)
            } else {
                None
            }
        })
}

#[derive(Debug, Clone, Copy)]
enum ResponseContentType {
    Json,
    Sse,
}

/// Injects `_meta` entries for known widgets into tools, resources, and templates within the MCP payload.
fn augment_widget_metadata(payload: &mut Value) {
    let Some(result) = payload.get_mut("result") else {
        tracing::trace!("augment_widget_metadata: no result field present");
        return;
    };

    // Attach widget metadata to any tool definitions returned by the MCP handler.
    if let Some(tools) = result.get_mut("tools").and_then(Value::as_array_mut) {
        for tool in tools {
            if let Some(object) = tool.as_object_mut() {
                if let Some(name) = object.get("name").and_then(Value::as_str) {
                    if let Some(widget) = crate::widgets::get_widget_by_id(name) {
                        tracing::trace!(
                            "augment_widget_metadata: injecting metadata for tool '{name}'"
                        );
                        object
                            .entry("_meta".to_string())
                            .or_insert_with(|| serde_json::Value::Object(widget.meta().0));
                    } else {
                        tracing::trace!(
                            "augment_widget_metadata: tool '{name}' not found in registry"
                        );
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
                        tracing::trace!(
                            "augment_widget_metadata: injecting metadata for resource '{uri}'"
                        );
                        object
                            .entry("_meta".to_string())
                            .or_insert_with(|| serde_json::Value::Object(widget.meta().0));
                    } else {
                        tracing::trace!(
                            "augment_widget_metadata: resource '{uri}' not found in registry"
                        );
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
                        // Template URIs mirror resource URIs, so reuse the same metadata payload.
                        tracing::trace!(
                            "augment_widget_metadata: injecting metadata for template '{uri}'"
                        );
                        object
                            .entry("_meta".to_string())
                            .or_insert_with(|| serde_json::Value::Object(widget.meta().0));
                    } else {
                        tracing::trace!(
                            "augment_widget_metadata: template '{uri}' not found in registry"
                        );
                    }
                }
            }
        }
    }
}

/// Removes and returns the next complete SSE event (terminated by a blank line) from the buffer.
fn drain_complete_event(buffer: &mut String) -> Option<String> {
    let boundary = buffer.find("\n\n")?;
    let mut extracted: String = buffer.drain(..boundary + 2).collect();
    if extracted.ends_with("\n\n") {
        extracted.truncate(extracted.len() - 2);
    }
    Some(extracted)
}

/// Converts an SSE event payload into a `Frame`, augmenting metadata and normalising terminators.
fn frame_from_event(event: String) -> (Frame<Bytes>, bool) {
    let (mut processed, event_changed) = augment_sse_event(&event);
    if !processed.ends_with("\n\n") {
        processed.push_str("\n\n");
    }
    (Frame::data(Bytes::from(processed)), event_changed)
}

#[cfg_attr(not(test), allow(dead_code))]
/// Attempts to augment every SSE event in the provided stream, returning `None` when no changes occur.
fn augment_sse_stream(original: &str) -> Option<String> {
    let normalized = original.replace("\r\n", "\n");
    let mut changed_any = false;
    let mut output = String::with_capacity(normalized.len());

    // Walk each SSE event (terminated by a blank line) and try to inject widget metadata.
    for segment in normalized.split_inclusive("\n\n") {
        let (event_body, separator) = if segment.ends_with("\n\n") {
            (&segment[..segment.len() - 2], "\n\n")
        } else {
            (segment, "")
        };

        let (processed_event, event_changed) = augment_sse_event(event_body);
        if event_changed {
            tracing::trace!("augment_sse_stream: augmented SSE event detected");
            changed_any = true;
        }

        output.push_str(&processed_event);
        output.push_str(separator);
    }

    if changed_any {
        Some(output)
    } else {
        tracing::trace!("augment_sse_stream: no SSE events modified");
        None
    }
}

/// Augments a single SSE event in-place, returning the rewritten payload and whether it changed.
fn augment_sse_event(event: &str) -> (String, bool) {
    if event.is_empty() {
        return (String::new(), false);
    }

    // Track whether any `data:` lines were rewritten so callers can decide whether to flush the event.
    let mut event_changed = false;
    let mut lines_out = Vec::new();

    for line in event.split('\n') {
        if let Some(rest) = line.strip_prefix("data:") {
            let trimmed = rest.trim_start();
            if trimmed.is_empty() {
                lines_out.push(line.to_string());
                continue;
            }

            if let Ok(mut json_value) = serde_json::from_str::<Value>(trimmed) {
                let original_value = json_value.clone();
                augment_widget_metadata(&mut json_value);
                if json_value != original_value {
                    tracing::trace!("augment_sse_event: modified JSON data line");
                    event_changed = true;
                }

                if let Ok(serialized) = serde_json::to_string(&json_value) {
                    let prefix = &rest[..rest.len() - trimmed.len()];
                    lines_out.push(format!("data:{}{}", prefix, serialized));
                    continue;
                }
            } else {
                tracing::trace!(
                    "augment_sse_event: skipping non-JSON data line '{}'",
                    trimmed
                );
            }
        }

        lines_out.push(line.to_string());
    }

    (lines_out.join("\n"), event_changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::initialize_widgets_for_tests;

    /// Ensures widget metadata augmentation decorates known tools and leaves unknown ones unchanged.
    #[test]
    fn augment_widget_metadata_inserts_tool_meta() {
        initialize_widgets_for_tests();
        let mut payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "tools": [
                    {"name": "pizza-map"},
                    {"name": "pizza-carousel"},
                    {"name": "unknown-tool"}
                ],
                "resources": [
                    {"uri": "ui://widget/pizza-map.html"},
                    {"uri": "ui://widget/unknown.html"}
                ],
                "resourceTemplates": [
                    {"uriTemplate": "ui://widget/pizza-map.html"},
                    {"uriTemplate": "ui://widget/unknown.html"}
                ]
            }
        });

        augment_widget_metadata(&mut payload);

        for key in ["tools", "resources", "resourceTemplates"] {
            let entries = payload["result"][key]
                .as_array()
                .expect("section should remain an array");

            let (field, known_value, unknown_value) = match key {
                "tools" => ("name", "pizza-map", "unknown-tool"),
                "resources" => (
                    "uri",
                    "ui://widget/pizza-map.html",
                    "ui://widget/unknown.html",
                ),
                "resourceTemplates" => (
                    "uriTemplate",
                    "ui://widget/pizza-map.html",
                    "ui://widget/unknown.html",
                ),
                _ => unreachable!(),
            };

            let known_entry = entries
                .iter()
                .find(|entry| entry.get(field).and_then(Value::as_str) == Some(known_value))
                .expect("expected known entry");
            assert_eq!(
                known_entry["_meta"]["openai/outputTemplate"],
                "ui://widget/pizza-map.html"
            );
            assert!(
                known_entry["_meta"]["openai/widgetAccessible"]
                    .as_bool()
                    .unwrap(),
                "meta should include openai/widgetAccessible"
            );

            let unknown_entry = entries
                .iter()
                .find(|entry| entry.get(field).and_then(Value::as_str) == Some(unknown_value))
                .expect("expected unknown entry");
            assert!(
                unknown_entry.get("_meta").is_none(),
                "Unexpected meta attached to unknown entry"
            );
        }
    }

    /// Validates that SSE payloads carrying JSON tool results receive injected widget metadata.
    #[test]
    fn augment_sse_stream_injects_meta() {
        initialize_widgets_for_tests();
        let original = concat!(
            "event: message\r\n",
            "data: {\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[{\"name\":\"pizza-map\"}]}}\r\n",
            "\r\n"
        );

        let augmented = augment_sse_stream(original).expect("stream should be augmented");
        assert!(
            augmented.contains("\"_meta\""),
            "Augmented stream must contain _meta"
        );
        assert!(
            augmented.contains("\"openai/outputTemplate\""),
            "Widget metadata should be injected"
        );
    }

    /// Confirms that non-JSON SSE messages pass through without modification.
    #[test]
    fn augment_sse_stream_preserves_non_json_data() {
        let original = concat!(": heartbeat\n", "data: ping\n", "\n");
        assert!(
            augment_sse_stream(original).is_none(),
            "Non-JSON SSE payloads should remain untouched"
        );
    }
}
