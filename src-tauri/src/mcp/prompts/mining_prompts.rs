// Copyright 2024. The Tari Project

use super::MCPPrompt;
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::UniverseAppState;
use crate::configs::config_mining::ConfigMining;
use crate::configs::trait_config::ConfigImpl;

/// Mining optimization prompt
pub struct MiningOptimizationPrompt;

#[async_trait::async_trait]
impl MCPPrompt for MiningOptimizationPrompt {
    async fn get_prompt(
        &self,
        _args: HashMap<String, Value>,
        app_state: Arc<UniverseAppState>,
    ) -> Result<String> {
        let cpu_status = app_state.cpu_miner_status_watch_rx.borrow().clone();
        let gpu_status = app_state.gpu_latest_status.borrow().clone();
        let mining_config = ConfigMining::content().await;
        
        let prompt = format!(
            r#"# Tari Universe Mining Analysis

You are helping optimize mining operations for Tari Universe. Here's the current status:

## Current Mining Status
- CPU Mining: {} (Hash Rate: {:.2} H/s)
- GPU Mining: {} (Hash Rate: {:.2} H/s)
- Total Hash Rate: {:.2} H/s
- Mining Mode: {:?}
- CPU Threads Enabled: {}
- GPU Mining Enabled: {}

## Available Actions
You can help by:
1. Starting/stopping CPU or GPU mining
2. Switching between mining modes (Eco, Ludicrous, Custom)
3. Enabling/disabling mining types
4. Analyzing performance and suggesting optimizations

## Performance Analysis
Based on the current status, provide recommendations for:
- Whether mining is profitable at current rates
- Optimal mining mode for the hardware
- Energy efficiency considerations
- When to mine vs when to stop

Please analyze the current mining setup and provide actionable recommendations."#,
            if cpu_status.is_mining { "ACTIVE" } else { "STOPPED" },
            cpu_status.hash_rate,
            if gpu_status.is_mining { "ACTIVE" } else { "STOPPED" },
            gpu_status.hash_rate,
            cpu_status.hash_rate + gpu_status.hash_rate,
            mining_config.mode(),
            mining_config.cpu_mining_enabled(),
            mining_config.gpu_mining_enabled()
        );

        Ok(prompt)
    }

    fn name(&self) -> &str {
        "mining_optimization"
    }

    fn description(&self) -> &str {
        "Get a comprehensive mining optimization analysis and recommendations"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }
}
