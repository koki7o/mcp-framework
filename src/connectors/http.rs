/// HTTP connector for MCP
use super::base::{Connector, ConnectorConfig};
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::error::{Result, Error};
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::Mutex;

/// HTTP-based MCP connector
pub struct HttpConnector {
    config: ConnectorConfig,
    client: Client,
    connected: Arc<Mutex<bool>>,
}

impl HttpConnector {
    /// Create a new HTTP connector
    pub fn new(config: ConnectorConfig) -> Self {
        Self {
            config,
            client: Client::new(),
            connected: Arc::new(Mutex::new(false)),
        }
    }

    /// Create HTTP connector with default config
    pub fn default() -> Self {
        Self::new(ConnectorConfig::default())
    }
}

#[async_trait::async_trait]
impl Connector for HttpConnector {
    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        if !*self.connected.lock().await {
            return Err(Error::ConnectionError("Not connected".to_string()));
        }

        let response = self
            .client
            .post(&self.config.url)
            .json(&request)
            .timeout(std::time::Duration::from_secs(self.config.timeout_secs))
            .send()
            .await
            .map_err(|e| Error::ConnectionError(e.to_string()))?;

        response
            .json::<JsonRpcResponse>()
            .await
            .map_err(|e| Error::ConnectionError(e.to_string()))
    }

    async fn connect(&mut self) -> Result<()> {
        *self.connected.lock().await = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        *self.connected.lock().await = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        futures::executor::block_on(async { *self.connected.lock().await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_connector_creation() {
        let connector = HttpConnector::default();
        assert!(!connector.is_connected());
    }

    #[tokio::test]
    async fn test_http_connector_connect() {
        let mut connector = HttpConnector::default();
        assert!(connector.connect().await.is_ok());
        assert!(connector.is_connected());
    }
}
