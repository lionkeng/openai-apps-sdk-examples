//! MCP server handler for Pizzaz widgets

use crate::{types::ToolInput, widgets};
use anyhow::{Context, Result};
use rmcp::{
    handler::server::ServerHandler,
    model::{
        self, AnnotateAble, CallToolRequestParam, CallToolResult as McpCallToolResult, Content,
        ErrorData, Implementation, InitializeRequestParam, InitializeResult,
        ListResourceTemplatesResult, ListResourcesResult, ListToolsResult, Meta,
        PaginatedRequestParam, ProtocolVersion, RawResource, RawResourceTemplate, ResourceContents,
        ResourcesCapability, ServerCapabilities, Tool as McpTool, ToolsCapability,
    },
    service::{NotificationContext, RequestContext, RoleServer},
};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::{future::Future, sync::Arc};

/// High-level tool information for tests and internal conversion.
#[derive(Debug, Clone)]
pub struct WidgetTool {
    pub name: String,
    pub title: String,
    pub description: String,
    pub input_schema: JsonValue,
    pub meta: JsonValue,
}

/// Result of invoking a widget tool.
#[derive(Debug, Clone)]
pub struct WidgetCallResult {
    pub content: Vec<Content>,
    pub structured_content: JsonValue,
    pub meta: JsonValue,
}

/// Represents a widget resource entry.
#[derive(Debug, Clone)]
pub struct WidgetResource {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: String,
    pub meta: JsonValue,
}

/// HTML content returned when reading a widget resource.
#[derive(Debug, Clone)]
pub struct WidgetResourceContent {
    pub uri: String,
    pub mime_type: String,
    pub text: String,
    pub meta: JsonValue,
}

/// Resource template definition for widgets.
#[derive(Debug, Clone)]
pub struct WidgetResourceTemplate {
    pub uri_template: String,
    pub name: String,
    pub description: String,
    pub mime_type: String,
    pub meta: JsonValue,
}

/// MCP server handler for Pizzaz widgets.
#[derive(Debug, Clone, Default)]
pub struct PizzazServerHandler;

impl PizzazServerHandler {
    /// Creates a new handler instance.
    pub fn new() -> Self {
        Self
    }

    /// Lists all widget tools for internal use.
    pub async fn list_widget_tools(&self) -> Vec<WidgetTool> {
        widgets::get_all_widgets()
            .iter()
            .map(|widget| WidgetTool {
                name: widget.id.clone(),
                title: widget.title.clone(),
                description: widget.title.clone(),
                input_schema: build_tool_input_schema(),
                meta: widget.meta(),
            })
            .collect()
    }

    /// Calls a widget tool with structured arguments.
    pub async fn call_widget_tool(
        &self,
        name: &str,
        arguments: JsonValue,
    ) -> Result<WidgetCallResult> {
        let widget =
            widgets::get_widget_by_id(name).with_context(|| format!("Unknown tool: {name}"))?;

        let input: ToolInput =
            serde_json::from_value(arguments).context("Invalid tool arguments")?;

        let content = Content::text(widget.response_text.clone());
        let mut structured = JsonMap::new();
        structured.insert(
            "pizzaTopping".to_string(),
            JsonValue::String(input.pizza_topping),
        );

        Ok(WidgetCallResult {
            content: vec![content],
            structured_content: JsonValue::Object(structured),
            meta: widget.meta(),
        })
    }

    /// Lists widget resources for internal use.
    pub async fn list_widget_resources(&self) -> Vec<WidgetResource> {
        widgets::get_all_widgets()
            .iter()
            .map(|widget| WidgetResource {
                uri: widget.template_uri.clone(),
                name: widget.title.clone(),
                description: format!("{} widget markup", widget.title),
                mime_type: HTML_WIDGET_MIME.to_string(),
                meta: widget.meta(),
            })
            .collect()
    }

