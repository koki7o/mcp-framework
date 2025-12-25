//! Browser Automation with Anthropic Claude
//!
//! This example demonstrates browser automation using Playwright MCP with Anthropic Claude.
//! The agent navigates websites and performs tasks autonomously.
//!
//! Run with:
//! ```bash
//! cargo run --example browser_agent_anthropic
//! ```
//!
//! Prerequisites:
//! - Install Playwright MCP: `npm install -g @playwright/mcp@latest`
//! - Install Firefox browser: `npx playwright install firefox`
//! - Set ANTHROPIC_API_KEY in .env file

use mcp_framework::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    mcp_framework::load_env();

    println!("Browser Automation with Anthropic Claude\n");

    // Use Playwright-installed Firefox with isolated profile
    // Note: Chromium has compatibility issues with Playwright MCP
    let client = McpClient::new("stdio://npx @playwright/mcp --browser firefox --isolated");

    // Create Anthropic LLM provider
    // Note: Anthropic adapter doesn't need with_mcp_client() - the Agent handles tool execution
    let llm = AnthropicAdapter::from_env("claude-sonnet-4-20250514".to_string())?;

    // Create agent
    let mut agent = Agent::new(
        client,
        std::sync::Arc::new(llm),
        AgentConfig {
            max_iterations: 30,
            max_tokens: Some(4096),
        },
    );

    // Run browser automation task
    let result = agent
        .run(
            "Navigate to https://github.com/koki7o/mcp-framework, \
             find the main topic of the project and write a summary",
        )
        .await?;

    println!("\n{}", result);

    Ok(())
}
