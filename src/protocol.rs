/// Protocol types and compatibility layer over rmcp SDK.

use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// Custom serializer for params that outputs empty object instead of null
fn serialize_params<S>(params: &Option<Value>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match params {
        Some(value) => value.serialize(serializer),
        None => serde_json::json!({}).serialize(serializer),
    }
}

// Re-export types from official SDK
pub use rmcp::model::{
    Tool as RmcpTool,
    Prompt as RmcpPrompt,
    Resource as RmcpResource,
    TextContent,
    ImageContent,
    EmbeddedResource,
};

/// MCP Protocol version
pub const MCP_VERSION: &str = "2025-11-05";

/// Unique identifier for requests
pub type RequestId = String;

/// JSON-RPC Request - compatibility type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: RequestId,
    pub method: String,
    #[serde(serialize_with = "serialize_params")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    pub fn new(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: Uuid::new_v4().to_string(),
            method: method.into(),
            params,
        }
    }
}

/// JSON-RPC Response - compatibility type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC Error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

pub use rmcp::model::Tool as RmcpToolType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<ToolInputSchema>,
}

/// Tool input schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInputSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub properties: HashMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

/// Resource definition - compatibility wrapper
pub type Resource = RmcpResource;

/// Prompt definition - compatibility wrapper
pub type Prompt = RmcpPrompt;

/// Prompt argument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub required: Option<bool>,
}

/// Tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

/// Tool result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub content: Vec<ResultContent>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "isError")]
    pub is_error: Option<bool>,
}

/// Result content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResultContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
}

/// Server capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {
    #[serde(skip_serializing_if = "Option::is_none", rename = "listChanged")]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "listChanged")]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsCapability {
    #[serde(skip_serializing_if = "Option::is_none", rename = "listChanged")]
    pub list_changed: Option<bool>,
}

/// Message role
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Role {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
}

/// Content block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: Vec<ResultContent>,
        #[serde(skip_serializing_if = "Option::is_none", rename = "isError")]
        is_error: Option<bool>,
    },
    #[serde(rename = "image")]
    Image {
        source: ImageSource,
    },
}

/// Image source
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ImageSource {
    #[serde(rename = "base64")]
    Base64 { data: String, media_type: String },
    #[serde(rename = "url")]
    Url { url: String },
}

/// Message with role and content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

impl Message {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: text.into(),
            }],
        }
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: vec![ContentBlock::Text {
                text: text.into(),
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsonrpc_request_creation() {
        let req = JsonRpcRequest::new("tools/list", None);
        assert_eq!(req.method, "tools/list");
        assert_eq!(req.jsonrpc, "2.0");
    }

    #[test]
    fn test_jsonrpc_request_serialization_with_params() {
        // When params is provided, it should be serialized as-is
        let req = JsonRpcRequest::new("tools/call", Some(serde_json::json!({"name": "test"})));
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"params\":{\"name\":\"test\"}"));
    }

    #[test]
    fn test_jsonrpc_request_serialization_without_params() {
        // When params is None, it should serialize as empty object {}
        let req = JsonRpcRequest::new("tools/list", None);
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"params\":{}"), "Expected params: {{}}, got: {}", json);
    }

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, Role::User);
    }
}

