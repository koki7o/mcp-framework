//! Example: MCP Agent with Tools Demo
//!
//! This example demonstrates how an LLM agent uses the 8 tools
//! to intelligently answer questions and perform tasks.
//!
//! The agent will:
//! 1. Start an MCP server (localhost:3000) with 8 tools
//! 2. Initialize Claude as the LLM
//! 3. Receive user queries
//! 4. See available tools (calculator, weather, json_parser, etc.)
//! 5. Decide which tools to call
//! 6. Call them with appropriate parameters
//! 7. Synthesize results into a helpful answer
//!
//! Everything runs in a single process - no need for separate terminals!
//!
//! Requires ANTHROPIC_API_KEY environment variable to be set in .env or environment.
//!
//! Run with:
//! ```bash
//! cargo run --example agent_with_tools_demo --release
//! ```
//!
//! The example will:
//! - Start MCP server on http://localhost:3000
//! - Initialize Claude
//! - Run 5 demo queries
//! - Show Claude calling tools and synthesizing complete answers

use mcp_framework::prelude::*;
use mcp_framework::server::{McpServer, ServerConfig, ToolHandler};
use mcp_framework::agent::AgentConfig;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    mcp_framework::load_env();

    println!("🤖 MCP Agent with Tools Demo\n");
    println!("This agent will use tools to answer your questions.\n");

    // Create a tool handler with the same 8 tools as with_tools.rs
    struct DemoToolHandler;

    #[async_trait::async_trait]
    impl ToolHandler for DemoToolHandler {
        async fn execute(&self, name: &str, arguments: serde_json::Value) -> Result<Vec<ResultContent>> {
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
                        "divide" => if b != 0.0 { a / b } else { return Err(Error::InvalidRequest("Division by zero".to_string())) },
                        "power" => a.powf(b),
                        "sqrt" => if a >= 0.0 { a.sqrt() } else { return Err(Error::InvalidRequest("Cannot take sqrt of negative".to_string())) },
                        _ => return Err(Error::InvalidRequest(format!("Unknown operation: {}", op))),
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
                _ => Err(Error::ToolNotFound(name.to_string())),
            }
        }
    }

    // Create the MCP server with tools
    let handler = Arc::new(DemoToolHandler);
    let config = ServerConfig {
        name: "Demo Tools Server".to_string(),
        version: "1.0.0".to_string(),
        capabilities: ServerCapabilities {
            tools: Some(ToolsCapability { list_changed: Some(false) }),
            resources: None,
            prompts: None,
        },
    };

    let server = Arc::new(McpServer::new(config, handler));

    // Register all 8 tools
    let tools = vec![
        Tool {
            name: "echo".to_string(),
            description: Some("Echo back a message".to_string()),
            input_schema: Some(ToolInputSchema {
                schema_type: "object".to_string(),
                properties: {
                    let mut p = std::collections::HashMap::new();
                    p.insert("message".to_string(), serde_json::json!({ "type": "string" }));
                    p
                },
                required: Some(vec!["message".to_string()]),
            }),
        },
        Tool {
            name: "calculator".to_string(),
            description: Some("Perform math operations: add, subtract, multiply, divide, power, sqrt".to_string()),
            input_schema: Some(ToolInputSchema {
                schema_type: "object".to_string(),
                properties: {
                    let mut p = std::collections::HashMap::new();
                    p.insert("operation".to_string(), serde_json::json!({ "type": "string", "enum": ["add", "subtract", "multiply", "divide", "power", "sqrt"] }));
                    p.insert("a".to_string(), serde_json::json!({ "type": "number" }));
                    p.insert("b".to_string(), serde_json::json!({ "type": "number" }));
                    p
                },
                required: Some(vec!["operation".to_string(), "a".to_string(), "b".to_string()]),
            }),
        },
        Tool {
            name: "get_weather".to_string(),
            description: Some("Get current weather for a city (supports: san francisco, new york, los angeles, london, tokyo)".to_string()),
            input_schema: Some(ToolInputSchema {
                schema_type: "object".to_string(),
                properties: {
                    let mut p = std::collections::HashMap::new();
                    p.insert("location".to_string(), serde_json::json!({ "type": "string" }));
                    p
                },
                required: Some(vec!["location".to_string()]),
            }),
        },
        Tool {
            name: "search_text".to_string(),
            description: Some("Search for pattern occurrences in text".to_string()),
            input_schema: Some(ToolInputSchema {
                schema_type: "object".to_string(),
                properties: {
                    let mut p = std::collections::HashMap::new();
                    p.insert("text".to_string(), serde_json::json!({ "type": "string" }));
                    p.insert("pattern".to_string(), serde_json::json!({ "type": "string" }));
                    p
                },
                required: Some(vec!["text".to_string(), "pattern".to_string()]),
            }),
        },
        Tool {
            name: "string_length".to_string(),
            description: Some("Get the length of a string in characters".to_string()),
            input_schema: Some(ToolInputSchema {
                schema_type: "object".to_string(),
                properties: {
                    let mut p = std::collections::HashMap::new();
                    p.insert("text".to_string(), serde_json::json!({ "type": "string" }));
                    p
                },
                required: Some(vec!["text".to_string()]),
            }),
        },
        Tool {
            name: "text_reverse".to_string(),
            description: Some("Reverse a text string".to_string()),
            input_schema: Some(ToolInputSchema {
                schema_type: "object".to_string(),
                properties: {
                    let mut p = std::collections::HashMap::new();
                    p.insert("text".to_string(), serde_json::json!({ "type": "string" }));
                    p
                },
                required: Some(vec!["text".to_string()]),
            }),
        },
        Tool {
            name: "json_parser".to_string(),
            description: Some("Validate and parse JSON strings, returns formatted output".to_string()),
            input_schema: Some(ToolInputSchema {
                schema_type: "object".to_string(),
                properties: {
                    let mut p = std::collections::HashMap::new();
                    p.insert("json".to_string(), serde_json::json!({ "type": "string" }));
                    p
                },
                required: Some(vec!["json".to_string()]),
            }),
        },
        Tool {
            name: "http_status".to_string(),
            description: Some("Look up the meaning of HTTP status codes".to_string()),
            input_schema: Some(ToolInputSchema {
                schema_type: "object".to_string(),
                properties: {
                    let mut p = std::collections::HashMap::new();
                    p.insert("code".to_string(), serde_json::json!({ "type": "integer" }));
                    p
                },
                required: Some(vec!["code".to_string()]),
            }),
        },
    ];

    for tool in &tools {
        server.register_tool(tool.clone());
    }

    println!("✅ Registered {} tools:", tools.len());
    for tool in &tools {
        println!("   • {}: {}", tool.name, tool.description.as_ref().unwrap_or(&"No description".to_string()));
    }

    // Create Anthropic Claude adapter
    println!("\n🔧 Initializing Claude LLM...");
    let claude = AnthropicAdapter::from_env("claude-sonnet-4-5".to_string())
        .map_err(|e| {
            eprintln!("Error: Failed to initialize Claude adapter. Make sure ANTHROPIC_API_KEY is set.");
            eprintln!("Details: {}", e);
            e
        })?
        .with_system_prompt(
            "You are a helpful assistant with access to various tools. Use the available tools to help answer user questions and complete tasks. Be concise and clear in your responses.".to_string()
        );

    // Create agent config
    let agent_config = AgentConfig {
        max_iterations: 10,
        max_tokens: Some(2048),
    };

    // Start JSON-RPC HTTP server in background task
    let server_clone = server.clone();
    let _server_task = tokio::spawn(async move {
        use axum::{
            extract::State,
            http::StatusCode,
            routing::post,
            Json, Router,
        };
        use tokio::net::TcpListener;

        #[derive(Clone)]
        struct ServerState {
            server: Arc<McpServer>,
        }

        // JSON-RPC endpoint handler
        async fn handle_rpc(
            State(state): State<ServerState>,
            Json(request): Json<JsonRpcRequest>,
        ) -> (StatusCode, Json<JsonRpcResponse>) {
            let response = state.server.handle_request(request).await;
            (StatusCode::OK, Json(response))
        }

        let state = ServerState { server: server_clone };
        let router = Router::new()
            .route("/", post(handle_rpc))
            .with_state(state);

        let listener = TcpListener::bind("127.0.0.1:3000")
            .await
            .expect("Failed to bind to 127.0.0.1:3000");

        println!("🌐 MCP Server listening on http://localhost:3000");

        axum::serve(listener, router)
            .await
            .expect("Failed to start server");
    });

    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    println!("✅ Agent starting with 8 tools available...\n");

    // Create a client that communicates with the local server via JSON-RPC HTTP
    let client = McpClient::new("http://localhost:3000/");

    // Create the agent
    let mut agent = Agent::new(client, Arc::new(claude), agent_config);

    // Demo queries showing different tool usage
    let demo_queries = vec![
        "What is 15 + 27? And what's the weather in New York?",
        "Can you reverse the word 'hello' and tell me how many characters it has?",
        "What is the square root of 144?",
        "Tell me what HTTP status code 404 means",
        "Can you count how many times the letter 'o' appears in 'hello world'?",
    ];

    println!("\n🚀 Running demo queries with Claude agent:\n");
    println!("The agent will use tools to answer questions about math, weather, text operations, and HTTP status codes.");
    println!("\n📌 While the agent runs, you can also:");
    println!("   • Visit http://localhost:8123 to use the Inspector");
    println!("   • Manually test tools in the web UI");
    println!("   • See request/response history");
    println!("\n{}", "=".repeat(80));

    for (i, query) in demo_queries.iter().enumerate() {
        println!("\n📝 Query {}: {}", i + 1, query);
        println!("{}", "-".repeat(80));

        match agent.run(*query).await {
            Ok(response) => {
                println!("🤖 Agent Response:");
                println!("{}", response);
            }
            Err(e) => {
                eprintln!("❌ Error: {}", e);
            }
        }

        println!();
    }

    println!("{}", "=".repeat(80));
    println!("\n✅ Demo completed!");
    println!("\nWhat you just saw:");
    println!("1. The agent read your question");
    println!("2. The agent decided which tools to use");
    println!("3. The agent called the tools with appropriate parameters");
    println!("4. The agent received results from the tools");
    println!("5. The agent synthesized the results into a helpful answer\n");
    println!("This is the power of the MCP Framework - LLMs can dynamically call tools!");

    println!("\n✨ Done! The MCP Server (localhost:3000) is still running.");
    println!("Press Ctrl+C to stop.\n");

    // Keep the application running so server stays alive
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}
