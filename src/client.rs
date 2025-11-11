use crate::protocol::*;
use crate::error::{Error, Result};
use dashmap::DashMap;
use reqwest::Client as HttpClient;
use serde_json::{json, Value};
use std::sync::Arc;

/// MCP Client for communicating with MCP servers
#[derive(Clone)]
pub struct McpClient {
    server_url: String,
    http_client: HttpClient,
    #[allow(dead_code)]
    pending_requests: Arc<DashMap<String, tokio::sync::oneshot::Sender<JsonRpcResponse>>>,
}

impl McpClient {
    /// Create a new MCP client
    pub fn new(server_url: impl Into<String>) -> Self {
        Self {
            server_url: server_url.into(),
            http_client: HttpClient::new(),
            pending_requests: Arc::new(DashMap::new()),
        }
    }

    /// Initialize connection with server
    pub async fn initialize(&self) -> Result<Value> {
        let request = JsonRpcRequest::new("initialize", None);
        let response = self.send_request(request).await?;

        if let Some(result) = response.result {
            Ok(result)
        } else if let Some(error) = response.error {
            Err(Error::ServerError(error.message))
        } else {
            Err(Error::InternalError("No result or error in response".to_string()))
        }
    }

    /// List available tools on the server
    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
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
    pub async fn call_tool(&self, name: &str, arguments: serde_json::Value) -> Result<ToolResult> {
        let params = json!({
            "name": name,
            "arguments": arguments
        });

        let request = JsonRpcRequest::new("tools/call", Some(params));
        let response = self.send_request(request).await?;

        if let Some(result) = response.result {
            let tool_result = serde_json::from_value::<ToolResult>(result)
                .map_err(|e| Error::SerializationError(e))?;
            Ok(tool_result)
        } else if let Some(error) = response.error {
            Err(Error::ServerError(error.message))
        } else {
            Err(Error::InternalError("No result in response".to_string()))
        }
    }

    /// List available resources
    pub async fn list_resources(&self) -> Result<Vec<Resource>> {
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

    /// Read a resource
    pub async fn read_resource(&self, uri: &str) -> Result<String> {
        let params = json!({ "uri": uri });
        let request = JsonRpcRequest::new("resources/read", Some(params));
        let response = self.send_request(request).await?;

        if let Some(result) = response.result {
            let content = result
                .get("contents")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::InvalidRequest("Invalid resource content".to_string()))?
                .to_string();
            Ok(content)
        } else if let Some(error) = response.error {
            Err(Error::ServerError(error.message))
        } else {
            Err(Error::InternalError("No result in response".to_string()))
        }
    }

    /// List available prompts
    pub async fn list_prompts(&self) -> Result<Vec<Prompt>> {
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

    /// Get a specific prompt
    pub async fn get_prompt(&self, name: &str) -> Result<Prompt> {
        let params = json!({ "name": name });
        let request = JsonRpcRequest::new("prompts/get", Some(params));
        let response = self.send_request(request).await?;

        if let Some(result) = response.result {
            let prompt = serde_json::from_value::<Prompt>(result)
                .map_err(|e| Error::SerializationError(e))?;
            Ok(prompt)
        } else if let Some(error) = response.error {
            Err(Error::ServerError(error.message))
        } else {
            Err(Error::InternalError("No result in response".to_string()))
        }
    }

    /// Send a JSON-RPC request to the server
    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        let json_body = serde_json::to_string(&request)
            .map_err(|e| Error::SerializationError(e))?;

        let response = self
            .http_client
            .post(&self.server_url)
            .header("Content-Type", "application/json")
            .body(json_body)
            .send()
            .await
            .map_err(|e| Error::RequestError(e.to_string()))?;

        let response_text = response
            .text()
            .await
            .map_err(|e| Error::RequestError(e.to_string()))?;

        let json_response: JsonRpcResponse = serde_json::from_str(&response_text)
            .map_err(|e| Error::SerializationError(e))?;

        Ok(json_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = McpClient::new("http://localhost:8000");
        assert_eq!(client.server_url, "http://localhost:8000");
    }
}
