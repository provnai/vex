//! MCP client implementation
//!
//! Provides a client for connecting to MCP servers and calling tools.
//! This implementation uses tokio-tungstenite for WebSocket communication.

use super::types::{McpConfig, McpError, McpToolInfo};
use crate::tool::{Capability, Tool, ToolDefinition};
use crate::tool_error::ToolError;
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{error, info};

/// JSON-RPC Request
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Value,
    id: u64,
}

/// JSON-RPC Response
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    _jsonrpc: String,
    result: Option<Value>,
    error: Option<JsonRpcError>,
    id: Option<u64>,
}

/// JSON-RPC Error
#[derive(Debug, Deserialize)]
struct JsonRpcError {
    _code: i32,
    message: String,
    #[allow(dead_code)]
    data: Option<Value>,
}

enum McpCommand {
    Call {
        method: String,
        params: Value,
        resp_tx: oneshot::Sender<Result<Value, McpError>>,
    },
    Shutdown,
}

/// MCP Client for connecting to and interacting with MCP servers.
pub struct McpClient {
    server_url: String,
    command_tx: mpsc::Sender<McpCommand>,
    connected: Arc<RwLock<bool>>,
    tools_cache: Arc<RwLock<Option<Vec<McpToolInfo>>>>,
}

impl McpClient {
    /// Connect to an MCP server.
    pub async fn connect(url: &str, config: McpConfig) -> Result<Self, McpError> {
        let is_localhost = url.contains("localhost") 
            || url.contains("127.0.0.1") 
            || url.contains("[::1]")
            || url.contains("0.0.0.0");

        if config.require_tls
            && !is_localhost
            && !url.starts_with("wss://")
            && !url.starts_with("https://")
        {
            return Err(McpError::TlsRequired);
        }

        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| McpError::ConnectionFailed(e.to_string()))?;

        info!(url = url, "Connected to MCP server");

        let (command_tx, mut command_rx) = mpsc::channel::<McpCommand>(32);
        let connected = Arc::new(RwLock::new(true));
        let connected_clone = connected.clone();

        // Background task for WebSocket handling
        tokio::spawn(async move {
            let (mut ws_tx, mut ws_rx) = ws_stream.split();
            let mut pending_requests: HashMap<u64, oneshot::Sender<Result<Value, McpError>>> =
                HashMap::new();
            let mut next_id = 1u64;

            loop {
                tokio::select! {
                    // Handle commands from the client
                    Some(cmd) = command_rx.recv() => {
                        match cmd {
                            McpCommand::Call { method, params, resp_tx } => {
                                let id = next_id;
                                next_id += 1;

                                let req = JsonRpcRequest {
                                    jsonrpc: "2.0".to_string(),
                                    method,
                                    params,
                                    id,
                                };

                                let json = serde_json::to_string(&req).unwrap();
                                if let Err(e) = ws_tx.send(Message::Text(json)).await {
                                    error!("WS send failed: {}", e);
                                    let _ = resp_tx.send(Err(McpError::ConnectionFailed(e.to_string())));
                                    break;
                                }
                                pending_requests.insert(id, resp_tx);
                            }
                            McpCommand::Shutdown => break,
                        }
                    }

                    // Handle messages from the server
                    Some(msg) = ws_rx.next() => {
                        match msg {
                            Ok(Message::Text(text)) => {
                                if let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(&text) {
                                    if let Some(id) = resp.id {
                                        if let Some(tx) = pending_requests.remove(&id) {
                                            if let Some(err) = resp.error {
                                                let _ = tx.send(Err(McpError::ExecutionFailed(err.message)));
                                            } else {
                                                let _ = tx.send(Ok(resp.result.unwrap_or(Value::Null)));
                                            }
                                        }
                                    }
                                }
                            }
                            Ok(Message::Close(_)) => {
                                info!("MCP server closed connection");
                                break;
                            }
                            Err(e) => {
                                error!("WS read error: {}", e);
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }

            *connected_clone.write().await = false;
        });

        let client = Self {
            server_url: url.to_string(),
            command_tx,
            connected,
            tools_cache: Arc::new(RwLock::new(None)),
        };

        // Initialize MCP protocol
        client.initialize().await?;

        Ok(client)
    }

    async fn initialize(&self) -> Result<(), McpError> {
        let params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "vex-client",
                "version": "0.1.5"
            }
        });

        self.call_raw("initialize", params).await?;
        // Note: notifications/initialized is often skipped in simple clients but can be added
        Ok(())
    }

    async fn call_raw(&self, method: &str, params: Value) -> Result<Value, McpError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.command_tx
            .send(McpCommand::Call {
                method: method.to_string(),
                params,
                resp_tx,
            })
            .await
            .map_err(|_| McpError::ConnectionFailed("Channel closed".into()))?;

        resp_rx
            .await
            .map_err(|_| McpError::ConnectionFailed("Response channel closed".into()))?
    }

    /// List available tools from the MCP server.
    pub async fn list_tools(&self) -> Result<Vec<McpToolInfo>, McpError> {
        if let Some(ref tools) = *self.tools_cache.read().await {
            return Ok(tools.clone());
        }

        let resp = self.call_raw("tools/list", Value::Null).await?;
        let tools: Vec<McpToolInfo> = serde_json::from_value(resp["tools"].clone())
            .map_err(|e| McpError::Serialization(e.to_string()))?;

        *self.tools_cache.write().await = Some(tools.clone());
        Ok(tools)
    }

    /// Call a tool on the MCP server.
    pub async fn call_tool(&self, name: &str, args: Value) -> Result<Value, McpError> {
        let params = serde_json::json!({
            "name": name,
            "arguments": args
        });

        self.call_raw("tools/call", params).await
    }

    pub fn server_url(&self) -> &str {
        &self.server_url
    }

    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    pub async fn disconnect(&self) {
        let _ = self.command_tx.send(McpCommand::Shutdown).await;
    }
}

/// Adapter that wraps an MCP tool to be used as a VEX Tool.
pub struct McpToolAdapter {
    client: Arc<McpClient>,
    info: McpToolInfo,
    definition: ToolDefinition,
}

impl McpToolAdapter {
    pub fn new(client: Arc<McpClient>, info: McpToolInfo) -> Self {
        let name: &'static str = Box::leak(info.name.clone().into_boxed_str());
        let description: &'static str = Box::leak(info.description.clone().into_boxed_str());
        let parameters: &'static str = Box::leak(
            serde_json::to_string(&info.input_schema)
                .unwrap_or_default()
                .into_boxed_str(),
        );

        let definition = ToolDefinition::new(name, description, parameters);

        Self {
            client,
            info,
            definition,
        }
    }
}

#[async_trait]
impl Tool for McpToolAdapter {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Network]
    }

    fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(30)
    }

    async fn execute(&self, args: Value) -> Result<Value, ToolError> {
        self.client
            .call_tool(&self.info.name, args)
            .await
            .map_err(|e| ToolError::execution_failed(&self.info.name, e.to_string()))
    }
}
