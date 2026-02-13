import unittest
from unittest.mock import patch, MagicMock
from src.tools.ollama_local import call_local_ollama

class TestOllamaLocalSecurity(unittest.TestCase):
    def test_security_blocks(self):
        """Test that SSRF attempts are blocked."""

        # Test 1: Subdomain attack
        result = call_local_ollama("test", host="http://localhost.attacker.com")
        self.assertIn("[Security Block]", result)
        self.assertIn("Host 'localhost.attacker.com' is not allowed", result)

        # Test 2: IP bypass
        result = call_local_ollama("test", host="http://127.0.0.1.nip.io")
        self.assertIn("[Security Block]", result)
        self.assertIn("Host '127.0.0.1.nip.io' is not allowed", result)

        # Test 3: Invalid scheme
        result = call_local_ollama("test", host="ftp://localhost")
        self.assertIn("[Security Block]", result)
        self.assertIn("Scheme 'ftp' not allowed", result)

    @patch("src.tools.ollama_local.requests.post")
    def test_valid_localhost(self, mock_post):
        """Test that valid localhost is allowed."""
        mock_response = MagicMock()
        mock_response.json.return_value = {"response": "ok"}
        mock_post.return_value = mock_response

        # Test valid host
        result = call_local_ollama("test", host="http://localhost:11434")

        # Should not be blocked
        self.assertNotIn("[Security Block]", result)
        self.assertEqual(result, "ok")

        # Verify call arguments
        args, kwargs = mock_post.call_args
        self.assertEqual(args[0], "http://localhost:11434/api/generate")

if __name__ == "__main__":
    unittest.main()
