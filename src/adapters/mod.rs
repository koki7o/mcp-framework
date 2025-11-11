/// LLM Provider Adapters for popular models
///
/// This module provides adapters to connect the MCP Agent with various LLM providers.
/// Each adapter implements the `LLMProvider` trait to enable seamless integration.
///
/// # Supported Providers
/// - OpenAI (GPT-5, GPT-4o)
/// - Anthropic (Claude Sonnet 4.5, Claude 3 Opus)
///
/// # Creating a Custom Adapter
///
/// To create a custom adapter for any LLM:
///
/// ```ignore
/// use mcp_framework::agent::LLMProvider;
/// use mcp_framework::protocol::{Message, Tool, ContentBlock};
///
/// pub struct MyLLMProvider {
///     api_key: String,
///     model: String,
/// }
///
/// #[async_trait::async_trait]
/// impl LLMProvider for MyLLMProvider {
///     async fn call(
///         &self,
///         messages: Vec<Message>,
///         tools: Vec<Tool>,
///     ) -> mcp_framework::Result<LLMResponse> {
///         // Convert messages to provider format
///         // Call provider API
///         // Convert response back to LLMResponse
///         todo!()
///     }
/// }
/// ```

pub mod openai;
pub mod anthropic;

pub use openai::OpenAIAdapter;
pub use anthropic::AnthropicAdapter;
