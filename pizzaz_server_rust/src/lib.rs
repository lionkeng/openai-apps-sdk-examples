//! Pizzaz MCP Server - Rust implementation
//!
//! This library provides an MCP server that exposes pizza-themed widgets
//! for integration with ChatGPT and other MCP clients.

pub mod handler;
pub mod types;
pub mod widgets;

#[cfg(test)]
mod test_helpers;

use async_stream::stream;
use axum::{
    http::{header, Request, Response},
    routing::any_service,
    Router,
};
use bytes::Bytes;
use futures::{future::BoxFuture, StreamExt};
use http_body::Frame;
use http_body_util::{combinators::BoxBody, BodyExt, Full, StreamBody};
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
    // Wrap the core MCP handler with the streamable transport so each request gets its own session.
    let streamable_service = StreamableHttpService::new(
        || Ok(handler::PizzazServerHandler::new()),
        session_manager,
        config,
    );

    // Add a response decorator that ensures widget metadata is present on all outgoing messages.
    let augmented_service = MetaAugmentService::new(streamable_service);

    Router::new()
        .route("/mcp", any_service(augmented_service))
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
                            .or_insert_with(|| widget.meta());
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
                            .or_insert_with(|| widget.meta());
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
                            .or_insert_with(|| widget.meta());
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

    /// Ensures widget metadata augmentation decorates known tools and leaves unknown ones unchanged.
    #[test]
    fn augment_widget_metadata_inserts_tool_meta() {
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
