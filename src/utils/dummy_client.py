"""
Dummy client for testing and fallback scenarios.
"""
from typing import Any

class DummyResponse:
    """Mock response object mimicking Gemini's response structure."""
    def __init__(self, text: str):
        self.text = text

class DummyModels:
    """Mock models interface."""
    def __init__(self, response_text: str):
        self.response_text = response_text

    def generate_content(self, model: Any, contents: Any) -> DummyResponse:
        return DummyResponse(self.response_text)

class DummyClient:
    """
    A dummy client that returns a fixed response.
    Useful for testing or when the API key is missing.
    """
    def __init__(self, response_text: str = "Task completed", role: str = None):
        if role and not response_text.startswith(f"[{role}]"):
            response_text = f"[{role}] {response_text}"
        self.models = DummyModels(response_text)
