use crate::error::Result;
use crate::server::McpServer;
use axum::{
    extract::State,
    http::StatusCode,
    response::Html,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::net::TcpListener;

/// Inspector state shared across handlers
#[derive(Clone)]
pub struct InspectorState {
    server_name: String,
    server_version: String,
    pub captured_requests: Arc<parking_lot::Mutex<Vec<InspectorRequest>>>,
    pub captured_responses: Arc<parking_lot::Mutex<Vec<InspectorResponse>>>,
    pub tools: Arc<parking_lot::Mutex<Vec<crate::protocol::Tool>>>,
    pub server: Option<Arc<McpServer>>,
}

impl InspectorState {
    pub fn new(server_name: String, server_version: String) -> Self {
        Self {
            server_name,
            server_version,
            captured_requests: Arc::new(parking_lot::Mutex::new(Vec::new())),
            captured_responses: Arc::new(parking_lot::Mutex::new(Vec::new())),
            tools: Arc::new(parking_lot::Mutex::new(Vec::new())),
            server: None,
        }
    }

    /// Capture a response
    pub fn capture_response(
        &self,
        method: String,
        result: Option<serde_json::Value>,
        error: Option<String>,
    ) {
        let now = chrono::Local::now().to_rfc3339();
        let response = InspectorResponse {
            timestamp: now,
            request_method: method,
            result,
            error,
        };
        self.captured_responses.lock().push(response);
    }
}

/// Captured request for inspection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectorRequest {
    pub timestamp: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

/// Captured response for inspection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectorResponse {
    pub timestamp: String,
    pub request_method: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// MCP Inspector - Web-based debugger for MCP servers
pub struct Inspector {
    state: InspectorState,
    #[allow(dead_code)]
    listener: Option<TcpListener>,
}

impl Inspector {
    /// Create a new inspector
    pub fn new(server_name: String, server_version: String) -> Self {
        Self {
            state: InspectorState::new(server_name, server_version),
            listener: None,
        }
    }

    /// Capture a request
    pub fn capture_request(&self, method: String, params: Option<serde_json::Value>) {
        let now = chrono::Local::now().to_rfc3339();
        let request = InspectorRequest {
            timestamp: now,
            method,
            params,
        };
        self.state.captured_requests.lock().push(request);
    }

    /// Capture a response
    pub fn capture_response(
        &self,
        method: String,
        result: Option<serde_json::Value>,
        error: Option<String>,
    ) {
        let now = chrono::Local::now().to_rfc3339();
        let response = InspectorResponse {
            timestamp: now,
            request_method: method,
            result,
            error,
        };
        self.state.captured_responses.lock().push(response);
    }

    /// Get number of captured requests
    pub fn request_count(&self) -> usize {
        self.state.captured_requests.lock().len()
    }

    /// Get number of captured responses
    pub fn response_count(&self) -> usize {
        self.state.captured_responses.lock().len()
    }

    /// Set the available tools
    pub fn set_tools(&mut self, tools: Vec<crate::protocol::Tool>) {
        *self.state.tools.lock() = tools;
    }

    /// Set the MCP server for tool execution
    pub fn set_server(&mut self, server: Arc<McpServer>) {
        self.state.server = Some(server);
    }

    /// Start the web server
    pub async fn start(&mut self, addr: &str) -> Result<()> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| crate::error::Error::ConnectionError(e.to_string()))?;

        let state = self.state.clone();
        let router = self.build_router(state);

        axum::serve(listener, router)
            .await
            .map_err(|e| crate::error::Error::ConnectionError(e.to_string()))?;

        Ok(())
    }

    fn build_router(&self, state: InspectorState) -> Router {
        Router::new()
            .route("/", get(handle_index))
            .route("/api/requests", get(handle_get_requests))
            .route("/api/responses", get(handle_get_responses))
            .route("/api/clear", post(handle_clear))
            .route("/api/server-info", get(handle_server_info))
            .route("/api/tools", get(handle_get_tools))
            .route("/api/call-tool", post(handle_call_tool))
            .with_state(state)
    }
}

