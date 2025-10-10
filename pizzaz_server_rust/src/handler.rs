//! MCP server handler for Pizzaz widgets

use crate::{types::ToolInput, widgets};
use anyhow::Result;

/// MCP server handler for Pizzaz widgets
#[derive(Debug, Clone)]
pub struct PizzazServerHandler;

/// Tool definition for MCP
#[derive(Debug, Clone)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub meta: Option<serde_json::Value>,
}

/// Result of calling a tool
#[derive(Debug, Clone)]
pub struct CallToolResult {
    pub content: Vec<serde_json::Value>,
    pub structured_content: serde_json::Value,
    pub meta: Option<serde_json::Value>,
}

/// Resource definition for MCP
#[derive(Debug, Clone)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: String,
    pub meta: Option<serde_json::Value>,
}

/// Resource content
#[derive(Debug, Clone)]
pub struct ResourceContent {
    pub uri: String,
    pub mime_type: String,
    pub text: String,
    pub meta: Option<serde_json::Value>,
}

/// Resource template
#[derive(Debug, Clone)]
pub struct ResourceTemplate {
    pub uri_template: String,
    pub name: String,
    pub description: String,
    pub mime_type: String,
    pub meta: Option<serde_json::Value>,
}

impl PizzazServerHandler {
    /// Creates a new handler instance
    pub fn new() -> Self {
        Self
    }

    /// Lists all available tools
    pub async fn list_tools(&self) -> Vec<Tool> {
        widgets::get_all_widgets()
            .iter()
            .map(|widget| Tool {
                name: widget.id.clone(),
                description: widget.title.clone(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "pizzaTopping": {
                            "type": "string",
                            "description": "Topping to mention when rendering the widget."
                        }
                    },
                    "required": ["pizzaTopping"],
                    "additionalProperties": false
                }),
                meta: Some(widget.meta()),
            })
            .collect()
    }

    /// Calls a tool with given arguments
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<CallToolResult> {
        // Look up widget
        let widget = widgets::get_widget_by_id(name)
            .ok_or_else(|| anyhow::anyhow!("Unknown tool: {}", name))?;

        // Parse and validate arguments
        let input: ToolInput = serde_json::from_value(arguments)
            .map_err(|e| anyhow::anyhow!("Invalid tool arguments: {}", e))?;

        // Build response
        Ok(CallToolResult {
            content: vec![serde_json::json!({
                "type": "text",
                "text": widget.response_text,
            })],
            structured_content: serde_json::json!({
                "pizzaTopping": input.pizza_topping,
            }),
            meta: Some(widget.meta()),
        })
    }

    /// Lists all available resources
    pub async fn list_resources(&self) -> Vec<Resource> {
        widgets::get_all_widgets()
            .iter()
            .map(|widget| Resource {
                uri: widget.template_uri.clone(),
                name: widget.title.clone(),
                description: format!("{} widget markup", widget.title),
                mime_type: "text/html+skybridge".to_string(),
                meta: Some(widget.meta()),
            })
            .collect()
    }

    /// Reads a resource by URI
    pub async fn read_resource(&self, uri: &str) -> Result<ResourceContent> {
        let widget = widgets::get_widget_by_uri(uri)
            .ok_or_else(|| anyhow::anyhow!("Unknown resource: {}", uri))?;

        Ok(ResourceContent {
            uri: widget.template_uri.clone(),
            mime_type: "text/html+skybridge".to_string(),
            text: widget.html.clone(),
            meta: Some(widget.meta()),
        })
    }

    /// Lists all resource templates
    pub async fn list_resource_templates(&self) -> Vec<ResourceTemplate> {
        widgets::get_all_widgets()
            .iter()
            .map(|widget| ResourceTemplate {
                uri_template: widget.template_uri.clone(),
                name: widget.title.clone(),
                description: format!("{} widget markup", widget.title),
                mime_type: "text/html+skybridge".to_string(),
                meta: Some(widget.meta()),
            })
            .collect()
    }
}

