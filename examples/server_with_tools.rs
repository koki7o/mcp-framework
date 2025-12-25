use mcp_framework::prelude::*;
use mcp_framework::server::{McpServer, ServerConfig, ToolHandler};
use mcp_framework::inspector::Inspector;
use std::sync::Arc;
use serde_json::{json, Value};
use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Json, Router,
};
use tokio::net::TcpListener;

/// Comprehensive tool handler with 8 different tools
struct ComprehensiveToolHandler;

#[async_trait::async_trait]
impl ToolHandler for ComprehensiveToolHandler {
    async fn execute(&self, name: &str, arguments: Value) -> mcp_framework::Result<Vec<ResultContent>> {
        match name {
            "echo" => {
                let message = arguments.get("message").and_then(|v| v.as_str()).unwrap_or("no message");
                Ok(vec![ResultContent::Text { text: format!("Echo: {}", message) }])
            }
            "calculator" => {
                let op = arguments.get("operation").and_then(|v| v.as_str()).unwrap_or("add");
                let a = arguments.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let b = arguments.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let result = match op {
                    "add" => a + b,
                    "subtract" => a - b,
                    "multiply" => a * b,
                    "divide" => if b != 0.0 { a / b } else { return Err(mcp_framework::error::Error::InvalidRequest("Division by zero".to_string())) },
                    "power" => a.powf(b),
                    "sqrt" => if a >= 0.0 { a.sqrt() } else { return Err(mcp_framework::error::Error::InvalidRequest("Cannot take sqrt of negative".to_string())) },
                    _ => return Err(mcp_framework::error::Error::InvalidRequest(format!("Unknown operation: {}", op))),
                };
                Ok(vec![ResultContent::Text { text: format!("Result: {}", result) }])
            }
            "get_weather" => {
                let location = arguments.get("location").and_then(|v| v.as_str()).unwrap_or("unknown");
                let weather = match location.to_lowercase().as_str() {
                    "san francisco" | "sf" => "72°F, Cloudy, 65% humidity",
                    "new york" | "nyc" => "65°F, Rainy, 80% humidity",
                    "los angeles" | "la" => "82°F, Sunny, 45% humidity",
                    "london" => "59°F, Overcast, 75% humidity",
                    "tokyo" => "75°F, Clear, 55% humidity",
                    _ => "72°F, Sunny, 50% humidity",
                };
                Ok(vec![ResultContent::Text { text: format!("Weather in {}: {}", location, weather) }])
            }
            "search_text" => {
                let text = arguments.get("text").and_then(|v| v.as_str()).unwrap_or("");
                let pattern = arguments.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
                let count = text.matches(pattern).count();
                Ok(vec![ResultContent::Text { text: format!("Found {} occurrence(s) of '{}' in text", count, pattern) }])
            }
            "string_length" => {
                let text = arguments.get("text").and_then(|v| v.as_str()).unwrap_or("");
                Ok(vec![ResultContent::Text { text: format!("String length: {} characters", text.len()) }])
            }
            "text_reverse" => {
                let text = arguments.get("text").and_then(|v| v.as_str()).unwrap_or("");
                let reversed: String = text.chars().rev().collect();
                Ok(vec![ResultContent::Text { text: format!("Reversed text: {}", reversed) }])
            }
            "json_parser" => {
                let json_str = arguments.get("json").and_then(|v| v.as_str()).unwrap_or("{}");
                match serde_json::from_str::<serde_json::Value>(json_str) {
                    Ok(parsed) => Ok(vec![ResultContent::Text { text: format!("Valid JSON: {}", serde_json::to_string_pretty(&parsed).unwrap_or_default()) }]),
                    Err(e) => Ok(vec![ResultContent::Text { text: format!("Invalid JSON: {}", e) }]),
                }
            }
            "http_status" => {
                let code = arguments.get("code").and_then(|v| v.as_i64()).unwrap_or(200) as u32;
                let status = match code {
                    200 => "OK - Request successful",
                    201 => "Created - Resource created successfully",
                    400 => "Bad Request - Invalid request",
                    401 => "Unauthorized - Authentication required",
                    403 => "Forbidden - Access denied",
                    404 => "Not Found - Resource not found",
                    500 => "Internal Server Error",
                    503 => "Service Unavailable",
                    _ => "Unknown status code",
                };
                Ok(vec![ResultContent::Text { text: format!("HTTP {}: {}", code, status) }])
            }
            _ => Err(mcp_framework::error::Error::ToolNotFound(name.to_string())),
        }
    }
}

#[derive(Clone)]
struct ServerState {
    server: Arc<McpServer>,
}

// Handler: POST / (JSON-RPC endpoint)
async fn handle_rpc(
    State(state): State<ServerState>,
    Json(request): Json<JsonRpcRequest>,
) -> (StatusCode, Json<JsonRpcResponse>) {
    let response = state.server.handle_request(request).await;
    (StatusCode::OK, Json(response))
}