async fn handle_index() -> Html<&'static str> {
    Html(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>MCP Inspector</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }

        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            background: #f8f8f8;
            color: #333;
            min-height: 100vh;
        }

        .header {
            background: white;
            padding: 20px;
            border-bottom: 1px solid #eee;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }

        .header h1 {
            font-size: 24px;
            font-weight: 600;
        }

        .status-badge {
            font-size: 14px;
            color: #666;
        }

        .container {
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
        }

        .grid {
            display: grid;
            grid-template-columns: 200px 1fr;
            gap: 20px;
        }

        .sidebar {
            background: white;
            padding: 0;
        }

        .sidebar h2 {
            font-size: 12px;
            font-weight: 600;
            text-transform: uppercase;
            margin-bottom: 12px;
            color: #999;
        }

        .nav-item {
            padding: 10px 0;
            margin-bottom: 5px;
            cursor: pointer;
            font-size: 14px;
            color: #666;
            border-bottom: 1px solid #eee;
        }

        .nav-item:hover {
            color: #000;
        }

        .nav-item.active {
            color: #000;
            font-weight: 600;
        }

        .content {
            background: white;
            padding: 20px;
        }

        .section {
            display: none;
        }

        .section.active {
            display: block;
        }

        .section h2 {
            font-size: 18px;
            font-weight: 600;
            margin-bottom: 20px;
            color: #000;
        }

        .tool-list {
            display: grid;
            gap: 12px;
        }

        .tool-card {
            background: white;
            border: 1px solid #eee;
            padding: 15px;
            cursor: pointer;
        }

        .tool-card:hover {
            border-color: #000;
            background: #fafafa;
        }

        .tool-card h3 {
            font-size: 15px;
            color: #000;
            margin-bottom: 8px;
        }

        .tool-card p {
            font-size: 13px;
            color: #666;
            margin-bottom: 12px;
        }

        .tool-form {
            background: #fafafa;
            padding: 15px;
            margin-top: 12px;
        }

        .form-group {
            margin-bottom: 12px;
        }

        .form-group label {
            display: block;
            font-size: 13px;
            font-weight: 600;
            margin-bottom: 5px;
            color: #333;
        }

        .form-group input {
            width: 100%;
            padding: 8px;
            background: white;
            border: 1px solid #ddd;
            color: #333;
            font-size: 14px;
        }

        .form-group input:focus {
            outline: none;
            border-color: #000;
        }

        .button-group {
            display: flex;
            gap: 10px;
            margin-top: 15px;
        }

        button {
            flex: 1;
            padding: 10px 16px;
            border: 1px solid #ddd;
            background: white;
            font-size: 14px;
            font-weight: 600;
            cursor: pointer;
        }

        .btn-primary {
            background: black;
            color: white;
            border: 1px solid black;
        }

        .btn-primary:hover {
            background: #333;
            border-color: #333;
        }

        .btn-secondary {
            background: white;
            color: #333;
        }

        .btn-secondary:hover {
            background: #f8f8f8;
        }

        .result {
            background: #fafafa;
            border-left: 2px solid #ddd;
            padding: 15px;
            margin-top: 15px;
            display: none;
        }

        .result.show {
            display: block;
        }

        .result.error {
            background: #fff5f5;
            border-left-color: #f56565;
        }

        .result-title {
            font-size: 12px;
            font-weight: 600;
            margin-bottom: 8px;
            color: #666;
        }

        .result-content {
            background: white;
            padding: 10px;
            border: 1px solid #eee;
            font-family: monospace;
            font-size: 12px;
            overflow-x: auto;
            max-height: 300px;
            overflow-y: auto;
        }

        .server-info {
            display: grid;
            grid-template-columns: repeat(2, 1fr);
            gap: 12px;
            margin-bottom: 20px;
        }

        .info-card {
            background: #fafafa;
            border: 1px solid #eee;
            padding: 15px;
        }

        .info-card h3 {
            font-size: 12px;
            font-weight: 600;
            color: #999;
            text-transform: uppercase;
            margin-bottom: 8px;
        }

        .info-card .value {
            font-size: 18px;
            color: #667eea;
            font-weight: 600;
        }

        .history-item {
            background: rgba(0, 0, 0, 0.2);
            padding: 12px;
            border-radius: 6px;
            margin-bottom: 8px;
            font-size: 13px;
            cursor: pointer;
            transition: all 0.2s;
            border-left: 3px solid transparent;
        }

        .history-item:hover {
            background: rgba(102, 126, 234, 0.1);
            border-left-color: #667eea;
        }

        .loading {
            text-align: center;
            padding: 40px;
            color: #a0aec0;
        }

        .spinner {
            display: inline-block;
            width: 20px;
            height: 20px;
            border: 3px solid rgba(102, 126, 234, 0.2);
            border-radius: 50%;
            border-top-color: #667eea;
            animation: spin 0.8s linear infinite;
        }

        @keyframes spin {
            to { transform: rotate(360deg); }
        }

        .empty-state {
            text-align: center;
            padding: 40px;
            color: #a0aec0;
        }

        .empty-state h3 {
            color: #cbd5e0;
            margin-bottom: 8px;
        }
    </style>
</head>
<body>
    <div class="header">
        <h1>MCP Inspector</h1>
        <div class="status-badge connected">
            <span>Connected</span>
        </div>
    </div>

    <div class="container">
        <div class="grid">
            <div class="sidebar">
                <h2>Navigation</h2>
                <div class="nav-item active" onclick="switchTab('overview')">📊 Overview</div>
                <div class="nav-item" onclick="switchTab('tools')">🔧 Tools</div>
                <div class="nav-item" onclick="switchTab('resources')">📁 Resources</div>
                <div class="nav-item" onclick="switchTab('prompts')">💬 Prompts</div>
                <div class="nav-item" onclick="switchTab('history')">📜 History</div>

                <h2 style="margin-top: 24px;">Actions</h2>
                <button class="nav-item" style="width: 100%; text-align: center; background: rgba(239, 68, 68, 0.1); border-left-color: #ef4444; color: #ef4444;" onclick="clearAll()">
                    🗑️ Clear All
                </button>
            </div>

            <div class="content">
                <!-- Overview Tab -->
                <div id="overview" class="section active">
                    <h2>Server Overview</h2>
                    <div class="server-info" id="server-info-cards"></div>
                    <div>
                        <h3 style="color: #667eea; margin-bottom: 16px;">Recent Activity</h3>
                        <div id="recent-activity"></div>
                    </div>
                </div>

                <!-- Tools Tab -->
                <div id="tools" class="section">
                    <h2>Available Tools</h2>
                    <div id="tools-list" class="tool-list"></div>
                </div>

                <!-- Resources Tab -->
                <div id="resources" class="section">
                    <h2>Available Resources</h2>
                    <div id="resources-list" class="tool-list"></div>
                </div>

                <!-- Prompts Tab -->
                <div id="prompts" class="section">
                    <h2>Available Prompts</h2>
                    <div id="prompts-list" class="tool-list"></div>
                </div>

                <!-- History Tab -->
                <div id="history" class="section">
                    <h2>Request History</h2>
                    <div id="history-list" class="tool-list"></div>
                </div>
            </div>
        </div>
    </div>

    <script>
        function switchTab(tabName) {
            document.querySelectorAll('.section').forEach(s => s.classList.remove('active'));
            document.querySelectorAll('.nav-item').forEach(n => n.classList.remove('active'));
            document.getElementById(tabName).classList.add('active');
            event.target.classList.add('active');
            loadTabData(tabName);
        }

        async function loadTabData(tabName) {
            switch(tabName) {
                case 'overview':
                    await loadServerInfo();
                    break;
                case 'tools':
                    await loadTools();
                    break;
                case 'resources':
                    await loadResources();
                    break;
                case 'history':
                    await loadHistory();
                    break;
            }
        }

        async function loadServerInfo() {
            try {
                const res = await fetch('/api/server-info');
                const data = await res.json();

                document.getElementById('server-info-cards').innerHTML = `
                    <div class="info-card">
                        <h3>Server Name</h3>
                        <div class="value">${data.name}</div>
                    </div>
                    <div class="info-card">
                        <h3>Version</h3>
                        <div class="value">${data.version}</div>
                    </div>
                    <div class="info-card">
                        <h3>Total Requests</h3>
                        <div class="value">${data.totalRequests}</div>
                    </div>
                    <div class="info-card">
                        <h3>Total Responses</h3>
                        <div class="value">${data.totalResponses}</div>
                    </div>
                `;

                await loadHistory();
            } catch(e) {
                document.getElementById('server-info-cards').innerHTML = `<div class="empty-state"><p>Error loading server info</p></div>`;
            }
        }

        async function loadTools() {
            try {
                const res = await fetch('/api/tools');
                const tools = await res.json();

                if (tools.length === 0) {
                    document.getElementById('tools-list').innerHTML = `<div class="empty-state"><h3>No tools available</h3></div>`;
                    return;
                }

                document.getElementById('tools-list').innerHTML = tools.map(tool => `
                    <div class="tool-card">
                        <h3>${tool.name}</h3>
                        <p>${tool.description || 'No description'}</p>
                        <button class="btn-primary" onclick="editTool('${tool.name}')">Test Tool</button>
                    </div>
                `).join('');
            } catch(e) {
                document.getElementById('tools-list').innerHTML = `<div class="empty-state"><p>Error loading tools</p></div>`;
            }
        }

        async function loadResources() {
            document.getElementById('resources-list').innerHTML = `<div class="empty-state"><h3>Resources</h3><p>Resource support coming soon</p></div>`;
        }

        async function loadHistory() {
            try {
                const res = await fetch('/api/requests');
                const requests = await res.json();

                if (requests.length === 0) {
                    document.getElementById('recent-activity').innerHTML = `<div class="empty-state"><p>No activity yet</p></div>`;
                    document.getElementById('history-list').innerHTML = `<div class="empty-state"><p>No requests yet</p></div>`;
                    return;
                }

                const recent = requests.slice(-5).reverse();
                document.getElementById('recent-activity').innerHTML = recent.map(r => `
                    <div class="history-item">
                        <strong>${r.method}</strong> - ${r.timestamp}
                    </div>
                `).join('');

                document.getElementById('history-list').innerHTML = requests.reverse().map((r, i) => `
                    <div class="history-item">
                        <div><strong>${r.method}</strong></div>
                        <div style="font-size: 12px; color: #a0aec0; margin-top: 4px;">${r.timestamp}</div>
                    </div>
                `).join('');
            } catch(e) {
                document.getElementById('recent-activity').innerHTML = `<div class="empty-state"><p>Error loading history</p></div>`;
            }
        }

        async function editTool(toolName) {
            const tools = await fetch('/api/tools').then(r => r.json());
            const tool = tools.find(t => t.name === toolName);

            if (!tool) {
                alert('Tool not found');
                return;
            }

            let formHTML = `<div style="padding: 20px; max-width: 500px;">
                <h3>${tool.name}</h3>
                <p style="color: #a0aec0; margin-bottom: 20px;">${tool.description || 'No description'}</p>
                <form id="tool-form" style="display: flex; flex-direction: column; gap: 15px;">`;

            // Generate form fields based on tool schema
            if (tool.input_schema && tool.input_schema.properties) {
                const props = tool.input_schema.properties;
                for (const [key, schema] of Object.entries(props)) {
                    const isRequired = tool.input_schema.required && tool.input_schema.required.includes(key);
                    formHTML += `
                        <div>
                            <label style="display: block; margin-bottom: 5px; color: #e2e8f0;">${key}${isRequired ? ' *' : ''}</label>
                            <input type="text" name="${key}" placeholder="${key}" style="width: 100%; padding: 8px; background: #2d3748; border: 1px solid #4a5568; color: #e2e8f0; border-radius: 4px;" ${isRequired ? 'required' : ''}>
                        </div>`;
                }
            }

            formHTML += `
                <button type="submit" class="btn-primary" style="width: 100%; margin-top: 10px;">Execute Tool</button>
            </form></div>`;

            // Show modal with form
            const modal = document.createElement('div');
            modal.style.cssText = 'position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.7); display: flex; align-items: center; justify-content: center; z-index: 1000;';
            modal.innerHTML = `<div style="background: #1a202c; border: 1px solid #4a5568; border-radius: 8px; box-shadow: 0 20px 25px rgba(0,0,0,0.3);">${formHTML}</div>`;
            document.body.appendChild(modal);

            document.getElementById('tool-form').addEventListener('submit', async (e) => {
                e.preventDefault();
                const formData = new FormData(e.target);
                const args = {};
                for (const [key, value] of formData.entries()) {
                    args[key] = value;
                }

                try {
                    const res = await fetch('/api/call-tool', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ tool_name: toolName, arguments: args })
                    });
                    const result = await res.json();
                    alert(`Tool Result:\n${JSON.stringify(result, null, 2)}`);
                    modal.remove();
                } catch(err) {
                    alert(`Error executing tool: ${err.message}`);
                }
            });

            modal.addEventListener('click', (e) => {
                if (e.target === modal) modal.remove();
            });
        }

        async function clearAll() {
            if (confirm('Clear all captured data?')) {
                await fetch('/api/clear', { method: 'POST' });
                location.reload();
            }
        }

        // Initial load
        loadServerInfo();
        setInterval(() => {
            if (document.getElementById('overview').classList.contains('active')) {
                loadServerInfo();
            }
        }, 5000);
    </script>
