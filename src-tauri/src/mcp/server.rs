// Copyright 2024. The Tari Project

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
// use uuid::Uuid; // Not used yet

use crate::UniverseAppState;
use crate::mcp::security::MCPConfig;
use crate::mcp::resources::*;
use crate::mcp::tools::*;
use crate::mcp::events::*;

const LOG_TARGET: &str = "tari::universe::mcp::server";

/// MCP Server implementation for Tari Universe
pub struct TariMCPServer {
    app_state: Arc<UniverseAppState>,
    app_handle: tauri::AppHandle,
    config: MCPConfig,
    resources: Vec<Box<dyn MCPResource + Send + Sync>>,
    tools: Vec<Box<dyn MCPTool + Send + Sync>>,
    // WebSocket event streaming components
    event_manager: Option<Arc<MCPEventManager>>,
    websocket_server: Option<MCPWebSocketServer>,
    event_bridge: Option<MCPEventBridge>,
}

impl TariMCPServer {
    /// Create a new MCP server instance
    pub async fn new(app_state: Arc<UniverseAppState>, app_handle: tauri::AppHandle, config: MCPConfig) -> Result<Self> {
        config.validate()?;
        
        let mut server = Self {
            app_state,
            app_handle,
            config,
            resources: Vec::new(),
            tools: Vec::new(),
            event_manager: None,
            websocket_server: None,
            event_bridge: None,
        };

        server.register_resources();
        server.register_tools();

        log::info!(target: LOG_TARGET, "MCP server initialized with {} resources and {} tools", 
                  server.resources.len(), server.tools.len());

        Ok(server)
    }

    /// Register all available resources
    fn register_resources(&mut self) {
        // Wallet resources
        self.resources.push(Box::new(WalletBalanceResource));
        self.resources.push(Box::new(WalletAddressResource));
        self.resources.push(Box::new(TransactionHistoryResource));
        self.resources.push(Box::new(CoinbaseTransactionsResource));

        // Mining resources
        self.resources.push(Box::new(MiningStatusResource));
        self.resources.push(Box::new(MiningConfigResource));
        self.resources.push(Box::new(HardwareInfoResource));
        self.resources.push(Box::new(P2PoolStatsResource));

        // State resources
        self.resources.push(Box::new(AppStateResource));
        self.resources.push(Box::new(NodeStatusResource));
        self.resources.push(Box::new(NetworkStatsResource));
        self.resources.push(Box::new(ExternalDependenciesResource));
    }

    /// Register all available tools
    fn register_tools(&mut self) {
        // Mining tools
        self.tools.push(Box::new(StartCpuMiningTool));
        self.tools.push(Box::new(StopCpuMiningTool));
        self.tools.push(Box::new(StartGpuMiningTool));
        self.tools.push(Box::new(StopGpuMiningTool));
        self.tools.push(Box::new(SetMiningModeTool));

        // Config tools
        self.tools.push(Box::new(GetMiningConfigTool));
        self.tools.push(Box::new(SetCpuMiningEnabledTool));
        self.tools.push(Box::new(SetGpuMiningEnabledTool));
        self.tools.push(Box::new(GetAppSettingsTool));

        // Wallet tools
        self.tools.push(Box::new(ValidateAddressTool));
        self.tools.push(Box::new(SendTariTool)); // Requires permission
    }

    /// Initialize WebSocket event streaming (optional feature)
    pub async fn initialize_websocket_streaming(&mut self) -> Result<()> {
        log::info!(target: LOG_TARGET, "Initializing WebSocket event streaming...");

        // Create event manager
        let event_manager = Arc::new(MCPEventManager::new());
        
        // Create WebSocket server
        let websocket_server = MCPWebSocketServer::new(event_manager.clone(), self.config.clone());
        
        // Create event bridge
        let event_bridge = MCPEventBridge::new(event_manager.clone(), self.app_state.clone());

        // Store components
        self.event_manager = Some(event_manager);
        self.websocket_server = Some(websocket_server);
        self.event_bridge = Some(event_bridge);

        log::info!(target: LOG_TARGET, "WebSocket event streaming initialized");
        Ok(())
    }

