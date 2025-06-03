// Copyright 2024. The Tari Project

use super::event_types::{EventFilter, MCPEvent};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Client subscription configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSubscription {
    /// Unique client identifier
    pub client_id: String,
    /// Event filter configuration
    pub filter: EventFilter,
    /// When this subscription was created
    pub created_at: u64,
    /// Optional subscription metadata
    pub metadata: Option<SubscriptionMetadata>,
}

/// Additional metadata about the subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionMetadata {
    /// Human-readable client name
    pub client_name: Option<String>,
    /// Client version or type
    pub client_version: Option<String>,
    /// User agent or description
    pub user_agent: Option<String>,
    /// Custom tags for organization
    pub tags: Vec<String>,
}

impl EventSubscription {
    /// Create a new subscription with default filter
    pub fn new(client_id: String) -> Self {
        Self {
            client_id,
            filter: EventFilter::default(),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: None,
        }
    }

    /// Create a subscription with custom filter
    pub fn with_filter(client_id: String, filter: EventFilter) -> Self {
        Self {
            client_id,
            filter,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: None,
        }
    }

    /// Add metadata to the subscription
    pub fn with_metadata(mut self, metadata: SubscriptionMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Check if this subscription is interested in the given event
    pub fn is_interested_in(&self, event: &MCPEvent) -> bool {
        self.filter.should_include(event)
    }

    /// Get a human-readable description of this subscription
    pub fn description(&self) -> String {
        if let Some(meta) = &self.metadata {
            if let Some(name) = &meta.client_name {
                return format!("{} ({})", name, self.client_id);
            }
        }
        self.client_id.clone()
    }

    /// Get subscription age in seconds
    pub fn age_seconds(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(self.created_at)
    }
}

/// WebSocket message types for subscription management
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SubscriptionMessage {
    /// Subscribe to events
    #[serde(rename = "subscribe")]
    Subscribe {
        filter: EventFilter,
        metadata: Option<SubscriptionMetadata>,
    },

    /// Unsubscribe from events
    #[serde(rename = "unsubscribe")]
    Unsubscribe,

    /// Update subscription filter
    #[serde(rename = "update_filter")]
    UpdateFilter { filter: EventFilter },

    /// Get current subscription status
    #[serde(rename = "get_status")]
    GetStatus,

    /// Ping to keep connection alive
    #[serde(rename = "ping")]
    Ping,
}

/// WebSocket response messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SubscriptionResponse {
    /// Subscription successful
    #[serde(rename = "subscribed")]
    Subscribed {
        client_id: String,
        filter: EventFilter,
    },

    /// Unsubscription successful
    #[serde(rename = "unsubscribed")]
    Unsubscribed { client_id: String },

    /// Filter updated
    #[serde(rename = "filter_updated")]
    FilterUpdated { filter: EventFilter },

    /// Current subscription status
    #[serde(rename = "status")]
    Status {
        subscription: Option<EventSubscription>,
        connection_time: u64,
        events_received: u64,
    },

    /// Pong response to ping
    #[serde(rename = "pong")]
    Pong,

    /// Error occurred
    #[serde(rename = "error")]
    Error { message: String, code: Option<u32> },

    /// Event stream message
    #[serde(rename = "event")]
    Event { event: MCPEvent },
}

/// Connection statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStats {
    /// When connection was established
    pub connected_at: u64,
    /// Total events sent to this client
    pub events_sent: u64,
    /// Total messages received from client
    pub messages_received: u64,
    /// Last activity timestamp
    pub last_activity: u64,
    /// Connection status
    pub status: ConnectionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionStatus {
    #[serde(rename = "connected")]
    Connected,
    #[serde(rename = "disconnected")]
    Disconnected,
    #[serde(rename = "error")]
    Error(String),
}

impl ConnectionStats {
    pub fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            connected_at: now,
            events_sent: 0,
            messages_received: 0,
            last_activity: now,
            status: ConnectionStatus::Connected,
        }
    }

    pub fn record_event_sent(&mut self) {
        self.events_sent += 1;
        self.last_activity = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    pub fn record_message_received(&mut self) {
        self.messages_received += 1;
        self.last_activity = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    pub fn connection_duration(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(self.connected_at)
    }
}
