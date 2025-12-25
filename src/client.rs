/// MCP Client for communicating with MCP servers.
///
/// Supports multiple transports:
/// - `http://` or `https://` - HTTP transport
/// - `stdio://command args` - Subprocess transport

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

/// MCP Client supporting single or multiple server connections.
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
    /// Create a new client for a single server.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: Some(url.into()),
            session: None,
            servers_config: HashMap::new(),
            sessions: Arc::new(DashMap::new()),
            initialized: Arc::new(Mutex::new(false)),
        }
    }

    /// Create a client for managing multiple servers.
    pub fn new_multi() -> Self {
        Self {
            url: None,
            session: None,
            servers_config: HashMap::new(),
            sessions: Arc::new(DashMap::new()),
            initialized: Arc::new(Mutex::new(false)),
        }
    }

    /// Add a server configuration.
    pub fn add_server(&mut self, config: MCPServerConfig) {
        self.servers_config.insert(config.name.clone(), config);
    }

    pub fn server_names(&self) -> Vec<String> {
        self.servers_config.keys().cloned().collect()
    }

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

    pub async fn list_tools_for_server(&self, server_name: &str) -> Result<Vec<Tool>> {
        if let Some(mut session_ref) = self.sessions.get_mut(server_name) {
            session_ref.refresh_tools().await?;
            Ok(session_ref.get_tools())
        } else {
            Err(Error::ServerError(format!("No active session for server '{}'", server_name)))
        }
    }

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

    pub async fn close_session(&self, server_name: &str) -> Result<()> {
        if let Some((_, mut session)) = self.sessions.remove(server_name) {
            session.disconnect().await?;
            tracing::info!("Closed session for server '{}'", server_name);
        }
        Ok(())
    }

    pub async fn close_all_sessions(&self) -> Result<()> {
        let server_names: Vec<_> = self.sessions.iter().map(|r| r.key().clone()).collect();
        for name in server_names {
            self.close_session(&name).await.ok();
        }
        Ok(())
    }

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
