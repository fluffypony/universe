// Copyright 2024. The Tari Project

use super::MCPResource;
use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::UniverseAppState;

/// Wallet balance resource
pub struct WalletBalanceResource;

#[async_trait::async_trait]
impl MCPResource for WalletBalanceResource {
    async fn get_data(&self, app_state: Arc<UniverseAppState>) -> Result<Value> {
        let balance = app_state
            .wallet_state_watch_rx
            .borrow()
            .clone()
            .and_then(|state| state.balance);

        match balance {
            Some(balance) => Ok(json!({
                "available_balance": balance.available_balance.0,
                "timelocked_balance": balance.timelocked_balance.0,
                "pending_incoming_balance": balance.pending_incoming_balance.0,
                "pending_outgoing_balance": balance.pending_outgoing_balance.0,
                "balance_formatted": {
                    "available": format!("{:.6} tXTR", balance.available_balance.0 as f64 / 1_000_000.0),
                    "timelocked": format!("{:.6} tXTR", balance.timelocked_balance.0 as f64 / 1_000_000.0),
                    "pending_incoming": format!("{:.6} tXTR", balance.pending_incoming_balance.0 as f64 / 1_000_000.0),
                    "pending_outgoing": format!("{:.6} tXTR", balance.pending_outgoing_balance.0 as f64 / 1_000_000.0),
                }
            })),
            None => Ok(json!({
                "available_balance": 0,
                "timelocked_balance": 0,
                "pending_incoming_balance": 0,
                "pending_outgoing_balance": 0,
                "error": "Wallet balance not available",
                "balance_formatted": {
                    "available": "0.000000 tXTR",
                    "timelocked": "0.000000 tXTR",
                    "pending_incoming": "0.000000 tXTR",
                    "pending_outgoing": "0.000000 tXTR",
                }
            }))
        }
    }

    fn name(&self) -> &str {
        "wallet_balance"
    }

    fn description(&self) -> &str {
        "Current wallet balance including available, timelocked, and pending amounts"
    }
}

/// Wallet address resource
pub struct WalletAddressResource;

#[async_trait::async_trait]
impl MCPResource for WalletAddressResource {
    async fn get_data(&self, app_state: Arc<UniverseAppState>) -> Result<Value> {
        let tari_address = app_state.tari_address.read().await;
        
        Ok(json!({
            "address_base58": tari_address.to_base58(),
            "address_emoji": tari_address.to_emoji_string(),
            "network": tari_address.network().to_string(),
            "features": format!("{:?}", tari_address.features()),
        }))
    }

    fn name(&self) -> &str {
        "wallet_address"
    }

    fn description(&self) -> &str {
        "Current wallet address in Base58 and emoji formats"
    }
}

/// Transaction history resource
pub struct TransactionHistoryResource;

#[async_trait::async_trait]
impl MCPResource for TransactionHistoryResource {
    async fn get_data(&self, app_state: Arc<UniverseAppState>) -> Result<Value> {
        let transactions = app_state
            .wallet_manager
            .get_transactions_history(false, Some(20))
            .await
            .unwrap_or_default();

        let transactions_json: Vec<Value> = transactions
            .into_iter()
            .map(|tx| json!({
                "tx_id": tx.tx_id,
                "source_address": tx.source_address,
                "dest_address": tx.dest_address,
                "status": format!("{:?}", tx.status),
                "direction": format!("{:?}", tx.direction),
                "amount": tx.amount.0,
                "amount_formatted": format!("{:.6} tXTR", tx.amount.0 as f64 / 1_000_000.0),
                "fee": tx.fee,
                "fee_formatted": format!("{:.6} tXTR", tx.fee as f64 / 1_000_000.0),
                "timestamp": tx.timestamp,
                "payment_id": tx.payment_id,
                "cancelled": tx.is_cancelled,
            }))
            .collect();

        Ok(json!({
            "transactions": transactions_json,
            "count": transactions_json.len(),
        }))
    }

    fn name(&self) -> &str {
        "transaction_history"
    }

    fn description(&self) -> &str {
        "Recent transaction history (last 20 transactions)"
    }
}

/// Coinbase transactions resource (mining rewards)
pub struct CoinbaseTransactionsResource;

#[async_trait::async_trait]
impl MCPResource for CoinbaseTransactionsResource {
    async fn get_data(&self, app_state: Arc<UniverseAppState>) -> Result<Value> {
        let coinbase_transactions = app_state
            .wallet_manager
            .get_coinbase_transactions(false, Some(20))
            .await
            .unwrap_or_default();

        let transactions_json: Vec<Value> = coinbase_transactions
            .into_iter()
            .map(|tx| json!({
                "tx_id": tx.tx_id,
                "source_address": tx.source_address,
                "dest_address": tx.dest_address,
                "status": format!("{:?}", tx.status),
                "amount": tx.amount.0,
                "amount_formatted": format!("{:.6} tXTR", tx.amount.0 as f64 / 1_000_000.0),
                "fee": tx.fee,
                "fee_formatted": format!("{:.6} tXTR", tx.fee as f64 / 1_000_000.0),
                "timestamp": tx.timestamp,
                "payment_id": tx.payment_id,
                "mined_height": tx.mined_in_block_height,
            }))
            .collect();

        Ok(json!({
            "coinbase_transactions": transactions_json,
            "count": transactions_json.len(),
            "total_mined": transactions_json.iter()
                .map(|tx| tx["amount"].as_u64().unwrap_or(0))
                .sum::<u64>(),
        }))
    }

    fn name(&self) -> &str {
        "coinbase_transactions"
    }

    fn description(&self) -> &str {
        "Recent coinbase transactions (mining rewards)"
    }
}
