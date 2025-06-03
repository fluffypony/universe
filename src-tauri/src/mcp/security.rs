// Copyright 2024. The Tari Project

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::str::FromStr;

use crate::configs::config_core::ConfigCore;
use crate::configs::trait_config::ConfigImpl;

const LOG_TARGET: &str = "tari::universe::mcp::security";

/// MCP Server configuration and security settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPConfig {
    /// Whether the MCP server is enabled
    pub enabled: bool,
    /// Allow wallet send operations (high security)
    pub allow_wallet_send: bool,
    /// Allowed host addresses that can connect
    pub allowed_host_addresses: Vec<String>,
    /// Port for the MCP server (0 = random available port)
    pub port: u16,
    /// Enable audit logging for all MCP operations
    pub audit_logging: bool,
}

impl Default for MCPConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            allow_wallet_send: false,
            allowed_host_addresses: vec![
                "127.0.0.1".to_string(),
                "::1".to_string(),
            ],
            port: 0, // Random available port
            audit_logging: true,
        }
    }
}

impl MCPConfig {
    /// Load MCP configuration from the core config
    pub async fn load() -> Result<Self> {
        #[cfg(feature = "mcp-server")]
        {
            let core_config = ConfigCore::content().await;
            Ok(Self {
                enabled: *core_config.mcp_enabled(),
                allow_wallet_send: *core_config.mcp_allow_wallet_send(),
                allowed_host_addresses: core_config.mcp_allowed_host_addresses().clone(),
                port: *core_config.mcp_port(),
                audit_logging: *core_config.mcp_audit_logging(),
            })
        }
        #[cfg(not(feature = "mcp-server"))]
        {
            Ok(Self::default())
        }
    }

    /// Check if the given host address is allowed to connect
    pub fn is_host_allowed(&self, host: &str) -> bool {
        // Parse the host to handle both IP addresses and hostnames
        if let Ok(ip) = IpAddr::from_str(host) {
            // Check if it's a loopback address
            if ip.is_loopback() {
                return true;
            }
            
            // Check against allowed list
            self.allowed_host_addresses.iter().any(|allowed| {
                if let Ok(allowed_ip) = IpAddr::from_str(allowed) {
                    ip == allowed_ip
                } else {
                    host == allowed
                }
            })
        } else {
            // Handle hostname comparison
            self.allowed_host_addresses.iter().any(|allowed| host == allowed)
        }
    }

    /// Check if wallet send operations are permitted
    pub fn can_send_wallet_transactions(&self) -> bool {
        self.allow_wallet_send
    }

    /// Validate security requirements for the current configuration
    pub fn validate(&self) -> Result<()> {
        // Ensure we're not binding to all interfaces unless explicitly configured
        if self.allowed_host_addresses.contains(&"0.0.0.0".to_string()) {
            log::warn!(target: LOG_TARGET, "MCP server configured to allow connections from any host - this may be insecure");
        }

        // Warn if wallet send is enabled
        if self.allow_wallet_send {
            log::warn!(target: LOG_TARGET, "MCP server configured to allow wallet send operations - ensure this is intended");
        }

        Ok(())
    }
}

/// Security audit log entry for MCP operations
#[derive(Debug, Serialize)]
pub struct MCPAuditEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub operation: String,
    pub client_id: Option<String>,
    pub success: bool,
    pub error: Option<String>,
    pub details: serde_json::Value,
}

impl MCPAuditEntry {
    pub fn new(operation: String) -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            operation,
            client_id: None,
            success: false,
            error: None,
            details: serde_json::Value::Null,
        }
    }

    pub fn with_client_id(mut self, client_id: String) -> Self {
        self.client_id = Some(client_id);
        self
    }

    pub fn with_success(mut self, success: bool) -> Self {
        self.success = success;
        self
    }

    pub fn with_error(mut self, error: String) -> Self {
        self.error = Some(error);
        self.success = false;
        self
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = details;
        self
    }

    /// Log this audit entry
    pub fn log(&self) {
        if self.success {
            log::info!(target: LOG_TARGET, "MCP Audit: {}", serde_json::to_string(self).unwrap_or_default());
        } else {
            log::warn!(target: LOG_TARGET, "MCP Audit: {}", serde_json::to_string(self).unwrap_or_default());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_allowed() {
        let config = MCPConfig::default();
        
        // Should allow localhost addresses
        assert!(config.is_host_allowed("127.0.0.1"));
        assert!(config.is_host_allowed("::1"));
        
        // Should not allow external addresses
        assert!(!config.is_host_allowed("192.168.1.1"));
        assert!(!config.is_host_allowed("0.0.0.0"));
        assert!(!config.is_host_allowed("example.com"));
    }

    #[test]
    fn test_default_config_security() {
        let config = MCPConfig::default();
        
        // Default should be secure
        assert!(!config.enabled);
        assert!(!config.allow_wallet_send);
        assert!(config.audit_logging);
        assert_eq!(config.port, 0);
        assert_eq!(config.allowed_host_addresses.len(), 2);
    }
}
