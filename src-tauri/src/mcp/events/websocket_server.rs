// Copyright 2024. The Tari Project

use super::{
    event_types::MCPEvent,
    subscription::{
        ConnectionStats, EventSubscription, SubscriptionMessage,
        SubscriptionResponse,
    },
    MCPEventManager,
};
use crate::mcp::security::MCPConfig;
use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{broadcast, RwLock},
};
use tokio_tungstenite::{
    accept_async, tungstenite::protocol::Message, WebSocketStream,
};
use uuid::Uuid;

const LOG_TARGET: &str = "tari::universe::mcp::websocket_server";

/// WebSocket server for MCP event streaming
pub struct MCPWebSocketServer {
    event_manager: Arc<MCPEventManager>,
    config: MCPConfig,
    client_connections: Arc<RwLock<HashMap<String, ClientConnection>>>,
    shutdown_tx: Option<broadcast::Sender<()>>,
}

/// Individual client connection handler
struct ClientConnection {
    client_id: String,
    socket: WebSocketStream<TcpStream>,
    stats: ConnectionStats,
    subscription: Option<EventSubscription>,
    event_rx: Option<broadcast::Receiver<MCPEvent>>,
}

impl MCPWebSocketServer {
    pub fn new(event_manager: Arc<MCPEventManager>, config: MCPConfig) -> Self {
        Self {
            event_manager,
            config,
            client_connections: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx: None,
        }
    }

