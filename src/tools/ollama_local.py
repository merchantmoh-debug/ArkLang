"""
Ollama Local Tool.

Direct integration with local Ollama instance for generation and model management.
"""

import asyncio
import logging
import json
import requests
from typing import List, Dict, Any, Optional
from urllib.parse import urlparse

from src.config import settings

logger = logging.getLogger("ollama_local")

class OllamaLocal:
    """Interface for local Ollama instance."""
    
    def __init__(self, host: str = "http://localhost:11434"):
        self.host = host.rstrip("/")

    def _validate_host(self) -> bool:
        """Validate host to prevent SSRF."""
        try:
            parsed = urlparse(self.host)
            if parsed.scheme not in ("http", "https"):
                return False
            if parsed.hostname not in ("127.0.0.1", "localhost"):
                return False
            return True
        except Exception:
            return False

    async def health_check(self) -> bool:
        """Verify Ollama is running."""
        if not self._validate_host():
            return False

        try:
            resp = await asyncio.to_thread(requests.get, f"{self.host}/api/tags", timeout=5)
            return resp.status_code == 200
        except Exception:
            return False

    async def list_models(self) -> List[str]:
        """List available models."""
        if not self._validate_host():
            return []

        try:
            resp = await asyncio.to_thread(requests.get, f"{self.host}/api/tags", timeout=10)
            resp.raise_for_status()
            models = resp.json().get("models", [])
            return [m["name"] for m in models]
        except Exception as e:
            logger.error(f"Failed to list models: {e}")
            return []

    async def pull_model(self, model: str) -> bool:
        """Pull a model if missing."""
        if not self._validate_host():
            return False

        logger.info(f"Pulling model {model}...")
        try:
            # Using streaming to avoid timeouts on large downloads
            # We run this in a thread, but iteration logic is tricky in to_thread if we want to log progress?
            # For simplicity, we just wait for completion or use a large timeout.
            # But pulling can take minutes.
            # We can use to_thread for the whole operation.

            def _pull():
                with requests.post(f"{self.host}/api/pull", json={"name": model}, stream=True, timeout=None) as resp:
                    resp.raise_for_status()
                    for line in resp.iter_lines():
                        if line:
                            try:
                                status = json.loads(line)
                                if "status" in status:
                                     pass # logger.debug(f"Pull status: {status['status']}")
                            except:
                                pass

            await asyncio.to_thread(_pull)
            return True
        except Exception as e:
            logger.error(f"Failed to pull model {model}: {e}")
            return False

    async def generate(
        self,
        prompt: str,
        model: str = "llama3",
        system: Optional[str] = None,
        stream: bool = False
    ) -> str:
        """Generate text using a model."""
        if not self._validate_host():
            return "Error: Invalid host configuration (SSRF protection)"

        if not await self.health_check():
             return "Error: Ollama is not running at " + self.host

        # Check if model exists, if not, pull it
        available_models = await self.list_models()
        model_exists = any(model == m or m.startswith(model + ":") for m in available_models)

        if not model_exists:
            logger.info(f"Model {model} not found locally. Attempting to pull...")
            if not await self.pull_model(model):
                return f"Error: Model {model} not found and pull failed."

        url = f"{self.host}/api/generate"
        payload = {
            "model": model,
            "prompt": prompt,
            "stream": False
        }
        if system:
            payload["system"] = system

        try:
            resp = await asyncio.to_thread(
                requests.post,
                url,
                json=payload,
                timeout=settings.LLM_TIMEOUT
            )
            resp.raise_for_status()
            return resp.json().get("response", "")
        except Exception as e:
            return f"Error generating text: {e}"


# Global instance
_ollama = OllamaLocal()

async def generate_ollama(prompt: str, model: str = "llama3") -> str:
    """Wrapper for Ollama generation."""
    return await _ollama.generate(prompt, model=model)

# Alias for tool discovery
ollama_generate = generate_ollama

# MCP Tool Definition
mcp_tool_def = {
    "name": "ollama_generate",
    "description": "Generate text using a local Ollama model.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "prompt": {"type": "string"},
            "model": {"type": "string", "default": "llama3"}
        },
        "required": ["prompt"]
    }
}
