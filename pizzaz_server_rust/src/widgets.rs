//! Widget definitions and registry

use std::collections::HashMap;
use std::sync::LazyLock;

/// Represents a Pizzaz widget with all metadata required for MCP integration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PizzazWidget {
    /// Unique identifier (tool name)
    pub id: String,
    /// Human-readable title
    pub title: String,
    /// UI template URI (e.g., "ui://widget/pizza-map.html")
    pub template_uri: String,
    /// Status message while tool is executing
    pub invoking: String,
    /// Status message after tool completes
    pub invoked: String,
    /// HTML markup for widget rendering
    pub html: String,
    /// Text response returned to client
    pub response_text: String,
}

impl PizzazWidget {
    /// Generates OpenAI-specific metadata for widget integration
    pub fn meta(&self) -> serde_json::Value {
        serde_json::json!({
            "openai/outputTemplate": self.template_uri,
            "openai/toolInvocation/invoking": self.invoking,
            "openai/toolInvocation/invoked": self.invoked,
            "openai/widgetAccessible": true,
            "openai/resultCanProduceWidget": true,
        })
    }
}

/// Static registry of all available widgets
static WIDGETS: LazyLock<Vec<PizzazWidget>> = LazyLock::new(|| {
    vec![
        PizzazWidget {
            id: "pizza-map".to_string(),
            title: "Show Pizza Map".to_string(),
            template_uri: "ui://widget/pizza-map.html".to_string(),
            invoking: "Hand-tossing a map".to_string(),
            invoked: "Served a fresh map".to_string(),
            html: r#"
<div id="pizzaz-root"></div>
<link rel="stylesheet" href="http://localhost:4444/pizzaz-2d2b.css">
<script type="module" src="http://localhost:4444/pizzaz-2d2b.js"></script>
            "#
            .trim()
            .to_string(),
            response_text: "Rendered a pizza map!".to_string(),
        },
        PizzazWidget {
            id: "pizza-carousel".to_string(),
            title: "Show Pizza Carousel".to_string(),
            template_uri: "ui://widget/pizza-carousel.html".to_string(),
            invoking: "Carousel some spots".to_string(),
            invoked: "Served a fresh carousel".to_string(),
            html: r#"
<div id="pizzaz-carousel-root"></div>
<link rel="stylesheet" href="http://localhost:4444/pizzaz-carousel-2d2b.css">
<script type="module" src="http://localhost:4444/pizzaz-carousel-2d2b.js"></script>
            "#
            .trim()
            .to_string(),
            response_text: "Rendered a pizza carousel!".to_string(),
        },
        PizzazWidget {
            id: "pizza-albums".to_string(),
            title: "Show Pizza Album".to_string(),
            template_uri: "ui://widget/pizza-albums.html".to_string(),
            invoking: "Hand-tossing an album".to_string(),
            invoked: "Served a fresh album".to_string(),
            html: r#"
<div id="pizzaz-albums-root"></div>
<link rel="stylesheet" href="http://localhost:4444/pizzaz-albums-2d2b.css">
<script type="module" src="http://localhost:4444/pizzaz-albums-2d2b.js"></script>
            "#
            .trim()
            .to_string(),
            response_text: "Rendered a pizza album!".to_string(),
        },
        PizzazWidget {
            id: "pizza-list".to_string(),
            title: "Show Pizza List".to_string(),
            template_uri: "ui://widget/pizza-list.html".to_string(),
            invoking: "Hand-tossing a list".to_string(),
            invoked: "Served a fresh list".to_string(),
            html: r#"
<div id="pizzaz-list-root"></div>
<link rel="stylesheet" href="http://localhost:4444/pizzaz-list-2d2b.css">
<script type="module" src="http://localhost:4444/pizzaz-list-2d2b.js"></script>
            "#
            .trim()
            .to_string(),
            response_text: "Rendered a pizza list!".to_string(),
        },
        PizzazWidget {
            id: "pizza-video".to_string(),
            title: "Show Pizza Video".to_string(),
            template_uri: "ui://widget/pizza-video.html".to_string(),
            invoking: "Hand-tossing a video".to_string(),
            invoked: "Served a fresh video".to_string(),
            html: r#"
<div id="pizzaz-video-root"></div>
<link rel="stylesheet" href="https://persistent.oaistatic.com/ecosystem-built-assets/pizzaz-video-0038.css">
<script type="module" src="https://persistent.oaistatic.com/ecosystem-built-assets/pizzaz-video-0038.js"></script>
            "#
            .trim()
            .to_string(),
            response_text: "Rendered a pizza video!".to_string(),
        },
    ]
});

/// Lookup index: widget ID -> widget reference
static WIDGETS_BY_ID: LazyLock<HashMap<&'static str, &'static PizzazWidget>> =
    LazyLock::new(|| WIDGETS.iter().map(|w| (w.id.as_str(), w)).collect());

/// Lookup index: template URI -> widget reference
static WIDGETS_BY_URI: LazyLock<HashMap<&'static str, &'static PizzazWidget>> =
    LazyLock::new(|| {
        WIDGETS
            .iter()
            .map(|w| (w.template_uri.as_str(), w))
            .collect()
    });

