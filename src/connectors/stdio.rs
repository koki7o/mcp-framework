/// Stdio connector for MCP - Standard input/output based connections
use super::base::Connector;
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::error::{Result, Error};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;

/// Stdio-based MCP connector for spawning and communicating with processes
pub struct StdioConnector {
    command: String,
    args: Vec<String>,
    env_vars: HashMap<String, String>,
    child: Arc<Mutex<Option<Child>>>,
    connected: Arc<Mutex<bool>>,
}

impl StdioConnector {
    /// Create a new stdio connector
    pub fn new(command: String, args: Vec<String>) -> Self {
        Self {
            command,
            args,
            env_vars: HashMap::new(),
            child: Arc::new(Mutex::new(None)),
            connected: Arc::new(Mutex::new(false)),
        }
    }

    /// Create stdio connector with just a command
    pub fn from_command(command: String) -> Self {
        Self::new(command, vec![])
    }

    /// Set environment variables to pass to the subprocess
    pub fn set_env(&mut self, env_vars: HashMap<String, String>) {
        self.env_vars = env_vars;
    }

    /// Add a single environment variable
    pub fn with_env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }
}

#[async_trait::async_trait]
impl Connector for StdioConnector {
    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        if !*self.connected.lock().await {
            return Err(Error::ConnectionError("Not connected".to_string()));
        }

        let mut child_lock = self.child.lock().await;
        let child = child_lock
            .as_mut()
            .ok_or_else(|| Error::ConnectionError("No process running".to_string()))?;

        // Send request as JSON line
        let json_str = serde_json::to_string(&request)
            .map_err(|e| Error::ConnectionError(e.to_string()))?;

        if let Some(stdin) = child.stdin.as_mut() {
            writeln!(stdin, "{}", json_str)
                .map_err(|e| Error::ConnectionError(e.to_string()))?;
        } else {
            return Err(Error::ConnectionError("No stdin available".to_string()));
        }

        // Read response from stdout
        if let Some(stdout) = child.stdout.as_mut() {
            let mut reader = BufReader::new(stdout);
            let mut response_line = String::new();
            reader
                .read_line(&mut response_line)
                .map_err(|e| Error::ConnectionError(e.to_string()))?;

            serde_json::from_str(&response_line)
                .map_err(|e| Error::ConnectionError(e.to_string()))
        } else {
            Err(Error::ConnectionError("No stdout available".to_string()))
        }
    }

    async fn connect(&mut self) -> Result<()> {
        let mut cmd = Command::new(&self.command);
        cmd.args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());

        // Set environment variables
        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        let child = cmd.spawn()
            .map_err(|e| Error::ConnectionError(format!("Failed to spawn process: {}", e)))?;

        *self.child.lock().await = Some(child);
        *self.connected.lock().await = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.lock().await.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
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
    fn test_stdio_connector_creation() {
        let connector = StdioConnector::from_command("echo".to_string());
        assert!(!connector.is_connected());
    }
}
