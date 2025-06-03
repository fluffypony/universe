// Copyright 2024. The Tari Project

//! Event Bridge - Connects Tari's internal event system to MCP event streams

use super::{
    event_types::MCPEvent,
    MCPEventManager,
};
use crate::{
    BaseNodeStatus, CpuMinerStatus, GpuMinerStatus, UniverseAppState,
    wallet_adapter::WalletBalance,
};
use anyhow::Result;
use std::sync::Arc;


const LOG_TARGET: &str = "tari::universe::mcp::event_bridge";

/// Bridges Tari's internal events to MCP event streams
pub struct MCPEventBridge {
    event_manager: Arc<MCPEventManager>,
    app_state: Arc<UniverseAppState>,
}

impl MCPEventBridge {
    pub fn new(event_manager: Arc<MCPEventManager>, app_state: Arc<UniverseAppState>) -> Self {
        Self {
            event_manager,
            app_state,
        }
    }

    /// Start monitoring all Tari events and bridging them to MCP
    pub async fn start(&self) -> Result<()> {
        log::info!(target: LOG_TARGET, "Starting MCP event bridge");

        // Start monitoring different event sources
        self.monitor_cpu_mining_status().await?;
        self.monitor_gpu_mining_status().await?;
        self.monitor_node_status().await?;
        self.monitor_wallet_balance().await?;
        // TODO: Add more event monitors as needed

        log::info!(target: LOG_TARGET, "MCP event bridge started successfully");
        Ok(())
    }

    /// Monitor CPU mining status changes
    async fn monitor_cpu_mining_status(&self) -> Result<()> {
        let event_manager = self.event_manager.clone();
        let mut cpu_status_rx = self.app_state.cpu_miner_status_watch_rx.as_ref().clone();
        let mut previous_status: Option<CpuMinerStatus> = None;

        tokio::spawn(async move {
            log::debug!(target: LOG_TARGET, "Started CPU mining status monitor");

            while cpu_status_rx.changed().await.is_ok() {
                let current_status = cpu_status_rx.borrow().clone();

                // Check if status actually changed
                if let Some(prev) = &previous_status {
                    if prev.is_mining == current_status.is_mining {
                        continue; // No significant change
                    }
                }

                log::debug!(target: LOG_TARGET, "CPU mining status changed: is_mining={}", current_status.is_mining);

                // Create MCP event
                let event = MCPEvent::MiningStatusChanged {
                    cpu_mining: current_status.is_mining,
                    gpu_mining: false, // We'll get this from GPU monitor
                    mode: "Unknown".to_string(), // TODO: Get actual mode
                    cpu_utilization: 0.0, // TODO: Get actual CPU utilization data
                    gpu_utilization: vec![], // From GPU monitor
                };

                if let Err(e) = event_manager.emit_event(event).await {
                    log::error!(target: LOG_TARGET, "Failed to emit CPU mining event: {}", e);
                }

                previous_status = Some(current_status);
            }

            log::debug!(target: LOG_TARGET, "CPU mining status monitor stopped");
        });

        Ok(())
    }

    /// Monitor GPU mining status changes
    async fn monitor_gpu_mining_status(&self) -> Result<()> {
        let event_manager = self.event_manager.clone();
        let mut gpu_status_rx = self.app_state.gpu_latest_status.as_ref().clone();
        let mut previous_status: Option<GpuMinerStatus> = None;

        tokio::spawn(async move {
            log::debug!(target: LOG_TARGET, "Started GPU mining status monitor");

            while gpu_status_rx.changed().await.is_ok() {
                let current_status = gpu_status_rx.borrow().clone();

                // Check if status actually changed
                if let Some(prev) = &previous_status {
                    if prev.is_mining == current_status.is_mining {
                        continue; // No significant change
                    }
                }

                log::debug!(target: LOG_TARGET, "GPU mining status changed: is_mining={}", current_status.is_mining);

                // Create MCP event
                let event = MCPEvent::MiningStatusChanged {
                    cpu_mining: false, // We'll get this from CPU monitor
                    gpu_mining: current_status.is_mining,
                    mode: "Unknown".to_string(), // TODO: Get actual mode
                    cpu_utilization: 0.0, // From CPU monitor
                    gpu_utilization: vec![], // TODO: Extract GPU utilization data
                };

                if let Err(e) = event_manager.emit_event(event).await {
                    log::error!(target: LOG_TARGET, "Failed to emit GPU mining event: {}", e);
                }

                previous_status = Some(current_status);
            }

            log::debug!(target: LOG_TARGET, "GPU mining status monitor stopped");
        });

        Ok(())
    }