    /// Start WebSocket event streaming
    pub async fn start_websocket_streaming(&mut self) -> Result<()> {
        if self.event_manager.is_none() {
            self.initialize_websocket_streaming().await?;
        }

        if let (Some(websocket_server), Some(event_bridge)) = 
            (self.websocket_server.as_mut(), self.event_bridge.as_ref()) {
            
            log::info!(target: LOG_TARGET, "Starting WebSocket event streaming...");
            
            // Start WebSocket server
            websocket_server.start().await?;
            
            // Start event bridge to monitor Tari events
            event_bridge.start().await?;
            
            log::info!(target: LOG_TARGET, "WebSocket event streaming started successfully");
        }

        Ok(())
    }

    /// Stop WebSocket event streaming
    pub async fn stop_websocket_streaming(&self) -> Result<()> {
        if let Some(websocket_server) = &self.websocket_server {
            log::info!(target: LOG_TARGET, "Stopping WebSocket event streaming...");
            websocket_server.stop().await?;
            log::info!(target: LOG_TARGET, "WebSocket event streaming stopped");
        }
        Ok(())
    }

    /// Get WebSocket streaming statistics
    pub async fn get_websocket_stats(&self) -> Option<WebSocketStats> {
        if let (Some(websocket_server), Some(event_bridge)) = 
            (&self.websocket_server, &self.event_bridge) {
            
            Some(WebSocketStats {
                connected_clients: websocket_server.client_count().await,
                bridge_stats: event_bridge.get_stats().await,
                client_stats: websocket_server.get_client_stats().await,
            })
        } else {
            None
        }
    }

    /// Emit a custom event through the WebSocket stream
    pub async fn emit_event(&self, event: MCPEvent) -> Result<()> {
        if let Some(event_bridge) = &self.event_bridge {
            event_bridge.emit_custom_event(event).await?;
        }
        Ok(())
    }

    /// Start the MCP server (traditional stdio mode)
    pub async fn start(&self) -> Result<()> {
        log::info!(target: LOG_TARGET, "Starting MCP server...");
        
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let mut reader = BufReader::new(stdin).lines();

        // Send server info
        self.send_server_info(&mut stdout).await?;

        while let Some(line) = reader.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }

