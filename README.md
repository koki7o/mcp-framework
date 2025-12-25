<div align="center">

<!-- Banner Image - Add your banner.png to docs/ directory -->
<img src="assets/banner.png" alt="MCP Framework Banner" width="800" style="margin-bottom: 20px;">

# MCP Framework - Rust Implementation

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/badge/MCP%20Framework-Rust-orange?style=for-the-badge&logo=rust&logoColor=white">
  <img alt="MCP Framework" src="https://img.shields.io/badge/MCP%20Framework-Rust-orange?style=for-the-badge&logo=rust&logoColor=white">
</picture>

Rust MCP framework for building AI agents. Connect agents to any MCP server (Playwright, filesystem, databases) with support for Claude and OpenAI. Multi-server support, HTTP and stdio transports.

---

<p>
  <a href="https://github.com/koki7o/mcp-framework/blob/main/LICENSE">
    <img alt="License" src="https://img.shields.io/badge/license-MIT-green" />
  </a>
  <a href="https://spec.modelcontextprotocol.io/">
    <img alt="MCP Spec" src="https://img.shields.io/badge/MCP-2025--11--11-blue" />
  </a>
  <a href="https://www.rust-lang.org/">
    <img alt="Rust" src="https://img.shields.io/badge/rust-1.70%2B-orange?logo=rust" />
  </a>
</p>

</div>

---

## What is mcp-framework?

A Rust framework for building AI agents that can use any MCP server. Includes:

- Agent framework with LLM integration (Claude, OpenAI)
- MCP client with multi-transport support (HTTP, stdio)
- MCP server implementation
- Web-based inspector for testing

---

## Features

| Feature | Status |
|---------|--------|
| MCP Server | Done |
| MCP Client | Done |
| AI Agent | Done |
| Web Inspector | Done |
| Claude Integration | Done |
| OpenAI Integration | Done |
| Browser Automation | Done |
| Session Management | Done |
| Resources | Planned |
| Prompts | Planned |
| Authentication | Planned |

### Example Tools

```
• echo           - String echo utility
• calculator     - Math: add, subtract, multiply, divide, power, sqrt
• get_weather    - Weather lookup for cities worldwide
• search_text    - Find pattern occurrences in text
• string_length  - Get character count
• text_reverse   - Reverse text strings
• json_parser    - Validate and format JSON
• http_status    - Look up HTTP status codes
```

---

## Quick Start

### Prerequisites

```bash
# Requires Rust 1.70+
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 1. Clone & Setup

```bash
git clone https://github.com/koki7o/mcp-framework
cd mcp-framework

# Create .env for API keys (optional but recommended)
cp .env.example .env
# Edit .env and add ANTHROPIC_API_KEY or OPENAI_API_KEY
```

### 2. Run Examples

**Minimal Server (1 tool):**
```bash
cargo run
```

**Server with 8 Tools + Inspector UI:**
```bash
cargo run --example server_with_tools
# Visit: http://localhost:8123
```

**AI Agent with Claude:**
```bash
# Requires ANTHROPIC_API_KEY in .env
cargo run --example anthropic_agent_demo_with_tools --release
```

**AI Agent with OpenAI:**
```bash
# Requires OPENAI_API_KEY in .env
cargo run --example openai_agent_demo_with_tools --release
```

**Browser Automation (OpenAI):**
```bash
# Requires OPENAI_API_KEY in .env
# Install: npm install -g @playwright/mcp@latest && npx playwright install firefox
cargo run --example browser_agent_openai
```

**Browser Automation (Claude):**
```bash
# Requires ANTHROPIC_API_KEY in .env
# Install: npm install -g @playwright/mcp@latest && npx playwright install firefox
cargo run --example browser_agent_anthropic
```

---

## Usage

### Build an AI Agent
```rust
use mcp_framework::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    mcp_framework::load_env();

    let client = McpClient::new("http://localhost:3000");
    let llm = AnthropicAdapter::from_env("claude-sonnet-4-5-20250929".to_string())?;
    let mut agent = Agent::new(client, Arc::new(llm), AgentConfig::default());

    let response = agent.run("What is 15 + 27?").await?;
    println!("{}", response);

    Ok(())
}
```

**Run Examples:**
- `cargo run --example anthropic_agent_demo_with_tools --release` - Claude demo
- `cargo run --example openai_agent_demo_with_tools --release` - OpenAI demo

---

### Create an MCP Server
```rust
use mcp_framework::prelude::*;
use mcp_framework::server::{McpServer, ServerConfig, ToolHandler};
use std::sync::Arc;

struct MyToolHandler;

#[async_trait::async_trait]
impl ToolHandler for MyToolHandler {
    async fn execute(&self, name: &str, arguments: serde_json::Value)
        -> Result<Vec<ResultContent>> {
        match name {
            "greet" => Ok(vec![ResultContent::Text {
                text: format!("Hello, {}!", arguments.get("name").and_then(|v| v.as_str()).unwrap_or("stranger"))
            }]),
            _ => Err(Error::ToolNotFound(name.to_string())),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = ServerConfig {
        name: "My Server".to_string(),
        version: "1.0.0".to_string(),
        capabilities: ServerCapabilities {
            tools: Some(ToolsCapability { list_changed: Some(false) }),
            resources: None,  // Not implemented yet
            prompts: None,    // Not implemented yet
        },
    };

    let server = McpServer::new(config, Arc::new(MyToolHandler));

    server.register_tool(Tool {
        name: "greet".to_string(),
        description: Some("Greet someone".to_string()),
        input_schema: None,
    });

    Ok(())
}
```

**Examples:**
- `cargo run` - Minimal server (1 tool)
- `cargo run --example server_with_tools` - Full example (8 tools + Inspector)

---

### Use MCP Client
```rust
use mcp_framework::prelude::*;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    let client = McpClient::new("http://localhost:3000");

    // List all tools
    let tools = client.list_tools().await?;
    println!("Available tools: {:?}", tools);

    // Call a tool
    let result = client.call_tool("echo", json!({
        "message": "Hello, MCP!"
    })).await?;
    println!("Result: {:?}", result);

    Ok(())
}
```

**Example:**
- `cargo run --example client_usage` - Full client usage example

---

### Debug with Inspector

Test and debug MCP servers with a web UI.

```bash
cargo run --example server_with_tools
# Open browser to: http://localhost:8123
```

Features:
- View registered tools
- Test tools with auto-generated forms
- Request/response history
- Real-time output inspection

---

## Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run with release optimizations
cargo test --release
```

---

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

---

## License

MIT License - see LICENSE file for details

---

<div align="center">

**Made with ❤️ for the MCP community**

[Report Issues](https://github.com/koki7o/mcp-framework/issues) • [Discussions](https://github.com/koki7o/mcp-framework/discussions)

</div>
