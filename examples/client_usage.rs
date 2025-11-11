use mcp_framework::client::McpClient;
use serde_json::json;

/// Example of how to use the MCP Client to connect to a server
#[tokio::main]
async fn main() -> mcp_framework::Result<()> {
    println!("📡 MCP Client Usage Example\n");

    // Create a client that connects to an MCP server
    let client = McpClient::new("http://localhost:3000");

    // List available tools
    println!("📋 Listing tools from server...\n");
    match client.list_tools().await {
        Ok(tools) => {
            println!("Found {} tools:", tools.len());
            for tool in &tools {
                println!("  • {} - {}", tool.name, tool.description.as_deref().unwrap_or("No description"));
            }
            println!();
        }
        Err(e) => println!("Error listing tools: {}\n", e),
    }

    // Call a tool
    println!("🔧 Calling the 'echo' tool...\n");
    match client.call_tool("echo", json!({"message": "Hello from client!"})).await {
        Ok(result) => {
            println!("Tool result:");
            for content in &result.content {
                match content {
                    mcp_framework::protocol::ResultContent::Text { text } => println!("  {}", text),
                    _ => println!("  [Binary content]"),
                }
            }
        }
        Err(e) => println!("Error calling tool: {}", e),
    }

    println!("\n✅ Client example complete");
    println!("   ℹ️  To test this example, run a server first:");
    println!("      Terminal 1: cargo run --example simple_server --release");
    println!("      Terminal 2: cargo run --example client_usage --release\n");

    Ok(())
}
