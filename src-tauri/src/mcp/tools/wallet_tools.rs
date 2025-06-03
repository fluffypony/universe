// Copyright 2024. The Tari Project

use super::MCPTool;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::UniverseAppState;
use crate::mcp::security::{MCPConfig, MCPAuditEntry};
use crate::utils::address_utils::verify_send;
use tari_common_types::tari_address::TariAddressFeatures;
use tauri::Manager;

/// Address validation tool
pub struct ValidateAddressTool;

#[async_trait::async_trait]
impl MCPTool for ValidateAddressTool {
    async fn execute(
        &self,
        args: HashMap<String, Value>,
        _app_state: Arc<UniverseAppState>,
        _app_handle: tauri::AppHandle,
        _config: &MCPConfig,
    ) -> Result<Value> {
        let address = args.get("address")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing required parameter: address"))?;

        let sending_method = args.get("sending_method")
            .and_then(|v| v.as_str())
            .map(|s| match s {
                "interactive" => TariAddressFeatures::INTERACTIVE,
                "one_sided" => TariAddressFeatures::ONE_SIDED,
                _ => TariAddressFeatures::ONE_SIDED,
            })
            .unwrap_or(TariAddressFeatures::ONE_SIDED);

        match verify_send(address.to_string(), sending_method) {
            Ok(_) => Ok(json!({
                "valid": true,
                "address": address,
                "sending_method": format!("{:?}", sending_method),
                "message": "Address is valid"
            })),
            Err(e) => Ok(json!({
                "valid": false,
                "address": address,
                "sending_method": format!("{:?}", sending_method),
                "error": e,
                "message": "Address validation failed"
            }))
        }
    }

    fn name(&self) -> &str {
        "validate_address"
    }

    fn description(&self) -> &str {
        "Validate a Tari address for sending transactions"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "address": {
                    "type": "string",
                    "description": "The Tari address to validate"
                },
                "sending_method": {
                    "type": "string",
                    "enum": ["interactive", "one_sided"],
                    "description": "The sending method to validate for",
                    "default": "one_sided"
                }
            },
            "required": ["address"]
        })
    }

    fn should_audit(&self) -> bool {
        false // Address validation is low-risk
    }
}

/// Send Tari transaction tool (requires wallet send permission)
pub struct SendTariTool;

#[async_trait::async_trait]
impl MCPTool for SendTariTool {
    async fn execute(
        &self,
        args: HashMap<String, Value>,
        _app_state: Arc<UniverseAppState>,
        app_handle: tauri::AppHandle,
        _config: &MCPConfig,
    ) -> Result<Value> {
        let audit = MCPAuditEntry::new("send_tari".to_string());

        let amount = args.get("amount")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing required parameter: amount"))?;

        let destination = args.get("destination")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing required parameter: destination"))?;

        let payment_id = args.get("payment_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Basic amount validation
        let amount_f64: f64 = amount.parse().map_err(|e| anyhow!("Invalid amount format: {}", e))?;
        if amount_f64 <= 0.0 {
            let error = "Amount must be greater than 0".to_string();
            audit.with_error(error.clone()).log();
            return Err(anyhow!(error));
        }

        // Validate address
        match verify_send(destination.to_string(), TariAddressFeatures::ONE_SIDED) {
            Ok(_) => {},
            Err(e) => {
                let error = format!("Invalid destination address: {}", e);
                audit.with_error(error.clone()).log();
                return Err(anyhow!(error));
            }
        }

        // Send the transaction using the real Tauri command
        let tx_result: Result<String, anyhow::Error> = {
            // Call the actual send command through AppHandle
            let result = app_handle.state::<crate::UniverseAppState>()
                .spend_wallet_manager
                .write()
                .await
                .send_one_sided_to_stealth_address(
                    amount.to_string(),
                    destination.to_string(), 
                    payment_id.clone(),
                    app_handle.state::<crate::UniverseAppState>()
                )
                .await;
                
            match result {
                Ok(_) => Ok(format!("Successfully sent {} tari to {}", amount, destination)),
                Err(e) => Err(anyhow!("Transaction failed: {}", e))
            }
        };
        
        match tx_result {
            Ok(_) => {
                audit.with_success(true)
                    .with_details(json!({
                        "amount": amount,
                        "destination": destination,
                        "payment_id": payment_id
                    }))
                    .log();
                Ok(json!({
                    "success": true,
                    "message": "Transaction simulation - MCP integration pending",
                    "amount": amount_f64,
                    "destination": destination,
                    "payment_id": payment_id
                }))
            }
            Err(e) => {
                let error_audit = MCPAuditEntry::new("send_tari".to_string());
                error_audit.with_error(e.to_string()).log();
                Err(anyhow!("Failed to send transaction: {}", e))
            }
        }
    }

    fn name(&self) -> &str {
        "send_tari"
    }

    fn description(&self) -> &str {
        "Send a Tari transaction to a destination address"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "amount": {
                    "type": "string",
                    "description": "Amount to send in Tari (e.g., '10.5')"
                },
                "destination": {
                    "type": "string",
                    "description": "Destination Tari address"
                },
                "payment_id": {
                    "type": "string",
                    "description": "Optional payment ID for the transaction"
                }
            },
            "required": ["amount", "destination"]
        })
    }

    fn requires_wallet_send_permission(&self) -> bool {
        true
    }
}