/// Returns all available widgets
pub fn get_all_widgets() -> &'static [PizzazWidget] {
    &WIDGETS
}

/// Looks up a widget by its ID (tool name)
pub fn get_widget_by_id(id: &str) -> Option<&'static PizzazWidget> {
    WIDGETS_BY_ID.get(id).copied()
}

/// Looks up a widget by its template URI
pub fn get_widget_by_uri(uri: &str) -> Option<&'static PizzazWidget> {
    WIDGETS_BY_URI.get(uri).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_widget_creation() {
        let widget = PizzazWidget {
            id: "pizza-map".to_string(),
            title: "Show Pizza Map".to_string(),
            template_uri: "ui://widget/pizza-map.html".to_string(),
            invoking: "Hand-tossing a map".to_string(),
            invoked: "Served a fresh map".to_string(),
            html: "<div id=\"pizzaz-root\"></div>".to_string(),
            response_text: "Rendered a pizza map!".to_string(),
        };

        assert_eq!(widget.id, "pizza-map");
        assert_eq!(widget.title, "Show Pizza Map");
    }

    #[test]
    fn test_widget_implements_traits() {
        let widget = create_test_widget();

        // Should implement Clone
        let cloned = widget.clone();
        assert_eq!(cloned.id, widget.id);

        // Should implement Debug
        let debug_str = format!("{:?}", widget);
        assert!(debug_str.contains("pizza-map"));
    }

    #[test]
    fn test_widget_meta() {
        let widget = create_test_widget();
        let meta = widget.meta();

        assert_eq!(meta["openai/outputTemplate"], "ui://widget/pizza-map.html");
        assert_eq!(meta["openai/toolInvocation/invoking"], "Hand-tossing a map");
        assert_eq!(meta["openai/toolInvocation/invoked"], "Served a fresh map");
        assert_eq!(meta["openai/widgetAccessible"], true);
        assert_eq!(meta["openai/resultCanProduceWidget"], true);
    }

    #[test]
    fn test_meta_contains_all_required_fields() {
        let widget = create_test_widget();
        let meta = widget.meta();

        let required_keys = [
            "openai/outputTemplate",
            "openai/toolInvocation/invoking",
            "openai/toolInvocation/invoked",
            "openai/widgetAccessible",
            "openai/resultCanProduceWidget",
        ];

        for key in &required_keys {
            assert!(meta.get(key).is_some(), "Missing required key: {}", key);
        }
    }

    #[test]
    fn test_get_all_widgets() {
        let widgets = get_all_widgets();
        assert_eq!(widgets.len(), 5);

        // Verify all expected widgets exist
        let ids: Vec<&str> = widgets.iter().map(|w| w.id.as_str()).collect();
        assert!(ids.contains(&"pizza-map"));
        assert!(ids.contains(&"pizza-carousel"));
        assert!(ids.contains(&"pizza-albums"));
        assert!(ids.contains(&"pizza-list"));
        assert!(ids.contains(&"pizza-video"));
    }

    #[test]
    fn test_get_widget_by_id_success() {
        let widget = get_widget_by_id("pizza-carousel");
        assert!(widget.is_some());

        let widget = widget.unwrap();
        assert_eq!(widget.id, "pizza-carousel");
        assert_eq!(widget.title, "Show Pizza Carousel");
    }

    #[test]
    fn test_get_widget_by_id_not_found() {
        let widget = get_widget_by_id("nonexistent");
        assert!(widget.is_none());
    }

    #[test]
    fn test_get_widget_by_uri_success() {
        let widget = get_widget_by_uri("ui://widget/pizza-albums.html");
        assert!(widget.is_some());

        let widget = widget.unwrap();
        assert_eq!(widget.id, "pizza-albums");
    }

    #[test]
    fn test_get_widget_by_uri_not_found() {
        let widget = get_widget_by_uri("ui://widget/invalid.html");
        assert!(widget.is_none());
    }

    #[test]
    fn test_widget_registry_consistency() {
        // All widgets accessible by ID should also be accessible by URI
        for widget in get_all_widgets() {
            let by_id = get_widget_by_id(&widget.id);
            let by_uri = get_widget_by_uri(&widget.template_uri);

            assert!(by_id.is_some());
            assert!(by_uri.is_some());

            // Should point to same widget
            assert_eq!(by_id.unwrap().id, by_uri.unwrap().id);
        }
    }

    #[cfg(test)]
    fn create_test_widget() -> PizzazWidget {
        PizzazWidget {
            id: "pizza-map".to_string(),
            title: "Show Pizza Map".to_string(),
            template_uri: "ui://widget/pizza-map.html".to_string(),
            invoking: "Hand-tossing a map".to_string(),
            invoked: "Served a fresh map".to_string(),
            html: "<div id=\"pizzaz-root\"></div>".to_string(),
            response_text: "Rendered!".to_string(),
        }
    }
}
