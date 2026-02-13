import sys
import os
import unittest
from unittest.mock import MagicMock, patch
from datetime import datetime

# Ensure src is in sys.path
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

# Mock dependencies to prevent import errors or side effects from agents
sys.modules["src.agents.router_agent"] = MagicMock()
sys.modules["src.agents.coder_agent"] = MagicMock()
sys.modules["src.agents.reviewer_agent"] = MagicMock()
sys.modules["src.agents.researcher_agent"] = MagicMock()

from src.swarm import MessageBus

class TestMessageBus(unittest.TestCase):
    """Unit tests for the MessageBus class."""

    def setUp(self):
        """Set up a fresh MessageBus instance for each test."""
        self.bus = MessageBus()

    def test_initialization(self):
        """Test that the message bus initializes with an empty list."""
        self.assertEqual(self.bus.messages, [])
        self.assertEqual(len(self.bus.get_all_messages()), 0)

    @patch('src.swarm.datetime')
    def test_send_message(self, mock_datetime):
        """Test sending a message and verifying its content and timestamp."""
        # Mock datetime to return a fixed time
        fixed_time = datetime(2023, 10, 27, 12, 0, 0)
        mock_datetime.now.return_value = fixed_time

        self.bus.send("agent_a", "agent_b", "task", "Hello World")

        messages = self.bus.get_all_messages()
        self.assertEqual(len(messages), 1)

        msg = messages[0]
        self.assertEqual(msg["from"], "agent_a")
        self.assertEqual(msg["to"], "agent_b")
        self.assertEqual(msg["type"], "task")
        self.assertEqual(msg["content"], "Hello World")
        self.assertEqual(msg["timestamp"], fixed_time.isoformat())

    def test_get_context_for(self):
        """Test retrieving context relevant to a specific agent."""
        # Add messages
        self.bus.send("agent_a", "agent_b", "task", "Message 1") # Relevant to A and B
        self.bus.send("agent_c", "agent_a", "reply", "Message 2") # Relevant to C and A
        self.bus.send("agent_b", "agent_c", "query", "Message 3") # Relevant to B and C (Not A)

        # Context for Agent A
        context_a = self.bus.get_context_for("agent_a")
        self.assertEqual(len(context_a), 2)
        self.assertEqual(context_a[0]["content"], "Message 1")
        self.assertEqual(context_a[1]["content"], "Message 2")

        # Context for Agent B
        context_b = self.bus.get_context_for("agent_b")
        self.assertEqual(len(context_b), 2)
        self.assertEqual(context_b[0]["content"], "Message 1")
        self.assertEqual(context_b[1]["content"], "Message 3")

        # Context for Agent D (non-existent)
        context_d = self.bus.get_context_for("agent_d")
        self.assertEqual(len(context_d), 0)

    def test_get_all_messages_returns_copy(self):
        """Test that get_all_messages returns a copy of the list."""
        self.bus.send("agent_a", "agent_b", "task", "Original")

        messages = self.bus.get_all_messages()
        messages.append({"fake": "message"}) # Modify the returned list

        # Verify the internal list is unchanged
        self.assertEqual(len(self.bus.messages), 1)
        self.assertEqual(self.bus.messages[0]["content"], "Original")

    def test_clear(self):
        """Test clearing the message bus."""
        self.bus.send("agent_a", "agent_b", "task", "Message 1")
        self.bus.send("agent_b", "agent_a", "reply", "Message 2")

        self.assertEqual(len(self.bus.get_all_messages()), 2)

        self.bus.clear()

        self.assertEqual(len(self.bus.get_all_messages()), 0)
        self.assertEqual(self.bus.messages, [])

    def test_edge_cases(self):
        """Test edge cases like empty strings."""
        self.bus.send("", "", "", "")
        messages = self.bus.get_all_messages()
        self.assertEqual(len(messages), 1)
        self.assertEqual(messages[0]["from"], "")
        self.assertEqual(messages[0]["content"], "")

if __name__ == "__main__":
    unittest.main()
