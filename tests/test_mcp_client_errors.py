import sys
from unittest.mock import MagicMock

class MockDependencies:
    @staticmethod
    def setup():
        class MockBaseSettings:
            """Mock for pydantic_settings.BaseSettings"""
            def __init__(self, **kwargs):
                for k, v in kwargs.items():
                    setattr(self, k, v)
        
        def mock_field(default=None, default_factory=None, **kwargs):
            if default_factory:
                return default_factory()
            return default
        
        mock_pydantic = MagicMock()
        mock_pydantic.Field = mock_field
        
        mock_pydantic_settings = MagicMock()
        mock_pydantic_settings.BaseSettings = MockBaseSettings
        mock_pydantic_settings.SettingsConfigDict = MagicMock()
        
        return {
            "pydantic": mock_pydantic,
            "pydantic_settings": mock_pydantic_settings
        }

import unittest
import asyncio
from unittest.mock import MagicMock, AsyncMock, patch

# Import the class under test
from src.mcp_client import MCPClientManager, MCPServerConfig

class TestMCPClientErrors(unittest.IsolatedAsyncioTestCase):
    @classmethod
    def setUpClass(cls):
        cls.original_modules = {}
        mocks = MockDependencies.setup()
        for name, mock_module in mocks.items():
            cls.original_modules[name] = sys.modules.get(name)
            sys.modules[name] = mock_module

    @classmethod
    def tearDownClass(cls):
        for name, original in cls.original_modules.items():
            if original is None:
                sys.modules.pop(name, None)
            else:
                sys.modules[name] = original
    
    def setUp(self):
        self.manager = MCPClientManager()

    async def test_import_error(self):
        """Test handling when 'mcp' library is not installed."""
        config = MCPServerConfig(name="test_server", transport="stdio", command="echo", args=[])

        # Ensure 'mcp' is not in sys.modules to trigger ImportError
        with patch.dict(sys.modules):
            if 'mcp' in sys.modules:
                del sys.modules['mcp']
            # Also remove submodules if any
            for key in list(sys.modules.keys()):
                if key.startswith('mcp.'):
                    del sys.modules[key]

            await self.manager._connect_server(config)

        connection = self.manager.servers["test_server"]
        self.assertFalse(connection.connected)
        self.assertIn("MCP library not installed", str(connection.error))

    async def test_unsupported_transport(self):
        """Test handling of unsupported transport type."""
        config = MCPServerConfig(name="test_server", transport="invalid_transport")

        await self.manager._connect_server(config)

        connection = self.manager.servers["test_server"]
        self.assertFalse(connection.connected)
        self.assertIn("Unsupported transport", str(connection.error))

    async def test_stdio_connection_error(self):
        """Test handling of connection error in stdio transport."""
        config = MCPServerConfig(name="test_server", transport="stdio", command="echo", args=[])

        # Mock mcp structure
        mock_mcp = MagicMock()
        mock_client_session = AsyncMock()

        # Mock stdio_client to raise exception
        mock_stdio_client = MagicMock(side_effect=Exception("Process launch failed"))

        modules = {
            'mcp': mock_mcp,
            'mcp.client': MagicMock(),
            'mcp.client.stdio': MagicMock(stdio_client=mock_stdio_client),
            'mcp.ClientSession': mock_client_session,
            'mcp.StdioServerParameters': MagicMock(),
        }

        with patch.dict(sys.modules, modules):
            await self.manager._connect_server(config)

        connection = self.manager.servers["test_server"]
        self.assertFalse(connection.connected)
        self.assertEqual("Process launch failed", str(connection.error))

    async def test_http_connection_error(self):
        """Test handling of connection error in http transport."""
        config = MCPServerConfig(name="test_server", transport="http", url="http://localhost:8080")

        # Mock mcp structure
        mock_mcp = MagicMock()
        mock_client_session = AsyncMock()

        # Mock streamablehttp_client to raise exception
        mock_streamablehttp_client = MagicMock(side_effect=Exception("Network unreachable"))

        modules = {
            'mcp': mock_mcp,
            'mcp.client': MagicMock(),
            'mcp.client.streamable_http': MagicMock(streamablehttp_client=mock_streamablehttp_client),
            'mcp.ClientSession': mock_client_session,
        }

        with patch.dict(sys.modules, modules):
            await self.manager._connect_server(config)

        connection = self.manager.servers["test_server"]
        self.assertFalse(connection.connected)
        self.assertEqual("Network unreachable", str(connection.error))

    async def test_stdio_success(self):
        """Test successful connection with stdio transport."""
        config = MCPServerConfig(name="test_server", transport="stdio", command="echo", args=[])

        # Mock mcp structure
        mock_mcp = MagicMock()

        # Create a mock for ClientSession instance
        mock_session_instance = AsyncMock()
        mock_session_instance.initialize = AsyncMock()
        mock_session_instance.__aenter__.return_value = mock_session_instance

        # Mock ClientSession class
        mock_client_session_cls = MagicMock(return_value=mock_session_instance)

        # Explicitly attach ClientSession to mock_mcp so 'from mcp import ClientSession' picks it up
        mock_mcp.ClientSession = mock_client_session_cls
        mock_mcp.StdioServerParameters = MagicMock()

        # Mock stdio_client context manager
        mock_cm = AsyncMock()
        mock_cm.__aenter__.return_value = (AsyncMock(), AsyncMock())
        mock_stdio_client = MagicMock(return_value=mock_cm)

        # For submodule imports, we need both sys.modules entries AND parent module attributes linked
        mock_mcp_client = MagicMock()
        mock_mcp_client_stdio = MagicMock(stdio_client=mock_stdio_client)

        # Link hierarchy for attribute access if needed
        mock_mcp.client = mock_mcp_client
        mock_mcp_client.stdio = mock_mcp_client_stdio

        modules = {
            'mcp': mock_mcp,
            'mcp.client': mock_mcp_client,
            'mcp.client.stdio': mock_mcp_client_stdio,
            # We don't strictly need 'mcp.ClientSession' in sys.modules if it's imported from 'mcp' package
        }

        with patch.dict(sys.modules, modules):
            # We also need to mock _discover_tools as it is called on success
            # We patch it on the instance
            self.manager._discover_tools = AsyncMock()

            await self.manager._connect_server(config)

        connection = self.manager.servers["test_server"]
        self.assertTrue(connection.connected)
        self.assertIsNone(connection.error)

        # Verify mocked calls
        mock_stdio_client.assert_called_once()
        mock_client_session_cls.assert_called_once()
        self.manager._discover_tools.assert_called_once()

if __name__ == "__main__":
    unittest.main()
