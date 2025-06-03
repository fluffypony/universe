// Copyright 2024. The Tari Project

//! MCP Resources - Read-only data endpoints for AI agents

pub mod wallet_resources;
pub mod mining_resources;
pub mod state_resources;

pub use wallet_resources::*;
pub use mining_resources::*;
pub use state_resources::*;

use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;

use crate::UniverseAppState;

/// Base trait for all MCP resources
#[async_trait::async_trait]
pub trait MCPResource {
    /// Get the resource data
    async fn get_data(&self, app_state: Arc<UniverseAppState>) -> Result<Value>;
    
    /// Get the resource name/identifier
    fn name(&self) -> &str;
    
    /// Get a human-readable description of this resource
    fn description(&self) -> &str;
    
    /// Get the MIME type of the resource data
    fn mime_type(&self) -> &str {
        "application/json"
    }
}
