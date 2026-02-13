import os
import sys
from unittest.mock import MagicMock, patch

# Mock missing dependencies BEFORE importing src modules
# This allows tests to run without google-genai or pydantic installed
mock_genai = MagicMock()
sys.modules["google"] = MagicMock()
sys.modules["google.genai"] = mock_genai

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
from src.config import settings

def test_execute_basic():
    """Test basic execution without context."""
    agent = BaseAgent(role="tester", system_prompt="You are a tester.")
    task = "Hello, world!"

    # Ensure clean state
    agent.reset_history()

    response = agent.execute(task)

    assert response == "[tester] Task completed"
    assert len(agent.conversation_history) == 2
    assert agent.conversation_history[0] == {"role": "user", "content": task}
    assert agent.conversation_history[1] == {"role": "assistant", "content": response}

def test_execute_with_context():
    """Test execution with context from other agents."""
    agent = BaseAgent(role="tester", system_prompt="You are a tester.")
    task = "Test with context"
    context = [
        {"from": "researcher", "content": "Found some info"},
        {"from": "coder", "content": "Wrote some code"}
    ]

    # Mock the generate_content to verify the prompt
    agent.client.models.generate_content = MagicMock()
    mock_response = MagicMock()
    mock_response.text = "Response with context"
    agent.client.models.generate_content.return_value = mock_response

    response = agent.execute(task, context=context)

    assert response == "Response with context"

    # Verify generate_content was called with expected prompt
    _, kwargs = agent.client.models.generate_content.call_args
    full_prompt = kwargs["contents"]

    assert "You are a tester." in full_prompt
    assert "Task: Test with context" in full_prompt
    assert "Context from other agents:" in full_prompt
    # Check strict formatting
    assert "[researcher]: Found some info" in full_prompt
    assert "[coder]: Wrote some code" in full_prompt

def test_execute_empty_context():
    """Test execution with empty context list behaves like None."""
    agent = BaseAgent(role="tester", system_prompt="You are a tester.")
    task = "Test with empty context"
    context = []

    # Mock again
    agent.client.models.generate_content = MagicMock()
    mock_response = MagicMock()
    mock_response.text = "Response without context"
    agent.client.models.generate_content.return_value = mock_response

    response = agent.execute(task, context=context)

    assert response == "Response without context"

    _, kwargs = agent.client.models.generate_content.call_args
    full_prompt = kwargs["contents"]

    # Should NOT contain "Context from other agents:"
    assert "Context from other agents:" not in full_prompt

def test_execute_response_fallback():
    """Test fallback when response object lacks .text attribute."""
    agent = BaseAgent(role="tester", system_prompt="You are a tester.")

    agent.client.models.generate_content = MagicMock()
    # Mock response that is just a string or object without .text
    class MockResponseNoText:
        def __str__(self):
            return "Fallback string response"

    mock_response = MockResponseNoText()
    agent.client.models.generate_content.return_value = mock_response

    response = agent.execute("Task fallback")

    assert response == "Fallback string response"

def test_execute_error_handling():
    """Test how the agent handles API errors."""
    agent = BaseAgent(role="tester", system_prompt="You are a tester.")

    # Mock an exception in generate_content
    agent.client.models.generate_content = MagicMock(side_effect=Exception("Connection failed"))

    # Clear history before test
    agent.reset_history()

    response = agent.execute("Fail task")

    assert response == "[tester] Error executing task: Connection failed"
    # History should not be updated on error
    assert len(agent.conversation_history) == 0

def test_reset_history():
    """Test clearing conversation history."""
    agent = BaseAgent(role="tester", system_prompt="You are a tester.")
    # Add manual history
    agent.conversation_history.append({"role": "user", "content": "foo"})
    assert len(agent.conversation_history) == 1

    agent.reset_history()
    assert len(agent.conversation_history) == 0

if __name__ == "__main__":
    # Run tests manually if pytest is not available or failing due to imports
    try:
        test_execute_basic()
        print("test_execute_basic: PASSED")
        test_execute_with_context()
        print("test_execute_with_context: PASSED")
        test_execute_empty_context()
        print("test_execute_empty_context: PASSED")
        test_execute_response_fallback()
        print("test_execute_response_fallback: PASSED")
        test_execute_error_handling()
        print("test_execute_error_handling: PASSED")
        test_reset_history()
        print("test_reset_history: PASSED")
        print("\nAll tests PASSED manually!")
    except Exception as e:
        print(f"\nTests FAILED: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