    /// Start the WebSocket server
    pub async fn start(&mut self) -> Result<()> {
        let addr = format!("127.0.0.1:{}", self.config.port + 1); // WebSocket on port + 1
        let listener = TcpListener::bind(&addr).await?;
        
        log::info!(target: LOG_TARGET, "MCP WebSocket server starting on {}", addr);

        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        let event_manager = self.event_manager.clone();
        let client_connections = self.client_connections.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut shutdown_rx = shutdown_rx;
            
            loop {
                tokio::select! {
                    // Accept new connections
                    result = listener.accept() => {
                        match result {
                            Ok((stream, addr)) => {
                                if let Err(e) = Self::handle_new_connection(
                                    stream,
                                    addr,
                                    event_manager.clone(),
                                    client_connections.clone(),
                                    config.clone(),
                                ).await {
                                    log::error!(target: LOG_TARGET, "Failed to handle connection: {}", e);
                                }
                            }
                            Err(e) => {
                                log::error!(target: LOG_TARGET, "Failed to accept connection: {}", e);
                            }
                        }
                    }
                    
                    // Shutdown signal
                    _ = shutdown_rx.recv() => {
                        log::info!(target: LOG_TARGET, "WebSocket server shutting down");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the WebSocket server
    pub async fn stop(&self) -> Result<()> {
        if let Some(shutdown_tx) = &self.shutdown_tx {
            let _ = shutdown_tx.send(());
        }
        
        // Close all client connections
        let mut connections = self.client_connections.write().await;
        for (client_id, mut connection) in connections.drain() {
            log::info!(target: LOG_TARGET, "Closing connection for client: {}", client_id);
            let _ = connection.socket.close(None).await;
        }

        Ok(())
    }

    /// Handle a new WebSocket connection
    async fn handle_new_connection(
        stream: TcpStream,
        addr: SocketAddr,
        event_manager: Arc<MCPEventManager>,
        client_connections: Arc<RwLock<HashMap<String, ClientConnection>>>,
        config: MCPConfig,
    ) -> Result<()> {
        // Check if this host is allowed
        let host_allowed = config.allowed_host_addresses.iter().any(|allowed| {
            if let Ok(allowed_addr) = allowed.parse::<std::net::IpAddr>() {
                addr.ip() == allowed_addr
            } else {
                false
            }
        });

        if !host_allowed {
            log::warn!(target: LOG_TARGET, "Rejected connection from unauthorized host: {}", addr);
            return Err(anyhow!("Host not allowed"));
        }

        log::info!(target: LOG_TARGET, "New WebSocket connection from: {}", addr);

        // Perform WebSocket handshake
        let ws_stream = accept_async(stream).await?;
        let client_id = Uuid::new_v4().to_string();

        let connection = ClientConnection {
            client_id: client_id.clone(),
            socket: ws_stream,
            stats: ConnectionStats::new(),
            subscription: None,
            event_rx: None,
        };

        // Store the connection
        {
            let mut connections = client_connections.write().await;
            connections.insert(client_id.clone(), connection);
        }

        // Handle this client's messages
        Self::handle_client_messages(client_id, event_manager, client_connections).await;

        Ok(())
    }

    /// Handle messages from a specific client
    async fn handle_client_messages(
        client_id: String,
        event_manager: Arc<MCPEventManager>,
        client_connections: Arc<RwLock<HashMap<String, ClientConnection>>>,
    ) {
        log::info!(target: LOG_TARGET, "Handling messages for client: {}", client_id);

        loop {
            let message = {
                let mut connections = client_connections.write().await;
                if let Some(connection) = connections.get_mut(&client_id) {
                    match connection.socket.next().await {
                        Some(Ok(msg)) => {
                            connection.stats.record_message_received();
                            Some(msg)
                        }
                        Some(Err(e)) => {
                            log::error!(target: LOG_TARGET, "WebSocket error for client {}: {}", client_id, e);
                            break;
                        }
                        None => {
                            log::info!(target: LOG_TARGET, "Client {} disconnected", client_id);
                            break;
                        }
                    }
                } else {
                    break;
                }
            };

            if let Some(msg) = message {
                if let Err(e) = Self::process_client_message(
                    &client_id,
                    msg,
                    &event_manager,
                    &client_connections,
                ).await {
                    log::error!(target: LOG_TARGET, "Error processing message from {}: {}", client_id, e);
                }
            }
        }

        // Clean up the connection
        event_manager.unsubscribe(&client_id).await;
        let mut connections = client_connections.write().await;
        connections.remove(&client_id);
        log::info!(target: LOG_TARGET, "Cleaned up connection for client: {}", client_id);
    }

    /// Process a message from a client
    async fn process_client_message(
        client_id: &str,
        message: Message,
        event_manager: &Arc<MCPEventManager>,
        client_connections: &Arc<RwLock<HashMap<String, ClientConnection>>>,
    ) -> Result<()> {
        match message {
            Message::Text(text) => {
                let sub_msg: SubscriptionMessage = serde_json::from_str(&text)?;
                Self::handle_subscription_message(client_id, sub_msg, event_manager, client_connections).await?;
            }
            Message::Close(_) => {
                log::info!(target: LOG_TARGET, "Client {} sent close message", client_id);
                return Err(anyhow!("Client closed connection"));
            }
            Message::Ping(data) => {
                // Respond with pong
                let mut connections = client_connections.write().await;
                if let Some(connection) = connections.get_mut(client_id) {
                    let _ = connection.socket.send(Message::Pong(data)).await;
                }
            }
            _ => {
                // Ignore other message types
            }
        }
        Ok(())
    }

    /// Handle subscription management messages
    async fn handle_subscription_message(
        client_id: &str,
        message: SubscriptionMessage,
        event_manager: &Arc<MCPEventManager>,
        client_connections: &Arc<RwLock<HashMap<String, ClientConnection>>>,
    ) -> Result<()> {
        let response = match message {
            SubscriptionMessage::Subscribe { filter, metadata } => {
                let subscription = EventSubscription::with_filter(client_id.to_string(), filter.clone());
                let subscription = if let Some(meta) = metadata {
                    subscription.with_metadata(meta)
                } else {
                    subscription
                };

                // Subscribe to events
                let event_rx = event_manager.subscribe(client_id.to_string(), subscription.clone()).await;

                // Update connection with subscription and event receiver
                {
                    let mut connections = client_connections.write().await;
                    if let Some(connection) = connections.get_mut(client_id) {
                        connection.subscription = Some(subscription.clone());
                        connection.event_rx = Some(event_rx);
                    }
                }

                // Start event forwarding task for this client
                Self::start_event_forwarding(client_id.to_string(), client_connections.clone());

                SubscriptionResponse::Subscribed {
                    client_id: client_id.to_string(),
                    filter,
                }
            }
            SubscriptionMessage::Unsubscribe => {
                event_manager.unsubscribe(client_id).await;
                
                // Update connection
                {
                    let mut connections = client_connections.write().await;
                    if let Some(connection) = connections.get_mut(client_id) {
                        connection.subscription = None;
                        connection.event_rx = None;
                    }
                }

                SubscriptionResponse::Unsubscribed {
                    client_id: client_id.to_string(),
                }
            }
            SubscriptionMessage::UpdateFilter { filter } => {
                // Update the subscription filter
                {
                    let mut connections = client_connections.write().await;
                    if let Some(connection) = connections.get_mut(client_id) {
                        if let Some(subscription) = &mut connection.subscription {
                            subscription.filter = filter.clone();
                        }
                    }
                }

                SubscriptionResponse::FilterUpdated { filter }
            }
            SubscriptionMessage::GetStatus => {
                let subscription = {
                    let connections = client_connections.read().await;
                    connections.get(client_id).and_then(|c| c.subscription.clone())
                };

                SubscriptionResponse::Status {
                    subscription,
                    connection_time: 0, // TODO: Calculate actual connection time
                    events_received: 0, // TODO: Track events received
                }
            }
            SubscriptionMessage::Ping => SubscriptionResponse::Pong,
        };

        // Send response to client
        Self::send_response_to_client(client_id, response, client_connections).await
    }

    /// Send a response to a specific client
    async fn send_response_to_client(
        client_id: &str,
        response: SubscriptionResponse,
        client_connections: &Arc<RwLock<HashMap<String, ClientConnection>>>,
    ) -> Result<()> {
        let response_text = serde_json::to_string(&response)?;
        
        let mut connections = client_connections.write().await;
        if let Some(connection) = connections.get_mut(client_id) {
            connection.socket.send(Message::Text(response_text.into())).await?;
        }

        Ok(())
    }

    /// Start forwarding events to a client
    fn start_event_forwarding(
        client_id: String,
        client_connections: Arc<RwLock<HashMap<String, ClientConnection>>>,
    ) {
        tokio::spawn(async move {
            log::info!(target: LOG_TARGET, "Starting event forwarding for client: {}", client_id);

            loop {
                let event = {
                    let mut connections = client_connections.write().await;
                    if let Some(connection) = connections.get_mut(&client_id) {
                        if let Some(event_rx) = &mut connection.event_rx {
                            match event_rx.recv().await {
                                Ok(event) => {
                                    // Check if subscription is interested in this event
                                    if let Some(subscription) = &connection.subscription {
                                        if subscription.is_interested_in(&event) {
                                            Some(event)
                                        } else {
                                            continue;
                                        }
                                    } else {
                                        continue;
                                    }
                                }
                                Err(broadcast::error::RecvError::Lagged(_)) => {
                                    log::warn!(target: LOG_TARGET, "Client {} lagged behind in events", client_id);
                                    continue;
                                }
                                Err(broadcast::error::RecvError::Closed) => {
                                    log::info!(target: LOG_TARGET, "Event channel closed for client: {}", client_id);
                                    break;
                                }
                            }
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                };

                if let Some(event) = event {
                    // Send event to client
                    // Create timestamped event wrapper for future use
                    let _stream_event = event.to_stream_event();
                    let response = SubscriptionResponse::Event { event };
                    
                    if let Ok(response_text) = serde_json::to_string(&response) {
                        let mut connections = client_connections.write().await;
                        if let Some(connection) = connections.get_mut(&client_id) {
                            if let Err(e) = connection.socket.send(Message::Text(response_text.into())).await {
                                log::error!(target: LOG_TARGET, "Failed to send event to client {}: {}", client_id, e);
                                break;
                            }
                            connection.stats.record_event_sent();
                        } else {
                            break;
                        }
                    }
                }
            }

            log::info!(target: LOG_TARGET, "Event forwarding stopped for client: {}", client_id);
        });
    }

    /// Get statistics for all connected clients
    pub async fn get_client_stats(&self) -> HashMap<String, ConnectionStats> {
        let connections = self.client_connections.read().await;
        connections
            .iter()
            .map(|(id, conn)| (id.clone(), conn.stats.clone()))
            .collect()
    }

    /// Get the number of connected clients
    pub async fn client_count(&self) -> usize {
        let connections = self.client_connections.read().await;
        connections.len()
    }
}
