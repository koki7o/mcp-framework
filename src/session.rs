/// Session to an MCP server. Wraps a connector and caches tools/resources/prompts.

use crate::connectors::base::Connector;
use crate::protocol::{Tool, Resource, Prompt, ToolResult};
use crate::error::Result;
use serde_json::Value;
use std::collections::HashMap;

pub struct Session {
    /// Unique name for this session (usually the server name)
    pub name: String,

    /// The underlying connector (HTTP, Stdio, SSE, etc.)
    connector: Box<dyn Connector>,

    /// Whether the session has been initialized
    initialized: bool,

    /// Cached tools from the server (refreshed when needed)
    tools_cache: HashMap<String, Tool>,

    /// Cached resources from the server
    resources_cache: HashMap<String, Resource>,

    /// Cached prompts from the server
    prompts_cache: HashMap<String, Prompt>,
}

impl Session {
    /// Create a new session with a connector
    pub fn new(name: impl Into<String>, connector: Box<dyn Connector>) -> Self {
        Self {
            name: name.into(),
            connector,
            initialized: false,
            tools_cache: HashMap::new(),
            resources_cache: HashMap::new(),
            prompts_cache: HashMap::new(),
        }
    }

    /// Establish the connection and initialize with the server
    pub async fn connect(&mut self) -> Result<()> {
        self.connector.connect().await?;
        Ok(())
    }

    /// Initialize the session (send initialize request to server)
    pub async fn initialize(&mut self) -> Result<Value> {
        let capabilities = self.connector.initialize().await?;
        self.initialized = true;
        self.refresh_tools().await.ok(); // Cache tools, but don't fail if it doesn't work
        Ok(capabilities)
    }

    /// Check if the session is connected
    pub fn is_connected(&self) -> bool {
        self.connector.is_connected()
    }

    /// Check if the session has been initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Disconnect from the server
    pub async fn disconnect(&mut self) -> Result<()> {
        self.connector.disconnect().await?;
        self.initialized = false;
        Ok(())
    }

    // =========================================================================
    // Tools
    // =========================================================================

    /// Refresh the tools cache by fetching from the server
    pub async fn refresh_tools(&mut self) -> Result<()> {
        let tools = self.connector.list_tools().await?;
        self.tools_cache.clear();
        for tool in tools {
            self.tools_cache.insert(tool.name.clone(), tool);
        }
        Ok(())
    }

    /// Get all cached tools
    pub fn get_tools(&self) -> Vec<Tool> {
        self.tools_cache.values().cloned().collect()
    }

    /// Get a specific tool by name
    pub fn get_tool(&self, name: &str) -> Option<Tool> {
        self.tools_cache.get(name).cloned()
    }

    /// Call a tool on the server
    pub async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<ToolResult> {
        self.connector.call_tool(tool_name, arguments).await
    }

    // =========================================================================
    // Resources
    // =========================================================================

    /// Refresh the resources cache by fetching from the server
    pub async fn refresh_resources(&mut self) -> Result<()> {
        let resources = self.connector.list_resources().await?;
        self.resources_cache.clear();
        for resource in resources {
            self.resources_cache.insert(resource.uri.clone(), resource);
        }
        Ok(())
    }

    /// Get all cached resources
    pub fn get_resources(&self) -> Vec<Resource> {
        self.resources_cache.values().cloned().collect()
    }

    /// Get a specific resource by URI
    pub fn get_resource(&self, uri: &str) -> Option<Resource> {
        self.resources_cache.get(uri).cloned()
    }

    /// Read a resource from the server
    pub async fn read_resource(&self, uri: &str) -> Result<String> {
        self.connector.read_resource(uri).await
    }

    // =========================================================================
    // Prompts
    // =========================================================================

    /// Refresh the prompts cache by fetching from the server
    pub async fn refresh_prompts(&mut self) -> Result<()> {
        let prompts = self.connector.list_prompts().await?;
        self.prompts_cache.clear();
        for prompt in prompts {
            self.prompts_cache.insert(prompt.name.clone(), prompt);
        }
        Ok(())
    }

    /// Get all cached prompts
    pub fn get_prompts(&self) -> Vec<Prompt> {
        self.prompts_cache.values().cloned().collect()
    }

    /// Get a specific prompt by name
    pub fn get_prompt_info(&self, name: &str) -> Option<Prompt> {
        self.prompts_cache.get(name).cloned()
    }

    /// Get a prompt from the server
    pub async fn get_prompt(&self, name: &str, arguments: Option<Value>) -> Result<Value> {
        self.connector.get_prompt(name, arguments).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        // Mock connector for testing
        struct MockConnector;

        #[async_trait::async_trait]
        impl Connector for MockConnector {
            async fn send_request(
                &self,
                _request: crate::protocol::JsonRpcRequest,
            ) -> Result<crate::protocol::JsonRpcResponse> {
                Ok(crate::protocol::JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: "1".to_string(),
                    result: None,
                    error: None,
                })
            }

            async fn connect(&mut self) -> Result<()> {
                Ok(())
            }

            async fn disconnect(&mut self) -> Result<()> {
                Ok(())
            }

            fn is_connected(&self) -> bool {
                true
            }
        }

        let connector = Box::new(MockConnector);
        let session = Session::new("test", connector);
        assert_eq!(session.name, "test");
        assert!(!session.is_initialized());
    }
}
