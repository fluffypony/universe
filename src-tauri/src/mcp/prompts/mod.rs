// Copyright 2024. The Tari Project

//! MCP Prompts - Template prompts for AI agents

pub mod mining_prompts;

// pub use mining_prompts::*; // Temporarily commented out as prompts are not used yet

use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::UniverseAppState;

/// Base trait for all MCP prompts
#[async_trait::async_trait]
pub trait MCPPrompt {
    /// Get the prompt content with the given arguments
    async fn get_prompt(
        &self,
        args: HashMap<String, Value>,
        app_state: Arc<UniverseAppState>,
    ) -> Result<String>;
    
    /// Get the prompt name/identifier
    fn name(&self) -> &str;
    
    /// Get a human-readable description of this prompt
    fn description(&self) -> &str;
    
    /// Get the JSON schema for the prompt's input arguments
    fn input_schema(&self) -> Value;
}
