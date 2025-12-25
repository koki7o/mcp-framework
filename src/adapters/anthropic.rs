/// Anthropic Claude adapter.

use crate::agent::LLMProvider;
use crate::protocol::{Message, Tool, ContentBlock, Role};
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Anthropic API request message
#[derive(Debug, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

/// Anthropic API tool input schema
#[derive(Debug, Serialize)]
struct AnthropicToolInput {
    #[serde(rename = "type")]
    type_field: String,
    properties: std::collections::HashMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<Vec<String>>,
}

/// Anthropic API tool definition
#[derive(Debug, Serialize)]
struct AnthropicTool {
    name: String,
    description: String,
    input_schema: AnthropicToolInput,
}

/// Anthropic API request
#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: i32,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
    temperature: f32,
    system: String,
}

/// Anthropic API response
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContentBlock>,
    #[serde(rename = "stop_reason")]
    stop_reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum AnthropicContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

/// Anthropic Claude LLM Provider
pub struct AnthropicAdapter {
    api_key: String,
    model: String,
    temperature: f32,
    max_tokens: i32,
    client: reqwest::Client,
    system_prompt: String,
}

impl AnthropicAdapter {
    /// Create a new Anthropic adapter
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            temperature: 0.7,
            max_tokens: 1024,
            client: reqwest::Client::new(),
            system_prompt: "You are a helpful AI assistant.".to_string(),
        }
    }

    /// Set temperature for response diversity
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// Set max tokens for response length
    pub fn with_max_tokens(mut self, max_tokens: i32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Set system prompt
    pub fn with_system_prompt(mut self, prompt: String) -> Self {
        self.system_prompt = prompt;
        self
    }

    /// Create from environment variable
    pub fn from_env(model: String) -> Result<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| Error::InternalError("ANTHROPIC_API_KEY not set".to_string()))?;
        Ok(Self::new(api_key, model))
    }
}

#[async_trait::async_trait]
impl LLMProvider for AnthropicAdapter {
    async fn call(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
    ) -> Result<crate::agent::LLMResponse> {
        // Convert MCP messages to Anthropic format
        let anthropic_messages: Vec<AnthropicMessage> = messages
            .iter()
            .filter_map(|msg| {
                let mut content_parts = Vec::new();

                for c in &msg.content {
                    match c {
                        ContentBlock::Text { text } => {
                            content_parts.push(text.clone());
                        }
                        ContentBlock::ToolResult { tool_use_id, content, is_error } => {
                            // Format tool results as text for Anthropic
                            // Extract text from ResultContent blocks
                            let result_strings: Vec<String> = content
                                .iter()
                                .filter_map(|rc| match rc {
                                    crate::protocol::ResultContent::Text { text } => Some(text.clone()),
                                    _ => None,
                                })
                                .collect();

                            let result_str = result_strings.join(" ");
                            let result_text = if is_error.unwrap_or(false) {
                                format!("[Tool {} error: {}]", tool_use_id, result_str)
                            } else {
                                format!("[Tool {} result: {}]", tool_use_id, result_str)
                            };
                            content_parts.push(result_text);
                        }
                        _ => {
                            // Skip other content types for now
                        }
                    }
                }

                let content = content_parts.join("\n");

                // Only include messages with non-empty content
                if content.is_empty() {
                    None
                } else {
                    Some(AnthropicMessage {
                        role: match msg.role {
                            Role::User => "user".to_string(),
                            Role::Assistant => "assistant".to_string(),
                        },
                        content,
                    })
                }
            })
            .collect();

        // Convert tools to Anthropic format
        let anthropic_tools: Option<Vec<AnthropicTool>> = if !tools.is_empty() {
            Some(
                tools
                    .iter()
                    .map(|tool| AnthropicTool {
                        name: tool.name.clone(),
                        description: tool.description.as_deref().unwrap_or("").to_string(),
                        input_schema: AnthropicToolInput {
                            type_field: tool
                                .input_schema
                                .as_ref()
                                .map(|s| s.schema_type.clone())
                                .unwrap_or_else(|| "object".to_string()),
                            properties: tool
                                .input_schema
                                .as_ref()
                                .map(|s| s.properties.clone())
                                .unwrap_or_default(),
                            required: tool.input_schema.as_ref().and_then(|s| s.required.clone()),
                        },
                    })
                    .collect(),
            )
        } else {
            None
        };

        // Create request
        let request = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            messages: anthropic_messages,
            tools: anthropic_tools,
            temperature: self.temperature,
            system: self.system_prompt.clone(),
        };

        // Make API call
        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::ConnectionError(format!("Anthropic API error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(Error::InternalError(format!(
                "Anthropic API error: {}",
                error_text
            )));
        }

        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| Error::InternalError(format!("Failed to parse Anthropic response: {}", e)))?;

        // Convert response to MCP format
        let content = anthropic_response
            .content
            .iter()
            .map(|block| match block {
                AnthropicContentBlock::Text { text } => {
                    ContentBlock::Text { text: text.clone() }
                }
                AnthropicContentBlock::ToolUse { id, name, input } => {
                    ContentBlock::ToolUse {
                        id: id.clone(),
                        name: name.clone(),
                        input: input.clone(),
                    }
                }
            })
            .collect();

        // Determine stop reason
        let stop_reason = if anthropic_response.stop_reason == "tool_use" {
            crate::agent::StopReason::ToolUse
        } else if anthropic_response.stop_reason == "max_tokens" {
            crate::agent::StopReason::MaxTokens
        } else {
            crate::agent::StopReason::EndTurn
        };

        Ok(crate::agent::LLMResponse {
            content,
            stop_reason,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_adapter_creation() {
        let adapter = AnthropicAdapter::new("sk-ant-test-key".to_string(), "claude-sonnet-4-5-20250929".to_string());
        assert_eq!(adapter.model, "claude-sonnet-4-5-20250929");
        assert_eq!(adapter.temperature, 0.7);
        assert_eq!(adapter.max_tokens, 1024);
    }

    #[test]
    fn test_anthropic_adapter_config() {
        let adapter = AnthropicAdapter::new("sk-ant-test-key".to_string(), "claude-sonnet-4-5-20250929".to_string())
            .with_temperature(0.5)
            .with_max_tokens(2000)
            .with_system_prompt("You are an expert programmer.".to_string());

        assert_eq!(adapter.temperature, 0.5);
        assert_eq!(adapter.max_tokens, 2000);
        assert_eq!(adapter.system_prompt, "You are an expert programmer.");
    }
}
