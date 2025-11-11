//! MCP Framework - A Rust implementation of the Model Context Protocol
//!
//! This library provides a comprehensive framework for building MCP servers, clients, and agents.
//!
//! ## Features
//! - Protocol types and structures
//! - MCP Server implementation
//! - MCP Client with multiple connection types
//! - AI Agent with LLM integration
//! - Web-based Inspector for debugging
//! - Authentication (Bearer, OAuth)
//! - Configuration management
//! - Session handling
//! - Logging support
//! - .env file support for configuration

/// Load environment variables from .env file
/// Call this in your main() function before creating adapters
pub fn load_env() {
    dotenv::dotenv().ok();
}

pub mod protocol;
pub mod server;
pub mod client;
pub mod agent;
pub mod inspector;
pub mod error;
pub mod adapters;

pub use error::{Error, Result};

pub mod prelude {
    pub use crate::protocol::*;
    pub use crate::server::*;
    pub use crate::client::*;
    pub use crate::agent::*;
    pub use crate::adapters::{OpenAIAdapter, AnthropicAdapter};
    pub use crate::error::{Error, Result};
}
