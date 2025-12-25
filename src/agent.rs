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

/// Event emitted during agent execution (for streaming/callbacks)
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Agent started processing
    Started,
    /// Thinking/calling LLM
    LlmCall { iteration: usize },
    /// LLM returned text response
    TextChunk { text: String },
    /// Tool is about to be called
    ToolCallStarted { tool_name: String },
    /// Tool execution completed
    ToolCallCompleted { tool_name: String, result: String },
    /// Tool execution failed
    ToolCallFailed { tool_name: String, error: String },
    /// Agent iteration completed
    IterationComplete { iteration: usize },
    /// Agent finished successfully
    Finished { response: String },
    /// Agent encountered an error
    Failed { error: String },
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
    /// System prompt for the agent (if None, LLM uses its default)
    system_prompt: Option<String>,
    /// Tools that are not allowed to be called
    disallowed_tools: Vec<String>,
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
            system_prompt: None,
            disallowed_tools: Vec::new(),
        }
    }

    /// Run the agent with a user prompt
    ///
    /// Preserves conversation history across calls for multi-turn interactions.
    /// To start fresh, call `clear_conversation()` before running.
    pub async fn run(&mut self, prompt: impl Into<String>) -> Result<String> {
        self.state = AgentState::Running;
        // Add new user message to conversation (preserving history)
        self.conversation.push_back(Message::user(prompt));

        let mut iterations = 0;
        let mut final_response = String::new();

        while iterations < self.config.max_iterations && self.state == AgentState::Running {
            iterations += 1;

            // Get available tools (filtered)
            let tools = self.get_available_tools().await?;

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

    /// Set the system prompt for the agent
    pub fn set_system_prompt(&mut self, prompt: String) {
        self.system_prompt = Some(prompt);
    }

    /// Get the current system prompt
    pub fn get_system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }

    /// Clear the system prompt (use LLM's default)
    pub fn clear_system_prompt(&mut self) {
        self.system_prompt = None;
    }

    /// Set tools that are not allowed to be called
    pub fn set_disallowed_tools(&mut self, tools: Vec<String>) {
        self.disallowed_tools = tools;
    }

    /// Get the list of disallowed tools
    pub fn get_disallowed_tools(&self) -> &[String] {
        &self.disallowed_tools
    }

    /// Add a tool to the disallowed list
    pub fn disallow_tool(&mut self, tool_name: String) {
        if !self.disallowed_tools.contains(&tool_name) {
            self.disallowed_tools.push(tool_name);
        }
    }

    /// Get available tools, excluding disallowed ones
    async fn get_available_tools(&self) -> Result<Vec<Tool>> {
        let all_tools = self.client.list_tools().await?;
        Ok(all_tools
            .into_iter()
            .filter(|t| !self.disallowed_tools.contains(&t.name))
            .collect())
    }

    /// Run with event callbacks for streaming.
    pub async fn run_with_events<F>(&mut self, prompt: impl Into<String>, mut on_event: F) -> Result<String>
    where
        F: FnMut(AgentEvent) + Send,
    {
        self.state = AgentState::Running;
        // Add new user message to conversation (preserving history)
        self.conversation.push_back(Message::user(prompt));

        on_event(AgentEvent::Started);

        let mut iterations = 0;
        let mut final_response = String::new();

        while iterations < self.config.max_iterations && self.state == AgentState::Running {
            iterations += 1;
            on_event(AgentEvent::LlmCall {
                iteration: iterations,
            });

            // Get available tools (filtered)
            let tools = self.get_available_tools().await?;

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
                        on_event(AgentEvent::TextChunk {
                            text: text.clone(),
                        });
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        has_tool_use = true;
                        self.state = AgentState::WaitingForToolResult;

                        on_event(AgentEvent::ToolCallStarted {
                            tool_name: name.clone(),
                        });

                        // Add assistant message with tool use (only once)
                        if !assistant_message_added {
                            self.conversation.push_back(Message {
                                role: Role::Assistant,
                                content: llm_response.content.clone(),
                            });
                            assistant_message_added = true;
                        }

                        // Execute tool
                        match self.client.call_tool(name, input.clone()).await {
                            Ok(tool_result) => {
                                let result_text = tool_result
                                    .content
                                    .iter()
                                    .filter_map(|c| match c {
                                        ResultContent::Text { text } => Some(text.clone()),
                                        _ => None,
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n");

                                on_event(AgentEvent::ToolCallCompleted {
                                    tool_name: name.clone(),
                                    result: result_text,
                                });

                                // Add tool result
                                self.conversation.push_back(Message {
                                    role: Role::User,
                                    content: vec![ContentBlock::ToolResult {
                                        tool_use_id: id.clone(),
                                        content: tool_result.content.clone(),
                                        is_error: tool_result.is_error,
                                    }],
                                });
                            }
                            Err(e) => {
                                on_event(AgentEvent::ToolCallFailed {
                                    tool_name: name.clone(),
                                    error: e.to_string(),
                                });
                                // Still add the error to conversation
                                self.conversation.push_back(Message {
                                    role: Role::User,
                                    content: vec![ContentBlock::ToolResult {
                                        tool_use_id: id.clone(),
                                        content: vec![ResultContent::Text {
                                            text: format!("Error: {}", e),
                                        }],
                                        is_error: Some(true),
                                    }],
                                });
                            }
                        }

                        self.state = AgentState::Running;
                    }
                    _ => {}
                }
            }

            on_event(AgentEvent::IterationComplete {
                iteration: iterations,
            });

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
            let err_msg = "Max iterations reached".to_string();
            on_event(AgentEvent::Failed {
                error: err_msg.clone(),
            });
            return Err(Error::InternalError(err_msg));
        }

        on_event(AgentEvent::Finished {
            response: final_response.clone(),
        });

        Ok(final_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test LLM provider for unit tests
    struct DummyLLMProvider;

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
