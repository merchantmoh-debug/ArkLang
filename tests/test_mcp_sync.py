
import unittest
import asyncio
from unittest.mock import MagicMock, AsyncMock, patch
from src.mcp_client import MCPClientManagerSync

class TestMCPClientManagerSync(unittest.TestCase):
    def setUp(self):
        # We mock the internal async manager to avoid real network calls
        # and dependency on external MCP servers.
        self.manager = MCPClientManagerSync()
        self.manager._async_manager.initialize = AsyncMock()
        self.manager._async_manager.shutdown = AsyncMock()
        self.manager._async_manager.get_tool_descriptions = MagicMock(return_value="Tool 1\nTool 2")
        self.manager._async_manager.get_status = MagicMock(return_value={"status": "ok"})
        self.manager._async_manager.get_all_tools_as_callables = MagicMock(return_value={
            "tool1": AsyncMock(return_value="result1")
        })

    def tearDown(self):
        self.manager.shutdown()

    def test_background_thread_lifecycle(self):
        """Verify thread starts and stops correctly."""
        self.assertTrue(self.manager._thread.is_alive())
        self.manager.shutdown()
        self.assertFalse(self.manager._thread.is_alive())
        self.assertTrue(self.manager._loop.is_closed())

    def test_methods_run_on_loop(self):
        """Verify methods execute on the background loop."""
        captured_loop = None
        async def mock_init():
            nonlocal captured_loop
            captured_loop = asyncio.get_running_loop()

        self.manager._async_manager.initialize = mock_init
        self.manager.initialize()

        self.assertIsNotNone(captured_loop)
        self.assertIs(captured_loop, self.manager._loop)

    def test_tool_call_sync_wrapper(self):
        """Verify tool calls are wrapped correctly and return values."""
        tools = self.manager.get_all_tools_as_callables()
        self.assertIn("tool1", tools)
        result = tools["tool1"]()
        self.assertEqual(result, "result1")

    def test_get_tool_descriptions(self):
        desc = self.manager.get_tool_descriptions()
        self.assertEqual(desc, "Tool 1\nTool 2")

    def test_get_status(self):
        status = self.manager.get_status()
        self.assertEqual(status, {"status": "ok"})

if __name__ == "__main__":
    unittest.main()
