import sys
import unittest
from unittest.mock import MagicMock, AsyncMock, patch
from dataclasses import dataclass, field

class TestMCPClientManager(unittest.IsolatedAsyncioTestCase):

    @classmethod
    def setUpClass(cls):
        # Check if dependencies are available
        cls.modules_to_restore = {}

        try:
            # Attempt to import normally
            import src.mcp_client
            cls.MCPClientManager = src.mcp_client.MCPClientManager
        except ImportError:
            # Prepare mocks if dependencies are missing
            cls.modules_to_restore = {
                k: sys.modules.get(k)
                for k in ["pydantic", "pydantic_settings", "src.config", "src.mcp_client"]
            }

            # Mock pydantic
            mock_pydantic = MagicMock()
            mock_pydantic.Field = lambda default=None, **kwargs: default
            sys.modules["pydantic"] = mock_pydantic

            mock_pydantic_settings = MagicMock()
            class MockBaseSettings:
                pass
            mock_pydantic_settings.BaseSettings = MockBaseSettings
            mock_pydantic_settings.SettingsConfigDict = MagicMock()
            sys.modules["pydantic_settings"] = mock_pydantic_settings

            # Mock src.config
            mock_config = MagicMock()
            mock_config.settings.MCP_ENABLED = True
            mock_config.settings.MCP_TOOL_PREFIX = "mcp_"
            mock_config.settings.MCP_SERVERS_CONFIG = "mcp_servers.json"

            @dataclass
            class MockMCPServerConfig:
                name: str
                transport: str = "stdio"
                command: str = None
                args: list = field(default_factory=list)
                url: str = None
                env: dict = field(default_factory=dict)
                enabled: bool = True

            mock_config.MCPServerConfig = MockMCPServerConfig
            sys.modules["src.config"] = mock_config

            # Import module under test
            # Ensure we reload if it was partially loaded
            if "src.mcp_client" in sys.modules:
                del sys.modules["src.mcp_client"]

            import src.mcp_client
            cls.MCPClientManager = src.mcp_client.MCPClientManager

    @classmethod
    def tearDownClass(cls):
        # Restore sys.modules to avoid side effects on other tests
        for name, module in cls.modules_to_restore.items():
            if module is None:
                sys.modules.pop(name, None)
            else:
                sys.modules[name] = module
        
        # Also clean up src.mcp_client specifically as it was re-imported during tests
        sys.modules.pop("src.mcp_client", None)

    async def test_call_tool_execution_error(self):
        """Test that call_tool handles exceptions raised by the tool execution."""
        manager = self.MCPClientManager()

        # Create a mock tool that raises an exception
        mock_tool = AsyncMock(side_effect=Exception("Execution failed"))

        # Mock get_all_tools_as_callables to return our mock tool
        with patch.object(manager, 'get_all_tools_as_callables', return_value={"test_tool": mock_tool}):
            success, result = await manager.call_tool("test_tool", {"arg": "val"})

            self.assertFalse(success)
            self.assertEqual(result, "Execution failed")
            mock_tool.assert_called_once_with(arg="val")

    async def test_call_tool_not_found(self):
        """Test that call_tool returns failure when tool is not found."""
        manager = self.MCPClientManager()

        # Mock get_all_tools_as_callables to return empty dict
        with patch.object(manager, 'get_all_tools_as_callables', return_value={}):
            success, result = await manager.call_tool("missing_tool", {})

            self.assertFalse(success)
            self.assertEqual(result, "Tool 'missing_tool' not found")

    async def test_call_tool_success(self):
        """Test successful tool execution."""
        manager = self.MCPClientManager()

        # Create a mock tool that returns a success result
        mock_tool = AsyncMock(return_value="Success Result")

        # Mock get_all_tools_as_callables to return our mock tool
        with patch.object(manager, 'get_all_tools_as_callables', return_value={"success_tool": mock_tool}):
            success, result = await manager.call_tool("success_tool", {"arg": "val"})

            self.assertTrue(success)
            self.assertEqual(result, "Success Result")
            mock_tool.assert_called_once_with(arg="val")

if __name__ == "__main__":
    unittest.main()
