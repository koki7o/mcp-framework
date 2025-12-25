/// LLM adapters for OpenAI and Anthropic.
///
/// Implement the `LLMProvider` trait to add support for other models.

pub mod openai;
pub mod anthropic;

pub use openai::OpenAIAdapter;
pub use anthropic::AnthropicAdapter;
