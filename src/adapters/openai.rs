/// OpenAI LLM Adapter using Responses API with Tool Execution Loop
///
/// This adapter integrates with OpenAI's Responses API and implements a tool execution loop
/// to handle function calls. When OpenAI requests tool execution, this adapter:
/// 1. Detects function_call outputs from OpenAI
/// 2. Executes the tools locally using the provided tool set
/// 3. Sends results back to OpenAI for synthesis
/// 4. Returns the final synthesized response
///
/// # How it Works
/// 1. User provides a prompt
/// 2. Adapter sends text input + tools to Responses API
/// 3. OpenAI may respond with function_call outputs
/// 4. Adapter executes tools and sends results back
/// 5. OpenAI synthesizes final response
/// 6. Final response is returned
///
/// # Example
///
/// ```ignore
/// use mcp_framework::adapters::OpenAIAdapter;
/// use mcp_framework::agent::Agent;
///
/// let adapter = OpenAIAdapter::from_env("gpt-5".to_string())?;
///
/// let agent = Agent::new(client, std::sync::Arc::new(adapter), config);
/// let result = agent.run("What is 15 + 27?").await?;
/// ```

use crate::agent::LLMProvider;
use crate::protocol::{Message, Tool, ContentBlock};
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// OpenAI Responses API tool definition
#[derive(Debug, Serialize, Clone)]
struct OpenAITool {
    #[serde(rename = "type")]
    type_field: String,
    name: String,
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parameters: Option<OpenAIToolParameters>,
}

/// OpenAI tool parameters (matches JSON Schema)
#[derive(Debug, Serialize, Clone)]
struct OpenAIToolParameters {
    #[serde(rename = "type")]
    type_field: String,
    properties: std::collections::HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<Vec<String>>,
}

/// OpenAI Responses API request
#[derive(Debug, Serialize)]
struct OpenAIResponsesRequest {
    model: String,
    input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAITool>>,
}

/// OpenAI Responses API response
#[derive(Debug, Deserialize)]
struct OpenAIResponsesResponse {
    output: Vec<serde_json::Value>,
    #[serde(default)]
    #[allow(dead_code)]
    id: String,
    #[serde(default)]
    #[allow(dead_code)]
    status: String,
}

/// Function call output from OpenAI
#[derive(Debug, Clone)]
struct FunctionCall {
    name: String,
    arguments_str: String,
    call_id: String,
}

/// Tool result to send back to OpenAI
#[derive(Debug, Serialize)]
struct ToolResult {
    #[serde(rename = "type")]
    type_field: String,
    call_id: String,
    content: Vec<ToolResultContent>,
}

/// Tool result content
#[derive(Debug, Serialize)]
struct ToolResultContent {
    #[serde(rename = "type")]
    type_field: String,
    text: String,
}

/// OpenAI LLM Provider
pub struct OpenAIAdapter {
    api_key: String,
    model: String,
    client: reqwest::Client,
    /// Optional MCP client for executing tools
    pub mcp_client: Option<std::sync::Arc<crate::client::McpClient>>,
}

