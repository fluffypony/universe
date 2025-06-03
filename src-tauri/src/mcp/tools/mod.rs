// Copyright 2024. The Tari Project

//! MCP Tools - Action endpoints for AI agents

pub mod wallet_tools;
pub mod mining_tools;
pub mod config_tools;

pub use wallet_tools::*;
pub use mining_tools::*;
pub use config_tools::*;

use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::UniverseAppState;
use crate::mcp::security::MCPConfig;

/// Base trait for all MCP tools
#[async_trait::async_trait]
pub trait MCPTool {
    /// Execute the tool with the given arguments
    async fn execute(
        &self,
        args: HashMap<String, Value>,
        app_state: Arc<UniverseAppState>,
        app_handle: tauri::AppHandle,
        config: &MCPConfig,
    ) -> Result<Value>;
    
    /// Get the tool name/identifier
    fn name(&self) -> &str;
    
    /// Get a human-readable description of this tool
    fn description(&self) -> &str;
    
    /// Get the JSON schema for the tool's input arguments
    fn input_schema(&self) -> Value;
    
    /// Check if this tool requires special permissions
    fn requires_wallet_send_permission(&self) -> bool {
        false
    }
    
    /// Check if this tool should be audited
    fn should_audit(&self) -> bool {
        true
    }
}