            match self.handle_message(&line).await {
                Ok(response) => {
                    if let Some(resp) = response {
                        stdout.write_all(resp.as_bytes()).await?;
                        stdout.write_all(b"\n").await?;
                        stdout.flush().await?;
                    }
                }
                Err(e) => {
                    log::error!(target: LOG_TARGET, "Error handling message: {}", e);
                    let error_response = json!({
                        "jsonrpc": "2.0",
                        "error": {
                            "code": -32603,
                            "message": e.to_string()
                        },
                        "id": null
                    });
                    stdout.write_all(error_response.to_string().as_bytes()).await?;
                    stdout.write_all(b"\n").await?;
                    stdout.flush().await?;
                }
            }
        }

        Ok(())
    }

    /// Send server information to client
    async fn send_server_info<W: AsyncWriteExt + Unpin>(&self, writer: &mut W) -> Result<()> {
        let server_info = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "logging": {},
                    "prompts": {
                        "listChanged": false
                    },
                    "resources": {
                        "subscribe": false,
                        "listChanged": false
                    },
                    "tools": {
                        "listChanged": false
                    }
                },
                "serverInfo": {
                    "name": "tari-universe-mcp-server",
                    "version": "1.0.0"
                }
            }
        });

        writer.write_all(server_info.to_string().as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
        Ok(())
    }

    /// Handle incoming MCP message
    async fn handle_message(&self, message: &str) -> Result<Option<String>> {
        let parsed: Value = serde_json::from_str(message)?;
        
        let method = parsed.get("method")
            .and_then(|m| m.as_str())
            .ok_or_else(|| anyhow!("Missing method in request"))?;

        let id = parsed.get("id").cloned();
        let params = parsed.get("params").cloned().unwrap_or(Value::Null);

        let id = id.unwrap_or_else(|| Value::Number(serde_json::Number::from(0)));
        
        match method {
            "initialize" => Ok(Some(self.handle_initialize(id).await?)),
            "resources/list" => Ok(Some(self.handle_list_resources(id).await?)),
            "resources/read" => Ok(Some(self.handle_read_resource(id, params).await?)),
            "tools/list" => Ok(Some(self.handle_list_tools(id).await?)),
            "tools/call" => Ok(Some(self.handle_call_tool(id, params).await?)),
            "ping" => Ok(Some(self.handle_ping(id).await?)),
            _ => {
                log::warn!(target: LOG_TARGET, "Unknown method: {}", method);
                Err(anyhow!("Unknown method: {}", method))
            }
        }
    }

    /// Handle initialize request
    async fn handle_initialize(&self, id: Value) -> Result<String> {
        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "logging": {},
                    "prompts": {
                        "listChanged": false
                    },
                    "resources": {
                        "subscribe": false,
                        "listChanged": false
                    },
                    "tools": {
                        "listChanged": false
                    }
                },
                "serverInfo": {
                    "name": "tari-universe-mcp-server",
                    "version": "1.0.0"
                }
            }
        });

        Ok(response.to_string())
    }

    /// Handle list resources request
    async fn handle_list_resources(&self, id: Value) -> Result<String> {
        let resources: Vec<Value> = self.resources.iter().map(|r| {
            json!({
                "uri": format!("tari://{}", r.name()),
                "name": r.name(),
                "description": r.description(),
                "mimeType": r.mime_type()
            })
        }).collect();

        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "resources": resources
            }
        });

        Ok(response.to_string())
    }

    /// Handle read resource request
    async fn handle_read_resource(&self, id: Value, params: Value) -> Result<String> {
        let uri = params.get("uri")
            .and_then(|u| u.as_str())
            .ok_or_else(|| anyhow!("Missing uri parameter"))?;

        // Extract resource name from URI
        let resource_name = uri.strip_prefix("tari://")
            .ok_or_else(|| anyhow!("Invalid URI format"))?;

        // Find the resource
        let resource = self.resources.iter()
            .find(|r| r.name() == resource_name)
            .ok_or_else(|| anyhow!("Resource not found: {}", resource_name))?;

        // Get resource data
        let data = resource.get_data(self.app_state.clone()).await?;

        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "contents": [{
                    "uri": uri,
                    "mimeType": resource.mime_type(),
                    "text": data.to_string()
                }]
            }
        });

        Ok(response.to_string())
    }

    /// Handle list tools request
    async fn handle_list_tools(&self, id: Value) -> Result<String> {
        let tools: Vec<Value> = self.tools.iter().map(|t| {
            json!({
                "name": t.name(),
                "description": t.description(),
                "inputSchema": t.input_schema()
            })
        }).collect();

        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": tools
            }
        });

        Ok(response.to_string())
    }

    /// Handle call tool request
    async fn handle_call_tool(&self, id: Value, params: Value) -> Result<String> {
        let tool_name = params.get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| anyhow!("Missing tool name"))?;

        let arguments = params.get("arguments")
            .and_then(|a| a.as_object())
            .map(|obj| {
                obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<HashMap<String, Value>>()
            })
            .unwrap_or_default();

        // Find the tool
        let tool = self.tools.iter()
            .find(|t| t.name() == tool_name)
            .ok_or_else(|| anyhow!("Tool not found: {}", tool_name))?;

        // Check permissions
        if tool.requires_wallet_send_permission() && !self.config.can_send_wallet_transactions() {
            return Ok(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32603,
                    "message": "Wallet send operations are disabled. Enable 'allow_wallet_send' in MCP configuration."
                }
            }).to_string());
        }

        // Execute the tool
        match tool.execute(arguments, self.app_state.clone(), self.app_handle.clone(), &self.config).await {
            Ok(result) => {
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [{
                            "type": "text",
                            "text": result.to_string()
                        }]
                    }
                });
                Ok(response.to_string())
            }
            Err(e) => {
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32603,
                        "message": e.to_string()
                    }
                });
                Ok(response.to_string())
            }
        }
    }

    /// Handle ping request
    async fn handle_ping(&self, id: Value) -> Result<String> {
        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "message": "pong"
            }
        });

        Ok(response.to_string())
    }
}

/// WebSocket streaming statistics
#[derive(Debug, Clone)]
pub struct WebSocketStats {
    pub connected_clients: usize,
    pub bridge_stats: EventBridgeStats,
    pub client_stats: std::collections::HashMap<String, ConnectionStats>,
}
