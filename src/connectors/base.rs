/// Base connector trait for MCP connections
use crate::protocol::{JsonRpcRequest, JsonRpcResponse, Tool, ToolResult, Resource, Prompt};
use crate::error::{Error, Result};
use serde_json::Value;

/// Configuration for connector
#[derive(Debug, Clone)]
pub struct ConnectorConfig {
    pub url: String,
    pub timeout_secs: u64,
    pub retry_attempts: usize,
}

impl Default for ConnectorConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:3000".to_string(),
            timeout_secs: 30,
            retry_attempts: 3,
        }
    }
}

/// Trait for different connection transports
///
/// Connectors handle the low-level communication with MCP servers.
/// Different transport mechanisms (HTTP, Stdio, SSE, WebSocket) all implement this trait.
///
/// The trait provides both low-level (send_request) and high-level methods (list_tools, call_tool).
/// Default implementations of high-level methods use send_request, but can be overridden
/// for transport-specific optimizations.
#[async_trait::async_trait]
pub trait Connector: Send + Sync {
    /// Send a raw JSON-RPC request and receive a response
    ///
    /// This is the fundamental operation all connectors must implement.
    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse>;

    /// Establish a connection to the MCP server
    async fn connect(&mut self) -> Result<()>;

    /// Close the connection to the MCP server
    async fn disconnect(&mut self) -> Result<()>;

    /// Check if connector is currently connected
    fn is_connected(&self) -> bool;

    // =====================================================================
    // High-level operations with default implementations
    // =====================================================================
    // These can be overridden by specific transports for optimization

    /// Initialize the MCP connection
    ///
    /// Sends the initialize request and returns server capabilities
    async fn initialize(&self) -> Result<Value> {
        let params = serde_json::json!({
            "protocolVersion": "2025-11-05",
            "capabilities": {
                "sampling": {}
            },
            "clientInfo": {
                "name": "mcp-framework",
                "version": "0.1.0"
            }
        });
        let request = JsonRpcRequest::new("initialize", Some(params));
        let response = self.send_request(request).await?;

        if let Some(result) = response.result {
            Ok(result)
        } else if let Some(error) = response.error {
            Err(Error::ServerError(error.message))
        } else {
            Err(Error::InternalError("No result or error in response".to_string()))
        }
    }

    /// List all available tools from the server
    async fn list_tools(&self) -> Result<Vec<Tool>> {
        let request = JsonRpcRequest::new("tools/list", None);
        let response = self.send_request(request).await?;

        if let Some(result) = response.result {
            let tools = result
                .get("tools")
                .and_then(|v| serde_json::from_value::<Vec<Tool>>(v.clone()).ok())
                .ok_or_else(|| Error::InvalidRequest("Invalid tools response".to_string()))?;
            Ok(tools)
        } else if let Some(error) = response.error {
            Err(Error::ServerError(error.message))
        } else {
            Err(Error::InternalError("No result in response".to_string()))
        }
    }

    /// Call a tool on the server
    async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<ToolResult> {
        let params = serde_json::json!({
            "name": tool_name,
            "arguments": arguments,
        });
        let request = JsonRpcRequest::new("tools/call", Some(params));
        let response = self.send_request(request).await?;

        if let Some(result) = response.result {
            serde_json::from_value::<ToolResult>(result)
                .map_err(|e| Error::InvalidRequest(format!("Invalid tool result: {}", e)))
        } else if let Some(error) = response.error {
            Err(Error::ServerError(error.message))
        } else {
            Err(Error::InternalError("No result in response".to_string()))
        }
    }

    /// List all available resources from the server
    async fn list_resources(&self) -> Result<Vec<Resource>> {
        let request = JsonRpcRequest::new("resources/list", None);
        let response = self.send_request(request).await?;

        if let Some(result) = response.result {
            let resources = result
                .get("resources")
                .and_then(|v| serde_json::from_value::<Vec<Resource>>(v.clone()).ok())
                .ok_or_else(|| Error::InvalidRequest("Invalid resources response".to_string()))?;
            Ok(resources)
        } else if let Some(error) = response.error {
            Err(Error::ServerError(error.message))
        } else {
            Err(Error::InternalError("No result in response".to_string()))
        }
    }

    /// Read a specific resource from the server
    async fn read_resource(&self, uri: &str) -> Result<String> {
        let params = serde_json::json!({
            "uri": uri,
        });
        let request = JsonRpcRequest::new("resources/read", Some(params));
        let response = self.send_request(request).await?;

        if let Some(result) = response.result {
            let content = result
                .get("contents")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|item| item.get("text"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::InvalidRequest("Invalid resource content".to_string()))?;
            Ok(content.to_string())
        } else if let Some(error) = response.error {
            Err(Error::ServerError(error.message))
        } else {
            Err(Error::InternalError("No result in response".to_string()))
        }
    }

    /// List all available prompts from the server
    async fn list_prompts(&self) -> Result<Vec<Prompt>> {
        let request = JsonRpcRequest::new("prompts/list", None);
        let response = self.send_request(request).await?;

        if let Some(result) = response.result {
            let prompts = result
                .get("prompts")
                .and_then(|v| serde_json::from_value::<Vec<Prompt>>(v.clone()).ok())
                .ok_or_else(|| Error::InvalidRequest("Invalid prompts response".to_string()))?;
            Ok(prompts)
        } else if let Some(error) = response.error {
            Err(Error::ServerError(error.message))
        } else {
            Err(Error::InternalError("No result in response".to_string()))
        }
    }

    /// Get a specific prompt from the server
    async fn get_prompt(&self, name: &str, arguments: Option<Value>) -> Result<Value> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments.unwrap_or(serde_json::json!({})),
        });
        let request = JsonRpcRequest::new("prompts/get", Some(params));
        let response = self.send_request(request).await?;

        if let Some(result) = response.result {
            Ok(result)
        } else if let Some(error) = response.error {
            Err(Error::ServerError(error.message))
        } else {
            Err(Error::InternalError("No result in response".to_string()))
        }
    }
}
