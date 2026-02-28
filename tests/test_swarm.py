"""
Swarm Orchestration Tests — Isolated from agent module namespace.

Uses setUpClass/tearDownClass to inject and clean up sys.modules mocks,
preventing collection-time pollution of src.agents.* for other test files.
"""
import sys
import os
import importlib
import unittest
from unittest.mock import MagicMock, patch

# Ensure src is in sys.path
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

_MOCKED_MODULES = [
    "src.agents.router_agent",
    "src.agents.coder_agent",
    "src.agents.reviewer_agent",
    "src.agents.researcher_agent",
]


class _SwarmTestMixin:
    """Mixin that manages sys.modules lifecycle for swarm tests."""

    _saved_modules = {}
    _swarm_module = None

    @classmethod
    def _install_mocks(cls):
        """Save originals and inject MagicMock for agent modules."""
        for mod in _MOCKED_MODULES:
            cls._saved_modules[mod] = sys.modules.get(mod)
            sys.modules[mod] = MagicMock()
        # Remove cached src.swarm so it re-imports with mocked agents
        cls._saved_modules["src.swarm"] = sys.modules.pop("src.swarm", None)

    @classmethod
    def _uninstall_mocks(cls):
        """Restore original sys.modules state."""
        for mod in _MOCKED_MODULES + ["src.swarm"]:
            original = cls._saved_modules.get(mod)
            if original is None:
                sys.modules.pop(mod, None)
            else:
                sys.modules[mod] = original
        cls._saved_modules.clear()

    @classmethod
    def _import_swarm(cls):
        """Import src.swarm with mocks active."""
        cls._install_mocks()
        cls._swarm_module = importlib.import_module("src.swarm")
        return cls._swarm_module


class TestMessageBus(unittest.TestCase, _SwarmTestMixin):

    @classmethod
    def setUpClass(cls):
        swarm = cls._import_swarm()
        cls.MessageBus = swarm.MessageBus

    @classmethod
    def tearDownClass(cls):
        cls._uninstall_mocks()

    def setUp(self):
        self.bus = self.MessageBus()

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


class TestSwarmOrchestrator(unittest.TestCase, _SwarmTestMixin):

    @classmethod
    def setUpClass(cls):
        swarm = cls._import_swarm()
        cls.SwarmOrchestrator = swarm.SwarmOrchestrator

    @classmethod
    def tearDownClass(cls):
        cls._uninstall_mocks()

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
        swarm = self.SwarmOrchestrator()
        
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

    @patch("src.swarm.RouterAgent")
    @patch("src.swarm.CoderAgent")
    @patch("src.swarm.ReviewerAgent")
    @patch("src.swarm.ResearcherAgent")
    def test_execute_multiple_agents(self, MockResearcher, MockReviewer, MockCoder, MockRouter):
        # Setup Mocks
        mock_router_instance = MockRouter.return_value
        mock_coder_instance = MockCoder.return_value
        mock_reviewer_instance = MockReviewer.return_value

        # Router Plan: Delegate to Coder then Reviewer
        mock_router_instance.analyze_and_delegate.return_value = [
            {"agent": "coder", "task": "Write utils.py"},
            {"agent": "reviewer", "task": "Review utils.py"}
        ]
        mock_router_instance.synthesize_results.return_value = "Final Synthesis: Code Written and Reviewed"

        # Worker Execution
        mock_coder_instance.execute.return_value = "Code Written"
        mock_reviewer_instance.execute.return_value = "Code Reviewed"

        # Initialize Swarm
        swarm = self.SwarmOrchestrator()

        # Execute
        result = swarm.execute("Build and review a utility", verbose=False)

        # Verify
        self.assertEqual(result, "Final Synthesis: Code Written and Reviewed")

        # Check Delegation sequence
        self.assertEqual(mock_coder_instance.execute.call_count, 1)
        self.assertEqual(mock_reviewer_instance.execute.call_count, 1)

        # Check Message Bus
        messages = swarm.get_message_log()
        self.assertTrue(any(m["content"] == "Write utils.py" for m in messages))
        self.assertTrue(any(m["content"] == "Code Written" for m in messages))
        self.assertTrue(any(m["content"] == "Review utils.py" for m in messages))
        self.assertTrue(any(m["content"] == "Code Reviewed" for m in messages))

    @patch("src.swarm.RouterAgent")
    @patch("src.swarm.CoderAgent")
    @patch("src.swarm.ReviewerAgent")
    @patch("src.swarm.ResearcherAgent")
    def test_execute_unknown_agent(self, MockResearcher, MockReviewer, MockCoder, MockRouter):
        # Setup Mocks
        mock_router_instance = MockRouter.return_value

        # Router Plan: Delegate to unknown agent
        mock_router_instance.analyze_and_delegate.return_value = [
            {"agent": "janitor", "task": "Clean up logs"}
        ]
        # The orchestrator will call synthesize_results with the error message
        mock_router_instance.synthesize_results.return_value = "Synthesis with error"

        # Initialize Swarm
        swarm = self.SwarmOrchestrator()

        # Execute
        result = swarm.execute("Clean logs", verbose=False)

        # Verify
        self.assertEqual(result, "Synthesis with error")

        # Check that synthesize_results received the error message
        args, _ = mock_router_instance.synthesize_results.call_args
        delegations, results = args
        self.assertEqual(len(results), 1)
        self.assertIn("Error: Unknown agent 'janitor'", results[0])

    @patch("src.swarm.RouterAgent")
    @patch("src.swarm.CoderAgent")
    @patch("src.swarm.ReviewerAgent")
    @patch("src.swarm.ResearcherAgent")
    def test_execute_empty_delegation(self, MockResearcher, MockReviewer, MockCoder, MockRouter):
        # Setup Mocks
        mock_router_instance = MockRouter.return_value

        # Router Plan: Empty list
        mock_router_instance.analyze_and_delegate.return_value = []
        mock_router_instance.synthesize_results.return_value = "Nothing to do"

        # Initialize Swarm
        swarm = self.SwarmOrchestrator()

        # Execute
        result = swarm.execute("Do nothing", verbose=False)

        # Verify
        self.assertEqual(result, "Nothing to do")

        # Verify synthesize_results was called with empty lists
        args, _ = mock_router_instance.synthesize_results.call_args
        delegations, results = args
        self.assertEqual(delegations, [])
        self.assertEqual(results, [])

if __name__ == '__main__':
    unittest.main()
