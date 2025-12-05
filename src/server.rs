use crate::protocol::*;
use crate::error::{Error, Result};
use async_trait::async_trait;
use dashmap::DashMap;
use serde_json::{json, Value};
use std::sync::Arc;

/// Handler for tool execution
#[async_trait]
pub trait ToolHandler: Send + Sync {
    async fn execute(&self, name: &str, arguments: Value) -> Result<Vec<ResultContent>>;
}

/// Handler for resource operations
#[async_trait]
pub trait ResourceHandler: Send + Sync {
    async fn get(&self, uri: &str) -> Result<Resource>;
    async fn list(&self) -> Result<Vec<Resource>>;
}

/// Handler for prompt operations
#[async_trait]
pub trait PromptHandler: Send + Sync {
    async fn get(&self, name: &str) -> Result<Prompt>;
    async fn list(&self) -> Result<Vec<Prompt>>;
}

/// MCP Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub name: String,
    pub version: String,
    pub capabilities: ServerCapabilities,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            name: "MCP Server".to_string(),
            version: "1.0.0".to_string(),
            capabilities: ServerCapabilities::default(),
        }
    }
}

/// MCP Server
pub struct McpServer {
    config: ServerConfig,
    tools: Arc<DashMap<String, Tool>>,
    resources: Arc<DashMap<String, Resource>>,
    prompts: Arc<DashMap<String, Prompt>>,
    tool_handler: Arc<dyn ToolHandler>,
    resource_handler: Option<Arc<dyn ResourceHandler>>,
    prompt_handler: Option<Arc<dyn PromptHandler>>,
}

impl McpServer {
    /// Create a new MCP server
    pub fn new(config: ServerConfig, tool_handler: Arc<dyn ToolHandler>) -> Self {
        Self {
            config,
            tools: Arc::new(DashMap::new()),
            resources: Arc::new(DashMap::new()),
            prompts: Arc::new(DashMap::new()),
            tool_handler,
            resource_handler: None,
            prompt_handler: None,
        }
    }

    /// Register a tool
    pub fn register_tool(&self, tool: Tool) {
        self.tools.insert(tool.name.to_string(), tool);
    }

    /// Register a resource
    pub fn register_resource(&self, resource: Resource) {
        self.resources.insert(resource.uri.clone(), resource);
    }

    /// Register a prompt
    pub fn register_prompt(&self, prompt: Prompt) {
        self.prompts.insert(prompt.name.to_string(), prompt);
    }

    /// Set resource handler
    pub fn set_resource_handler(&mut self, handler: Arc<dyn ResourceHandler>) {
        self.resource_handler = Some(handler);
    }

    /// Set prompt handler
    pub fn set_prompt_handler(&mut self, handler: Arc<dyn PromptHandler>) {
        self.prompt_handler = Some(handler);
    }

