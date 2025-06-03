# Tari Universe MCP Server Implementation

This document describes the Model Context Protocol (MCP) server implementation for Tari Universe, enabling AI agents to interact with the cryptocurrency mining and wallet application.

## Overview

The MCP server provides a standardized interface for AI agents to:
- Monitor wallet balances and transaction history
- Control mining operations (CPU and GPU)
- Access application state and configuration
- Perform secure wallet operations (with proper permissions)

## Architecture

```
AI Agent → stdio → MCP Server → UniverseAppState → Mining/Wallet Managers
```

### Core Components

- **MCP Server** (`src-tauri/src/mcp/server.rs`): JSON-RPC 2.0 server implementation
- **Resources** (`src-tauri/src/mcp/resources/`): Read-only data endpoints
- **Tools** (`src-tauri/src/mcp/tools/`): Action endpoints for controlling the application
- **Security** (`src-tauri/src/mcp/security.rs`): Permission management and audit logging
- **Prompts** (`src-tauri/src/mcp/prompts/`): Template prompts for AI agents

## Features

### Current Implementation Status

✅ **Completed:**
- Core MCP server infrastructure
- Read-only resources (wallet, mining, state)
- Basic mining control tools
- Security framework with audit logging
- Configuration management tools

⚠️ **Partially Implemented:**
- Mining control tools (status checking only)
- Wallet transaction tools (validation works, sending needs AppHandle)

🔄 **Needs Implementation:**
- Full integration with Tauri AppHandle
- Settings UI for MCP configuration
- Advanced mining optimization tools
- Real-time event streaming

## Resources (Read-Only Data)

### Wallet Resources

| Resource | URI | Description |
|----------|-----|-------------|
| `wallet_balance` | `tari://wallet_balance` | Current wallet balance (available, timelocked, pending) |
| `wallet_address` | `tari://wallet_address` | Wallet address in Base58 and emoji formats |
| `transaction_history` | `tari://transaction_history` | Recent transaction history (last 20) |
| `coinbase_transactions` | `tari://coinbase_transactions` | Mining rewards history |

### Mining Resources

| Resource | URI | Description |
|----------|-----|-------------|
| `mining_status` | `tari://mining_status` | Current CPU and GPU mining status |
| `mining_config` | `tari://mining_config` | Mining configuration settings |
| `hardware_info` | `tari://hardware_info` | Available CPU threads and GPU devices |
| `p2pool_stats` | `tari://p2pool_stats` | P2Pool mining statistics |

### State Resources

| Resource | URI | Description |
|----------|-----|-------------|
| `app_state` | `tari://app_state` | Application configuration and status |
| `node_status` | `tari://node_status` | Base node connectivity and sync status |
| `network_stats` | `tari://network_stats` | Network configuration and peer info |
| `external_dependencies` | `tari://external_dependencies` | Required dependency status |

## Tools (Actions)

### Mining Tools

| Tool | Description | Parameters |
|------|-------------|------------|
| `start_cpu_mining` | Start CPU mining operations | None |
| `stop_cpu_mining` | Stop CPU mining operations | None |
| `start_gpu_mining` | Start GPU mining operations | None |
| `stop_gpu_mining` | Stop GPU mining operations | None |
| `set_mining_mode` | Set mining mode | `mode`: "Eco", "Ludicrous", "Custom"<br>`custom_cpu_usage`: number (optional) |

### Configuration Tools

| Tool | Description | Parameters |
|------|-------------|------------|
| `get_mining_config` | Get current mining settings | None |
| `set_cpu_mining_enabled` | Enable/disable CPU mining | `enabled`: boolean |
| `set_gpu_mining_enabled` | Enable/disable GPU mining | `enabled`: boolean |
| `get_app_settings` | Get application configuration | None |

### Wallet Tools

