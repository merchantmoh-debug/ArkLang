import unittest
from unittest.mock import patch, MagicMock
import urllib.request
import urllib.error
import os
import sys

# Add meta directory to path to import ark
sys.path.append(os.path.join(os.path.dirname(__file__), '../meta'))

# Import the module to be tested
# We need to import ark, but since it's a script, we might need to import it carefully
# or just mock the functions if we were testing them in isolation.
# However, to test `detect_ai_mode` which we will add to ark.py, we need ark.py to have it.
# Since we haven't modified ark.py yet, we can't import `detect_ai_mode` from it.

# STRATEGY CHANGE:
# I will write the test script assuming the functions EXIST in ark.py.
# This test script will fail initially (ImportError or AttributeError), which is fine for TDD.
# But `ark.py` executes code on import (if main block isn't guarded properly, but it is).
# However, `ark.py` has imports that might fail if dependencies aren't met, but here they are standard.

import ark

class TestAIMode(unittest.TestCase):

    def setUp(self):
        # Reset the global ARK_AI_MODE before each test
        ark.ARK_AI_MODE = None

    @patch('urllib.request.urlopen')
    def test_detect_ai_mode_ollama(self, mock_urlopen):
        # Simulate successful connection to Ollama
        mock_response = MagicMock()
        mock_response.getcode.return_value = 200
        mock_urlopen.return_value.__enter__.return_value = mock_response

        mode = ark.detect_ai_mode()
        self.assertEqual(mode, "OLLAMA")
        self.assertEqual(ark.ARK_AI_MODE, "OLLAMA")

    @patch('urllib.request.urlopen')
    @patch.dict(os.environ, {"GOOGLE_API_KEY": "fake_key"})
    def test_detect_ai_mode_gemini(self, mock_urlopen):
        # Simulate Ollama down (URLError)
        mock_urlopen.side_effect = urllib.error.URLError("Connection refused")

        mode = ark.detect_ai_mode()
        self.assertEqual(mode, "GEMINI")
        self.assertEqual(ark.ARK_AI_MODE, "GEMINI")

    @patch('urllib.request.urlopen')
    @patch.dict(os.environ, {}, clear=True)
    def test_detect_ai_mode_mock(self, mock_urlopen):
        # Simulate Ollama down and no API Key
        mock_urlopen.side_effect = urllib.error.URLError("Connection refused")

        mode = ark.detect_ai_mode()
        self.assertEqual(mode, "MOCK")
        self.assertEqual(ark.ARK_AI_MODE, "MOCK")

    @patch('ark.detect_ai_mode')
    @patch('ark.ask_ollama')
    @patch('ark.ask_gemini')
    @patch('ark.ask_mock')
    def test_ask_ai_dispatch_ollama(self, mock_mock, mock_gemini, mock_ollama, mock_detect):
        mock_detect.return_value = "OLLAMA"
        mock_ollama.return_value = ark.ArkValue("Ollama Response", "String")

        args = [ark.ArkValue("Hello", "String")]
        result = ark.ask_ai(args)

        mock_ollama.assert_called_once()
        mock_gemini.assert_not_called()
        mock_mock.assert_not_called()
        self.assertEqual(result.val, "Ollama Response")

    @patch('ark.detect_ai_mode')
    @patch('ark.ask_ollama')
    @patch('ark.ask_gemini')
    @patch('ark.ask_mock')
    def test_ask_ai_dispatch_gemini(self, mock_mock, mock_gemini, mock_ollama, mock_detect):
        mock_detect.return_value = "GEMINI"
        mock_gemini.return_value = ark.ArkValue("Gemini Response", "String")

        args = [ark.ArkValue("Hello", "String")]
        with patch.dict(os.environ, {"GOOGLE_API_KEY": "key"}):
            result = ark.ask_ai(args)

        mock_gemini.assert_called_once()
        mock_ollama.assert_not_called()
        mock_mock.assert_not_called()
        self.assertEqual(result.val, "Gemini Response")

    @patch('ark.detect_ai_mode')
    @patch('ark.ask_ollama')
    @patch('ark.ask_gemini')
    @patch('ark.ask_mock')
    def test_ask_ai_dispatch_mock(self, mock_mock, mock_gemini, mock_ollama, mock_detect):
        mock_detect.return_value = "MOCK"
        mock_mock.return_value = ark.ArkValue("Mock Response", "String")

        args = [ark.ArkValue("Hello", "String")]
        result = ark.ask_ai(args)

        mock_mock.assert_called_once()
        self.assertEqual(result.val, "Mock Response")

if __name__ == '__main__':
    unittest.main()
