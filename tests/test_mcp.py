import asyncio
import unittest
import os
import sys
from typing import Optional, Dict, Any

from src.mcp_client import MCPClient, Transport, JSONRPCRequest, JSONRPCResponse, MCPError
from src.tools.mcp_tools import registry, initialize_registry
from src.tools.execution_tool import execute_ark
from src.tools.demo_tool import echo_message

class MockTransport(Transport):
    def __init__(self):
        self.connected = False
        self.sent_requests = []
        self.response_queue = asyncio.Queue()

    async def connect(self):
        self.connected = True

    async def send(self, request: JSONRPCRequest):
        self.sent_requests.append(request)
        # Auto-respond to initialize
        if request.method == "initialize":
            await self.response_queue.put(JSONRPCResponse(
                id=request.id,
                result={"capabilities": {}}
            ))
        elif request.method == "tools/list":
            await self.response_queue.put(JSONRPCResponse(
                id=request.id,
                result={"tools": [{"name": "test_tool", "description": "test"}]}
            ))

    async def receive(self) -> Optional[JSONRPCResponse]:
        return await self.response_queue.get()

    async def close(self):
        self.connected = False

    def is_connected(self) -> bool:
        return self.connected

class TestMCPClient(unittest.IsolatedAsyncioTestCase):
    async def test_client_lifecycle(self):
        transport = MockTransport()
        client = MCPClient(transport)

        await client.connect()
        self.assertTrue(transport.is_connected())

        tools = await client.list_tools()
        self.assertEqual(len(tools), 1)
        self.assertEqual(tools[0]["name"], "test_tool")

        await client.shutdown()
        self.assertFalse(transport.is_connected())

class TestToolRegistry(unittest.TestCase):
    def test_discovery(self):
        # Ensure we can discover tools
        initialize_registry()
        tools = registry.list_tools()

        tool_names = [t["name"] for t in tools]
        self.assertIn("execute_ark", tool_names)
        self.assertIn("openai_chat", tool_names)
        self.assertIn("ollama_generate", tool_names)
        self.assertIn("echo_message", tool_names)

        # Check handler
        handler = registry.get_handler("echo_message")
        self.assertIsNotNone(handler)
        self.assertEqual(handler("hello"), "Echo: hello")

class TestExecutionTool(unittest.IsolatedAsyncioTestCase):
    async def test_execute_ark(self):
        # We need to skip if meta/ark.py is not available or environment is limited

        code = 'sys.log("Hello from Ark")'

        if not os.path.exists("meta/ark.py"):
             print("Skipping Ark execution test: meta/ark.py not found")
             return

        # We execute invalid syntax to verify wrapper behavior
        result = await execute_ark("invalid syntax")

        self.assertIsInstance(result, dict)
        self.assertIn("stdout", result)
        self.assertIn("stderr", result)
        self.assertIn("exit_code", result)
        self.assertIn("duration_ms", result)

if __name__ == "__main__":
    unittest.main()
