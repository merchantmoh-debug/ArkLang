import os
import sys
from unittest.mock import MagicMock, patch

def test_openai_backend_flow():
    """Test BaseAgent with OpenAI backend (full flow)."""

    # Setup Mocks
    mock_genai = MagicMock()
    mock_requests = MagicMock()
    mock_pydantic = MagicMock()
    mock_settings_cls = MagicMock()

    # Mock Settings
    mock_settings_obj = MagicMock()
    mock_settings_obj.GOOGLE_API_KEY = ""
    mock_settings_obj.OPENAI_BASE_URL = "http://mock-openai-url/v1"
    mock_settings_obj.OPENAI_MODEL = "mock-gpt-4"
    mock_settings_obj.GEMINI_MODEL_NAME = "mock-gemini"

    # Prepare sys.modules patch dict
    modules_patch = {
        "google": MagicMock(),
        "google.genai": mock_genai,
        "requests": mock_requests,
        "pydantic": mock_pydantic,
        "pydantic_settings": MagicMock(BaseSettings=mock_settings_cls),
        "src.config": MagicMock(settings=mock_settings_obj),
        "src.tools.openai_proxy": MagicMock(),
    }

    # Remove src.agents.base_agent from sys.modules if it exists, to force re-import
    if "src.agents.base_agent" in sys.modules:
        del sys.modules["src.agents.base_agent"]

    # Apply patches
    with patch.dict(sys.modules, modules_patch):
        # Ensure src is in sys.path
        project_root = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
        if project_root not in sys.path:
            sys.path.insert(0, project_root)

        # Import BaseAgent (this uses the patched modules)
        import src.agents.base_agent
        from src.agents.base_agent import BaseAgent

        # Configure call_openai_chat mock
        # Note: src.agents.base_agent imports call_openai_chat from src.tools.openai_proxy
        # So it is available as src.agents.base_agent.call_openai_chat

        mock_response = "OpenAI Response"
        src.agents.base_agent.call_openai_chat.return_value = mock_response

        # Handle PYTEST_CURRENT_TEST
        old_env = os.environ.get("PYTEST_CURRENT_TEST")
        if "PYTEST_CURRENT_TEST" in os.environ:
            del os.environ["PYTEST_CURRENT_TEST"]

        try:
            # 1. Test Initialization
            agent = BaseAgent(role="tester", system_prompt="System Prompt")
            assert agent.use_openai_backend is True
            assert agent.client is None
            print("[PASS] Initialization check.")

            # 2. Test Execution
            task = "Test Task"
            response = agent.execute(task)

            # Debugging
            # print(f"DEBUG: response: {response!r}")

            # Assertions
            assert response == mock_response
            src.agents.base_agent.call_openai_chat.assert_called_once()

            # Check args
            args, kwargs = src.agents.base_agent.call_openai_chat.call_args
            prompt_arg = kwargs.get("prompt")
            assert "System Prompt" in prompt_arg
            assert "Task: Test Task" in prompt_arg

            print("[PASS] Execution check.")

        finally:
            if old_env:
                os.environ["PYTEST_CURRENT_TEST"] = old_env

if __name__ == "__main__":
    try:
        test_openai_backend_flow()
        print("\nAll OpenAI backend tests PASSED!")
    except Exception as e:
        print(f"\nTest FAILED: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
