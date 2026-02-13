import unittest
import sys
import os
from unittest.mock import patch, MagicMock

# Ensure src is in sys.path
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

class TestRouterAgent(unittest.TestCase):
    def setUp(self):
        # Clean up sys.modules to remove mocks from other tests (e.g., test_swarm.py)
        # This ensures we import the REAL RouterAgent class, not a mock.
        modules_to_clean = [
            "src.agents.router_agent",
            "src.agents.coder_agent",
            "src.agents.reviewer_agent",
            "src.agents.researcher_agent",
            "src.swarm"
        ]
        for module in modules_to_clean:
            if module in sys.modules:
                del sys.modules[module]

        # Mock environmental dependencies that are missing in the sandbox
        if "google" not in sys.modules:
            sys.modules["google"] = MagicMock()
        if "google.genai" not in sys.modules:
            sys.modules["google.genai"] = MagicMock()
        if "pydantic" not in sys.modules:
            sys.modules["pydantic"] = MagicMock()
        if "pydantic_settings" not in sys.modules:
            sys.modules["pydantic_settings"] = MagicMock()

        # Mock settings
        if "src.config" not in sys.modules:
            mock_settings = MagicMock()
            mock_settings.GOOGLE_API_KEY = "fake_key"
            mock_settings.GEMINI_MODEL_NAME = "gemini-pro"
            sys.modules["src.config"] = MagicMock()
            sys.modules["src.config"].settings = mock_settings

        # Local import to ensure we get the fresh module after cleanup
        from src.agents.router_agent import RouterAgent
        self.agent = RouterAgent()

    @patch('src.agents.router_agent.RouterAgent.execute')
    def test_analyze_and_delegate_single(self, mock_execute):
        """Test parsing of a single delegation."""
        mock_execute.return_value = """
DELEGATION:
- agent: coder
- task: Write a hello world script
"""
        task = "Write a hello world script"
        delegations = self.agent.analyze_and_delegate(task)

        self.assertEqual(len(delegations), 1)
        self.assertEqual(delegations[0]['agent'], 'coder')
        self.assertEqual(delegations[0]['task'], 'Write a hello world script')

    @patch('src.agents.router_agent.RouterAgent.execute')
    def test_analyze_and_delegate_multiple(self, mock_execute):
        """Test parsing of multiple delegations."""
        mock_execute.return_value = """
DELEGATION:
- agent: coder
- task: Implement the feature
- agent: reviewer
- task: Review the implementation
"""
        task = "Implement and review the feature"
        delegations = self.agent.analyze_and_delegate(task)

        self.assertEqual(len(delegations), 2)
        self.assertEqual(delegations[0]['agent'], 'coder')
        self.assertEqual(delegations[0]['task'], 'Implement the feature')
        self.assertEqual(delegations[1]['agent'], 'reviewer')
        self.assertEqual(delegations[1]['task'], 'Review the implementation')

    @patch('src.agents.router_agent.RouterAgent.execute')
    def test_analyze_and_delegate_fallback(self, mock_execute):
        """Test fallback when parsing fails (empty delegation list)."""
        mock_execute.return_value = "I cannot determine a structured delegation plan."

        task = "Write a python script"
        # The fallback logic in _simple_delegate should catch "write" -> coder
        delegations = self.agent.analyze_and_delegate(task)

        self.assertEqual(len(delegations), 1)
        self.assertEqual(delegations[0]['agent'], 'coder')
        self.assertEqual(delegations[0]['task'], task)

    @patch('src.agents.router_agent.RouterAgent.execute')
    def test_analyze_and_delegate_malformed(self, mock_execute):
        """Test handling of malformed output (e.g., missing task line)."""
        # This simulates a case where the model outputs an agent but forgets the task line immediately after
        mock_execute.return_value = """
DELEGATION:
- agent: coder
(no task provided)
"""
        task = "Do something ambiguous"
        # The code logic:
        # if line.startswith('- agent:'): current_delegation = {'agent': ...}
        # elif line.startswith('- task:') and current_delegation: current_delegation['task'] = ...
        # if current_delegation and 'task' in current_delegation: delegations.append(...)

        # Here, 'task' is never added to current_delegation, so it shouldn't be appended.
        # Thus, delegations should be empty, triggering fallback.

        delegations = self.agent.analyze_and_delegate(task)

        # Fallback for "Do something ambiguous" (no keywords matched in _simple_delegate default)
        # Default is coder
        self.assertEqual(len(delegations), 1)
        self.assertEqual(delegations[0]['agent'], 'coder')
        self.assertEqual(delegations[0]['task'], task)

    @patch('src.agents.router_agent.RouterAgent.execute')
    def test_analyze_and_delegate_partial_parse(self, mock_execute):
        """Test where one delegation is valid and another is malformed."""
        mock_execute.return_value = """
DELEGATION:
- agent: coder
- task: Write code
- agent: reviewer
(missing task)
"""
        task = "Write code and review"

        # The logic:
        # 1. agent: coder -> current={'agent': 'coder'}
        # 2. task: Write code -> current={'agent': 'coder', 'task': 'Write code'}
        # 3. agent: reviewer ->
        #    - previous current has 'task', so it is appended to delegations.
        #    - new current={'agent': 'reviewer'}
        # 4. (missing task) -> current remains {'agent': 'reviewer'}
        # End loop.
        # Final check: if current and 'task' in current -> append.
        # Here 'reviewer' has no task, so not appended.

        delegations = self.agent.analyze_and_delegate(task)

        self.assertEqual(len(delegations), 1)
        self.assertEqual(delegations[0]['agent'], 'coder')
        self.assertEqual(delegations[0]['task'], 'Write code')

if __name__ == '__main__':
    unittest.main()
