//! Shared types for MCP communication

use serde::{Deserialize, Serialize};

/// Input arguments for widget tool calls
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInput {
    /// Pizza topping to display in widget
    pub pizza_topping: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_input_from_json() {
        let json = r#"{"pizzaTopping": "pepperoni"}"#;
        let input: ToolInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.pizza_topping, "pepperoni");
    }

    #[test]
    fn test_tool_input_serialize() {
        let input = ToolInput {
            pizza_topping: "mushrooms".to_string(),
        };

        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("pizzaTopping"));
        assert!(json.contains("mushrooms"));
    }

    #[test]
    fn test_tool_input_missing_field() {
        let json = r#"{}"#;
        let result: Result<ToolInput, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_tool_input_wrong_type() {
        let json = r#"{"pizzaTopping": 123}"#;
        let result: Result<ToolInput, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_tool_input_null_value() {
        let json = r#"{"pizzaTopping": null}"#;
        let result: Result<ToolInput, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_tool_input_extra_fields_ignored() {
        let json = r#"{"pizzaTopping": "olives", "extraField": "ignored"}"#;
        let input: ToolInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.pizza_topping, "olives");
    }

    #[test]
    fn test_tool_input_empty_string() {
        let json = r#"{"pizzaTopping": ""}"#;
        let input: ToolInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.pizza_topping, "");
    }

    #[test]
    fn test_tool_input_round_trip() {
        let original = ToolInput {
            pizza_topping: "pepperoni and mushrooms".to_string(),
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ToolInput = serde_json::from_str(&json).unwrap();

        assert_eq!(original, deserialized);
    }
}
