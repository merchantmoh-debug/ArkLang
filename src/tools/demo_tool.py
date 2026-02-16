"""
Demo MCP Tool.

A simple echo tool for testing and demonstration.
"""

def echo_message(message: str) -> str:
    """Echo the message back."""
    return f"Echo: {message}"

# MCP Tool Definition
mcp_tool_def = {
    "name": "echo_message",
    "description": "Echoes the input message.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "message": {"type": "string"}
        },
        "required": ["message"]
    }
}