</body>
</html>
"#)
}

async fn handle_get_requests(State(state): State<InspectorState>) -> Json<Vec<InspectorRequest>> {
    let requests = state.captured_requests.lock().clone();
    Json(requests)
}

async fn handle_get_responses(State(state): State<InspectorState>) -> Json<Vec<InspectorResponse>> {
    let responses = state.captured_responses.lock().clone();
    Json(responses)
}

async fn handle_clear(State(state): State<InspectorState>) -> StatusCode {
    state.captured_requests.lock().clear();
    state.captured_responses.lock().clear();
    StatusCode::OK
}

async fn handle_server_info(State(state): State<InspectorState>) -> Json<serde_json::Value> {
    Json(json!({
        "name": state.server_name,
        "version": state.server_version,
        "totalRequests": state.captured_requests.lock().len(),
        "totalResponses": state.captured_responses.lock().len(),
    }))
}

async fn handle_get_tools(State(state): State<InspectorState>) -> Json<Vec<crate::protocol::Tool>> {
    // Capture the request
    state.captured_requests.lock().push(InspectorRequest {
        timestamp: chrono::Local::now().to_rfc3339(),
        method: "tools/list".to_string(),
        params: None,
    });

    let tools = state.tools.lock().clone();

    // Capture the response
    state.captured_responses.lock().push(InspectorResponse {
        timestamp: chrono::Local::now().to_rfc3339(),
        request_method: "tools/list".to_string(),
        result: Some(json!({ "tools": &tools })),
        error: None,
    });

    Json(tools)
}

