"""
OpenAI Proxy Tool.

Proxies requests to OpenAI-compatible APIs (OpenAI, Azure, Ollama, etc.)
with rate limiting, token counting, and cost estimation.
"""

import time
import asyncio
import logging
from typing import List, Dict, Any, Optional, Union
import requests

from src.config import settings

logger = logging.getLogger("openai_proxy")

class RateLimiter:
    """Simple rate limiter using a sliding window."""

    def __init__(self, max_requests: int = 60, window_seconds: int = 60):
        self.max_requests = max_requests
        self.window_seconds = window_seconds
        self.timestamps: List[float] = []

    async def acquire(self) -> bool:
        now = time.time()
        # Remove timestamps outside the window
        self.timestamps = [t for t in self.timestamps if now - t <= self.window_seconds]

        if len(self.timestamps) < self.max_requests:
            self.timestamps.append(now)
            return True
        return False

    async def wait(self):
        """Block until a request slot is available."""
        while not await self.acquire():
            await asyncio.sleep(0.1)


class OpenAIProxy:
    """Proxy for OpenAI-compatible APIs."""

    def __init__(self):
        self.base_url = settings.OPENAI_BASE_URL.rstrip("/")
        self.api_key = settings.OPENAI_API_KEY
        self.model = settings.OPENAI_MODEL
        # Default limit: 60 requests per minute
        self.rate_limiter = RateLimiter(max_requests=60, window_seconds=60)

    def _estimate_tokens(self, text: str) -> int:
        """Estimate token count (approx 4 chars per token)."""
        return len(text) // 4

    async def chat_completions(
        self,
        messages: List[Dict[str, str]],
        model: Optional[str] = None,
        temperature: float = 0.7,
        max_tokens: int = 1024,
        stop: Optional[Union[str, List[str]]] = None
    ) -> Dict[str, Any]:
        """
        Send a chat completion request.
        """
        target_model = model or self.model
        if not self.base_url:
            return {"error": "OPENAI_BASE_URL is not configured"}

        url = f"{self.base_url}/chat/completions"
        headers = {
            "Content-Type": "application/json"
        }
        if self.api_key:
            headers["Authorization"] = f"Bearer {self.api_key}"

        payload = {
            "model": target_model,
            "messages": messages,
            "temperature": temperature,
            "max_tokens": max_tokens
        }
        if stop:
            payload["stop"] = stop

        # Rate limiting
        await self.rate_limiter.wait()

        # Input token estimation
        input_text = "".join([m.get("content", "") for m in messages])
        input_tokens = self._estimate_tokens(input_text)

        try:
            # Run requests in thread
            response = await asyncio.to_thread(
                requests.post,
                url,
                json=payload,
                headers=headers,
                timeout=settings.LLM_TIMEOUT
            )
            response.raise_for_status()
            data = response.json()

            # Extract usage if available, else estimate
            usage = data.get("usage", {})
            if not usage:
                completion_text = data["choices"][0]["message"].get("content", "")
                output_tokens = self._estimate_tokens(completion_text)
                usage = {
                    "prompt_tokens": input_tokens,
                    "completion_tokens": output_tokens,
                    "total_tokens": input_tokens + output_tokens
                }
                data["usage"] = usage

            return data

        except requests.exceptions.RequestException as e:
            logger.error(f"Request failed: {e}")
            return {"error": str(e)}
        except ValueError:
            return {"error": "Invalid JSON response"}

    async def embeddings(self, input_text: Union[str, List[str]], model: str = "text-embedding-3-small") -> Dict[str, Any]:
        """
        Get embeddings for input text.
        """
        if not self.base_url:
             return {"error": "OPENAI_BASE_URL is not configured"}

        url = f"{self.base_url}/embeddings"
        headers = {
            "Content-Type": "application/json"
        }
        if self.api_key:
            headers["Authorization"] = f"Bearer {self.api_key}"

        payload = {
            "model": model,
            "input": input_text
        }

        await self.rate_limiter.wait()

        try:
            response = await asyncio.to_thread(
                requests.post,
                url,
                json=payload,
                headers=headers,
                timeout=settings.LLM_TIMEOUT
            )
            response.raise_for_status()
            return response.json()
        except Exception as e:
            return {"error": str(e)}


# Global instance
_proxy = OpenAIProxy()


async def openai_chat(
    prompt: str,
    system: Optional[str] = None,
    model: Optional[str] = None
) -> str:
    """
    Simple wrapper for chat completions.
    """
    messages = []
    if system:
        messages.append({"role": "system", "content": system})
    messages.append({"role": "user", "content": prompt})

    result = await _proxy.chat_completions(messages, model=model)

    if "error" in result:
        return f"Error: {result['error']}"

    try:
        return result["choices"][0]["message"]["content"]
    except (KeyError, IndexError):
        return "Error: Unexpected response format"


# MCP Tool Definition
mcp_tool_def = {
    "name": "openai_chat",
    "description": "Send a prompt to an OpenAI-compatible LLM.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "prompt": {"type": "string"},
            "system": {"type": "string"},
            "model": {"type": "string"}
        },
        "required": ["prompt"]
    }
}


def call_openai_chat(
    prompt: str,
    system: Optional[str] = None,
    model: Optional[str] = None
) -> str:
    """
    Synchronous wrapper for OpenAI-compatible chat completions.
    Uses requests.post directly.
    """
    base_url = settings.OPENAI_BASE_URL
    if not base_url:
        return "Error: OPENAI_BASE_URL is not configured."

    use_model = model or settings.OPENAI_MODEL
    if not use_model:
        return "Error: OPENAI_MODEL is not configured."

    api_key = getattr(settings, "OPENAI_API_KEY", "")
    timeout = getattr(settings, "LLM_TIMEOUT", 30)

    messages = []
    if system:
        messages.append({"role": "system", "content": system})
    messages.append({"role": "user", "content": prompt})

    url = f"{base_url.rstrip('/')}/chat/completions"
    headers = {
        "Content-Type": "application/json",
    }
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"

    payload = {
        "model": use_model,
        "messages": messages,
    }

    try:
        response = requests.post(url, headers=headers, json=payload, timeout=timeout)
        response.raise_for_status()
    except requests.RequestException as e:
        return f"Error calling OpenAI-compatible API: {e}"

    try:
        data = response.json()
    except (ValueError, Exception) as e:
        return f"Error: Could not parse JSON response: {response.text}"

    try:
        return data["choices"][0]["message"]["content"]
    except (KeyError, IndexError, TypeError):
        return f"Error: Unexpected response format: {data}"
