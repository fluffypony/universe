// Copyright 2024. The Tari Project

//! MCP Event Streaming System
//!
//! This module provides real-time event streaming for AI agents connected to the MCP server.
//! Instead of polling resources, agents can subscribe to live event streams for instant updates.

pub mod event_types;
pub mod subscription;
pub mod websocket_server;
pub mod event_bridge;

pub use event_types::*;
pub use subscription::*;
pub use websocket_server::*;
pub use event_bridge::*;

use anyhow::Result;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Event manager that coordinates between Tari's internal events and MCP clients
pub struct MCPEventManager {
    /// Broadcast channel for sending events to all subscribers
    event_sender: broadcast::Sender<MCPEvent>,
    /// Track active subscriptions by client ID
    subscriptions: Arc<tokio::sync::RwLock<HashMap<String, EventSubscription>>>,
}

impl MCPEventManager {
    pub fn new() -> Self {
        let (event_sender, _) = broadcast::channel(1000); // Buffer up to 1000 events
        
        Self {
            event_sender,
            subscriptions: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Subscribe a client to specific event types
    pub async fn subscribe(&self, client_id: String, subscription: EventSubscription) -> broadcast::Receiver<MCPEvent> {
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.insert(client_id, subscription);
        self.event_sender.subscribe()
    }

    /// Unsubscribe a client
    pub async fn unsubscribe(&self, client_id: &str) {
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.remove(client_id);
    }

    /// Emit an event to all subscribed clients
    pub async fn emit_event(&self, event: MCPEvent) -> Result<()> {
        // Check if any clients are subscribed to this event type
        let subscriptions = self.subscriptions.read().await;
        let has_subscribers = subscriptions.values().any(|sub| sub.is_interested_in(&event));
        
        if has_subscribers {
            if let Err(_) = self.event_sender.send(event) {
                // All receivers have been dropped, which is fine
                log::debug!("No active event receivers");
            }
        }
        
        Ok(())
    }

    /// Get the number of active subscriptions
    pub async fn subscriber_count(&self) -> usize {
        let subscriptions = self.subscriptions.read().await;
        subscriptions.len()
    }

    /// Get subscription details for a specific client
    pub async fn get_subscription(&self, client_id: &str) -> Option<EventSubscription> {
        let subscriptions = self.subscriptions.read().await;
        subscriptions.get(client_id).cloned()
    }

    /// List all active client IDs
    pub async fn list_clients(&self) -> Vec<String> {
        let subscriptions = self.subscriptions.read().await;
        subscriptions.keys().cloned().collect()
    }
}
