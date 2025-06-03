#!/usr/bin/env python3
"""
Example MCP client for Tari Universe
Demonstrates how an AI agent would interact with the MCP server
"""

import json
import subprocess
import sys
from typing import Dict, Any, Optional

class TariMCPClient:
    """Simple MCP client for demonstrating Tari Universe integration"""
    
    def __init__(self, tari_universe_path: str):
        self.process = None
        self.tari_universe_path = tari_universe_path
        self.request_id = 0
    
    def start(self):
        """Start the Tari Universe MCP server"""
        # Note: This assumes Tari Universe has been built with --features mcp-server
        # and MCP is enabled in configuration
        self.process = subprocess.Popen(
            [self.tari_universe_path, "--mcp"],  # Hypothetical MCP flag
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )
        
        # Initialize the connection
        self._send_request("initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "roots": {"listChanged": False},
                "sampling": {}
            },
            "clientInfo": {
                "name": "tari-mcp-example",
                "version": "1.0.0"
            }
        })
    
    def _send_request(self, method: str, params: Dict[str, Any], id: Optional[int] = None) -> Dict[str, Any]:
        """Send a JSON-RPC request to the MCP server"""
        if id is None:
            self.request_id += 1
            id = self.request_id
        
        request = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": id
        }
        
        request_str = json.dumps(request) + "\n"
        self.process.stdin.write(request_str)
        self.process.stdin.flush()
        
        # Read response
        response_str = self.process.stdout.readline()
        return json.loads(response_str)
    
    def list_resources(self) -> Dict[str, Any]:
        """List all available resources"""
        return self._send_request("resources/list", {})
    
    def read_resource(self, uri: str) -> Dict[str, Any]:
        """Read a specific resource"""
        return self._send_request("resources/read", {"uri": uri})
    
    def list_tools(self) -> Dict[str, Any]:
        """List all available tools"""
        return self._send_request("tools/list", {})
    
    def call_tool(self, name: str, arguments: Dict[str, Any]) -> Dict[str, Any]:
        """Call a specific tool"""
        return self._send_request("tools/call", {
            "name": name,
            "arguments": arguments
        })
    
    def get_wallet_balance(self) -> Dict[str, Any]:
        """Get current wallet balance"""
        response = self.read_resource("tari://wallet_balance")
        if "result" in response:
            content = response["result"]["contents"][0]["text"]
            return json.loads(content)
        return {}
    
    def get_mining_status(self) -> Dict[str, Any]:
        """Get current mining status"""
        response = self.read_resource("tari://mining_status")
        if "result" in response:
            content = response["result"]["contents"][0]["text"]
            return json.loads(content)
        return {}
    
    def start_mining(self, cpu: bool = True, gpu: bool = False) -> bool:
        """Start mining operations"""
        success = True
        
        if cpu:
            response = self.call_tool("start_cpu_mining", {})
            if "error" in response:
                print(f"Failed to start CPU mining: {response['error']['message']}")
                success = False
        
        if gpu:
            response = self.call_tool("start_gpu_mining", {})
            if "error" in response:
                print(f"Failed to start GPU mining: {response['error']['message']}")
                success = False
        
        return success
    
    def stop_mining(self) -> bool:
        """Stop all mining operations"""
        cpu_response = self.call_tool("stop_cpu_mining", {})
        gpu_response = self.call_tool("stop_gpu_mining", {})
        
        return "error" not in cpu_response and "error" not in gpu_response
    
    def optimize_mining(self) -> str:
        """AI-driven mining optimization"""
        # Get current status
        balance = self.get_wallet_balance()
        mining_status = self.get_mining_status()
        
        # Simple optimization logic
        available_balance = balance.get("available_balance", 0)
        is_mining = (mining_status.get("cpu_mining", {}).get("is_mining", False) or 
                    mining_status.get("gpu_mining", {}).get("is_mining", False))
        
        recommendations = []
        
        if available_balance < 1000000:  # Less than 1 tXTR
            if not is_mining:
                recommendations.append("💡 Start mining to earn more Tari")
                self.start_mining(cpu=True)
                recommendations.append("✅ Started CPU mining")
            else:
                recommendations.append("✅ Mining is active - earning Tari")
        else:
            recommendations.append("💰 Good balance - mining is optional")
        
        # Check mining efficiency
        cpu_hash_rate = mining_status.get("cpu_mining", {}).get("hash_rate", 0)
        gpu_hash_rate = mining_status.get("gpu_mining", {}).get("hash_rate", 0)
        
        if cpu_hash_rate > 0:
            recommendations.append(f"🔥 CPU mining at {cpu_hash_rate:.2f} H/s")
        if gpu_hash_rate > 0:
            recommendations.append(f"⚡ GPU mining at {gpu_hash_rate:.2f} H/s")
        
        return "\n".join(recommendations)
    
    def close(self):
        """Close the MCP connection"""
        if self.process:
            self.process.terminate()
            self.process.wait()

def main():
    """Example usage of the Tari MCP client"""
    if len(sys.argv) < 2:
        print("Usage: python mcp_client_example.py <path_to_tari_universe>")
        sys.exit(1)
    
    tari_path = sys.argv[1]
    client = TariMCPClient(tari_path)
    
    try:
        print("🚀 Starting Tari Universe MCP client...")
        client.start()
        
        print("\n📋 Available Resources:")
        resources = client.list_resources()
        if "result" in resources:
            for resource in resources["result"]["resources"]:
                print(f"  - {resource['name']}: {resource['description']}")
        
        print("\n🛠️ Available Tools:")
        tools = client.list_tools()
        if "result" in tools:
            for tool in tools["result"]["tools"]:
                print(f"  - {tool['name']}: {tool['description']}")
        
        print("\n💰 Current Wallet Balance:")
        balance = client.get_wallet_balance()
        if balance:
            available = balance.get("balance_formatted", {}).get("available", "0 tXTR")
            print(f"  Available: {available}")
        
        print("\n⛏️ Mining Status:")
        mining = client.get_mining_status()
        if mining:
            cpu_mining = mining.get("cpu_mining", {}).get("is_mining", False)
            gpu_mining = mining.get("gpu_mining", {}).get("is_mining", False)
            print(f"  CPU Mining: {'✅ Active' if cpu_mining else '❌ Inactive'}")
            print(f"  GPU Mining: {'✅ Active' if gpu_mining else '❌ Inactive'}")
        
        print("\n🤖 AI Mining Optimization:")
        optimization = client.optimize_mining()
        print(optimization)
        
    except Exception as e:
        print(f"❌ Error: {e}")
    finally:
        client.close()
        print("\n👋 MCP client closed")

if __name__ == "__main__":
    main()
