// Copyright 2024. The Tari Project
//! MCP Server Tests

#[cfg(test)]
mod tests {
    use crate::mcp::security::{MCPConfig, MCPAuditEntry};
    use crate::mcp::tools::*;
    use crate::mcp::resources::*;
    use serde_json::json;

    #[test]
    fn test_mcp_config_validation() {
        let config = MCPConfig {
            enabled: true,
            allow_wallet_send: false,
            allowed_host_addresses: vec!["127.0.0.1".to_string()],
            port: 3030,
            audit_logging: true,
        };

        assert!(config.enabled);
        assert!(!config.allow_wallet_send);
        assert_eq!(config.port, 3030);
        assert_eq!(config.allowed_host_addresses, vec!["127.0.0.1"]);
        assert!(config.audit_logging);
    }

    #[test]
    fn test_mcp_config_default() {
        let config = MCPConfig::default();
        assert!(!config.enabled);
        assert!(!config.allow_wallet_send);
        assert_eq!(config.port, 0); // Random available port
        assert!(config.audit_logging);
        assert!(!config.allowed_host_addresses.is_empty());
    }

    #[test]
    fn test_mcp_audit_entry() {
        let audit = MCPAuditEntry::new("test_operation".to_string());
        assert_eq!(audit.operation, "test_operation");
        assert!(audit.timestamp.timestamp() > 0);
        assert!(!audit.success);
        assert!(audit.error.is_none());
        assert_eq!(audit.details, serde_json::Value::Null);
    }

    #[test]
    fn test_mcp_audit_entry_chaining() {
        let audit = MCPAuditEntry::new("test_operation".to_string())
            .with_success(true)
            .with_details(json!({"key": "value"}))
            .with_error("test error".to_string());

        assert!(!audit.success); // with_error sets success to false
        assert_eq!(audit.details, json!({"key": "value"}));
        assert!(audit.error.is_some());
        assert_eq!(audit.error.unwrap(), "test error");
    }

    #[test]
    fn test_validate_address_tool_schema() {
        let tool = ValidateAddressTool;
        let schema = tool.input_schema();
        
        assert!(schema.is_object());
        let properties = schema.get("properties").unwrap();
        assert!(properties.get("address").is_some());
        assert!(properties.get("sending_method").is_some());
    }

    #[test]
    fn test_send_tari_tool_schema() {
        let tool = SendTariTool;
        let schema = tool.input_schema();
        
        assert!(schema.is_object());
        let properties = schema.get("properties").unwrap();
        assert!(properties.get("amount").is_some());
        assert!(properties.get("destination").is_some());
        assert!(properties.get("payment_id").is_some());
    }

    #[test]
    fn test_mining_tools_schemas() {
        let start_cpu = StartCpuMiningTool;
        let stop_cpu = StopCpuMiningTool;
        let start_gpu = StartGpuMiningTool;
        let stop_gpu = StopGpuMiningTool;

        // Test that all mining tools have valid schemas
        assert!(start_cpu.input_schema().is_object());
        assert!(stop_cpu.input_schema().is_object());
        assert!(start_gpu.input_schema().is_object());
        assert!(stop_gpu.input_schema().is_object());
    }

    #[test]
    fn test_config_tools_schemas() {
        let get_mining_config = GetMiningConfigTool;
        let set_cpu_enabled = SetCpuMiningEnabledTool;
        let set_gpu_enabled = SetGpuMiningEnabledTool;

        assert!(get_mining_config.input_schema().is_object());
        assert!(set_cpu_enabled.input_schema().is_object());
        assert!(set_gpu_enabled.input_schema().is_object());
    }

    #[test]
    fn test_resource_creation() {
        // Test that all resources can be created
        let _wallet_balance = WalletBalanceResource;
        let _mining_status = MiningStatusResource;
        let _node_status = NodeStatusResource;
        let _network_stats = NetworkStatsResource;
        let _p2pool_stats = P2PoolStatsResource;
        let _app_state = AppStateResource;
        let _transaction_history = TransactionHistoryResource;
        
        // If we reach here, all resources can be instantiated
        assert!(true);
    }

