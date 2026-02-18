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

from src.agents.base_agent import BaseAgent

class TestResearcherAgent(unittest.TestCase):
    def setUp(self):
        # Force reload of the module to get the real class
        if "src.agents.researcher_agent" in sys.modules:
            del sys.modules["src.agents.researcher_agent"]

        from src.agents.researcher_agent import ResearcherAgent
        self.agent = ResearcherAgent()

    def test_initialization(self):
        """Test that the agent initializes with correct role and system prompt."""
        self.assertEqual(self.agent.role, "researcher")
        self.assertIn("You are the Researcher Agent", self.agent.system_prompt)
        self.assertIn("expertise", self.agent.system_prompt)
        self.assertIn("Best Practices", self.agent.system_prompt)

    def test_execute_basic(self):
        """Test basic execution using the dummy client."""
        task = "Research the benefits of unit testing"
        response = self.agent.execute(task)

        self.assertEqual(response, "[researcher] Task completed")
        self.assertEqual(len(self.agent.conversation_history), 2)
        self.assertEqual(self.agent.conversation_history[0]["role"], "user")
        self.assertEqual(self.agent.conversation_history[0]["content"], task)
        self.assertEqual(self.agent.conversation_history[1]["role"], "assistant")
        self.assertEqual(self.agent.conversation_history[1]["content"], response)

    def test_inheritance(self):
        """Test that ResearcherAgent correctly inherits from BaseAgent."""
        self.assertIsInstance(self.agent, BaseAgent)

if __name__ == "__main__":
    unittest.main()
