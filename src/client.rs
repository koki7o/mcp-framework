/// MCP Client for communicating with MCP servers
///
/// Intelligently handles multiple transport types based on URL scheme:
/// - `http://host:port` → HTTP transport
/// - `https://host:port` → HTTPS transport
/// - `stdio://command args...` → Subprocess transport
///
/// The client automatically selects the appropriate connector based on configuration.
/// No need to specify transport types - just provide URLs or configs and it works.

use crate::protocol::*;
use crate::error::{Error, Result};
use crate::config::MCPServerConfig;
use crate::session::Session;
use crate::connectors::StdioConnector;
use crate::connectors::base::Connector;
use crate::connectors::http::HttpConnector;
use serde_json::Value;
use std::collections::HashMap;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// MCP Client - Main entry point for MCP communication
///
/// Supports both single-server and multi-server operation with auto-detected transports.
///
/// # Single-Server Mode
/// ```ignore
/// let client = McpClient::new("http://localhost:3000");
/// let tools = client.list_tools().await?;
/// let result = client.call_tool("tool_name", json!({})).await?;
/// ```
///
/// # Multi-Server Mode
/// ```ignore
/// let mut client = McpClient::new_multi();
/// client.add_server(MCPServerConfig::http("db", "http://localhost:3000"));
/// client.add_server(MCPServerConfig::from_command("playwright", "npx @playwright/mcp"));
/// client.create_all_sessions().await?;
///
/// let tools = client.list_all_tools().await?;
/// let result = client.call_tool_on_server("server_name", "tool_name", json!({})).await?;
/// ```
#[derive(Clone)]
pub struct McpClient {
    // Single-server mode
    url: Option<String>,
    session: Option<Arc<Mutex<Session>>>,

    // Multi-server mode
    servers_config: HashMap<String, MCPServerConfig>,
    sessions: Arc<DashMap<String, Session>>,

    // Shared state
    initialized: Arc<Mutex<bool>>,
}