/// Request body for tool execution
#[derive(Debug, Deserialize)]
pub struct ToolExecutionRequest {
    pub tool_name: String,
    pub arguments: serde_json::Value,
}

async fn handle_call_tool(
    State(state): State<InspectorState>,
    Json(req): Json<ToolExecutionRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Capture the request
    state.captured_requests.lock().push(InspectorRequest {
        timestamp: chrono::Local::now().to_rfc3339(),
        method: format!("tools/call/{}", req.tool_name),
        params: Some(req.arguments.clone()),
    });

    // Check if server is available
    if let Some(server) = &state.server {
        match server.handle_tool_call(&req.tool_name, req.arguments.clone()).await {
            Ok(result) => {
                state.capture_response(
                    format!("tools/call/{}", req.tool_name),
                    Some(json!(&result)),
                    None,
                );
                (StatusCode::OK, Json(json!(result)))
            }
            Err(e) => {
                state.capture_response(
                    format!("tools/call/{}", req.tool_name),
                    None,
                    Some(e.to_string()),
                );
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": e.to_string()
                    })),
                )
            }
        }
    } else {
        // Fallback if server not set
        (
            StatusCode::OK,
            Json(json!({
                "id": uuid::Uuid::new_v4().to_string(),
                "content": [
                    {
                        "type": "text",
                        "text": format!("Tool '{}' execution not yet integrated", req.tool_name)
                    }
                ]
            })),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inspector_creation() {
        let inspector = Inspector::new("Test Server".to_string(), "1.0.0".to_string());
        assert_eq!(inspector.state.server_name, "Test Server");
    }

    #[test]
    fn test_capture_request() {
        let inspector = Inspector::new("Test Server".to_string(), "1.0.0".to_string());
        inspector.capture_request("tools/list".to_string(), None);
        assert_eq!(inspector.state.captured_requests.lock().len(), 1);
    }

    #[test]
    fn test_capture_response() {
        let inspector = Inspector::new("Test Server".to_string(), "1.0.0".to_string());
        inspector.capture_response("tools/list".to_string(), Some(json!([])), None);
        assert_eq!(inspector.state.captured_responses.lock().len(), 1);
    }
}
