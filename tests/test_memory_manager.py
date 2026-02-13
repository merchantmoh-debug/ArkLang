import unittest
import os
import json
import shutil
import tempfile
from pathlib import Path
from unittest.mock import patch
from cryptography.fernet import Fernet
from src.memory import MemoryManager

class TestMemoryManager(unittest.TestCase):
    def setUp(self):
        # Create a temporary directory for each test
        self.test_dir = tempfile.mkdtemp()
        self.old_cwd = os.getcwd()
        os.chdir(self.test_dir)

        # Ensure environment variable doesn't leak into tests
        self.old_env_key = os.environ.get("MEMORY_ENCRYPTION_KEY")
        if "MEMORY_ENCRYPTION_KEY" in os.environ:
            del os.environ["MEMORY_ENCRYPTION_KEY"]

        self.memory_file = "test_memory.enc"

    def tearDown(self):
        # Restore environment
        if self.old_env_key:
            os.environ["MEMORY_ENCRYPTION_KEY"] = self.old_env_key
        elif "MEMORY_ENCRYPTION_KEY" in os.environ:
             del os.environ["MEMORY_ENCRYPTION_KEY"]

        # Restore working directory and cleanup
        os.chdir(self.old_cwd)
        shutil.rmtree(self.test_dir)

    def test_initialization_creates_key(self):
        """Test that initialization creates a .memory_key file if missing."""
        key_file = Path(".memory_key")
        self.assertFalse(key_file.exists())

        manager = MemoryManager(memory_file=self.memory_file)

        self.assertTrue(key_file.exists())
        self.assertIsNotNone(manager._fernet)
        self.assertEqual(manager.get_history(), [])

    def test_persistence_and_encryption(self):
        """Test that data is saved encrypted and can be reloaded."""
        manager = MemoryManager(memory_file=self.memory_file)
        manager.add_entry("user", "Hello, world!")
        manager.save_memory()

        # Verify file exists
        self.assertTrue(os.path.exists(self.memory_file))

        # Verify content is not plain JSON
        with open(self.memory_file, "rb") as f:
            content = f.read()
            # Try to parse as JSON directly (should fail if encrypted)
            with self.assertRaises(json.JSONDecodeError):
                json.loads(content.decode("utf-8"))

            # Should be a Fernet token
            # Fernet tokens start with base64url encoded version + timestamp
            # But simpler check is just that we can decrypt it with the manager's key
            manager._fernet.decrypt(content)

        # Reload in new instance
        new_manager = MemoryManager(memory_file=self.memory_file)
        history = new_manager.get_history()
        self.assertEqual(len(history), 1)
        self.assertEqual(history[0]["content"], "Hello, world!")

    def test_legacy_migration(self):
        """Test migration from plain JSON 'agent_memory.json' to encrypted file."""
        legacy_file = "agent_memory.json"
        legacy_data = {
            "summary": "Old summary",
            "history": [{"role": "user", "content": "Legacy content"}]
        }

        with open(legacy_file, "w") as f:
            json.dump(legacy_data, f)

        # Initialize manager - should trigger migration
        # Note: Manager defaults to checking "agent_memory.json" if main file missing
        # We need to make sure we use a filename that triggers this logic if passed explicitly?
        # The logic is: `if not os.path.exists(self.memory_file) and os.path.exists(legacy_file):`
        # So passing any new filename works as long as it doesn't exist yet.
        manager = MemoryManager(memory_file=self.memory_file)

        # Check migration results
        self.assertTrue(os.path.exists(legacy_file + ".bak"))
        self.assertTrue(os.path.exists(self.memory_file))

        # Verify loaded data
        self.assertEqual(manager.summary, "Old summary")
        self.assertEqual(len(manager.get_history()), 1)
        self.assertEqual(manager.get_history()[0]["content"], "Legacy content")

    def test_context_window_summarization(self):
        """Test context window management and summarization logic."""
        manager = MemoryManager(memory_file=self.memory_file)

        # Add 5 messages
        for i in range(5):
            manager.add_entry("user", f"Message {i}")

        # Request context window with max_messages=2
        # Should summarize the first 3 messages
        context = manager.get_context_window(
            system_prompt="System Prompt",
            max_messages=2
        )

        # Expected: System Prompt, Summary Message, Message 3, Message 4
        self.assertEqual(len(context), 4)
        self.assertEqual(context[0]["content"], "System Prompt")
        self.assertTrue("Previous Summary" in context[1]["content"])
        self.assertEqual(context[2]["content"], "Message 3")
        self.assertEqual(context[3]["content"], "Message 4")

        # Verify summary was updated in manager
        self.assertNotEqual(manager.summary, "")
        self.assertIn("Message 0", manager.summary)
        self.assertIn("Message 1", manager.summary)
        self.assertIn("Message 2", manager.summary)

    def test_corrupt_memory_file(self):
        """Test handling of a corrupt memory file."""
        # Write garbage to the file
        with open(self.memory_file, "wb") as f:
            f.write(b"NOT_A_VALID_FERNET_TOKEN_OR_JSON")

        # Should initialize with empty memory and log warning (no crash)
        manager = MemoryManager(memory_file=self.memory_file)
        self.assertEqual(manager.get_history(), [])

    def test_clear_memory(self):
        """Test clearing memory."""
        manager = MemoryManager(memory_file=self.memory_file)
        manager.add_entry("user", "test")
        manager.summary = "summary"
        manager.save_memory()

        manager.clear_memory()

        self.assertEqual(manager.get_history(), [])
        self.assertEqual(manager.summary, "")

        # Verify persistence of clear
        new_manager = MemoryManager(memory_file=self.memory_file)
        self.assertEqual(new_manager.get_history(), [])

    def test_legacy_migration_corrupt_file(self):
        """Test that a corrupt legacy file is not migrated and remains untouched."""
        legacy_file = "agent_memory.json"

        # Ensure target memory file does not exist, so migration logic triggers
        if os.path.exists(self.memory_file):
            os.remove(self.memory_file)

        with open(legacy_file, "w") as f:
            f.write("INVALID_JSON")

        try:
            # Initialize manager - should attempt migration but fail gracefully
            manager = MemoryManager(memory_file=self.memory_file)

            # Check migration failure
            self.assertTrue(os.path.exists(legacy_file))  # Should still exist
            self.assertFalse(os.path.exists(legacy_file + ".bak"))  # Should not be renamed

            # Memory should be empty (since load failed)
            self.assertEqual(manager.get_history(), [])
        finally:
            # Explicit cleanup
            if os.path.exists(legacy_file):
                os.remove(legacy_file)

    def test_plaintext_memory_file_fallback(self):
        """Test fallback to loading plaintext memory file if encryption fails."""
        # Create a plaintext memory file (simulating older version or key loss)
        plaintext_data = {
            "summary": "Plaintext Summary",
            "history": [{"role": "system", "content": "Plaintext Content"}]
        }
        with open(self.memory_file, "w") as f:
            json.dump(plaintext_data, f)

        # Initialize manager - should detect plaintext and load it
        manager = MemoryManager(memory_file=self.memory_file)

        # Check loaded data
        self.assertEqual(manager.summary, "Plaintext Summary")
        self.assertEqual(len(manager.get_history()), 1)
        self.assertEqual(manager.get_history()[0]["content"], "Plaintext Content")

        # Verify file is now encrypted
        with open(self.memory_file, "rb") as f:
            content = f.read()
            # Should fail to decode as utf-8 json directly
            try:
                json.loads(content.decode("utf-8"))
                self.fail("File was not encrypted after load!")
            except json.JSONDecodeError:
                pass  # Good, likely encrypted
            except UnicodeDecodeError:
                pass  # Good, encrypted binary data

            # Should be decryptable
            manager._fernet.decrypt(content)

if __name__ == "__main__":
    unittest.main()
