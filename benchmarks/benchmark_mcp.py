import asyncio
import time
import json
from dataclasses import dataclass
from typing import Any, Callable, Dict, List, Optional
from src.mcp_client import MCPClientManager, MCPTool, MCPServerConnection, MCPServerConfig

# Mock classes to avoid dependencies
class MockSession:
    async def call_tool(self, tool_name: str, arguments: Dict[str, Any] = None):
        return "Tool executed"

    async def list_tools(self):
        class ToolResponse:
            tools = []
        return ToolResponse()

class BenchmarkMCP:
    def __init__(self, num_servers=10, tools_per_server=100):
        self.manager = MCPClientManager()
        self.num_servers = num_servers
        self.tools_per_server = tools_per_server

    async def setup(self):
        print(f"Setting up benchmark with {self.num_servers} servers and {self.tools_per_server} tools each...")

        for i in range(self.num_servers):
            config = MCPServerConfig(name=f"server_{i}", command="echo", args=[], env={})
            connection = MCPServerConnection(config=config)
            connection.connected = True
            connection.session = MockSession()

            # Create dummy tools
            for j in range(self.tools_per_server):
                tool = MCPTool(
                    name=f"tool_{j}",
                    description=f"Description for tool {j} on server {i}. " * 10, # Make description somewhat long
                    server_name=f"server_{i}",
                    input_schema={"type": "object", "properties": {"arg": {"type": "string"}}}, # Simple schema
                    original_name=f"tool_{j}"
                )
                connection.tools.append(tool)

                # Simulate _discover_tools caching
                prefixed_name = tool.get_prefixed_name(self.manager.tool_prefix)
                connection.tool_wrappers[prefixed_name] = self.manager._create_tool_wrapper(connection, tool)

            self.manager.servers[f"server_{i}"] = connection

    async def run(self):
        start_time = time.time()
        callables = self.manager.get_all_tools_as_callables()
        end_time = time.time()

        duration = end_time - start_time
        num_tools = len(callables)

        print(f"Benchmark Result: Created {num_tools} tool wrappers in {duration:.4f} seconds")
        print(f"Average time per tool: {duration / num_tools * 1000:.4f} ms")

        # Verify callability (optional, just to check correctness)
        first_tool_name = list(callables.keys())[0]
        result = await callables[first_tool_name](arg="test")
        # print(f"Executed first tool: {result}")

if __name__ == "__main__":
    benchmark = BenchmarkMCP(num_servers=50, tools_per_server=200) # 10000 tools total
    asyncio.run(benchmark.setup())
    asyncio.run(benchmark.run())
