import unittest
from unittest.mock import MagicMock, patch
import sys
import os

# Ensure the src module can be imported
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '../..')))

# Mock dependencies for import time
mock_config = MagicMock()
mock_config.settings.MCP_TOOL_PREFIX = "mcp_"
mock_config.settings.MCP_ENABLED = True

mock_mcp_client_module = MagicMock()
mock_mcp_client_module.MCPClientManager = MagicMock

# We need to patch sys.modules BEFORE importing the module under test
# And keep it patched during the tests because the function does a local import.

# Create the patcher
modules_patcher = patch.dict(sys.modules, {
    "src.config": mock_config,
    "pydantic": MagicMock(),
    "src.mcp_client": mock_mcp_client_module
})

# Start the patcher
modules_patcher.start()

# Now import the module
from src.tools import mcp_tools

# We can stop the patcher here if we want to re-patch in the class,
# but simpler to just let it persist or use a class decorator that re-applies it.
# However, if we stop it now, 'mcp_tools' module is loaded.
# Inside 'list_mcp_servers', 'from src.mcp_client import ...' runs.
# If we stop the patcher, sys.modules loses 'src.mcp_client'.
# So the function will fail.
# We must keep the patch active during tests.

class TestMCPTools(unittest.TestCase):
    """Test suite for MCP tools integration."""

    @classmethod
    def setUpClass(cls):
        # Ensure patch is active if we stopped it (we didn't yet)
        pass

    @classmethod
    def tearDownClass(cls):
        # Stop the global patcher to be clean
        modules_patcher.stop()

    def setUp(self):
        """Set up test fixtures."""
        self.mock_manager = MagicMock()

    @patch.object(mcp_tools, "_get_mcp_manager")
    def test_manager_not_initialized(self, mock_get_manager):
        """Test when MCP manager is not initialized (returns None)."""
        mock_get_manager.return_value = None
        result = mcp_tools.list_mcp_servers()
        self.assertIn("MCP integration is not initialized", result)
        self.assertIn("Enable it in settings", result)

    @patch.object(mcp_tools, "_get_mcp_manager")
    def test_mcp_disabled(self, mock_get_manager):
        """Test when MCP is disabled in settings."""
        mock_get_manager.return_value = self.mock_manager
        self.mock_manager.get_status.return_value = {"enabled": False}

        result = mcp_tools.list_mcp_servers()
        self.assertIn("MCP integration is disabled", result)
        self.assertIn("Set MCP_ENABLED=true", result)

    @patch.object(mcp_tools, "_get_mcp_manager")
    def test_no_servers_configured(self, mock_get_manager):
        """Test when enabled but no servers are configured."""
        mock_get_manager.return_value = self.mock_manager
        self.mock_manager.get_status.return_value = {
            "enabled": True,
            "servers": {}
        }

        result = mcp_tools.list_mcp_servers()
        self.assertIn("No MCP servers configured", result)

    @patch.object(mcp_tools, "_get_mcp_manager")
    def test_servers_connected_and_disconnected(self, mock_get_manager):
        """Test mixed state of connected and disconnected servers."""
        mock_get_manager.return_value = self.mock_manager
        self.mock_manager.get_status.return_value = {
            "enabled": True,
            "servers": {
                "github": {
                    "connected": True,
                    "transport": "stdio",
                    "tools_count": 15,
                    "error": None
                },
                "database": {
                    "connected": False,
                    "transport": "http",
                    "tools_count": 0,
                    "error": "Connection refused"
                }
            }
        }

        result = mcp_tools.list_mcp_servers()

        # Verify header
        self.assertIn("MCP Servers Status", result)

        # Verify github (connected)
        self.assertIn("github", result)
        self.assertIn("(stdio)", result)
        self.assertIn("Connected", result)
        self.assertIn("15 tools", result)

        # Verify database (disconnected)
        self.assertIn("database", result)
        self.assertIn("(http)", result)
        self.assertIn("Disconnected", result)
        self.assertIn("Connection refused", result)

    @patch.object(mcp_tools, "_get_mcp_manager")
    def test_generic_exception(self, mock_get_manager):
        """Test generic exception handling during execution."""
        # Simulate an error during get_status
        mock_get_manager.return_value = self.mock_manager
        self.mock_manager.get_status.side_effect = Exception("Unexpected network error")

        result = mcp_tools.list_mcp_servers()
        self.assertIn("Error getting MCP status", result)
        self.assertIn("Unexpected network error", result)
