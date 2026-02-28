import unittest
import sys
import os
from unittest.mock import MagicMock, patch

# Mock missing dependencies BEFORE importing src modules
# This allows tests to run without google-genai, requests, or pydantic installed
mock_genai = MagicMock()
sys.modules["google"] = MagicMock()
sys.modules["google.genai"] = mock_genai
sys.modules["requests"] = MagicMock()

mock_pydantic = MagicMock()
sys.modules["pydantic"] = mock_pydantic

class MockBaseSettings:
    def __init__(self, **kwargs):
        pass
    def __getattr__(self, name):
        return MagicMock()

mock_pydantic_settings = MagicMock()
mock_pydantic_settings.BaseSettings = MockBaseSettings
sys.modules["pydantic_settings"] = mock_pydantic_settings

# Ensure src is in sys.path
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

# Set environment variable for BaseAgent dummy client
os.environ["PYTEST_CURRENT_TEST"] = "true"

from src.agents.coder_agent import CoderAgent

class TestCoderAgent(unittest.TestCase):
    def setUp(self):
        self.agent = CoderAgent()

    def test_initialization(self):
        """Test that the agent initializes with correct role and system prompt."""
        self.assertEqual(self.agent.role, "coder")
        self.assertIn("You are the Coder Agent", self.agent.system_prompt)
        self.assertIn("software development", self.agent.system_prompt)
        self.assertIn("Python best practices", self.agent.system_prompt)
        self.assertIn("Google style", self.agent.system_prompt)

    def test_execute_basic(self):
        """Test basic execution using the dummy client."""
        task = "Write a function to calculate Fibonacci numbers."
        response = self.agent.execute(task)

        self.assertEqual(response, "[coder] Task completed")
        self.assertEqual(len(self.agent.conversation_history), 2)
        self.assertEqual(self.agent.conversation_history[0]["role"], "user")
        self.assertEqual(self.agent.conversation_history[0]["content"], task)
        self.assertEqual(self.agent.conversation_history[1]["role"], "assistant")
        self.assertEqual(self.agent.conversation_history[1]["content"], response)

    def test_inheritance(self):
        """Test that CoderAgent correctly inherits from BaseAgent."""
        # Get BaseAgent from the same module that CoderAgent uses
        # to avoid class identity mismatch from mock vs real pydantic paths
        import inspect
        base_classes = [cls.__name__ for cls in inspect.getmro(type(self.agent))]
        self.assertIn("BaseAgent", base_classes)

if __name__ == "__main__":
    unittest.main()
