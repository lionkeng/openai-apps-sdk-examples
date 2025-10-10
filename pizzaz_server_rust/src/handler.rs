//! MCP server handler for Pizzaz widgets

/// MCP server handler for Pizzaz widgets
#[derive(Debug, Clone)]
pub struct PizzazServerHandler;

impl PizzazServerHandler {
    /// Creates a new handler instance
    pub fn new() -> Self {
        Self
    }
}

impl Default for PizzazServerHandler {
    fn default() -> Self {
        Self::new()
    }
}
