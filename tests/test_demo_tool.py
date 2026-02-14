"""
Unit tests for src/tools/demo_tool.py
"""
import unittest
from src.tools.demo_tool import greet_user, reverse_text


class TestDemoTool(unittest.TestCase):
    """Test suite for the demo tool functions."""

    def test_greet_user_standard(self):
        """Test greet_user with a standard name."""
        name = "Alice"
        expected = "Hello, Alice! 🎉 Welcome to the Antigravity Agent with dynamic tool loading!"
        self.assertEqual(greet_user(name), expected)

    def test_greet_user_empty(self):
        """Test greet_user with an empty name."""
        name = ""
        expected = "Hello, ! 🎉 Welcome to the Antigravity Agent with dynamic tool loading!"
        self.assertEqual(greet_user(name), expected)

    def test_greet_user_special_chars(self):
        """Test greet_user with special characters."""
        name = "Bob@123"
        expected = "Hello, Bob@123! 🎉 Welcome to the Antigravity Agent with dynamic tool loading!"
        self.assertEqual(greet_user(name), expected)

    def test_reverse_text_standard(self):
        """Test reverse_text with a standard string."""
        text = "hello"
        expected = "olleh"
        self.assertEqual(reverse_text(text), expected)

    def test_reverse_text_empty(self):
        """Test reverse_text with an empty string."""
        text = ""
        expected = ""
        self.assertEqual(reverse_text(text), expected)

    def test_reverse_text_palindrome(self):
        """Test reverse_text with a palindrome."""
        text = "madam"
        expected = "madam"
        self.assertEqual(reverse_text(text), expected)

    def test_reverse_text_special_chars(self):
        """Test reverse_text with special characters and emojis."""
        text = "Hello 🌍"
        expected = "🌍 olleH"
        self.assertEqual(reverse_text(text), expected)


if __name__ == "__main__":
    unittest.main()
