import sys
import unittest
import json
import os
from unittest.mock import MagicMock, patch

# Handle missing dependencies for test environment
try:
    import pydantic
except ImportError:
    sys.modules["pydantic"] = MagicMock()

try:
    import pydantic_settings
except ImportError:
    sys.modules["pydantic_settings"] = MagicMock()

try:
    import cryptography
except ImportError:
    # Mock cryptography if missing
    sys.modules["cryptography"] = MagicMock()
    sys.modules["cryptography.fernet"] = MagicMock()
    mock_fernet_cls = MagicMock()
    mock_fernet_inst = MagicMock()
    # Simulate decryption failure for fallback testing
    mock_fernet_inst.decrypt.side_effect = Exception("Decryption failed")
    mock_fernet_inst.encrypt.return_value = b'encrypted_data'
    mock_fernet_cls.return_value = mock_fernet_inst
    sys.modules["cryptography.fernet"].Fernet = mock_fernet_cls

# Import module under test
# Note: src.config will be imported by src.memory, so pydantic mock must trigger before
from src.memory import MemoryManager

class TestMemoryFallback(unittest.TestCase):
    def setUp(self):
        self.test_file = "test_memory_fallback.json"
        self.initial_data = {
            "summary": "Previous summary",
            "history": [{"role": "user", "content": "hello"}]
        }
        with open(self.test_file, "w") as f:
            json.dump(self.initial_data, f)

        # Patch settings.MEMORY_FILE
        # We patch it on the module where it's used (src.memory) or source (src.config)
        # src.memory imports settings from src.config
        self.settings_patcher = patch("src.config.settings.MEMORY_FILE", self.test_file)
        self.settings_patcher.start()

    def tearDown(self):
        self.settings_patcher.stop()
        if os.path.exists(self.test_file):
            os.remove(self.test_file)
        if os.path.exists(self.test_file + ".tmp"):
             os.remove(self.test_file + ".tmp")

    def test_plaintext_fallback_save_logic(self):
        """
        Verify that when a plaintext memory file is encountered (decryption fails),
        the data is loaded and immediately saved as encrypted WITHOUT redundant processing.
        """
        # Patch _save_memory_task to verify it's called with correct data
        with patch.object(MemoryManager, "_save_memory_task") as mock_save:
             # We need to ensure encryption init runs.
             # If cryptography is mocked, Fernet() returns our mock.
             # If real, it tries to read key.
             # We might need to mock environment for key to avoid key generation message?
             with patch.dict(os.environ, {"MEMORY_ENCRYPTION_KEY": "test_key_12345"}):
                 mm = MemoryManager(memory_file=self.test_file)

             # Wait for async task if needed
             if mm._last_save_future:
                 mm._last_save_future.result()

             # Verify save was triggered
             self.assertTrue(mock_save.called, "Save task should have been executed on fallback")

             # Verify arguments passed to the save task
             # The signature is (summary, history)
             args, _ = mock_save.call_args
             saved_summary = args[0]
             saved_history = args[1]

             self.assertEqual(saved_summary, "Previous summary")
             self.assertEqual(saved_history, [{"role": "user", "content": "hello"}])

             # Verify in-memory state is also correct
             self.assertEqual(mm.summary, "Previous summary")
             self.assertEqual(len(mm.get_history()), 1)

if __name__ == "__main__":
    unittest.main()
