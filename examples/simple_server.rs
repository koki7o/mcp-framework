//! Simple MCP server with calculator.
//!
//! cargo run --example simple_server

use mcp_framework::prelude::*;
use mcp_framework::server::{McpServer, ServerConfig, ToolHandler};
use std::sync::Arc;
use serde_json::{json, Value};

struct CalculatorHandler;

#[async_trait::async_trait]
impl ToolHandler for CalculatorHandler {
    async fn execute(&self, name: &str, arguments: Value) -> mcp_framework::Result<Vec<ResultContent>> {
        match name {
            "add" => {
                let a = arguments.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let b = arguments.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
                Ok(vec![ResultContent::Text { text: format!("{} + {} = {}", a, b, a + b) }])
            }
            "multiply" => {
                let a = arguments.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let b = arguments.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
                Ok(vec![ResultContent::Text { text: format!("{} * {} = {}", a, b, a * b) }])
            }
            _ => Err(mcp_framework::error::Error::ToolNotFound(name.to_string())),
        }
    }
}

#[tokio::main]
async fn main() -> mcp_framework::Result<()> {
    println!("Simple MCP Server with Calculator\n");

    let config = ServerConfig {
        name: "Calculator Server".to_string(),
        version: "1.0.0".to_string(),
        capabilities: ServerCapabilities {
            tools: Some(ToolsCapability { list_changed: Some(false) }),
            resources: None,
            prompts: None,
        },
    };

    let handler = Arc::new(CalculatorHandler);
    let server = McpServer::new(config, handler);

    // Register two simple tools
    server.register_tool(Tool {
        name: "add".to_string(),
        description: Some("Add two numbers".to_string()),
        input_schema: Some(ToolInputSchema {
            schema_type: "object".to_string(),
            properties: {
                let mut p = std::collections::HashMap::new();
                p.insert("a".to_string(), json!({ "type": "number" }));
                p.insert("b".to_string(), json!({ "type": "number" }));
                p
            },
            required: Some(vec!["a".to_string(), "b".to_string()]),
        }),
    });

    server.register_tool(Tool {
        name: "multiply".to_string(),
        description: Some("Multiply two numbers".to_string()),
        input_schema: Some(ToolInputSchema {
            schema_type: "object".to_string(),
            properties: {
                let mut p = std::collections::HashMap::new();
                p.insert("a".to_string(), json!({ "type": "number" }));
                p.insert("b".to_string(), json!({ "type": "number" }));
                p
            },
            required: Some(vec!["a".to_string(), "b".to_string()]),
        }),
    });

    println!("✅ Server ready with 2 tools: add, multiply");
    println!("   This is a simple example. See with_tools.rs for more features\n");

    Ok(())
}
