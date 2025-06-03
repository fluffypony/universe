// Copyright 2024. The Tari Project

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

/// All possible event types that can be streamed to MCP clients
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum MCPEvent {
    /// Wallet balance has changed
    #[serde(rename = "wallet.balance_changed")]
    WalletBalanceChanged {
        available: String,
        timelocked: String,
        total: String,
    },

    /// New transaction received or status changed
    #[serde(rename = "wallet.transaction_update")]
    WalletTransactionUpdate {
        tx_id: String,
        direction: String, // "inbound" or "outbound"
        amount: String,
        status: String,
        confirmation_count: u64,
        timestamp: u64,
    },

    /// Mining status has changed (CPU or GPU)
    #[serde(rename = "mining.status_changed")]
    MiningStatusChanged {
        cpu_mining: bool,
        gpu_mining: bool,
        mode: String,
        cpu_utilization: f64,
        gpu_utilization: Vec<f64>,
    },

    /// Mining mode has been changed
    #[serde(rename = "mining.mode_changed")]
    MiningModeChanged {
        previous_mode: String,
        new_mode: String,
        timestamp: u64,
    },

    /// Block has been found/mined
    #[serde(rename = "mining.block_found")]
    BlockFound {
        height: u64,
        hash: String,
        reward: String,
        timestamp: u64,
    },

    /// Node sync status has changed
    #[serde(rename = "node.sync_status_changed")]
    NodeSyncStatusChanged {
        is_synced: bool,
        sync_progress: f64,
        height: u64,
        network_height: u64,
        num_connections: usize,
    },

    /// Base node connection status changed
    #[serde(rename = "node.connection_changed")]
    NodeConnectionChanged {
        connected: bool,
        peer_count: usize,
        network: String,
    },

    /// P2Pool statistics update
    #[serde(rename = "p2pool.stats_update")]
    P2PoolStatsUpdate {
        hash_rate: u64,
        share_count: u64,
        pool_hash_rate: u64,
        connected_miners: u32,
    },

    /// Application configuration has changed
    #[serde(rename = "app.config_changed")]
    AppConfigChanged {
        component: String, // "mining", "wallet", "core", etc.
        changes: Value,    // JSON object of what changed
    },

    /// Error or warning occurred
    #[serde(rename = "app.error")]
    AppError {
        severity: String, // "error", "warning", "info"
        component: String,
        message: String,
        details: Option<Value>,
    },

    /// Application status update
    #[serde(rename = "app.status_update")]
    AppStatusUpdate {
        component: String,
        status: String,
        message: Option<String>,
    },
}

impl MCPEvent {
    /// Get the event type as a string
    pub fn event_type(&self) -> &'static str {
        match self {
            MCPEvent::WalletBalanceChanged { .. } => "wallet.balance_changed",
            MCPEvent::WalletTransactionUpdate { .. } => "wallet.transaction_update",
            MCPEvent::MiningStatusChanged { .. } => "mining.status_changed",
            MCPEvent::MiningModeChanged { .. } => "mining.mode_changed",
            MCPEvent::BlockFound { .. } => "mining.block_found",
            MCPEvent::NodeSyncStatusChanged { .. } => "node.sync_status_changed",
            MCPEvent::NodeConnectionChanged { .. } => "node.connection_changed",
            MCPEvent::P2PoolStatsUpdate { .. } => "p2pool.stats_update",
            MCPEvent::AppConfigChanged { .. } => "app.config_changed",
            MCPEvent::AppError { .. } => "app.error",
            MCPEvent::AppStatusUpdate { .. } => "app.status_update",
        }
    }

    /// Get the event category (before the dot)
    pub fn category(&self) -> &str {
        self.event_type().split('.').next().unwrap_or("unknown")
    }

    /// Create a timestamped event wrapper for transmission
    pub fn to_stream_event(&self) -> StreamEvent {
        StreamEvent {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            event: self.clone(),
        }
    }
}

/// Wrapper for events sent over WebSocket with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    /// Unique event ID
    pub id: String,
    /// Unix timestamp when event was created
    pub timestamp: u64,
    /// The actual event data
    #[serde(flatten)]
    pub event: MCPEvent,
}

/// Event categories for easier filtering
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EventCategory {
    #[serde(rename = "wallet")]
    Wallet,
    #[serde(rename = "mining")]
    Mining,
    #[serde(rename = "node")]
    Node,
    #[serde(rename = "p2pool")]
    P2Pool,
    #[serde(rename = "app")]
    App,
    #[serde(rename = "all")]
    All,
}

impl EventCategory {
    /// Check if this category matches an event
    pub fn matches(&self, event: &MCPEvent) -> bool {
        match self {
            EventCategory::All => true,
            EventCategory::Wallet => event.category() == "wallet",
            EventCategory::Mining => event.category() == "mining",
            EventCategory::Node => event.category() == "node", 
            EventCategory::P2Pool => event.category() == "p2pool",
            EventCategory::App => event.category() == "app",
        }
    }
}

/// Event filtering options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    /// Categories to include
    pub categories: Vec<EventCategory>,
    /// Specific event types to include (if empty, include all in categories)
    pub event_types: Vec<String>,
    /// Minimum severity for error events
    pub min_severity: Option<String>,
}

impl Default for EventFilter {
    fn default() -> Self {
        Self {
            categories: vec![EventCategory::All],
            event_types: vec![],
            min_severity: Some("info".to_string()),
        }
    }
}

impl EventFilter {
    /// Check if this filter should include the given event
    pub fn should_include(&self, event: &MCPEvent) -> bool {
        // Check categories
        if !self.categories.iter().any(|cat| cat.matches(event)) {
            return false;
        }

        // Check specific event types (if specified)
        if !self.event_types.is_empty() {
            if !self.event_types.contains(&event.event_type().to_string()) {
                return false;
            }
        }

        // Check severity for error events
        if let MCPEvent::AppError { severity, .. } = event {
            if let Some(min_sev) = &self.min_severity {
                let severity_level = match severity.as_str() {
                    "error" => 3,
                    "warning" => 2,
                    "info" => 1,
                    _ => 0,
                };
                let min_level = match min_sev.as_str() {
                    "error" => 3,
                    "warning" => 2,
                    "info" => 1,
                    _ => 0,
                };
                if severity_level < min_level {
                    return false;
                }
            }
        }

        true
    }
}
