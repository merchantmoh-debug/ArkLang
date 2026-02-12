import sys
import os
import unittest
from unittest.mock import MagicMock, patch

# Ensure src is in sys.path
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

# Mock dependencies
sys.modules["src.agents.router_agent"] = MagicMock()
sys.modules["src.agents.coder_agent"] = MagicMock()
sys.modules["src.agents.reviewer_agent"] = MagicMock()
sys.modules["src.agents.researcher_agent"] = MagicMock()

from src.swarm import MessageBus, SwarmOrchestrator

class TestMessageBus(unittest.TestCase):
    def setUp(self):
        self.bus = MessageBus()

    def test_send_and_retrieve(self):
        self.bus.send("router", "coder", "task", "Write code")
        messages = self.bus.get_all_messages()
        self.assertEqual(len(messages), 1)
        self.assertEqual(messages[0]["from"], "router")
        self.assertEqual(messages[0]["to"], "coder")
        self.assertEqual(messages[0]["content"], "Write code")

    def test_get_context_for(self):
        self.bus.send("router", "coder", "task", "Task A")
        self.bus.send("router", "reviewer", "task", "Task B")
        self.bus.send("coder", "router", "result", "Result A")

        coder_context = self.bus.get_context_for("coder")
        self.assertEqual(len(coder_context), 2) # Task A + Result A
        
        reviewer_context = self.bus.get_context_for("reviewer")
        self.assertEqual(len(reviewer_context), 1) # Task B

    def test_clear(self):
        self.bus.send("router", "coder", "task", "Task A")
        self.bus.clear()
        self.assertEqual(len(self.bus.get_all_messages()), 0)


class TestSwarmOrchestrator(unittest.TestCase):
    @patch("src.swarm.RouterAgent")
    @patch("src.swarm.CoderAgent")
    @patch("src.swarm.ReviewerAgent")
    @patch("src.swarm.ResearcherAgent")
    def test_execute_flow(self, MockResearcher, MockReviewer, MockCoder, MockRouter):
        # Setup Mocks
        mock_router_instance = MockRouter.return_value
        mock_coder_instance = MockCoder.return_value
        
        # Router Plan
        mock_router_instance.analyze_and_delegate.return_value = [
            {"agent": "coder", "task": "Write utils.py"}
        ]
        mock_router_instance.synthesize_results.return_value = "Final Synthesis"
        
        # Worker Execution
        mock_coder_instance.execute.return_value = "Code Written"

        # Initialize Swarm
        swarm = SwarmOrchestrator()
        
        # Execute
        result = swarm.execute("Build a utility", verbose=False)
        
        # Verify
        self.assertEqual(result, "Final Synthesis")
        
        # Check Delegation
        mock_router_instance.analyze_and_delegate.assert_called_with("Build a utility")
        mock_coder_instance.execute.assert_called()
        
        # Check Message Bus
        messages = swarm.get_message_log()
        self.assertTrue(any(m["content"] == "Write utils.py" for m in messages))
        self.assertTrue(any(m["content"] == "Code Written" for m in messages))

if __name__ == "__main__":
    unittest.main()
