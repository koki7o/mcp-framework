/// Configuration management for MCP applications
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for a single MCP server connection
///
/// Supports multiple transport types:
/// - HTTP/HTTPS: `url: "http://localhost:3000"`
/// - Stdio/subprocess: `command: "npx"`, `args: ["@playwright/mcp"]`
/// - SSE: `url: "http://localhost:3000/events"`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
    /// Display name for this server
    pub name: String,

    /// URL for HTTP/HTTPS/SSE connections
    /// Example: "http://localhost:3000", "https://example.com/mcp"
    #[serde(default)]
    pub url: Option<String>,

    /// Command for stdio subprocess connections
    /// Example: "npx"
    #[serde(default)]
    pub command: Option<String>,

    /// Arguments for stdio subprocess connections
    /// Example: ["@playwright/mcp"]
    #[serde(default)]
    pub args: Option<Vec<String>>,

    /// Environment variables for subprocess
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,

    /// Headers for HTTP connections
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,

    /// Whether to auto-connect on startup
    #[serde(default = "default_true")]
    pub auto_connect: bool,
}

/// Helper function for serde default value
fn default_true() -> bool {
    true
}

impl MCPServerConfig {
    /// Create a new HTTP/HTTPS server config
    pub fn http(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            url: Some(url.into()),
            command: None,
            args: None,
            env: None,
            headers: None,
            auto_connect: true,
        }
    }

    /// Create a new stdio/subprocess server config
    pub fn stdio(name: impl Into<String>, command: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            name: name.into(),
            url: None,
            command: Some(command.into()),
            args: Some(args),
            env: None,
            headers: None,
            auto_connect: true,
        }
    }

    /// Create a stdio config from a shell command string
    /// Example: "npx @playwright/mcp"
    pub fn from_command(name: impl Into<String>, command_str: &str) -> Self {
        let parts: Vec<&str> = command_str.split_whitespace().collect();
        if parts.is_empty() {
            return Self {
                name: name.into(),
                url: None,
                command: None,
                args: None,
                env: None,
                headers: None,
                auto_connect: true,
            };
        }

        Self {
            name: name.into(),
            url: None,
            command: Some(parts[0].to_string()),
            args: Some(parts[1..].iter().map(|s| s.to_string()).collect()),
            env: None,
            headers: None,
            auto_connect: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_server_config_http() {
        let config = MCPServerConfig::http("test", "http://localhost:3000");
        assert_eq!(config.name, "test");
        assert_eq!(config.url, Some("http://localhost:3000".to_string()));
    }

    #[test]
    fn test_mcp_server_config_from_command() {
        let config = MCPServerConfig::from_command("playwright", "npx @playwright/mcp");
        assert_eq!(config.name, "playwright");
        assert_eq!(config.command, Some("npx".to_string()));
    }
}