    /// Monitor base node status changes
    async fn monitor_node_status(&self) -> Result<()> {
        let event_manager = self.event_manager.clone();
        let mut node_status_rx = self.app_state.node_status_watch_rx.as_ref().clone();
        let mut previous_status: Option<BaseNodeStatus> = None;

        tokio::spawn(async move {
            log::debug!(target: LOG_TARGET, "Started node status monitor");

            while node_status_rx.changed().await.is_ok() {
                let current_status = node_status_rx.borrow().clone();

                // Check if significant status changed
                let should_emit = if let Some(prev) = &previous_status {
                    prev.is_synced != current_status.is_synced ||
                    prev.num_connections != current_status.num_connections ||
                    (current_status.block_height as i64 - prev.block_height as i64).abs() > 10
                } else {
                    true // First status
                };

                if should_emit {
                    log::debug!(target: LOG_TARGET, "Node status changed: synced={}, connections={}, height={}", 
                        current_status.is_synced, 
                        current_status.num_connections,
                        current_status.block_height
                    );

                    // Create sync status event
                    let sync_event = MCPEvent::NodeSyncStatusChanged {
                        is_synced: current_status.is_synced,
                        sync_progress: if current_status.is_synced { 100.0 } else { 0.0 }, // TODO: Calculate actual progress
                        height: current_status.block_height,
                        network_height: current_status.block_height, // TODO: Get actual network height
                        num_connections: current_status.num_connections as usize,
                    };

                    if let Err(e) = event_manager.emit_event(sync_event).await {
                        log::error!(target: LOG_TARGET, "Failed to emit node sync event: {}", e);
                    }

                    // Create connection status event if connections changed significantly
                    if let Some(prev) = &previous_status {
                        if (prev.num_connections as i32 - current_status.num_connections as i32).abs() > 2 {
                            let conn_event = MCPEvent::NodeConnectionChanged {
                                connected: current_status.num_connections > 0,
                                peer_count: current_status.num_connections as usize,
                                network: "mainnet".to_string(), // TODO: Get actual network
                            };

                            if let Err(e) = event_manager.emit_event(conn_event).await {
                                log::error!(target: LOG_TARGET, "Failed to emit node connection event: {}", e);
                            }
                        }
                    }
                }

                previous_status = Some(current_status);
            }

            log::debug!(target: LOG_TARGET, "Node status monitor stopped");
        });

        Ok(())
    }

    /// Monitor wallet balance changes
    async fn monitor_wallet_balance(&self) -> Result<()> {
        let event_manager = self.event_manager.clone();
        let mut wallet_state_rx = self.app_state.wallet_state_watch_rx.as_ref().clone();
        let mut previous_balance: Option<WalletBalance> = None;

        tokio::spawn(async move {
            log::debug!(target: LOG_TARGET, "Started wallet balance monitor");

            while wallet_state_rx.changed().await.is_ok() {
                let wallet_state = wallet_state_rx.borrow().clone();
                
                // Skip if no wallet state or balance
                let wallet_state = match wallet_state {
                    Some(state) => state,
                    None => continue,
                };
                let current_balance = match wallet_state.balance {
                    Some(balance) => balance,
                    None => continue,
                };

                // Check if balance actually changed
                let should_emit = if let Some(prev) = &previous_balance {
                    prev.available_balance != current_balance.available_balance ||
                    prev.timelocked_balance != current_balance.timelocked_balance
                } else {
                    true // First balance
                };

                if should_emit {
                    log::debug!(target: LOG_TARGET, "Wallet balance changed: available={}, timelocked={}", 
                        current_balance.available_balance, 
                        current_balance.timelocked_balance
                    );

                    let total_balance = current_balance.available_balance + current_balance.timelocked_balance;

                    let event = MCPEvent::WalletBalanceChanged {
                        available: format!("{:.6}", current_balance.available_balance.as_u64() as f64 / 1_000_000.0),
                        timelocked: format!("{:.6}", current_balance.timelocked_balance.as_u64() as f64 / 1_000_000.0),
                        total: format!("{:.6}", total_balance.as_u64() as f64 / 1_000_000.0),
                    };

                    if let Err(e) = event_manager.emit_event(event).await {
                        log::error!(target: LOG_TARGET, "Failed to emit wallet balance event: {}", e);
                    }
                }

                previous_balance = Some(current_balance.clone());
            }

            log::debug!(target: LOG_TARGET, "Wallet balance monitor stopped");
        });

        Ok(())
    }

    /// Emit a custom event (for use by other parts of the application)
    pub async fn emit_custom_event(&self, event: MCPEvent) -> Result<()> {
        self.event_manager.emit_event(event).await
    }

    /// Emit an application error event
    pub async fn emit_error(&self, component: &str, message: &str, severity: &str) -> Result<()> {
        let event = MCPEvent::AppError {
            severity: severity.to_string(),
            component: component.to_string(),
            message: message.to_string(),
            details: None,
        };

        self.event_manager.emit_event(event).await
    }

    /// Emit a configuration change event
    pub async fn emit_config_change(&self, component: &str, changes: serde_json::Value) -> Result<()> {
        let event = MCPEvent::AppConfigChanged {
            component: component.to_string(),
            changes,
        };

        self.event_manager.emit_event(event).await
    }

    /// Get statistics about the event bridge
    pub async fn get_stats(&self) -> EventBridgeStats {
        EventBridgeStats {
            active_monitors: 4, // CPU, GPU, Node, Wallet
            subscribers: self.event_manager.subscriber_count().await,
        }
    }
}

/// Statistics about the event bridge
#[derive(Debug, Clone)]
pub struct EventBridgeStats {
    pub active_monitors: u32,
    pub subscribers: usize,
}
