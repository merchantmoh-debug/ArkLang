import os
import json
import shutil
import unittest
from pathlib import Path
from src.memory import MemoryManager
from src.config import settings

# Force config to use a temp file for testing
settings.MEMORY_FILE = "test_memory.enc"

class TestMemorySecurity(unittest.TestCase):
    def setUp(self):
        # Clean up any existing test files
        self.cleanup()

    def tearDown(self):
        self.cleanup()

    def cleanup(self):
        files = [
            "test_memory.enc",
            "test_memory.json",
            "test_memory.json.bak",
            "agent_memory.json",
            "agent_memory.json.bak",
            ".memory_key"
        ]
        for f in files:
            if os.path.exists(f):
                os.remove(f)

    def test_key_generation(self):
        """Verify that a key file is generated if missing."""
        self.assertFalse(os.path.exists(".memory_key"))
        mem = MemoryManager(memory_file="test_memory.enc")
        self.assertTrue(os.path.exists(".memory_key"))
        with open(".memory_key", "rb") as f:
            key = f.read()
        self.assertEqual(len(key), 44)  # Fernet keys are 32 bytes base64 encoded -> 44 chars

    def test_encryption_storage(self):
        """Verify that data is stored encrypted."""
        mem = MemoryManager(memory_file="test_memory.enc")
        mem.add_entry("user", "This is a secret message.")

        self.assertTrue(os.path.exists("test_memory.enc"))
        with open("test_memory.enc", "rb") as f:
            content = f.read()

        # Should not be valid JSON
        with self.assertRaises(json.JSONDecodeError):
            json.loads(content)

        # Should contain binary garbage (high entropy), not plaintext
        self.assertNotIn(b"secret message", content)

    def test_decryption_retrieval(self):
        """Verify that data can be retrieved correctly."""
        mem = MemoryManager(memory_file="test_memory.enc")
        mem.add_entry("user", "Data persistence check.")

        # Re-initialize memory manager (simulate restart)
        mem2 = MemoryManager(memory_file="test_memory.enc")
        history = mem2.get_history()

        self.assertEqual(len(history), 1)
        self.assertEqual(history[0]["content"], "Data persistence check.")

    def test_migration_from_legacy(self):
        """Verify migration from plaintext JSON to encrypted file."""
        legacy_file = "agent_memory.json"

        # Create a fake legacy memory file
        legacy_data = {
            "summary": "Old summary",
            "history": [{"role": "user", "content": "I am from the past."}]
        }
        with open(legacy_file, "w") as f:
            json.dump(legacy_data, f)

        # Initialize with the new encrypted target file
        # The migration logic in memory.py specifically looks for 'agent_memory.json'
        # So we must rely on that hardcoded check or the fact that we use default settings in the app
        # In our test setup, we pass 'test_memory.enc', but the code checks for 'agent_memory.json' explicitly as legacy source

        mem = MemoryManager(memory_file="test_memory.enc")

        # Check if migration happened
        self.assertTrue(os.path.exists("test_memory.enc"))
        self.assertTrue(os.path.exists(legacy_file + ".bak"))
        self.assertFalse(os.path.exists(legacy_file)) # Original should be moved

        # Check content
        history = mem.get_history()
        self.assertEqual(len(history), 1)
        self.assertEqual(history[0]["content"], "I am from the past.")
        self.assertEqual(mem.summary, "Old summary")

        # Check if the new file is actually encrypted
        with open("test_memory.enc", "rb") as f:
            content = f.read()
        self.assertNotIn(b"I am from the past", content)

if __name__ == "__main__":
    unittest.main()
