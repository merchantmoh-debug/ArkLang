import unittest
import asyncio
import json
import time
from unittest.mock import MagicMock, AsyncMock
from src.mcp_client import MCPClientManager, MCPServerConnection, MCPServerConfig, MCPTool
from src.config import settings

class TestMCPClient(unittest.TestCase):
    def setUp(self):
        # Nuclear Option: Force reload src.config to clear any mock pollution from other tests
        if 'src.config' in sys.modules:
            import importlib
            import src.config
            importlib.reload(src.config)
            # Also reload mcp_client as it depends on config
            if 'src.mcp_client' in sys.modules:
                import src.mcp_client
                importlib.reload(src.mcp_client)

    def test_tool_wrapper_creation(self):
        """
        Verify that get_all_tools_as_callables correctly creates wrappers
        and that those wrappers function as expected.
        """
        # Setup
        manager = MCPClientManager()
        config = MCPServerConfig(name="test_server", command="echo", args=[], env={})
        connection = MCPServerConnection(config=config)
        connection.connected = True

        # Mock session
        mock_session = AsyncMock()
        # Mock call_tool to return a simple string or object depending on what the wrapper expects
        # The wrapper checks for .content or .structuredContent attributes on the result object
        # or just casts to str(result). Let's return a simple string for now.
        mock_session.call_tool.return_value = "Tool result"
        connection.session = mock_session

        # Add a tool
        tool = MCPTool(
            name="test_tool",
            description="Test tool description",
            server_name="test_server",
            input_schema={"type": "object", "properties": {"arg": {"type": "string"}}},
            original_name="test_tool"
        )
        connection.tools.append(tool)
        manager.servers["test_server"] = connection

        # Action
        callables = manager.get_all_tools_as_callables()

        # Verification 1: Wrapper existence
        prefixed_name = f"{settings.MCP_TOOL_PREFIX}test_server_test_tool"
        self.assertIn(prefixed_name, callables)
        wrapper = callables[prefixed_name]
        self.assertTrue(callable(wrapper))

        # Verification 2: Metadata (Docstring and Name)
        self.assertEqual(wrapper.__name__, prefixed_name)
        self.assertIn("Test tool description", wrapper.__doc__)
        self.assertIn("Server: test_server", wrapper.__doc__)
        self.assertIn('"type": "object"', wrapper.__doc__) # Schema check

        # Verification 3: Execution
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        try:
            result = loop.run_until_complete(wrapper(arg="value"))
        finally:
            loop.close()

        self.assertEqual(result, "Tool result")

        # Verification 4: Verify mock call
        mock_session.call_tool.assert_called_once_with("test_tool", arguments={"arg": "value"})

    def test_get_all_tools_as_callables_multiple_calls(self):
        """
        Verify that subsequent calls to get_all_tools_as_callables return
        consistent results (this tests caching logic if present).
        """
        manager = MCPClientManager()
        config = MCPServerConfig(name="s1", command="echo", args=[], env={})
        conn = MCPServerConnection(config=config)
        conn.connected = True
        conn.session = AsyncMock()

        tool = MCPTool(name="t1", description="d1", server_name="s1", input_schema={}, original_name="t1")
        conn.tools.append(tool)
        manager.servers["s1"] = conn

        callables1 = manager.get_all_tools_as_callables()
        callables2 = manager.get_all_tools_as_callables()

        self.assertEqual(callables1.keys(), callables2.keys())
        # Check if they are the SAME function objects (identity check)
        # Currently, the code caches the entire dict in self._tool_cache, so this should be true.
        key = list(callables1.keys())[0]
        self.assertIs(callables1[key], callables2[key])

    def test_discover_tools_populates_cache(self):
        """
        Verify that _discover_tools correctly populates the tool_wrappers cache.
        """
        async def run_test():
            manager = MCPClientManager()
            connection = MCPServerConnection(config=MCPServerConfig(name="s1", command="echo", args=[], env={}))
            connection.connected = True

            # Mock session.list_tools
            mock_tool = MagicMock()
            mock_tool.name = "t1"
            mock_tool.description = "d1"
            # Simulate inputSchema attribute
            mock_tool.inputSchema = {"type": "object"}

            mock_response = MagicMock()
            mock_response.tools = [mock_tool]

            mock_session = AsyncMock()
            mock_session.list_tools.return_value = mock_response
            connection.session = mock_session

            # Action
            await manager._discover_tools(connection)

            # Verify
            prefixed_name = f"{settings.MCP_TOOL_PREFIX}s1_t1"
            self.assertTrue(connection.tool_wrappers)
            self.assertIn(prefixed_name, connection.tool_wrappers)
            self.assertTrue(callable(connection.tool_wrappers[prefixed_name]))

        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        try:
            loop.run_until_complete(run_test())
        finally:
            loop.close()

    def test_initialize_parallelism(self):
        """
        Verify that multiple MCP servers connect in parallel using asyncio.gather.
        """
        original_enabled = settings.MCP_ENABLED
        try:
            async def run_test():
                # Enable MCP
                settings.MCP_ENABLED = True

                manager = MCPClientManager()

                # Create 5 configs
                configs = [
                    MCPServerConfig(name=f"s{i}", transport="stdio", command="echo")
                    for i in range(5)
                ]
                manager._load_server_configs = AsyncMock(return_value=configs)

                # Mock delay: 0.2s
                async def mock_connect(config):
                    await asyncio.sleep(0.2)

                # IMPORTANT: Patch the instance method directly before calling initialize
                manager._connect_server = AsyncMock(side_effect=mock_connect)

                start = time.perf_counter()
                await manager.initialize()
                end = time.perf_counter()

                duration = end - start

                # Sequential: 5 * 0.2 = 1.0s
                # Parallel: ~0.2s + overhead
                # Allow up to 0.6s to account for setup/teardown and overhead
                self.assertLess(duration, 0.6, f"MCP initialization took {duration:.4f}s, expected < 0.6s (Parallel)")
                self.assertEqual(manager._connect_server.call_count, 5)

            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            try:
                loop.run_until_complete(run_test())
            finally:
                loop.close()
        finally:
            settings.MCP_ENABLED = original_enabled

if __name__ == '__main__':
    unittest.main()
