// Copyright 2024. The Tari Project
//
// Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
// following conditions are met:
//
// 1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
// disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
// following disclaimer in the documentation and/or other materials provided with the distribution.
//
// 3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
// products derived from this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
// INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
// SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
// WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
// USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

//! MCP (Model Context Protocol) Server for Tari Universe
//!
//! This module implements an MCP server that enables AI agents to interact with
//! the Tari Universe application, providing controlled access to wallet operations,
//! mining controls, and application state.

pub mod prompts;
pub mod resources;
pub mod security;
pub mod server;
pub mod tools;
pub mod events;

#[cfg(test)]
pub mod tests;

pub use security::MCPConfig;
pub use server::TariMCPServer;

use anyhow::Result;
use std::sync::Arc;

use crate::UniverseAppState;

/// Initialize and start the MCP server
pub async fn start_mcp_server(
    app_state: Arc<UniverseAppState>,
    app_handle: tauri::AppHandle,
) -> Result<()> {
    let config = MCPConfig::load().await?;

    if !config.enabled {
        log::info!("MCP server is disabled in configuration");
        return Ok(());
    }

    let mut server = TariMCPServer::new(app_state, app_handle, config).await?;
    
    // Start WebSocket event streaming
    server.start_websocket_streaming().await?;
    
    // Start traditional stdio MCP server
    server.start().await?;

    log::info!("MCP server started successfully");
    Ok(())
}