impl OpenAIAdapter {
    /// Create a new OpenAI adapter
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
            mcp_client: None,
        }
    }

    /// Create from environment variables
    /// Expects: OPENAI_API_KEY
    pub fn from_env(model: String) -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| Error::InternalError("OPENAI_API_KEY not set".to_string()))?;

        Ok(Self::new(api_key, model))
    }

    /// Set the MCP client for tool execution
    pub fn with_mcp_client(mut self, client: std::sync::Arc<crate::client::McpClient>) -> Self {
        self.mcp_client = Some(client);
        self
    }

    /// Extract function calls from response output
    fn extract_function_calls(output: &[Value]) -> Vec<FunctionCall> {
        let mut calls = Vec::new();
        for output_item in output {
            if let Some(obj) = output_item.as_object() {
                if let Some(output_type) = obj.get("type").and_then(|v| v.as_str()) {
                    if output_type == "function_call" {
                        if let (Some(name), Some(arguments_str), Some(call_id)) = (
                            obj.get("name").and_then(|v| v.as_str()),
                            obj.get("arguments").and_then(|v| v.as_str()),
                            obj.get("call_id").and_then(|v| v.as_str()),
                        ) {
                            calls.push(FunctionCall {
                                name: name.to_string(),
                                arguments_str: arguments_str.to_string(),
                                call_id: call_id.to_string(),
                            });
                        }
                    }
                }
            }
        }
        calls
    }

    /// Extract text from response output
    fn extract_text(output: &[Value]) -> String {
        let mut text_content = String::new();
        for output_item in output {
            if let Some(obj) = output_item.as_object() {
                if let Some(output_type) = obj.get("type").and_then(|v| v.as_str()) {
                    match output_type {
                        "text" => {
                            // Old format: direct text field
                            if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                                text_content.push_str(text);
                            }
                        }
                        "message" => {
                            // New format: content array with output_text objects
                            if let Some(content_array) = obj.get("content").and_then(|v| v.as_array()) {
                                for content_item in content_array {
                                    if let Some(content_obj) = content_item.as_object() {
                                        if let Some(content_type) = content_obj.get("type").and_then(|v| v.as_str()) {
                                            if content_type == "output_text" {
                                                if let Some(text) = content_obj.get("text").and_then(|v| v.as_str()) {
                                                    if !text_content.is_empty() {
                                                        text_content.push('\n');
                                                    }
                                                    text_content.push_str(text);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        _ => {} // Skip other output types
                    }
                }
            }
        }
        text_content
    }
}

#[async_trait::async_trait]
impl LLMProvider for OpenAIAdapter {
    async fn call(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
    ) -> Result<crate::agent::LLMResponse> {
        // Convert messages to a single input string
        let mut input = messages
            .iter()
            .filter_map(|msg| {
                msg.content
                    .iter()
                    .filter_map(|c| match c {
                        ContentBlock::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
                    .into()
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        // Convert MCP tools to OpenAI tool format
        let openai_tools = if !tools.is_empty() {
            Some(
                tools
                    .iter()
                    .map(|tool| {
                        let parameters = tool.input_schema.as_ref().map_or_else(
                            || {
                                // Tool doesn't have input schema - create a default one
                                // Try to infer parameters from tool name and description
                                let desc = tool.description.as_deref().unwrap_or("").to_lowercase();
                                let mut properties = std::collections::HashMap::new();
                                let mut required = Vec::new();

                                // Common parameters based on tool name and description
                                if tool.name.contains("navigate") || desc.contains("url") {
                                    properties.insert("url".to_string(), serde_json::json!({
                                        "type": "string",
                                        "description": "The URL to navigate to"
                                    }));
                                    required.push("url".to_string());
                                }
                                if desc.contains("text") || desc.contains("click") {
                                    properties.insert("text".to_string(), serde_json::json!({
                                        "type": "string",
                                        "description": "The text to search for or interact with"
                                    }));
                                    required.push("text".to_string());
                                }
                                if desc.contains("key") || desc.contains("press") {
                                    properties.insert("key".to_string(), serde_json::json!({
                                        "type": "string",
                                        "description": "The key to press"
                                    }));
                                    required.push("key".to_string());
                                }
                                if desc.contains("selector") || desc.contains("element") {
                                    properties.insert("selector".to_string(), serde_json::json!({
                                        "type": "string",
                                        "description": "CSS selector for the element"
                                    }));
                                    required.push("selector".to_string());
                                }
                                if properties.is_empty() {
                                    // Fallback: create a generic 'params' parameter
                                    properties.insert("params".to_string(), serde_json::json!({
                                        "type": "object",
                                        "description": "Parameters for this tool"
                                    }));
                                }

                                OpenAIToolParameters {
                                    type_field: "object".to_string(),
                                    properties,
                                    required: if required.is_empty() { None } else { Some(required) },
                                }
                            },
                            |schema| {
                                OpenAIToolParameters {
                                    type_field: schema.schema_type.clone(),
                                    properties: schema.properties.clone(),
                                    required: schema.required.clone(),
                                }
                            }
                        );

                        OpenAITool {
                            type_field: "function".to_string(),
                            name: tool.name.clone(),
                            description: tool.description.clone(),
                            parameters: Some(parameters),
                        }
                    })
                    .collect::<Vec<_>>()
            )
        } else {
            None
        };

        // Tool execution loop
        let max_iterations = 20;

        for _iteration in 0..max_iterations {
            // Create request for Responses API
            let request = OpenAIResponsesRequest {
                model: self.model.clone(),
                input: input.clone(),
                tools: openai_tools.clone(),
            };
            // Make API call to Responses API endpoint
            let response = self
                .client
                .post("https://api.openai.com/v1/responses")
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await
                .map_err(|e| Error::ConnectionError(format!("OpenAI API error: {}", e)))?;

            if !response.status().is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(Error::InternalError(format!(
                    "OpenAI API error: {}",
                    error_text
                )));
            }

            let response_text = response.text().await
                .map_err(|e| Error::ConnectionError(format!("Failed to read response: {}", e)))?;

            let openai_response: OpenAIResponsesResponse = serde_json::from_str(&response_text)
                .map_err(|e| Error::InternalError(format!("Failed to parse OpenAI response: {} (body: {})", e, response_text)))?;

            // Check for function calls
            let function_calls = Self::extract_function_calls(&openai_response.output);

            if !function_calls.is_empty() {
                // Execute all function calls and collect results
                let mut tool_results = Vec::new();

                for call in function_calls {

                    // Parse arguments from JSON string
                    let arguments: Value = match serde_json::from_str(&call.arguments_str) {
                        Ok(args) => args,
                        Err(_) => {
                            json!({})
                        }
                    };

                    // Execute the tool if MCP client is available, otherwise return placeholder
                    let result_text = if let Some(mcp_client) = &self.mcp_client {
                        // Execute tool via MCP client (real execution!)
                        match mcp_client.call_tool(&call.name, arguments).await {
                            Ok(tool_result) => {
                                // Format the tool result
                                let formatted_result = tool_result
                                    .content
                                    .iter()
                                    .filter_map(|c| match c {
                                        crate::protocol::ResultContent::Text { text } => Some(text.clone()),
                                        _ => None,
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n");

                                // Check if the tool returned an error
                                if tool_result.is_error == Some(true) {
                                    // Return error to LLM so it knows the tool call FAILED
                                    return Err(Error::InternalError(
                                        format!("Tool '{}' failed with error: {}", call.name, formatted_result)
                                    ));
                                }

                                formatted_result
                            }
                            Err(e) => {
                                let error_msg = format!("Error executing tool '{}': {}", call.name, e);
                                return Err(Error::InternalError(error_msg));
                            }
                        }
                    } else {
                        // No MCP client - return placeholder indicating tool was called
                        match tools.iter().find(|t| t.name == call.name) {
                            Some(tool) => {
                                format!("Tool '{}' executed with arguments: {}", tool.name, call.arguments_str)
                            }
                            None => {
                                format!("Tool '{}' not found", call.name)
                            }
                        }
                    };

                    tool_results.push(ToolResult {
                        type_field: "tool_result".to_string(),
                        call_id: call.call_id,
                        content: vec![ToolResultContent {
                            type_field: "text".to_string(),
                            text: result_text,
                        }],
                    });
                }

                // Append tool results to input for next iteration
                input.push_str("\n\nTool execution results:\n");
                for result in &tool_results {
                    let result_text = result.content.iter()
                        .map(|c| c.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" ");
                    input.push_str(&format!(
                        "- Call ID {}: {}\n",
                        result.call_id,
                        result_text
                    ));
                }
            } else {
                // No function calls, extract and return the text response
                let text_content = Self::extract_text(&openai_response.output);

                if !text_content.is_empty() {
                    return Ok(crate::agent::LLMResponse {
                        content: vec![ContentBlock::Text { text: text_content }],
                        stop_reason: crate::agent::StopReason::EndTurn,
                    });
                } else {
                    return Ok(crate::agent::LLMResponse {
                        content: vec![ContentBlock::Text { text: "No response generated.".to_string() }],
                        stop_reason: crate::agent::StopReason::EndTurn,
                    });
                }
            }
        }

        // Max iterations reached without getting final answer
        Err(Error::InternalError(
            "Max tool execution iterations reached without final response".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_adapter_creation() {
        let adapter = OpenAIAdapter::new(
            "test-key".to_string(),
            "gpt-5".to_string(),
        );
        assert_eq!(adapter.model, "gpt-5");
    }

}