impl McpClient {
    /// Create a new MCP client for a single server
    ///
    /// Automatically detects the transport from the URL scheme.
    /// Does not connect until you call `initialize()` or perform an operation.
    ///
    /// # Examples
    /// ```ignore
    /// // HTTP
    /// let client = McpClient::new("http://localhost:3000");
    ///
    /// // Subprocess (Playwright)
    /// let client = McpClient::new("stdio://npx @playwright/mcp");
    /// ```
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: Some(url.into()),
            session: None,
            servers_config: HashMap::new(),
            sessions: Arc::new(DashMap::new()),
            initialized: Arc::new(Mutex::new(false)),
        }
    }

    /// Create a new empty client for multi-server mode
    ///
    /// Use `add_server()` to configure servers, then `create_all_sessions()` to connect.
    pub fn new_multi() -> Self {
        Self {
            url: None,
            session: None,
            servers_config: HashMap::new(),
            sessions: Arc::new(DashMap::new()),
            initialized: Arc::new(Mutex::new(false)),
        }
    }

    /// Add a server configuration (multi-server mode)
    pub fn add_server(&mut self, config: MCPServerConfig) {
        self.servers_config.insert(config.name.clone(), config);
    }

    /// Get list of configured server names
    pub fn server_names(&self) -> Vec<String> {
        self.servers_config.keys().cloned().collect()
    }

    /// Create a connector from a URL by detecting the scheme
    fn create_connector_from_url(url: &str) -> Result<Box<dyn Connector>> {
        if url.starts_with("http://") || url.starts_with("https://") {
            // HTTP/HTTPS transport
            let config = crate::connectors::base::ConnectorConfig {
                url: url.to_string(),
                timeout_secs: 30,
                retry_attempts: 3,
            };
            Ok(Box::new(HttpConnector::new(config)))
        } else if url.starts_with("stdio://") {
            // Stdio/subprocess transport
            let command_part = &url[8..]; // Remove "stdio://"
            let parts: Vec<&str> = command_part.split_whitespace().collect();

            if parts.is_empty() {
                return Err(Error::InvalidRequest("No command specified in stdio:// URL".to_string()));
            }

            let command = parts[0].to_string();
            let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

            Ok(Box::new(StdioConnector::new(command, args)))
        } else {
            Err(Error::InvalidRequest(format!(
                "Unsupported URL scheme. Use http://, https://, or stdio:// - got: {}",
                url
            )))
        }
    }

    /// Create a session from a config
    async fn create_session_from_config(&self, config: &MCPServerConfig) -> Result<Session> {
        let url = if let Some(url) = &config.url {
            url.clone()
        } else if let (Some(cmd), Some(args)) = (&config.command, &config.args) {
            format!("stdio://{} {}", cmd, args.join(" "))
        } else {
            return Err(Error::InvalidRequest(
                format!("Server '{}' has no valid transport configuration", config.name),
            ));
        };

        let mut connector = Self::create_connector_from_url(&url)?;
        connector.connect().await?;

        let mut session = Session::new(config.name.clone(), connector);
        session.initialize().await?;

        Ok(session)
    }

    // =========================================================================
    // Single-Server Mode Operations
    // =========================================================================

    /// Initialize the single-server connection
    pub async fn initialize(&mut self) -> Result<Value> {
        if let Some(url) = &self.url {
            let connector = Self::create_connector_from_url(url)?;
            let mut session = Session::new("default", connector);
            session.connect().await?;
            let capabilities = session.initialize().await?;
            self.session = Some(Arc::new(Mutex::new(session)));
            *self.initialized.lock().await = true;
            Ok(capabilities)
        } else {
            Err(Error::InternalError("No server URL configured".to_string()))
        }
    }

    /// List tools from the server
    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        if let Some(session_arc) = &self.session {
            let mut session = session_arc.lock().await;
            session.refresh_tools().await?;
            Ok(session.get_tools())
        } else if let Some(url) = &self.url {
            // Create session on-demand if not yet initialized
            let connector = Self::create_connector_from_url(url)?;
            let mut session = Session::new("default", connector);
            session.connect().await?;
            session.initialize().await?;
            session.refresh_tools().await?;
            Ok(session.get_tools())
        } else {
            Err(Error::InternalError("No server configured".to_string()))
        }
    }

    /// Call a tool on the server
    pub async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<ToolResult> {
        if let Some(session_arc) = &self.session {
            let session = session_arc.lock().await;
            session.call_tool(tool_name, arguments).await
        } else if let Some(url) = &self.url {
            // Create session on-demand
            let connector = Self::create_connector_from_url(url)?;
            let mut session = Session::new("default", connector);
            session.connect().await?;
            session.initialize().await?;
            session.call_tool(tool_name, arguments).await
        } else {
            Err(Error::InternalError("No server configured".to_string()))
        }
    }

    /// List resources from the server
    pub async fn list_resources(&self) -> Result<Vec<Resource>> {
        if let Some(session_arc) = &self.session {
            let mut session = session_arc.lock().await;
            session.refresh_resources().await?;
            Ok(session.get_resources())
        } else if let Some(url) = &self.url {
            let connector = Self::create_connector_from_url(url)?;
            let mut session = Session::new("default", connector);
            session.connect().await?;
            session.initialize().await?;
            session.refresh_resources().await?;
            Ok(session.get_resources())
        } else {
            Err(Error::InternalError("No server configured".to_string()))
        }
    }

    /// Read a resource from the server
    pub async fn read_resource(&self, uri: &str) -> Result<String> {
        if let Some(session_arc) = &self.session {
            let session = session_arc.lock().await;
            session.read_resource(uri).await
        } else if let Some(url) = &self.url {
            let connector = Self::create_connector_from_url(url)?;
            let mut session = Session::new("default", connector);
            session.connect().await?;
            session.initialize().await?;
            session.read_resource(uri).await
        } else {
            Err(Error::InternalError("No server configured".to_string()))
        }
    }

    /// List prompts from the server
    pub async fn list_prompts(&self) -> Result<Vec<Prompt>> {
        if let Some(session_arc) = &self.session {
            let mut session = session_arc.lock().await;
            session.refresh_prompts().await?;
            Ok(session.get_prompts())
        } else if let Some(url) = &self.url {
            let connector = Self::create_connector_from_url(url)?;
            let mut session = Session::new("default", connector);
            session.connect().await?;
            session.initialize().await?;
            session.refresh_prompts().await?;
            Ok(session.get_prompts())
        } else {
            Err(Error::InternalError("No server configured".to_string()))
        }
    }

    // =========================================================================
    // Multi-Server Mode Operations
    // =========================================================================

    /// Create sessions for all configured servers
    pub async fn create_all_sessions(&self) -> Result<()> {
        let server_names = self.server_names();
        let mut errors = Vec::new();

        for name in server_names {
            if let Some(config) = self.servers_config.get(&name) {
                match self.create_session_from_config(config).await {
                    Ok(session) => {
                        self.sessions.insert(name.clone(), session);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to create session for '{}': {}", name, e);
                        errors.push((name, e));
                    }
                }
            }
        }

        if !errors.is_empty() {
            let error_msg = errors
                .iter()
                .map(|(name, e)| format!("{}: {}", name, e))
                .collect::<Vec<_>>()
                .join("; ");
            tracing::warn!("Some servers failed to connect: {}", error_msg);
        }

        Ok(())
    }

    /// List tools from a specific server (multi-server mode)
    pub async fn list_tools_for_server(&self, server_name: &str) -> Result<Vec<Tool>> {
        if let Some(mut session_ref) = self.sessions.get_mut(server_name) {
            session_ref.refresh_tools().await?;
            Ok(session_ref.get_tools())
        } else {
            Err(Error::ServerError(format!("No active session for server '{}'", server_name)))
        }
    }

    /// Call a tool on a specific server (multi-server mode)
    pub async fn call_tool_on_server(
        &self,
        server_name: &str,
        tool_name: &str,
        arguments: Value,
    ) -> Result<ToolResult> {
        if let Some(session_ref) = self.sessions.get(server_name) {
            session_ref.value().call_tool(tool_name, arguments).await
        } else {
            Err(Error::ServerError(format!("No active session for server '{}'", server_name)))
        }
    }

    /// List all tools from all servers
    pub async fn list_all_tools(&self) -> Result<Vec<(String, Vec<Tool>)>> {
        let mut all_tools = Vec::new();

        // Collect server names first to avoid holding references
        let server_names: Vec<_> = self.sessions.iter().map(|r| r.key().clone()).collect();

        for server_name in server_names {
            if let Some(mut session_ref) = self.sessions.get_mut(&server_name) {
                match session_ref.refresh_tools().await {
                    Ok(_) => {
                        let tools = session_ref.get_tools();
                        all_tools.push((server_name, tools));
                    }
                    Err(e) => {
                        tracing::warn!("Failed to list tools from '{}': {}", server_name, e);
                    }
                }
            }
        }

        Ok(all_tools)
    }

    /// Close a session
    pub async fn close_session(&self, server_name: &str) -> Result<()> {
        if let Some((_, mut session)) = self.sessions.remove(server_name) {
            session.disconnect().await?;
            tracing::info!("Closed session for server '{}'", server_name);
        }
        Ok(())
    }

    /// Close all sessions
    pub async fn close_all_sessions(&self) -> Result<()> {
        let server_names: Vec<_> = self.sessions.iter().map(|r| r.key().clone()).collect();
        for name in server_names {
            self.close_session(&name).await.ok();
        }
        Ok(())
    }

    /// Check if client is connected
    pub fn is_connected(&self) -> bool {
        self.sessions.len() > 0 || self.session.is_some()
    }
}

impl Default for McpClient {
    fn default() -> Self {
        Self::new_multi()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_new() {
        let client = McpClient::new("http://localhost:3000");
        assert!(client.url.is_some());
    }

    #[test]
    fn test_client_multi_mode() {
        let mut client = McpClient::new_multi();
        client.add_server(MCPServerConfig::http("test", "http://localhost:3000"));
        assert_eq!(client.server_names().len(), 1);
    }

    #[test]
    fn test_connector_url_detection_http() {
        let result = McpClient::create_connector_from_url("http://localhost:3000");
        assert!(result.is_ok());
    }

    #[test]
    fn test_connector_url_detection_stdio() {
        let result = McpClient::create_connector_from_url("stdio://npx @playwright/mcp");
        assert!(result.is_ok());
    }

    #[test]
    fn test_connector_url_detection_invalid() {
        let result = McpClient::create_connector_from_url("ftp://invalid");
        assert!(result.is_err());
    }
}
