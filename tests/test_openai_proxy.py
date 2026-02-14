import sys
import unittest
from unittest.mock import MagicMock, patch
import os

# Add src to path
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

class TestOpenAIProxy(unittest.TestCase):
    """Test suite for the call_openai_chat function in src/tools/openai_proxy.py."""

    def setUp(self):
        # Create mocks
        self.mock_requests = MagicMock()

        # Define exception classes for requests
        class MockRequestException(Exception): pass
        class MockHTTPError(MockRequestException): pass
        class MockConnectionError(MockRequestException): pass

        self.mock_requests.RequestException = MockRequestException
        self.mock_requests.HTTPError = MockHTTPError
        self.mock_requests.ConnectionError = MockConnectionError

        self.mock_settings = MagicMock()
        self.mock_settings.OPENAI_BASE_URL = "http://mock-openai-url/v1"
        self.mock_settings.OPENAI_API_KEY = "mock-api-key"
        self.mock_settings.OPENAI_MODEL = "mock-gpt-4"
        self.mock_settings.LLM_TIMEOUT = 30

        self.mock_config = MagicMock()
        self.mock_config.settings = self.mock_settings

        # Patch sys.modules
        self.modules_patcher = patch.dict(sys.modules, {
            "requests": self.mock_requests,
            "src.config": self.mock_config,
            "pydantic": MagicMock(),
            "pydantic_settings": MagicMock(),
        })
        self.modules_patcher.start()

        # Ensure we re-import the module to pick up the mocks
        if 'src.tools.openai_proxy' in sys.modules:
            del sys.modules['src.tools.openai_proxy']

        import src.tools.openai_proxy
        self.module = src.tools.openai_proxy

    def tearDown(self):
        self.modules_patcher.stop()

    def test_call_openai_chat_success(self):
        """Test a successful API call."""
        # Setup mock response
        mock_response = MagicMock()
        mock_response.json.return_value = {
            "choices": [
                {
                    "message": {
                        "content": "Hello, world!"
                    }
                }
            ]
        }
        mock_response.raise_for_status.return_value = None
        self.mock_requests.post.return_value = mock_response

        # Call the function
        result = self.module.call_openai_chat(prompt="Hi there")

        # Verify result
        self.assertEqual(result, "Hello, world!")

        # Verify requests.post call
        self.mock_requests.post.assert_called_once()
        args, kwargs = self.mock_requests.post.call_args
        self.assertEqual(args[0], "http://mock-openai-url/v1/chat/completions")
        self.assertEqual(kwargs['headers']['Authorization'], "Bearer mock-api-key")
        self.assertEqual(kwargs['json']['model'], "mock-gpt-4")
        self.assertEqual(kwargs['json']['messages'][0]['content'], "Hi there")

    def test_missing_base_url(self):
        """Test error when OPENAI_BASE_URL is missing."""
        self.mock_settings.OPENAI_BASE_URL = ""

        result = self.module.call_openai_chat(prompt="test")
        self.assertIn("Error: OPENAI_BASE_URL is not configured.", result)

    def test_missing_model(self):
        """Test error when OPENAI_MODEL is missing."""
        self.mock_settings.OPENAI_MODEL = ""

        result = self.module.call_openai_chat(prompt="test")
        self.assertIn("Error: OPENAI_MODEL is not configured.", result)

    def test_api_http_error(self):
        """Test handling of HTTP errors from the API."""
        self.mock_requests.post.side_effect = self.mock_requests.HTTPError("500 Server Error")

        result = self.module.call_openai_chat(prompt="test")
        self.assertIn("Error calling OpenAI-compatible API:", result)
        self.assertIn("500 Server Error", result)

    def test_json_decode_error(self):
        """Test handling of invalid JSON responses."""
        mock_response = MagicMock()
        mock_response.raise_for_status.return_value = None
        mock_response.json.side_effect = ValueError("Invalid JSON")
        mock_response.text = "Invalid JSON Body"
        self.mock_requests.post.return_value = mock_response

        result = self.module.call_openai_chat(prompt="test")
        self.assertIn("Error: Could not parse JSON response:", result)
        self.assertIn("Invalid JSON Body", result)

    def test_malformed_response(self):
        """Test handling of a valid JSON response with unexpected structure."""
        mock_response = MagicMock()
        mock_response.json.return_value = {"unexpected": "data"}
        mock_response.raise_for_status.return_value = None
        self.mock_requests.post.return_value = mock_response

        result = self.module.call_openai_chat(prompt="test")
        self.assertIn("{'unexpected': 'data'}", result)

    def test_system_prompt_inclusion(self):
        """Test that system prompt is correctly included in messages."""
        mock_response = MagicMock()
        mock_response.json.return_value = {"choices": [{"message": {"content": "ok"}}]}
        self.mock_requests.post.return_value = mock_response

        self.module.call_openai_chat(prompt="user prompt", system="system instruction")

        args, kwargs = self.mock_requests.post.call_args
        messages = kwargs['json']['messages']
        self.assertEqual(messages[0], {"role": "system", "content": "system instruction"})
        self.assertEqual(messages[1], {"role": "user", "content": "user prompt"})

if __name__ == '__main__':
    unittest.main()
