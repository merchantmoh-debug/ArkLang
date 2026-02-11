import unittest
from unittest.mock import patch
from src.memory import MemoryManager
import os

class TestSecurityNegative(unittest.TestCase):
    def setUp(self):
        # Clean up any test files
        if os.path.exists("test_fail_closed.enc"):
            os.remove("test_fail_closed.enc")
        if os.path.exists(".memory_key"):
            # Don't delete .memory_key if it's real, but here we run in isolation hopefully
            # Actually, let's backup .memory_key if needed, or use a temp dir.
            # For simplicity in this env, we'll just be careful.
            pass

    def test_fail_closed_on_init_error(self):
        """
        Ensure MemoryManager fails closed (raises exception) if encryption init fails.
        Currently, the code catches the exception and proceeds with plaintext (Fail Open).
        This test expects a ValueError, so it should FAIL on the current codebase.
        """
        # We mock Fernet to raise an error when initialized
        with patch('src.memory.Fernet') as MockFernet:
            MockFernet.side_effect = ValueError("Simulated Encryption Failure")

            # We expect the constructor to propagate the error, NOT suppress it.
            with self.assertRaises(ValueError):
                MemoryManager(memory_file="test_fail_closed.enc")

    def test_no_plaintext_fallback(self):
        """
        Ensure save_memory does NOT fallback to plaintext if encryption is missing.
        """
        # To test this, we need a MemoryManager instance where _fernet is None.
        # Since __init__ calls _init_encryption, we can mock _init_encryption to fail silently
        # (simulating the current behavior) and then verify save_memory fails.

        # Create instance but bypass __init__ logic or mock it
        with patch.object(MemoryManager, '_init_encryption', return_value=None):
            mm = MemoryManager(memory_file="test_fail_closed.enc")
            # Manually ensure _fernet is None (as per current broken logic)
            mm._fernet = None
            mm.summary = "Test"
            mm._memory = []

            # Attempt to save.
            # Current behavior: Writes plaintext (Fail Open).
            # Desired behavior: Raises RuntimeError (Fail Closed).

            try:
                mm.save_memory()
                # If we get here, it saved (Fail Open).
                # We check if file exists and is plaintext.
                if os.path.exists("test_fail_closed.enc"):
                    with open("test_fail_closed.enc", 'rb') as f:
                        content = f.read()
                        if b'"summary": "Test"' in content:
                             self.fail("Security Vulnerability: Saved memory in plaintext!")
            except RuntimeError:
                # This is what we want!
                pass
            except Exception as e:
                # Any other error is also better than plaintext
                print(f"Caught expected error: {e}")

if __name__ == '__main__':
    unittest.main()
