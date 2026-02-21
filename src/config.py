import os
from pathlib import Path
from typing import List, Optional, Set, Dict, Any

try:
    from pydantic import Field
    from pydantic_settings import BaseSettings, SettingsConfigDict

    class MCPServerConfig(BaseSettings):
        """Configuration for a single MCP server."""

        name: str = Field(description="Unique name for the MCP server")
        transport: str = Field(
            default="stdio", description="Transport type: stdio, http, sse"
        )
        command: Optional[str] = Field(
            default=None, description="Command to run for stdio transport"
        )
        args: List[str] = Field(
            default_factory=list, description="Arguments for the command"
        )
        url: Optional[str] = Field(default=None, description="URL for http/sse transport")
        env: dict = Field(
            default_factory=dict, description="Environment variables for the server"
        )
        enabled: bool = Field(default=True, description="Whether this server is enabled")

        model_config = SettingsConfigDict(extra="ignore")


    class ArkConfig(BaseSettings):
        """
        Ark Sovereign Computing Stack Configuration.
        Priority: Environment Variables (ARK_*) > Config File > Defaults.
        """

        # --- Core Ark Settings ---
        ARK_MODEL: str = Field(default="gpt-4", description="LLM model name")
        ARK_TEMPERATURE: float = Field(default=0.7, description="LLM temperature")
        ARK_MAX_TOKENS: int = Field(default=4096, description="Max output tokens")
        ARK_SANDBOX_TYPE: str = Field(default="auto", description="Sandbox type: auto, docker, local")
        ARK_MEMORY_KEY: Optional[str] = Field(default=None, description="Master encryption key for memory")
        ARK_DEBUG: bool = Field(default=False, description="Enable debug logging")

        # --- Legacy Agent Settings (Backward Compatibility) ---
        GOOGLE_API_KEY: str = Field(default="", description="Google API Key for Gemini")
        GEMINI_MODEL_NAME: str = Field(default="gemini-2.0-flash-exp", description="Gemini Model Name")

        OPENAI_BASE_URL: str = Field(default="", description="Base URL for OpenAI-compatible API")
        OPENAI_API_KEY: str = Field(default="", description="OpenAI API Key")
        OPENAI_MODEL: str = Field(default="gpt-4o-mini", description="OpenAI Model Name")

        AGENT_NAME: str = Field(default="AntigravityAgent", description="Agent Name")
        DEBUG_MODE: bool = Field(default=False, description="Debug Mode (Legacy)")
        LLM_TIMEOUT: int = Field(default=30, description="Timeout in seconds for LLM API calls")

        MEMORY_FILE: str = Field(default="agent_memory.enc", description="Memory file path")

        # Sandbox Security
        BANNED_IMPORTS: Set[str] = Field(
            default={
                "os", "sys", "subprocess", "shutil", "importlib", "socket",
                "pickle", "urllib", "http", "xml", "base64", "pty", "pdb",
                "platform", "venv", "ensurepip", "site", "imp", "posix", "nt"
            },
            description="Set of banned modules for local sandbox execution"
        )
        BANNED_FUNCTIONS: Set[str] = Field(
            default={
                "open", "exec", "eval", "compile", "__import__", "input",
                "exit", "quit", "help", "dir", "vars", "globals", "locals",
                "breakpoint", "memoryview", "getattr", "setattr", "delattr",
                "__builtins__"
            },
            description="Set of banned built-in functions for local sandbox execution"
        )
        BANNED_ATTRIBUTES: Set[str] = Field(
            default={
                "__subclasses__", "__bases__", "__globals__", "__code__",
                "__closure__", "__func__", "__self__", "__module__", "__dict__",
                "__builtins__"
            },
            description="Set of banned attributes for local sandbox execution"
        )

        # MCP Configuration
        MCP_ENABLED: bool = Field(default=False, description="Enable MCP integration")
        MCP_SERVERS_CONFIG: str = Field(
            default="mcp_servers.json", description="Path to MCP servers configuration file"
        )
        MCP_CONNECTION_TIMEOUT: int = Field(
            default=30, description="Timeout in seconds for MCP server connections"
        )
        MCP_TOOL_PREFIX: str = Field(
            default="mcp_", description="Prefix for MCP tool names to avoid conflicts"
        )

        model_config = SettingsConfigDict(
            env_prefix="ARK_",  # Prefix for environment variables
            env_file=[
                str(Path.home() / ".ark" / "config.toml"),
                str(Path.home() / ".ark" / "config.json"),
                ".env"
            ],
            env_file_encoding="utf-8",
            extra="ignore",
        )

        def get(self, key: str, default: Any = None) -> Any:
            """Get a configuration value with precedence."""
            return getattr(self, key, default)

    # Singleton instance
    settings = ArkConfig()
    config = settings

except ImportError:
    # Fallback for environments without pydantic (e.g. basic test runner)
    from .config_fallback import settings, config
