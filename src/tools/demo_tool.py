"""
Demo MCP Tool.

A simple echo tool for testing and demonstration.
"""

def echo_message(message: str) -> str:
    """Echo the message back."""
    return f"Echo: {message}"


def greet_user(name: str) -> str:
    """Greet a user by name (backward-compat alias)."""
    return f"Hello, {name}! 🎉 Welcome to the Antigravity Agent with dynamic tool loading!"


def reverse_text(text: str) -> str:
    """Reverse the input text (backward-compat alias)."""
    return text[::-1]


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

