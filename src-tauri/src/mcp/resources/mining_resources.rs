// Copyright 2024. The Tari Project

use super::MCPResource;
use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::UniverseAppState;
use crate::configs::config_mining::ConfigMining;
use crate::configs::trait_config::ConfigImpl;

/// Mining status resource
pub struct MiningStatusResource;

#[async_trait::async_trait]
impl MCPResource for MiningStatusResource {
    async fn get_data(&self, app_state: Arc<UniverseAppState>) -> Result<Value> {
        let cpu_status = app_state.cpu_miner_status_watch_rx.borrow().clone();
        let gpu_status = app_state.gpu_latest_status.borrow().clone();
        
        Ok(json!({
            "cpu_mining": {
                "is_mining": cpu_status.is_mining,
                "hash_rate": cpu_status.hash_rate,
                "estimated_earnings": cpu_status.estimated_earnings,
                "is_connected": cpu_status.connection.is_connected,
            },
            "gpu_mining": {
                "is_mining": gpu_status.is_mining,
                "hash_rate": gpu_status.hash_rate,
                "estimated_earnings": gpu_status.estimated_earnings,
                "is_mining": gpu_status.is_mining,
            },
            "overall": {
                "any_mining": cpu_status.is_mining || gpu_status.is_mining,
                "total_hash_rate": cpu_status.hash_rate + gpu_status.hash_rate,
                "total_estimated_earnings": cpu_status.estimated_earnings + gpu_status.estimated_earnings,
            }
        }))
    }

    fn name(&self) -> &str {
        "mining_status"
    }

    fn description(&self) -> &str {
        "Current mining status for CPU and GPU miners"
    }
}

/// Mining configuration resource
pub struct MiningConfigResource;

#[async_trait::async_trait]
impl MCPResource for MiningConfigResource {
    async fn get_data(&self, _app_state: Arc<UniverseAppState>) -> Result<Value> {
        let config = ConfigMining::content().await;
        
        Ok(json!({
            "cpu_mining_enabled": config.cpu_mining_enabled(),
            "gpu_mining_enabled": config.gpu_mining_enabled(),
            "mining_mode": format!("{:?}", config.mode()),
            "mine_on_app_start": config.mine_on_app_start(),
            "custom_max_cpu_usage": config.custom_max_cpu_usage(),
            "custom_max_gpu_usage": config.custom_max_gpu_usage(),
            "gpu_engine": format!("{:?}", config.gpu_engine()),
            "mining_time_ms": config.mining_time(),
        }))
    }

    fn name(&self) -> &str {
        "mining_config"
    }

    fn description(&self) -> &str {
        "Current mining configuration settings"
    }
}

/// Hardware information resource
pub struct HardwareInfoResource;

#[async_trait::async_trait]
impl MCPResource for HardwareInfoResource {
    async fn get_data(&self, app_state: Arc<UniverseAppState>) -> Result<Value> {
        // Get available CPU threads
        let max_cpu_threads = std::thread::available_parallelism()
            .map(|cores| cores.get())
            .unwrap_or(1);

        // Get GPU devices
        let gpu_devices = app_state
            .gpu_miner
            .read()
            .await
            .get_gpu_devices()
            .await
            .unwrap_or_default();

        let gpu_info: Vec<Value> = gpu_devices
            .into_iter()
            .map(|gpu| json!({
                "device_name": gpu.device_name,
                "device_index": gpu.device_index,
                "max_threads": 8192, // As per the original code logic
            }))
            .collect();

        Ok(json!({
            "cpu": {
                "max_threads": max_cpu_threads,
                "available_threads": max_cpu_threads,
            },
            "gpu": {
                "devices": gpu_info,
                "device_count": gpu_info.len(),
                "available": !gpu_info.is_empty(),
            }
        }))
    }

    fn name(&self) -> &str {
        "hardware_info"
    }

    fn description(&self) -> &str {
        "Available hardware information for mining (CPU and GPU)"
    }
}

/// P2Pool statistics resource
pub struct P2PoolStatsResource;

#[async_trait::async_trait]
impl MCPResource for P2PoolStatsResource {
    async fn get_data(&self, app_state: Arc<UniverseAppState>) -> Result<Value> {
        let p2pool_stats = app_state.p2pool_latest_status.borrow().clone();
        
        match p2pool_stats {
            Some(stats) => Ok(json!({
                "is_enabled": true,
                "stats": {
                    "connected": stats.connected_since.is_some(),
                    "peer_id": &stats.peer_id,
                    "squad": &stats.squad,
                    "randomx_stats": {
                        "height": stats.randomx_stats.height,
                    },
                    "sha3x_stats": {
                        "height": stats.sha3x_stats.height,
                    },
                }
            })),
            None => Ok(json!({
                "is_enabled": false,
                "stats": null,
                "message": "P2Pool stats not available"
            }))
        }
    }

    fn name(&self) -> &str {
        "p2pool_stats"
    }

    fn description(&self) -> &str {
        "P2Pool mining statistics and status"
    }
}