    #[test]
    fn test_tool_names_and_descriptions() {
        // Test that all tools have proper names and descriptions
        let tools: Vec<&dyn MCPTool> = vec![
            &ValidateAddressTool,
            &SendTariTool,
            &StartCpuMiningTool,
            &StopCpuMiningTool,
            &StartGpuMiningTool,
            &StopGpuMiningTool,
            &SetMiningModeTool,
            &GetMiningConfigTool,
            &SetCpuMiningEnabledTool,
            &SetGpuMiningEnabledTool,
        ];

        for tool in tools {
            let name = tool.name();
            let description = tool.description();
            
            assert!(!name.is_empty(), "Tool name should not be empty");
            assert!(!description.is_empty(), "Tool description should not be empty");
            assert!(name.len() > 3, "Tool name should be descriptive");
            assert!(description.len() > 10, "Tool description should be descriptive");
        }
    }

    #[test]
    fn test_resource_names_and_descriptions() {
        // Test that all resources have proper names and descriptions
        let resources: Vec<&dyn MCPResource> = vec![
            &WalletBalanceResource,
            &MiningStatusResource,
            &NodeStatusResource,
            &NetworkStatsResource,
            &P2PoolStatsResource,
            &AppStateResource,
            &TransactionHistoryResource,
        ];

        for resource in resources {
            let name = resource.name();
            let description = resource.description();
            
            assert!(!name.is_empty(), "Resource name should not be empty");
            assert!(!description.is_empty(), "Resource description should not be empty");
            assert!(name.len() > 3, "Resource name should be descriptive");
            assert!(description.len() > 10, "Resource description should be descriptive");
        }
    }

    #[test]
    fn test_mcp_config_serialization() {
        let config = MCPConfig {
            enabled: true,
            allow_wallet_send: false,
            allowed_host_addresses: vec!["127.0.0.1".to_string(), "::1".to_string()],
            port: 3030,
            audit_logging: true,
        };

        // Test that config can be serialized and deserialized
        let json_str = serde_json::to_string(&config).expect("Should serialize");
        let deserialized: MCPConfig = serde_json::from_str(&json_str).expect("Should deserialize");
        
        assert_eq!(config.enabled, deserialized.enabled);
        assert_eq!(config.allow_wallet_send, deserialized.allow_wallet_send);
        assert_eq!(config.port, deserialized.port);
        assert_eq!(config.allowed_host_addresses, deserialized.allowed_host_addresses);
        assert_eq!(config.audit_logging, deserialized.audit_logging);
    }

    #[test]
    fn test_tool_input_schema_structure() {
        let tool = SendTariTool;
        let schema = tool.input_schema();
        
        // Verify schema structure
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"].is_object());
        assert!(schema["required"].is_array());
        
        let properties = &schema["properties"];
        assert!(properties["amount"].is_object());
        assert!(properties["destination"].is_object());
        assert!(properties["payment_id"].is_object());
        
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("amount")));
        assert!(required.contains(&json!("destination")));
    }

    #[test]
    fn test_audit_entry_success_chaining() {
        let audit = MCPAuditEntry::new("test_op".to_string())
            .with_success(true)
            .with_details(json!({"test": "data"}));

        assert!(audit.success);
        assert_eq!(audit.details, json!({"test": "data"}));
        assert!(audit.error.is_none());
    }

    #[test]
    fn test_mining_tool_creation() {
        // Test that mining tools can be instantiated
        let _start_cpu = StartCpuMiningTool;
        let _stop_cpu = StopCpuMiningTool; 
        let _start_gpu = StartGpuMiningTool;
        let _stop_gpu = StopGpuMiningTool;
        let _set_mode = SetMiningModeTool;
        
        assert!(true);
    }

    #[test]
    fn test_wallet_tool_creation() {
        // Test that wallet tools can be instantiated
        let _validate = ValidateAddressTool;
        let _send = SendTariTool;
        
        assert!(true);
    }

    #[test]
    fn test_config_tool_creation() {
        // Test that config tools can be instantiated
        let _get_mining = GetMiningConfigTool;
        let _set_cpu = SetCpuMiningEnabledTool;
        let _set_gpu = SetGpuMiningEnabledTool;
        
        assert!(true);
    }
}