| Tool | Description | Parameters | Permissions |
|------|-------------|------------|-------------|
| `validate_address` | Validate Tari address | `address`: string<br>`sending_method`: "interactive" or "one_sided" | None |
| `send_tari` | Send Tari transaction | `amount`: string<br>`destination`: string<br>`payment_id`: string (optional) | `allow_wallet_send` |

## Security Model

### Permission System

- **Default State**: MCP server disabled, all sensitive operations blocked
- **Localhost Only**: Server only binds to 127.0.0.1 and ::1
- **Granular Permissions**: Settings control for wallet send operations
- **Audit Logging**: All operations logged with timestamps and details

### Configuration

```rust
pub struct MCPConfig {
    pub enabled: bool,                        // Default: false
    pub allow_wallet_send: bool,             // Default: false
    pub allowed_host_addresses: Vec<String>, // Default: ["127.0.0.1", "::1"]
    pub port: u16,                          // Default: 0 (random)
    pub audit_logging: bool,                // Default: true
}
```

### Audit Logging

All MCP operations are logged with:
- Timestamp
- Operation name
- Success/failure status
- Client identification (when available)
- Operation details
- Error messages (if applicable)

## Usage Examples

### Starting the MCP Server

```bash
# Build with MCP support
cargo build --features mcp-server

# Server automatically starts if enabled in configuration
```

### AI Agent Integration

```javascript
// Example Claude/GPT interaction
"Please check my Tari wallet balance and start mining if profitable"

// AI agent would:
// 1. Read wallet_balance resource
// 2. Read mining_status resource  
// 3. Analyze profitability
// 4. Call start_cpu_mining tool if beneficial
```

### Resource Access

```json
// Request wallet balance
{
  "jsonrpc": "2.0",
  "method": "resources/read",
  "params": {
    "uri": "tari://wallet_balance"
  },
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "contents": [{
      "uri": "tari://wallet_balance",
      "mimeType": "application/json",
      "text": "{\"available_balance\":1000000,\"timelocked_balance\":0,...}"
    }]
  }
}
```

### Tool Execution

```json
// Start CPU mining
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "start_cpu_mining",
    "arguments": {}
  },
  "id": 2
}
```

## Development Status

### Compilation Requirements

The MCP server is feature-gated and requires explicit enabling:

```bash
# Enable MCP server during compilation
cargo build --features mcp-server

# Without MCP feature (default)
cargo build
```

### Current Limitations

1. **AppHandle Integration**: Some tools need Tauri AppHandle for full functionality
2. **Configuration UI**: No settings interface for enabling/configuring MCP
3. **Real-time Events**: No event streaming for live updates
4. **Testing**: Limited test coverage for MCP functionality

### Future Enhancements

1. **Settings Integration**
   - Add MCP configuration to application settings UI
   - User-friendly permission management
   - Visual audit log viewer

2. **Enhanced Functionality**
   - Real-time event streaming for live mining status
   - Advanced mining optimization recommendations
   - Wallet backup and recovery tools
   - Node management operations

3. **Security Improvements**
   - Client authentication mechanisms
   - Rate limiting for tools
   - Enhanced permission granularity
   - Integration with system keychains

4. **Performance Optimizations**
   - Resource caching strategies
   - Async operation queuing
   - Connection pooling for multiple agents

## API Documentation

For complete API documentation including JSON schemas for all resources and tools, see the implementation files:

- **Resources**: `src-tauri/src/mcp/resources/`
- **Tools**: `src-tauri/src/mcp/tools/`
- **Server**: `src-tauri/src/mcp/server.rs`

## Contributing

To contribute to the MCP implementation:

1. Enable the MCP feature: `cargo build --features mcp-server`
2. Run tests: `cargo test --features mcp-server`
3. Check compilation: `cargo check --features mcp-server`
4. Follow existing patterns for new resources/tools
5. Update documentation for any API changes

## References

- [Model Context Protocol Specification](https://spec.modelcontextprotocol.io/)
- [Tari Protocol Documentation](https://docs.tari.com/)
- [Tauri Framework](https://tauri.app/)
