<div align="center">

<!-- Banner Image - Add your banner.png to docs/ directory -->
<img src="assets/banner.png" alt="MCP Framework Banner" width="800" style="margin-bottom: 20px;">

# 🚀 MCP Framework - Rust Implementation

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/badge/MCP%20Framework-Rust-orange?style=for-the-badge&logo=rust&logoColor=white">
  <img alt="MCP Framework" src="https://img.shields.io/badge/MCP%20Framework-Rust-orange?style=for-the-badge&logo=rust&logoColor=white">
</picture>

**Production-Ready Rust Implementation** of the [Model Context Protocol](https://modelcontextprotocol.io) with blazing-fast performance, comprehensive tools, and a web-based inspector.

---

<p>
  <a href="https://github.com/anthropics/mcp/blob/main/LICENSE">
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

## 🌐 What is mcp-framework?

**mcp-framework** is a complete, production-ready Rust implementation of the Model Context Protocol, enabling you to:

- 🤖 **Build AI Agents** - Create intelligent agents with LLM integration (Claude, OpenAI) and multi-step reasoning
- 🛠️ **Create MCP Servers** - Register tools, resources, and prompts easily
- 📡 **Connect to MCP Servers** - HTTP client for programmatic tool access
- 🔍 **Debug with Inspector** - Beautiful web-based dashboard for testing tools
- ⚡ **High Performance** - Blazing-fast Rust implementation
- 🛡️ **Type-Safe** - Leverage Rust's type system for safety and reliability

---

## ✨ Key Features

### 🎯 Core Components

| Feature | Status | Details |
|---------|--------|---------|
| **MCP Server** | ✅ Complete | Register tools, handle execution, JSON-RPC protocol |
| **MCP Client** | ✅ Complete | HTTP-based client for calling remote tools |
| **AI Agent** | ✅ Complete | Agentic loop with pluggable LLM providers |
| **Web Inspector** | ✅ Complete | Interactive UI at `http://localhost:8123` |
| **Claude Integration** | ✅ Complete | AnthropicAdapter for Claude models |
| **OpenAI Integration** | ✅ Complete | OpenAIAdapter for GPT models |
| **Protocol Types** | ✅ Complete | Tools and Messages (Core MCP protocol) |
| **Session Management** | ✅ Complete | Conversation history in Agent |
| **Resources** | ⏳ Planned | For serving files and data to clients |
| **Prompts** | ⏳ Planned | Callable prompt templates with dynamic generation |
| **Authentication** | ⏳ Planned | Bearer tokens, OAuth 2.0 support |
| **Configuration** | ⏳ Planned | File-based config loading |
| **.env Support** | ✅ Complete | Load API keys from environment files |

### 🛠️ 8 Built-in Example Tools

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

## 📦 Quick Start

### Prerequisites

```bash
# Requires Rust 1.70+
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 1. Clone & Setup

```bash
git clone <repo-url>
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

---

## 🎯 What Do You Want to Build?

### 🤖 Build an AI Agent

Create intelligent agents that can use MCP tools to accomplish complex tasks.

**Quick Example:**
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

### 🛠️ Create an MCP Server

Build your own MCP servers with custom tools.

**Quick Example:**
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
            resources: None,
            prompts: None,
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
- `cargo run --example server_with_tools` - Comprehensive example (8 tools + Inspector)

---

### 📡 Use MCP Client

Connect to MCP servers and call tools programmatically.

**Quick Example:**
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

### 🔍 Debug with Inspector

Test and debug MCP servers interactively with a web-based UI.

```bash
cargo run --example server_with_tools
# Open browser to: http://localhost:8123
```

The Inspector provides:
- 📋 View all registered tools with descriptions
- 🧪 Test tools interactively with auto-generated forms
- 📊 See full request/response history
- 🔍 Inspect tool outputs and errors in real-time

---

## 📁 Project Structure

```
mcp-framework/
├── src/
│   ├── lib.rs                ← Main library entry point (prelude + exports)
│   ├── protocol.rs           ← MCP type definitions (Tools, Messages, Protocol)
│   ├── server.rs             ← McpServer implementation & tool registration
│   ├── client.rs             ← McpClient implementation (HTTP-based)
│   ├── agent.rs              ← AI Agent with agentic loop & LLM integration
│   ├── inspector.rs          ← Web-based debugging UI (localhost:8123)
│   ├── error.rs              ← Error types and JSON-RPC codes
│   └── adapters/
│       ├── mod.rs
│       ├── anthropic.rs      ← Claude (Anthropic) LLM adapter
│       └── openai.rs         ← OpenAI GPT LLM adapter
├── examples/
│   ├── server_with_tools.rs              ← 8-tool server with Inspector
│   ├── anthropic_agent_demo_with_tools.rs ← Claude agent example
│   ├── openai_agent_demo_with_tools.rs    ← OpenAI agent example
│   └── client_usage.rs                    ← Client usage example
├── assets/
│   └── banner.png           
├── Cargo.toml
├── Cargo.lock
├── LICENSE                   ← MIT License
├── .env.example              ← Environment variables template
├── .gitignore
└── README.md
```

---

## 🚀 Core API Reference

### Create a Server

```rust
let config = ServerConfig {
    name: "My Server".to_string(),
    version: "1.0.0".to_string(),
    capabilities: ServerCapabilities {
        tools: Some(ToolsCapability { list_changed: Some(false) }),
        resources: None,  // Not implemented yet
        prompts: None,    // Not implemented yet
    },
};

let handler = Arc::new(MyToolHandler);
let server = McpServer::new(config, handler);
```

### Register a Tool

```rust
use std::collections::HashMap;
use serde_json::json;

let mut properties = HashMap::new();
properties.insert("param".to_string(), json!({"type": "string"}));

server.register_tool(Tool {
    name: "my_tool".to_string(),
    description: Some("Does something useful".to_string()),
    input_schema: Some(ToolInputSchema {
        schema_type: "object".to_string(),
        properties,
        required: Some(vec!["param".to_string()]),
    }),
});
```

### Implement ToolHandler

```rust
#[async_trait::async_trait]
impl ToolHandler for MyHandler {
    async fn execute(&self, name: &str, arguments: Value)
        -> Result<Vec<ResultContent>> {
        match name {
            "my_tool" => {
                // Extract and validate arguments
                let param = arguments.get("param")
                    .and_then(|v| v.as_str())?;

                // Implement your logic
                let result = do_something(param);

                Ok(vec![ResultContent::Text {
                    text: result.to_string()
                }])
            }
            _ => Err(Error::ToolNotFound(name.to_string())),
        }
    }
}
```

### Create an Agent

```rust
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    mcp_framework::load_env();

    let client = McpClient::new("http://localhost:3000");
    let llm = AnthropicAdapter::from_env("claude-sonnet-4-5-20250929".to_string())?;

    let mut agent = Agent::new(client, Arc::new(llm), AgentConfig {
        max_iterations: 10,
        max_tokens: Some(2048),
    });

    let response = agent.run("Your query here").await?;
    println!("Response: {}", response);

    Ok(())
}
```

### Use the Client

```rust
let client = McpClient::new("http://localhost:3000");

// List available tools
let tools = client.list_tools().await?;

// Call a tool
let result = client.call_tool("echo", json!({
    "message": "Hello!"
})).await?;
```

---

## 🧪 Testing

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

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

---

## 📄 License

MIT License - see LICENSE file for details

---

## 🔗 Resources

- **[Model Context Protocol](https://modelcontextprotocol.io)** - Official MCP website
- **[MCP Specification](https://spec.modelcontextprotocol.io/)** - Official protocol specification
- **[Rust Book](https://doc.rust-lang.org/book/)** - Learn Rust
- **[Tokio Docs](https://tokio.rs/)** - Async runtime documentation
- **[Serde Documentation](https://serde.rs/)** - Serialization framework

---

<div align="center">

**Made with ❤️ for the MCP community**

[Report Issues](https://github.com/your-repo/issues) • [Discussions](https://github.com/your-repo/discussions) • [Contributing](./CONTRIBUTING.md)

</div>