    /// Reads the content for a specific widget resource.
    pub async fn read_widget_resource(&self, uri: &str) -> Result<WidgetResourceContent> {
        let widget =
            widgets::get_widget_by_uri(uri).with_context(|| format!("Unknown resource: {uri}"))?;

        Ok(WidgetResourceContent {
            uri: widget.template_uri.clone(),
            mime_type: HTML_WIDGET_MIME.to_string(),
            text: widget.html.clone(),
            meta: widget.meta(),
        })
    }

    /// Lists all widget resource templates.
    pub async fn list_widget_resource_templates(&self) -> Vec<WidgetResourceTemplate> {
        widgets::get_all_widgets()
            .iter()
            .map(|widget| WidgetResourceTemplate {
                uri_template: widget.template_uri.clone(),
                name: widget.title.clone(),
                description: format!("{} widget markup", widget.title),
                mime_type: HTML_WIDGET_MIME.to_string(),
                meta: widget.meta(),
            })
            .collect()
    }
}

const HTML_WIDGET_MIME: &str = "text/html+skybridge";

fn build_tool_input_schema() -> JsonValue {
    serde_json::json!({
        "type": "object",
        "properties": {
            "pizzaTopping": {
                "type": "string",
                "description": "Topping to mention when rendering the widget."
            }
        },
        "required": ["pizzaTopping"],
        "additionalProperties": false
    })
}

fn value_to_meta(value: JsonValue) -> Meta {
    match value {
        JsonValue::Object(map) => Meta(map),
        _ => Meta::default(),
    }
}

fn value_to_map(value: &JsonValue) -> JsonMap<String, JsonValue> {
    value.as_object().cloned().unwrap_or_default()
}

fn widget_call_result_to_mcp(result: WidgetCallResult) -> McpCallToolResult {
    McpCallToolResult {
        content: result.content,
        structured_content: Some(result.structured_content),
        is_error: Some(false),
        meta: Some(value_to_meta(result.meta)),
    }
}

fn widget_resource_content_to_mcp(content: WidgetResourceContent) -> ResourceContents {
    ResourceContents::TextResourceContents {
        uri: content.uri,
        mime_type: Some(content.mime_type),
        text: content.text,
        meta: Some(value_to_meta(content.meta)),
    }
}

fn widget_tool_to_mcp(tool: WidgetTool) -> McpTool {
    let mut mcp_tool = McpTool::new(tool.name.clone(), tool.description.clone(), {
        let map = value_to_map(&tool.input_schema);
        Arc::new(map)
    });
    mcp_tool.title = Some(tool.title);
    // Metadata is injected later by the HTTP augmentation layer.
    mcp_tool
}

fn widget_resource_to_mcp(resource: WidgetResource) -> model::Resource {
    RawResource {
        uri: resource.uri,
        name: resource.name.clone(),
        title: Some(resource.name),
        description: Some(resource.description),
        mime_type: Some(resource.mime_type),
        size: None,
        icons: None,
    }
    .no_annotation()
}

fn widget_template_to_mcp(template: WidgetResourceTemplate) -> model::ResourceTemplate {
    RawResourceTemplate {
        uri_template: template.uri_template,
        name: template.name,
        title: None,
        description: Some(template.description),
        mime_type: Some(template.mime_type),
    }
    .no_annotation()
}

impl ServerHandler for PizzazServerHandler {
    fn ping(
        &self,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<(), ErrorData>> + Send + '_ {
        async move { Ok(()) }
    }

    fn initialize(
        &self,
        _request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<InitializeResult, ErrorData>> + Send + '_ {
        async move {
            let capabilities = ServerCapabilities::builder()
                .enable_tools_with(ToolsCapability {
                    list_changed: Some(false),
                })
                .enable_resources_with(ResourcesCapability {
                    subscribe: Some(false),
                    list_changed: Some(false),
                })
                .build();

            Ok(InitializeResult {
                protocol_version: ProtocolVersion::V_2024_11_05,
                capabilities,
                server_info: Implementation {
                    name: "pizzaz-rust".to_string(),
                    title: Some("Pizzaz MCP Server (Rust)".to_string()),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    icons: None,
                    website_url: None,
                },
                instructions: Some(
                    "Use the pizza-themed tools to render widgets in ChatGPT.".to_string(),
                ),
            })
        }
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, ErrorData>> + Send + '_ {
        async move {
            let tools = self
                .list_widget_tools()
                .await
                .into_iter()
                .map(widget_tool_to_mcp)
                .collect();

            Ok(ListToolsResult {
                tools,
                next_cursor: None,
            })
        }
    }

    fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<McpCallToolResult, ErrorData>> + Send + '_ {
        async move {
            let result = self
                .call_widget_tool(
                    &request.name,
                    request
                        .arguments
                        .map(JsonValue::Object)
                        .unwrap_or_else(|| JsonValue::Object(JsonMap::new())),
                )
                .await
                .map_err(|err| ErrorData::invalid_params(err.to_string(), None))?;

            Ok(widget_call_result_to_mcp(result))
        }
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListResourcesResult, ErrorData>> + Send + '_ {
        async move {
            let resources = self
                .list_widget_resources()
                .await
                .into_iter()
                .map(widget_resource_to_mcp)
                .collect();

            Ok(ListResourcesResult {
                resources,
                next_cursor: None,
            })
        }
    }

    fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListResourceTemplatesResult, ErrorData>> + Send + '_ {
        async move {
            let resource_templates = self
                .list_widget_resource_templates()
                .await
                .into_iter()
                .map(widget_template_to_mcp)
                .collect();

            Ok(ListResourceTemplatesResult {
                resource_templates,
                next_cursor: None,
            })
        }
    }

    fn read_resource(
        &self,
        request: model::ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<model::ReadResourceResult, ErrorData>> + Send + '_ {
        async move {
            let content = self
                .read_widget_resource(&request.uri)
                .await
                .map_err(|err| ErrorData::invalid_params(err.to_string(), None))?;

            Ok(model::ReadResourceResult {
                contents: vec![widget_resource_content_to_mcp(content)],
            })
        }
    }

    fn get_prompt(
        &self,
        _request: model::GetPromptRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<model::GetPromptResult, ErrorData>> + Send + '_ {
        async move { Err(ErrorData::method_not_found::<model::GetPromptRequestMethod>()) }
    }

    fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<model::ListPromptsResult, ErrorData>> + Send + '_ {
        async move { Err(ErrorData::method_not_found::<model::ListPromptsRequestMethod>()) }
    }

    fn complete(
        &self,
        _request: model::CompleteRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<model::CompleteResult, ErrorData>> + Send + '_ {
        async move { Err(ErrorData::method_not_found::<model::CompleteRequestMethod>()) }
    }

    fn set_level(
        &self,
        _request: model::SetLevelRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<(), ErrorData>> + Send + '_ {
        async move { Ok(()) }
    }

    fn subscribe(
        &self,
        _request: model::SubscribeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<(), ErrorData>> + Send + '_ {
        async move { Err(ErrorData::method_not_found::<model::SubscribeRequestMethod>()) }
    }

    fn unsubscribe(
        &self,
        _request: model::UnsubscribeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<(), ErrorData>> + Send + '_ {
        async move { Err(ErrorData::method_not_found::<model::UnsubscribeRequestMethod>()) }
    }

    fn on_cancelled(
        &self,
        _notification: model::CancelledNotificationParam,
        _context: NotificationContext<RoleServer>,
    ) -> impl Future<Output = ()> + Send + '_ {
        async move {}
    }

    fn on_progress(
        &self,
        _notification: model::ProgressNotificationParam,
        _context: NotificationContext<RoleServer>,
    ) -> impl Future<Output = ()> + Send + '_ {
        async move {}
    }

    fn on_initialized(
        &self,
        _context: NotificationContext<RoleServer>,
    ) -> impl Future<Output = ()> + Send + '_ {
        async move {}
    }

    fn on_roots_list_changed(
        &self,
        _context: NotificationContext<RoleServer>,
    ) -> impl Future<Output = ()> + Send + '_ {
        async move {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_widget_tools_contains_expected_entries() {
        let handler = PizzazServerHandler::new();
        let tools = handler.list_widget_tools().await;
        let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_str()).collect();
        assert_eq!(tools.len(), 5);
        assert!(names.contains(&"pizza-map"));
        assert!(names.contains(&"pizza-carousel"));
        assert!(names.contains(&"pizza-albums"));
        assert!(names.contains(&"pizza-list"));
        assert!(names.contains(&"pizza-video"));
        for tool in tools {
            let meta = tool
                .meta
                .as_object()
                .expect("tool meta should be an object");
            assert!(meta.contains_key("openai/outputTemplate"));
            assert_eq!(meta["openai/widgetAccessible"], JsonValue::Bool(true));
        }
    }

    #[tokio::test]
    async fn test_call_widget_tool_includes_structured_content() {
        let handler = PizzazServerHandler::new();
        let result = handler
            .call_widget_tool("pizza-map", serde_json::json!({"pizzaTopping": "mushroom"}))
            .await
            .expect("tool call should succeed");

        assert_eq!(
            result.structured_content["pizzaTopping"],
            JsonValue::String("mushroom".into())
        );
        assert_eq!(result.content.len(), 1);
        let content = &result.content[0];
        match content.raw {
            model::RawContent::Text(ref text) => {
                assert!(text.text.contains("Rendered"));
            }
            _ => panic!("Expected text content"),
        }
        let meta = result.meta.as_object().expect("meta should be present");
        assert_eq!(meta["openai/widgetAccessible"], JsonValue::Bool(true));
        assert!(meta["openai/outputTemplate"].is_string());
    }

    #[tokio::test]
    async fn test_call_tool_result_serialization_includes_meta() {
        let handler = PizzazServerHandler::new();
        let result = handler
            .call_widget_tool("pizza-map", serde_json::json!({"pizzaTopping": "olives"}))
            .await
            .expect("tool call should succeed");

        let serialized = serde_json::to_value(widget_call_result_to_mcp(result))
            .expect("serialization succeeds");
        let meta = serialized
            .get("_meta")
            .expect("meta should be present")
            .as_object()
            .expect("meta should be an object");
        assert_eq!(
            meta.get("openai/outputTemplate")
                .expect("output template present"),
            "ui://widget/pizza-map.html"
        );
        assert_eq!(
            meta.get("openai/toolInvocation/invoked")
                .expect("invoked message present"),
            "Served a fresh map"
        );
    }

    #[tokio::test]
    async fn test_list_widget_resources() {
        let handler = PizzazServerHandler::new();
        let resources = handler.list_widget_resources().await;
        assert_eq!(resources.len(), 5);
        assert!(resources
            .iter()
            .any(|resource| resource.uri == "ui://widget/pizza-map.html"));
        for resource in resources {
            let meta = resource
                .meta
                .as_object()
                .expect("resource meta should be an object");
            assert!(meta.contains_key("openai/outputTemplate"));
        }
    }

    #[tokio::test]
    async fn test_read_widget_resource_returns_html() {
        let handler = PizzazServerHandler::new();
        let content = handler
            .read_widget_resource("ui://widget/pizza-map.html")
            .await
            .expect("resource should exist");
        assert_eq!(content.mime_type, HTML_WIDGET_MIME);
        assert!(content.text.contains("pizzaz"));
        let meta = content.meta.as_object().expect("meta should be present");
        assert!(meta["openai/outputTemplate"].is_string());
    }

    #[tokio::test]
    async fn test_list_widget_resource_templates() {
        let handler = PizzazServerHandler::new();
        let templates = handler.list_widget_resource_templates().await;
        assert_eq!(templates.len(), 5);
        assert!(templates
            .iter()
            .all(|template| template.uri_template.starts_with("ui://widget/")));
    }
}
