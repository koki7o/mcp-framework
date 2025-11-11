use crate::client::McpClient;
use crate::protocol::*;
use crate::error::{Error, Result};
use std::collections::VecDeque;

/// LLM interface trait
#[async_trait::async_trait]
pub trait LLMProvider: Send + Sync {
    /// Call the LLM with messages and tools
    async fn call(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
    ) -> Result<LLMResponse>;
}

/// LLM response
#[derive(Debug, Clone)]
pub struct LLMResponse {
    pub content: Vec<ContentBlock>,
    pub stop_reason: StopReason,
}

/// Reason the LLM stopped generating
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
}

/// Agent state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AgentState {
    Ready,
    Running,
    WaitingForToolResult,
    Done,
    Error,
}

/// Agentic loop configuration
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub max_iterations: usize,
    pub max_tokens: Option<usize>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            max_tokens: None,
        }
    }
}

/// MCP-powered AI Agent
pub struct Agent {
    client: McpClient,
    llm: std::sync::Arc<dyn LLMProvider>,
    config: AgentConfig,
    state: AgentState,
    conversation: VecDeque<Message>,
}

impl Agent {
    /// Create a new agent
    pub fn new(
        client: McpClient,
        llm: std::sync::Arc<dyn LLMProvider>,
        config: AgentConfig,
    ) -> Self {
        Self {
            client,
            llm,
            config,
            state: AgentState::Ready,
            conversation: VecDeque::new(),
        }
    }

    /// Run the agent with a user prompt
    pub async fn run(&mut self, prompt: impl Into<String>) -> Result<String> {
        self.state = AgentState::Running;
        self.conversation.clear();
        self.conversation.push_back(Message::user(prompt));

        let mut iterations = 0;
        let mut final_response = String::new();

        while iterations < self.config.max_iterations && self.state == AgentState::Running {
            iterations += 1;

            // Get available tools
            let tools = self.client.list_tools().await?;

            // Prepare messages for LLM
            let messages: Vec<Message> = self.conversation.iter().cloned().collect();

            // Call LLM
            let llm_response = self
                .llm
                .call(messages, tools)
                .await
                .map_err(|e| Error::LLMError(e.to_string()))?;

            // Process response
            let mut has_tool_use = false;
            let mut assistant_message_added = false;

            for content in &llm_response.content {
                match content {
                    ContentBlock::Text { text } => {
                        final_response.push_str(text);
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        has_tool_use = true;
                        self.state = AgentState::WaitingForToolResult;

                        // Add assistant message with tool use (only once)
                        if !assistant_message_added {
                            self.conversation.push_back(Message {
                                role: Role::Assistant,
                                content: llm_response.content.clone(),
                            });
                            assistant_message_added = true;
                        }

                        // Execute tool
                        let tool_result = self.client.call_tool(name, input.clone()).await?;

                        // Add tool result
                        self.conversation.push_back(Message {
                            role: Role::User,
                            content: vec![ContentBlock::ToolResult {
                                tool_use_id: id.clone(),
                                content: tool_result.content.clone(),
                                is_error: tool_result.is_error,
                            }],
                        });

                        self.state = AgentState::Running;
                    }
                    _ => {}
                }
            }

            // Check if we should stop
            if !has_tool_use || llm_response.stop_reason == StopReason::EndTurn {
                // Add final assistant message if not already added
                if !assistant_message_added {
                    self.conversation.push_back(Message {
                        role: Role::Assistant,
                        content: llm_response.content.clone(),
                    });
                }
                self.state = AgentState::Done;
            }
        }

        if iterations >= self.config.max_iterations {
            self.state = AgentState::Error;
            return Err(Error::InternalError(
                "Max iterations reached".to_string(),
            ));
        }

        Ok(final_response)
    }

    /// Get current state
    pub fn state(&self) -> AgentState {
        self.state
    }

    /// Get conversation history
    pub fn conversation(&self) -> Vec<Message> {
        self.conversation.iter().cloned().collect()
    }

    /// Clear conversation history
    pub fn clear_conversation(&mut self) {
        self.conversation.clear();
    }
}

/// Example LLM provider (stub for demonstration)
pub struct DummyLLMProvider;

#[async_trait::async_trait]
impl LLMProvider for DummyLLMProvider {
    async fn call(
        &self,
        messages: Vec<Message>,
        _tools: Vec<Tool>,
    ) -> Result<LLMResponse> {
        // Simple echo back the last user message
        let last_user_msg = messages
            .iter()
            .rev()
            .find(|m| m.role == Role::User)
            .and_then(|m| {
                m.content.iter().find_map(|c| match c {
                    ContentBlock::Text { text } => Some(text.clone()),
                    _ => None,
                })
            })
            .unwrap_or_default();

        Ok(LLMResponse {
            content: vec![ContentBlock::Text {
                text: format!("I received: {}", last_user_msg),
            }],
            stop_reason: StopReason::EndTurn,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_creation() {
        let client = McpClient::new("http://localhost:8000");
        let llm = std::sync::Arc::new(DummyLLMProvider);
        let agent = Agent::new(client, llm, AgentConfig::default());
        assert_eq!(agent.state, AgentState::Ready);
    }

    #[test]
    fn test_agent_config() {
        let config = AgentConfig::default();
        assert_eq!(config.max_iterations, 10);
    }
}
