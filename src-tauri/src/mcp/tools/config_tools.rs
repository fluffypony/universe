// Copyright 2024. The Tari Project

use super::MCPTool;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tari_common::configuration::Network;

use crate::UniverseAppState;
use crate::mcp::security::{MCPConfig, MCPAuditEntry};
use crate::configs::config_mining::ConfigMining;
use crate::configs::config_core::ConfigCore;
use crate::configs::trait_config::ConfigImpl;

/// Get mining configuration tool
pub struct GetMiningConfigTool;

#[async_trait::async_trait]
impl MCPTool for GetMiningConfigTool {
    async fn execute(
        &self,
        _args: HashMap<String, Value>,
        _app_state: Arc<UniverseAppState>,
        _app_handle: tauri::AppHandle,
        _config: &MCPConfig,
    ) -> Result<Value> {
        let mining_config = ConfigMining::content().await;
        
        Ok(json!({
            "cpu_mining_enabled": mining_config.cpu_mining_enabled(),
            "gpu_mining_enabled": mining_config.gpu_mining_enabled(),
            "mining_mode": format!("{:?}", mining_config.mode()),
            "mine_on_app_start": mining_config.mine_on_app_start(),
            "custom_max_cpu_usage": mining_config.custom_max_cpu_usage(),
            "custom_max_gpu_usage": mining_config.custom_max_gpu_usage(),
            "gpu_engine": format!("{:?}", mining_config.gpu_engine()),
            "mining_time_ms": mining_config.mining_time(),
        }))
    }

    fn name(&self) -> &str {
        "get_mining_config"
    }

    fn description(&self) -> &str {
        "Get current mining configuration settings"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    fn should_audit(&self) -> bool {
        false // Reading config is low-risk
    }
}

/// Enable/disable CPU mining tool
pub struct SetCpuMiningEnabledTool;

#[async_trait::async_trait]
impl MCPTool for SetCpuMiningEnabledTool {
    async fn execute(
        &self,
        args: HashMap<String, Value>,
        _app_state: Arc<UniverseAppState>,
        _app_handle: tauri::AppHandle,
        _config: &MCPConfig,
    ) -> Result<Value> {
        let audit = MCPAuditEntry::new("set_cpu_mining_enabled".to_string());

        let enabled = args.get("enabled")
            .and_then(|v| v.as_bool())
            .ok_or_else(|| anyhow!("Missing required parameter: enabled"))?;

        match crate::commands::set_cpu_mining_enabled(enabled).await {
            Ok(_) => {
                audit.with_success(true)
                    .with_details(json!({"cpu_mining_enabled": enabled}))
                    .log();
                Ok(json!({
                    "success": true,
                    "message": format!("CPU mining {}", if enabled { "enabled" } else { "disabled" }),
                    "cpu_mining_enabled": enabled
                }))
            }
            Err(e) => {
                audit.with_error(e.clone()).log();
                Err(anyhow!("Failed to set CPU mining enabled: {}", e))
            }
        }
    }

    fn name(&self) -> &str {
        "set_cpu_mining_enabled"
    }

    fn description(&self) -> &str {
        "Enable or disable CPU mining"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "enabled": {
                    "type": "boolean",
                    "description": "Whether to enable CPU mining"
                }
            },
            "required": ["enabled"]
        })
    }
}

/// Enable/disable GPU mining tool
pub struct SetGpuMiningEnabledTool;

#[async_trait::async_trait]
impl MCPTool for SetGpuMiningEnabledTool {
    async fn execute(
        &self,
        args: HashMap<String, Value>,
        _app_state: Arc<UniverseAppState>,
        _app_handle: tauri::AppHandle,
        _config: &MCPConfig,
    ) -> Result<Value> {
        let audit = MCPAuditEntry::new("set_gpu_mining_enabled".to_string());

        let enabled = args.get("enabled")
            .and_then(|v| v.as_bool())
            .ok_or_else(|| anyhow!("Missing required parameter: enabled"))?;

        match crate::commands::set_gpu_mining_enabled(enabled).await {
            Ok(_) => {
                audit.with_success(true)
                    .with_details(json!({"gpu_mining_enabled": enabled}))
                    .log();
                Ok(json!({
                    "success": true,
                    "message": format!("GPU mining {}", if enabled { "enabled" } else { "disabled" }),
                    "gpu_mining_enabled": enabled
                }))
            }
            Err(e) => {
                audit.with_error(format!("{:?}", e)).log();
                Err(anyhow!("Failed to set GPU mining enabled: {:?}", e))
            }
        }
    }

    fn name(&self) -> &str {
        "set_gpu_mining_enabled"
    }

    fn description(&self) -> &str {
        "Enable or disable GPU mining"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "enabled": {
                    "type": "boolean",
                    "description": "Whether to enable GPU mining"
                }
            },
            "required": ["enabled"]
        })
    }
}

/// Get application settings tool
pub struct GetAppSettingsTool;

#[async_trait::async_trait]
impl MCPTool for GetAppSettingsTool {
    async fn execute(
        &self,
        _args: HashMap<String, Value>,
        _app_state: Arc<UniverseAppState>,
        _app_handle: tauri::AppHandle,
        _config: &MCPConfig,
    ) -> Result<Value> {
        let core_config = ConfigCore::content().await;
        
        Ok(json!({
            "network": Network::get_current_or_user_setting_or_default().to_string(),
            "use_tor": core_config.use_tor(),
            "p2pool_enabled": core_config.is_p2pool_enabled(),
            "node_type": format!("{:?}", core_config.node_type()),
            "auto_update": core_config.auto_update(),
            "allow_telemetry": core_config.allow_telemetry(),
            "allow_notifications": core_config.allow_notifications(),
            "should_auto_launch": core_config.should_auto_launch(),
            "pre_release": core_config.pre_release(),
        }))
    }

    fn name(&self) -> &str {
        "get_app_settings"
    }

    fn description(&self) -> &str {
        "Get current application settings and configuration"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    fn should_audit(&self) -> bool {
        false // Reading settings is low-risk
    }
}
