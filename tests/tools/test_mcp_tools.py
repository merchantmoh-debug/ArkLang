import unittest
from unittest.mock import MagicMock, patch
import sys
import os

# Ensure the src module can be imported
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '../..')))

# Mock dependencies BEFORE import
_mock_config = MagicMock()
_mock_config.settings.MCP_TOOL_PREFIX = "mcp_"
_mock_config.settings.MCP_ENABLED = True

_mock_mcp_client_module = MagicMock()
_mock_mcp_client_module.MCPClientManager = MagicMock

# Patch sys.modules BEFORE import
_original_modules = {}
_patches = {
    "src.config": _mock_config,
    "pydantic": MagicMock(),
    "src.mcp_client": _mock_mcp_client_module,
}
for mod_name, mock_obj in _patches.items():
    _original_modules[mod_name] = sys.modules.get(mod_name)
    sys.modules[mod_name] = mock_obj

# NOW import the module under test
from src.tools import mcp_tools


class TestMCPTools(unittest.TestCase):
    """Test suite for MCP tools integration."""

    def setUp(self):
        """Set up test fixtures."""
        self.mock_manager = MagicMock()

    @patch("src.tools.mcp_tools._get_mcp_manager")
    def test_manager_not_initialized(self, mock_get_manager):
        """Test when MCP manager is not initialized (returns None)."""
        mock_get_manager.return_value = None
        result = mcp_tools.list_mcp_servers()
        self.assertIn("MCP integration is not initialized", result)
        self.assertIn("Enable it in settings", result)

    @patch("src.tools.mcp_tools._get_mcp_manager")
    def test_mcp_disabled(self, mock_get_manager):
        """Test when MCP is disabled in settings."""
        mock_get_manager.return_value = self.mock_manager
        self.mock_manager.get_status.return_value = {"enabled": False}

        result = mcp_tools.list_mcp_servers()
        self.assertIn("MCP integration is disabled", result)
        self.assertIn("Set MCP_ENABLED=true", result)

    @patch("src.tools.mcp_tools._get_mcp_manager")
    def test_no_servers_configured(self, mock_get_manager):
        """Test when enabled but no servers are configured."""
        mock_get_manager.return_value = self.mock_manager
        self.mock_manager.get_status.return_value = {
            "enabled": True,
            "servers": {}
        }

        result = mcp_tools.list_mcp_servers()
        self.assertIn("No MCP servers configured", result)

    @patch("src.tools.mcp_tools._get_mcp_manager")
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

    @patch("src.tools.mcp_tools._get_mcp_manager")
    def test_generic_exception(self, mock_get_manager):
        """Test generic exception handling during execution."""
        # Simulate an error during get_status
        mock_get_manager.return_value = self.mock_manager
        self.mock_manager.get_status.side_effect = Exception("Unexpected network error")

        result = mcp_tools.list_mcp_servers()
        self.assertIn("Error getting MCP status", result)
        self.assertIn("Unexpected network error", result)
