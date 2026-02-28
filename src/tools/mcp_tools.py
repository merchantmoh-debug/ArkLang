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

def _get_mcp_manager():
    """Get or create the singleton MCPClientManager instance.
    
    Returns None if MCP is not available or not configured.
    """
    try:
        from src.mcp_client import MCPClientManager
        return MCPClientManager()
    except ImportError:
        return None
    except Exception:
        return None


def list_mcp_servers() -> str:
    """List configured MCP servers with their status.
    
    Returns a human-readable status report of all MCP servers.
    """
    try:
        manager = _get_mcp_manager()
        if manager is None:
            return "MCP integration is not initialized. Enable it in settings."
        
        status = manager.get_status()
        
        if not status.get("enabled", True):
            return "MCP integration is disabled. Set MCP_ENABLED=true to enable."
        
        servers = status.get("servers", {})
        if not servers:
            return "No MCP servers configured."
        
        lines = ["MCP Servers Status", "=" * 40]
        for name, info in servers.items():
            transport = info.get("transport", "unknown")
            connected = info.get("connected", False)
            tools_count = info.get("tools_count", 0)
            error = info.get("error")
            
            status_str = "Connected" if connected else "Disconnected"
            line = f"  {name} ({transport}): {status_str}"
            if connected:
                line += f" — {tools_count} tools"
            if error:
                line += f" — Error: {error}"
            lines.append(line)
        
        return "\n".join(lines)
    except Exception as e:
        return f"Error getting MCP status: {e}"

