// Copyright 2024. The Tari Project

use super::MCPResource;
use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;
use tari_common::configuration::Network;

use crate::UniverseAppState;
use crate::configs::config_core::ConfigCore;
use crate::configs::trait_config::ConfigImpl;

/// Application state resource
pub struct AppStateResource;

#[async_trait::async_trait]
impl MCPResource for AppStateResource {
    async fn get_data(&self, app_state: Arc<UniverseAppState>) -> Result<Value> {
        let in_memory_config = app_state.in_memory_config.read().await;
        
        Ok(json!({
            "airdrop_url": in_memory_config.in_memory_config.airdrop_url,
            "airdrop_api_url": in_memory_config.in_memory_config.airdrop_api_url,
            "telemetry_api_url": in_memory_config.in_memory_config.telemetry_api_url,
            "exchange_id": in_memory_config.in_memory_config.exchange_id,
            "wallet_connect_project_id": in_memory_config.in_memory_config.wallet_connect_project_id,
            "is_universal_miner": in_memory_config.is_universal_miner(),
            "miner_type": format!("{:?}", in_memory_config.miner_type),
        }))
    }

    fn name(&self) -> &str {
        "app_state"
    }

    fn description(&self) -> &str {
        "Current application state and configuration"
    }
}

/// Node status resource
pub struct NodeStatusResource;

#[async_trait::async_trait]
impl MCPResource for NodeStatusResource {
    async fn get_data(&self, app_state: Arc<UniverseAppState>) -> Result<Value> {
        let base_node_status = app_state.node_status_watch_rx.borrow().clone();
        
        Ok(json!({
            "is_connected": base_node_status.num_connections > 0,
            "block_height": base_node_status.block_height,
            "block_time": base_node_status.block_time,
            "peer_count": base_node_status.num_connections,
            "is_synced": base_node_status.is_synced,
            "sync_status": if base_node_status.is_synced { "synced" } else { "syncing" }
        }))
    }

    fn name(&self) -> &str {
        "node_status"
    }

    fn description(&self) -> &str {
        "Base node connectivity and synchronization status"
    }
}

/// Network statistics resource
pub struct NetworkStatsResource;

#[async_trait::async_trait]
impl MCPResource for NetworkStatsResource {
    async fn get_data(&self, app_state: Arc<UniverseAppState>) -> Result<Value> {
        let base_node_status = app_state.node_status_watch_rx.borrow().clone();
        let config = ConfigCore::content().await;
        
        Ok(json!({
            "network": Network::get_current_or_user_setting_or_default().to_string(),
            "use_tor": config.use_tor(),
            "p2pool_enabled": config.is_p2pool_enabled(),
            "node_type": format!("{:?}", config.node_type()),
            "connection_status": {
                "base_node_connected": base_node_status.num_connections > 0,
                "peer_count": base_node_status.num_connections,
                "current_block_height": base_node_status.block_height,
            }
        }))
    }

    fn name(&self) -> &str {
        "network_stats"
    }

    fn description(&self) -> &str {
        "Network configuration and connection statistics"
    }
}

/// External dependencies status resource
pub struct ExternalDependenciesResource;

#[async_trait::async_trait]
impl MCPResource for ExternalDependenciesResource {
    async fn get_data(&self, _app_state: Arc<UniverseAppState>) -> Result<Value> {
        // This would typically check for external dependencies like MSVC redistributables on Windows
        // For now, return a basic structure
        Ok(json!({
            "status": "checking",
            "dependencies": [],
            "all_satisfied": true,
            "message": "External dependencies check not fully implemented in MCP server"
        }))
    }

    fn name(&self) -> &str {
        "external_dependencies"
    }

    fn description(&self) -> &str {
        "Status of required external dependencies"
    }
}