impl Default for PizzazServerHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_creation() {
        let handler = PizzazServerHandler::new();
        // Handler should be created without errors
        let _ = handler;
    }

    #[test]
    fn test_handler_implements_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PizzazServerHandler>();
    }

    #[test]
    fn test_handler_default() {
        let handler = PizzazServerHandler;
        let _ = handler;
    }

    // Tests for list_tools
    #[tokio::test]
    async fn test_list_tools_count() {
        let handler = PizzazServerHandler::new();
        let tools = handler.list_tools().await;
        assert_eq!(tools.len(), 5);
    }

    #[tokio::test]
    async fn test_list_tools_contains_expected_tools() {
        let handler = PizzazServerHandler::new();
        let tools = handler.list_tools().await;

        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"pizza-map"));
        assert!(tool_names.contains(&"pizza-carousel"));
        assert!(tool_names.contains(&"pizza-albums"));
        assert!(tool_names.contains(&"pizza-list"));
        assert!(tool_names.contains(&"pizza-video"));
    }

    #[tokio::test]
    async fn test_list_tools_schema_validation() {
        let handler = PizzazServerHandler::new();
        let tools = handler.list_tools().await;

        let pizza_map = tools.iter().find(|t| t.name == "pizza-map").unwrap();

        // Verify input schema structure
        assert_eq!(pizza_map.input_schema["type"], "object");
        assert!(pizza_map.input_schema["properties"]["pizzaTopping"].is_object());

        let required = pizza_map.input_schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("pizzaTopping")));

        assert_eq!(pizza_map.input_schema["additionalProperties"], false);
    }

    #[tokio::test]
    async fn test_list_tools_metadata() {
        let handler = PizzazServerHandler::new();
        let tools = handler.list_tools().await;

        for tool in &tools {
            let meta = tool.meta.as_ref().expect("Tool should have metadata");
            assert_eq!(meta["openai/widgetAccessible"], true);
            assert_eq!(meta["openai/resultCanProduceWidget"], true);
            assert!(meta["openai/outputTemplate"].is_string());
        }
    }

    // Tests for call_tool
    #[tokio::test]
    async fn test_call_tool_success() {
        let handler = PizzazServerHandler::new();
        let args = serde_json::json!({"pizzaTopping": "mushrooms"});

        let result = handler.call_tool("pizza-map", args).await;
        assert!(result.is_ok());

        let result = result.unwrap();

        // Verify text content
        assert_eq!(result.content.len(), 1);
        assert_eq!(result.content[0]["type"], "text");
        assert_eq!(result.content[0]["text"], "Rendered a pizza map!");

        // Verify structured content
        assert_eq!(result.structured_content["pizzaTopping"], "mushrooms");

        // Verify metadata
        let meta = result.meta.expect("Should have metadata");
        assert_eq!(meta["openai/widgetAccessible"], true);
    }

    #[tokio::test]
    async fn test_call_tool_unknown_tool() {
        let handler = PizzazServerHandler::new();
        let args = serde_json::json!({"pizzaTopping": "olives"});

        let result = handler.call_tool("nonexistent-tool", args).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.to_string().contains("Unknown tool"));
    }

    #[tokio::test]
    async fn test_call_tool_invalid_arguments() {
        let handler = PizzazServerHandler::new();

        // Missing required field
        let args = serde_json::json!({});
        let result = handler.call_tool("pizza-map", args).await;
        assert!(result.is_err());

        // Wrong type
        let args = serde_json::json!({"pizzaTopping": 123});
        let result = handler.call_tool("pizza-carousel", args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_call_tool_all_widgets() {
        let handler = PizzazServerHandler::new();
        let widget_ids = [
            "pizza-map",
            "pizza-carousel",
            "pizza-albums",
            "pizza-list",
            "pizza-video",
        ];

        for widget_id in &widget_ids {
            let args = serde_json::json!({"pizzaTopping": "pepperoni"});
            let result = handler.call_tool(widget_id, args).await;

            assert!(result.is_ok(), "Failed for widget: {}", widget_id);
        }
    }

    // Tests for list_resources
    #[tokio::test]
    async fn test_list_resources() {
        let handler = PizzazServerHandler::new();
        let resources = handler.list_resources().await;

        assert_eq!(resources.len(), 5);

        // Check one resource in detail
        let pizza_map = resources
            .iter()
            .find(|r| r.uri == "ui://widget/pizza-map.html")
            .expect("pizza-map resource should exist");

        assert_eq!(pizza_map.name, "Show Pizza Map");
        assert_eq!(pizza_map.mime_type, "text/html+skybridge");
        assert!(pizza_map.description.contains("widget markup"));
    }

    #[tokio::test]
    async fn test_read_resource_success() {
        let handler = PizzazServerHandler::new();

        let result = handler
            .read_resource("ui://widget/pizza-carousel.html")
            .await;
        assert!(result.is_ok());

        let content = result.unwrap();
        assert_eq!(content.uri, "ui://widget/pizza-carousel.html");
        assert_eq!(content.mime_type, "text/html+skybridge");
        assert!(content.text.contains("pizzaz-carousel-root"));

        let meta = content.meta.expect("Should have metadata");
        assert_eq!(meta["openai/widgetAccessible"], true);
    }

    #[tokio::test]
    async fn test_read_resource_not_found() {
        let handler = PizzazServerHandler::new();

        let result = handler.read_resource("ui://widget/invalid.html").await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.to_string().contains("Unknown resource"));
    }

    #[tokio::test]
    async fn test_list_resource_templates() {
        let handler = PizzazServerHandler::new();
        let templates = handler.list_resource_templates().await;

        assert_eq!(templates.len(), 5);

        for template in &templates {
            assert!(template.uri_template.starts_with("ui://widget/"));
            assert_eq!(template.mime_type, "text/html+skybridge");
        }
    }
}
