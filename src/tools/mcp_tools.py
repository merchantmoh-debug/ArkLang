"""
MCP Tool Registry and Management.

Provides a registry for discovering and managing tools (both local and remote MCP tools).
"""

import importlib
import inspect
import logging
import os
import pkgutil
from typing import Any, Callable, Dict, List, Optional, Tuple

from src.config import settings

logger = logging.getLogger("mcp_tools")

class ToolRegistry:
    """Registry for MCP tools."""

    def __init__(self):
        self._tools: Dict[str, Dict[str, Any]] = {}
        self._handlers: Dict[str, Callable] = {}

    def register(self, name: str, handler: Callable, schema: Dict[str, Any]):
        """Register a tool."""
        self._tools[name] = {
            "name": name,
            "description": schema.get("description", ""),
            "inputSchema": schema.get("inputSchema", {})
        }
        self._handlers[name] = handler
        logger.info(f"Registered tool: {name}")

    def get_tool(self, name: str) -> Optional[Dict[str, Any]]:
        """Get tool definition."""
        return self._tools.get(name)

    def get_handler(self, name: str) -> Optional[Callable]:
        """Get tool handler."""
        return self._handlers.get(name)

    def list_tools(self) -> List[Dict[str, Any]]:
        """List all registered tools."""
        return list(self._tools.values())

    def discover_tools(self, package_path: str = "src.tools"):
        """Auto-discover tools in the given package."""
        try:
            package = importlib.import_module(package_path)
            prefix = package.__name__ + "."

            for _, name, is_pkg in pkgutil.iter_modules(package.__path__, prefix):
                if is_pkg:
                    continue

                try:
                    module = importlib.import_module(name)

                    # Look for mcp_tool_def
                    if hasattr(module, "mcp_tool_def"):
                        tool_def = getattr(module, "mcp_tool_def")
                        tool_name = tool_def.get("name")

                        # Look for handler function
                        # Assuming handler has same name or is explicitly defined?
                        # Or we look for a function that matches name?
                        # Or maybe the module has a main function?

                        handler = None
                        if hasattr(module, tool_name):
                             handler = getattr(module, tool_name)
                        elif hasattr(module, "execute"): # Generic name
                             handler = getattr(module, "execute")
                        else:
                            # Try to find a function that looks like the tool
                            for attr_name, attr in inspect.getmembers(module):
                                if inspect.isfunction(attr) and attr_name == tool_name:
                                    handler = attr
                                    break

                        if handler and tool_name:
                            self.register(tool_name, handler, tool_def)
                        else:
                            logger.warning(f"Tool definition found in {name} but no matching handler for {tool_name}")

                except Exception as e:
                    logger.error(f"Error inspecting module {name}: {e}")

        except Exception as e:
            logger.error(f"Error discovering tools: {e}")


# Global Registry
registry = ToolRegistry()

def initialize_registry():
    """Initialize the registry by discovering tools."""
    registry.discover_tools()

def list_mcp_servers() -> str:
    """List configured MCP servers (wrapper for MCPClientManager)."""
    # This requires MCPClientManager to be initialized elsewhere or we init it here temporarily?
    # Usually the Agent manages the MCPClientManager.
    # We'll return a placeholder or connect if needed.
    from src.mcp_client import MCPClientManager

    manager = MCPClientManager()
    # We can't async connect here easily if this is sync.
    # But we can read config.
    if os.path.exists(manager.config_path):
        return f"Config found at {manager.config_path}. Use Agent to manage connections."
    return "No MCP servers configured."