#[tokio::main]
async fn main() -> mcp_framework::Result<()> {
    println!("MCP Framework - 8 Tools Example with Inspector\n");

    let config = ServerConfig {
        name: "Tools Server".to_string(),
        version: "1.0.0".to_string(),
        capabilities: ServerCapabilities {
            tools: Some(ToolsCapability { list_changed: Some(false) }),
            resources: None,
            prompts: None,
        },
    };

    let tool_handler = Arc::new(ComprehensiveToolHandler);
    let server = Arc::new(McpServer::new(config, tool_handler));

    // Register all 8 tools
    let tools_to_register = vec![
        Tool {
            name: "echo".to_string(),
            description: Some("Echo back a message".to_string()),
            input_schema: Some(ToolInputSchema {
                schema_type: "object".to_string(),
                properties: { let mut p = std::collections::HashMap::new(); p.insert("message".to_string(), json!({ "type": "string" })); p },
                required: Some(vec!["message".to_string()]),
            }),
        },
        Tool {
            name: "calculator".to_string(),
            description: Some("Math operations: add, subtract, multiply, divide, power, sqrt".to_string()),
            input_schema: Some(ToolInputSchema {
                schema_type: "object".to_string(),
                properties: { let mut p = std::collections::HashMap::new(); p.insert("operation".to_string(), json!({ "type": "string", "enum": ["add", "subtract", "multiply", "divide", "power", "sqrt"] })); p.insert("a".to_string(), json!({ "type": "number" })); p.insert("b".to_string(), json!({ "type": "number" })); p },
                required: Some(vec!["operation".to_string(), "a".to_string(), "b".to_string()]),
            }),
        },
        Tool {
            name: "get_weather".to_string(),
            description: Some("Get weather for cities worldwide".to_string()),
            input_schema: Some(ToolInputSchema {
                schema_type: "object".to_string(),
                properties: { let mut p = std::collections::HashMap::new(); p.insert("location".to_string(), json!({ "type": "string" })); p },
                required: Some(vec!["location".to_string()]),
            }),
        },
        Tool {
            name: "search_text".to_string(),
            description: Some("Search for text pattern occurrences".to_string()),
            input_schema: Some(ToolInputSchema {
                schema_type: "object".to_string(),
                properties: { let mut p = std::collections::HashMap::new(); p.insert("text".to_string(), json!({ "type": "string" })); p.insert("pattern".to_string(), json!({ "type": "string" })); p },
                required: Some(vec!["text".to_string(), "pattern".to_string()]),
            }),
        },
        Tool {
            name: "string_length".to_string(),
            description: Some("Get the length of a string".to_string()),
            input_schema: Some(ToolInputSchema {
                schema_type: "object".to_string(),
                properties: { let mut p = std::collections::HashMap::new(); p.insert("text".to_string(), json!({ "type": "string" })); p },
                required: Some(vec!["text".to_string()]),
            }),
        },
        Tool {
            name: "text_reverse".to_string(),
            description: Some("Reverse a text string".to_string()),
            input_schema: Some(ToolInputSchema {
                schema_type: "object".to_string(),
                properties: { let mut p = std::collections::HashMap::new(); p.insert("text".to_string(), json!({ "type": "string" })); p },
                required: Some(vec!["text".to_string()]),
            }),
        },
        Tool {
            name: "json_parser".to_string(),
            description: Some("Validate and parse JSON strings".to_string()),
            input_schema: Some(ToolInputSchema {
                schema_type: "object".to_string(),
                properties: { let mut p = std::collections::HashMap::new(); p.insert("json".to_string(), json!({ "type": "string" })); p },
                required: Some(vec!["json".to_string()]),
            }),
        },
        Tool {
            name: "http_status".to_string(),
            description: Some("Look up HTTP status codes".to_string()),
            input_schema: Some(ToolInputSchema {
                schema_type: "object".to_string(),
                properties: { let mut p = std::collections::HashMap::new(); p.insert("code".to_string(), json!({ "type": "integer" })); p },
                required: Some(vec!["code".to_string()]),
            }),
        },
    ];

    for tool in &tools_to_register {
        server.register_tool(tool.clone());
    }

    println!("Registered {} tools:", tools_to_register.len());
    for tool in &tools_to_register {
        println!("  - {}", tool.name);
    }

    // Setup inspector
    let mut inspector = Inspector::new("Tools Server".to_string(), "1.0.0".to_string());
    inspector.set_tools(tools_to_register.clone());
    inspector.set_server(server.clone());

    // Start MCP server on port 3000 (in background task)
    let server_state = ServerState {
        server: server.clone(),
    };

    let server_task = tokio::spawn(async move {
        let router = Router::new()
            .route("/", post(handle_rpc))
            .with_state(server_state);

        let listener = TcpListener::bind("127.0.0.1:3000")
            .await
            .expect("Failed to bind MCP server to 127.0.0.1:3000");

        println!("JSON-RPC Server listening on http://localhost:3000");

        axum::serve(listener, router)
            .await
            .expect("Failed to start MCP server");
    });

    // Start Inspector on port 8123 (in background task)
    let inspector_task = tokio::spawn(async move {
        println!("Inspector listening on http://localhost:8123");
        println!("Visit the URL to test tools interactively");
        println!("Press Ctrl+C to stop\n");

        if let Err(e) = inspector.start("127.0.0.1:8123").await {
            eprintln!("Inspector error: {}", e);
        }
    });

    // Wait for both tasks (they run indefinitely)
    tokio::select! {
        _ = server_task => {},
        _ = inspector_task => {},
    }

    Ok(())
}
