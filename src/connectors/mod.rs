/// Connection transport mechanisms for MCP
///
/// Supports multiple connection types:
/// - HTTP - Standard web-based connections
/// - WebSocket - Bidirectional communication
/// - Stdio - Standard input/output based connections

pub mod base;
pub mod http;
pub mod stdio;

pub use base::{Connector, ConnectorConfig};
pub use http::HttpConnector;
pub use stdio::StdioConnector;
