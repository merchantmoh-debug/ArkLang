import os
from typing import Any, Set

class ArkConfigFallback:
    """Fallback configuration when Pydantic is not available."""

    def __init__(self):
        # Core Ark Settings
        self.ARK_MODEL = os.getenv("ARK_MODEL", "gpt-4")
        self.ARK_TEMPERATURE = float(os.getenv("ARK_TEMPERATURE", "0.7"))
        self.ARK_MAX_TOKENS = int(os.getenv("ARK_MAX_TOKENS", "4096"))
        self.ARK_SANDBOX_TYPE = os.getenv("ARK_SANDBOX_TYPE", "auto")
        self.ARK_MEMORY_KEY = os.getenv("ARK_MEMORY_KEY")
        self.ARK_DEBUG = os.getenv("ARK_DEBUG", "False").lower() == "true"

        # Legacy Agent Settings
        self.GOOGLE_API_KEY = os.getenv("GOOGLE_API_KEY", "")
        self.GEMINI_MODEL_NAME = os.getenv("GEMINI_MODEL_NAME", "gemini-2.0-flash-exp")
        self.OPENAI_BASE_URL = os.getenv("OPENAI_BASE_URL", "")
        self.OPENAI_API_KEY = os.getenv("OPENAI_API_KEY", "")
        self.OPENAI_MODEL = os.getenv("OPENAI_MODEL", "gpt-4o-mini")
        self.AGENT_NAME = os.getenv("AGENT_NAME", "AntigravityAgent")
        self.DEBUG_MODE = os.getenv("DEBUG_MODE", "False").lower() == "true"
        self.LLM_TIMEOUT = int(os.getenv("LLM_TIMEOUT", "30"))
        self.MEMORY_FILE = os.getenv("MEMORY_FILE", "agent_memory.enc")

        # Sandbox Security
        self.BANNED_IMPORTS: Set[str] = {
            "os", "sys", "subprocess", "shutil", "importlib", "socket",
            "pickle", "urllib", "http", "xml", "base64", "pty", "pdb",
            "platform", "venv", "ensurepip", "site", "imp", "posix", "nt",
            "ctypes", "builtins"
        }
        self.BANNED_FUNCTIONS: Set[str] = {
            "open", "exec", "eval", "compile", "__import__", "input",
            "exit", "quit", "help", "dir", "vars", "globals", "locals",
            "breakpoint", "memoryview", "getattr", "setattr", "delattr",
            "__builtins__"
        }
        self.BANNED_ATTRIBUTES: Set[str] = {
            "__subclasses__", "__bases__", "__globals__", "__code__",
            "__closure__", "__func__", "__self__", "__module__", "__dict__",
            "__builtins__", "__class__", "__base__", "open"
        }

        # MCP Configuration
        self.MCP_ENABLED = os.getenv("MCP_ENABLED", "False").lower() == "true"
        self.MCP_SERVERS_CONFIG = os.getenv("MCP_SERVERS_CONFIG", "mcp_servers.json")
        self.MCP_CONNECTION_TIMEOUT = int(os.getenv("MCP_CONNECTION_TIMEOUT", "30"))
        self.MCP_TOOL_PREFIX = os.getenv("MCP_TOOL_PREFIX", "mcp_")

    def get(self, key: str, default: Any = None) -> Any:
        return getattr(self, key, default)

settings = ArkConfigFallback()
config = settings
