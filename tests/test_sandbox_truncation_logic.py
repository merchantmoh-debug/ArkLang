import unittest
import sys
import os

# Ensure src is in path
sys.path.append(os.path.abspath("."))

from src.sandbox.base import truncate_output

class TestTruncateOutput(unittest.TestCase):
    def test_no_truncation_needed(self):
        """Test that short text is not truncated."""
        text = "Short text"
        result, truncated = truncate_output(text, 100)
        self.assertEqual(result, text)
        self.assertFalse(truncated)

    def test_truncation_needed(self):
        """Test that long text is truncated correctly."""
        text = "This is a long text that needs truncation"
        trailer = "\n... (output truncated)"

        # We need max_bytes < len(text) to trigger truncation.
        # len(text) is approx 41.
        # Let's set max_bytes = 40.
        # limit = max(0, 40 - 32) = 8 bytes.
        # If len(text) > 40, we truncate to 8 bytes + trailer.
        # We need text to be > 40 bytes.
        text = "This is a long text that is definitely longer than 40 bytes."

        max_bytes = 40
        # limit = 8 bytes. "This is "
        expected_prefix = text[:8]

        result, truncated = truncate_output(text, max_bytes)

        self.assertTrue(truncated)
        self.assertEqual(result, expected_prefix + trailer)

    def test_exact_limit(self):
        """Test that text exactly at the limit is not truncated."""
        text = "12345"
        result, truncated = truncate_output(text, 5)
        self.assertEqual(result, text)
        self.assertFalse(truncated)

    def test_unicode_truncation(self):
        """Test that multi-byte characters are handled correctly during truncation."""
        # We want to force truncation such that the cut happens in the middle of a multi-byte char.
        # We want limit = 8 bytes.
        # So max_bytes = 40.
        # We need total text length > 40.

        prefix = "Price: €" # 7 bytes + 3 bytes = 10 bytes.
        padding = "a" * 100
        text = prefix + padding

        max_bytes = 40
        # limit = 8.
        # encoded[:8] includes "Price: " (7 bytes) + 1 byte of euro.
        # decoding drops the partial byte.
        # Result: "Price: " + trailer.

        result, truncated = truncate_output(text, max_bytes)

        trailer = "\n... (output truncated)"
        self.assertEqual(result, "Price: " + trailer)
        self.assertTrue(truncated)

    def test_small_limit(self):
        """Test behavior when max_bytes is smaller than the trailer reservation."""
        # We need text longer than max_bytes to trigger truncation.
        text = "This is longer than 10 chars"
        max_bytes = 10
        # limit = max(0, 10 - 32) = 0.
        # Result: "" + trailer.

        result, truncated = truncate_output(text, max_bytes)

        trailer = "\n... (output truncated)"
        self.assertEqual(result, trailer)
        self.assertTrue(truncated)

    def test_zero_or_negative_limit(self):
        """Test that 0 or negative max_bytes disables truncation."""
        text = "Any text" * 100 # Long text

        # 0 limit -> unlimited
        result, truncated = truncate_output(text, 0)
        self.assertEqual(result, text)
        self.assertFalse(truncated)

        # Negative limit -> unlimited
        result, truncated = truncate_output(text, -1)
        self.assertEqual(result, text)
        self.assertFalse(truncated)

if __name__ == "__main__":
    unittest.main()