    /// Handle initialize request
    pub async fn handle_initialize(&self) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: "1".to_string(),
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": self.config.capabilities,
                "serverInfo": {
                    "name": self.config.name,
                    "version": self.config.version,
                }
            })),
            error: None,
        }
    }

    /// Handle tools/list request
    pub async fn handle_tools_list(&self) -> Result<Vec<Tool>> {
        Ok(self
            .tools
            .iter()
            .map(|entry| entry.value().clone())
            .collect())
    }

    /// Handle tools/call request
    pub async fn handle_tool_call(&self, name: &str, arguments: Value) -> Result<ToolResult> {
        // Verify tool exists
        if !self.tools.contains_key(name) {
            return Err(Error::ToolNotFound(name.to_string()));
        }

        let content = self.tool_handler.execute(name, arguments).await?;

        Ok(ToolResult {
            id: Some(uuid::Uuid::new_v4().to_string()),
            content,
            is_error: None,
        })
    }

    /// Handle resources/list request
    pub async fn handle_resources_list(&self) -> Result<Vec<Resource>> {
        if let Some(handler) = &self.resource_handler {
            handler.list().await
        } else {
            Ok(self
                .resources
                .iter()
                .map(|entry| entry.value().clone())
                .collect())
        }
    }

    /// Handle resources/read request
    pub async fn handle_resource_read(&self, uri: &str) -> Result<String> {
        if let Some(handler) = &self.resource_handler {
            let resource = handler.get(uri).await?;
            Ok(resource.uri.to_string())
        } else if let Some(resource) = self.resources.get(uri) {
            Ok(resource.uri.to_string())
        } else {
            Err(Error::ResourceNotFound(uri.to_string()))
        }
    }

    /// Handle prompts/list request
    pub async fn handle_prompts_list(&self) -> Result<Vec<Prompt>> {
        if let Some(handler) = &self.prompt_handler {
            handler.list().await
        } else {
            Ok(self
                .prompts
                .iter()
                .map(|entry| entry.value().clone())
                .collect())
        }
    }

    /// Handle prompts/get request
    pub async fn handle_prompt_get(&self, name: &str) -> Result<Prompt> {
        if let Some(handler) = &self.prompt_handler {
            handler.get(name).await
        } else if let Some(prompt) = self.prompts.get(name) {
            Ok(prompt.value().clone())
        } else {
            Err(Error::ToolNotFound(name.to_string()))
        }
    }

    /// Handle a JSON-RPC request
    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize().await.result,
            "tools/list" => match self.handle_tools_list().await {
                Ok(tools) => Some(json!({ "tools": tools })),
                Err(e) => {
                    return JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id.clone(),
                        result: None,
                        error: Some(JsonRpcError {
                            code: e.error_code(),
                            message: e.to_string(),
                            data: None,
                        }),
                    }
                }
            },
            "tools/call" => {
                let params = match request.params {
                    Some(p) => p,
                    None => {
                        return JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: request.id.clone(),
                            result: None,
                            error: Some(JsonRpcError {
                                code: -32602,
                                message: "Missing params".to_string(),
                                data: None,
                            }),
                        }
                    }
                };

                let name = match params.get("name").and_then(|v| v.as_str()) {
                    Some(n) => n,
                    None => {
                        return JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: request.id.clone(),
                            result: None,
                            error: Some(JsonRpcError {
                                code: -32602,
                                message: "Missing tool name".to_string(),
                                data: None,
                            }),
                        }
                    }
                };

                let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

                match self.handle_tool_call(name, arguments).await {
                    Ok(result) => Some(json!(result)),
                    Err(e) => {
                        return JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: request.id.clone(),
                            result: None,
                            error: Some(JsonRpcError {
                                code: e.error_code(),
                                message: e.to_string(),
                                data: None,
                            }),
                        }
                    }
                }
            }
            "resources/list" => match self.handle_resources_list().await {
                Ok(resources) => Some(json!({ "resources": resources })),
                Err(e) => {
                    return JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id.clone(),
                        result: None,
                        error: Some(JsonRpcError {
                            code: e.error_code(),
                            message: e.to_string(),
                            data: None,
                        }),
                    }
                }
            },
            "prompts/list" => match self.handle_prompts_list().await {
                Ok(prompts) => Some(json!({ "prompts": prompts })),
                Err(e) => {
                    return JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id.clone(),
                        result: None,
                        error: Some(JsonRpcError {
                            code: e.error_code(),
                            message: e.to_string(),
                            data: None,
                        }),
                    }
                }
            },
            _ => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id.clone(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32601,
                        message: format!("Method not found: {}", request.method),
                        data: None,
                    }),
                }
            }
        };

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result,
            error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestToolHandler;

    #[async_trait]
    impl ToolHandler for TestToolHandler {
        async fn execute(&self, _name: &str, _arguments: Value) -> Result<Vec<ResultContent>> {
            Ok(vec![ResultContent::Text {
                text: "test result".to_string(),
            }])
        }
    }

    #[tokio::test]
    async fn test_server_creation() {
        let config = ServerConfig::default();
        let handler = Arc::new(TestToolHandler);
        let server = McpServer::new(config, handler);
        assert_eq!(server.config.name, "MCP Server");
    }

    #[tokio::test]
    async fn test_register_tool() {
        let config = ServerConfig::default();
        let handler = Arc::new(TestToolHandler);
        let server = McpServer::new(config, handler);

        let tool = Tool {
            name: "test_tool".to_string(),
            description: Some("A test tool".to_string()),
            input_schema: Some(ToolInputSchema {
                schema_type: "object".to_string(),
                properties: Default::default(),
                required: None,
            }),
        };

        server.register_tool(tool);
        assert!(server.tools.contains_key("test_tool"));
    }
}
