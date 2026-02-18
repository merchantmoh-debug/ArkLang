import unittest
import os
import builtins
from unittest.mock import patch, mock_open, MagicMock
from src.memory import MemoryManager

class TestMemoryEncryption(unittest.TestCase):
    def setUp(self):
        # We need to mock settings to avoid side effects during init
        self.settings_patcher = patch('src.memory.settings')
        self.mock_settings = self.settings_patcher.start()
        self.mock_settings.MEMORY_FILE = "test_memory.json"

        # Patch _load_memory to isolate _init_encryption testing
        self.load_memory_patcher = patch('src.memory.MemoryManager._load_memory')
        self.mock_load_memory = self.load_memory_patcher.start()

    def tearDown(self):
        self.load_memory_patcher.stop()
        self.settings_patcher.stop()

    @patch('src.memory.os.environ.get')
    @patch('src.memory.Path.exists')
    @patch('src.memory.Fernet')
    def test_init_encryption_with_env_var(self, mock_fernet_cls, mock_exists, mock_env_get):
        """Test initialization using MEMORY_ENCRYPTION_KEY environment variable."""
        # Setup
        key_bytes = b"env-var-key"
        mock_env_get.return_value = key_bytes.decode() # Environment vars are strings
        mock_exists.return_value = False

        # Execute
        manager = MemoryManager()

        # Verify
        self.assertIsNotNone(manager._fernet)
        mock_fernet_cls.assert_called_with(key_bytes)

    @patch('src.memory.os.environ.get')
    @patch('src.memory.Path.exists')
    @patch('builtins.open', new_callable=mock_open)
    @patch('src.memory.Fernet')
    def test_init_encryption_with_file(self, mock_fernet_cls, mock_file, mock_exists, mock_env_get):
        """Test initialization reading key from .memory_key file."""
        # Setup
        key_bytes = b"file-key"
        mock_env_get.return_value = None
        mock_exists.return_value = True
        mock_file.return_value.read.return_value = key_bytes

        # Execute
        manager = MemoryManager()

        # Verify
        mock_file.assert_called_with(unittest.mock.ANY, "rb")
        mock_fernet_cls.assert_called_with(key_bytes)

    @patch('src.memory.os.environ.get')
    @patch('src.memory.Path.exists')
    @patch('builtins.open', side_effect=OSError("Permission denied"))
    def test_init_encryption_file_read_error(self, mock_file, mock_exists, mock_env_get):
        """Test RuntimeError when reading .memory_key fails."""
        # Setup
        mock_env_get.return_value = None
        mock_exists.return_value = True

        # Execute & Verify
        with self.assertRaises(RuntimeError) as cm:
            MemoryManager()
        self.assertIn("Could not read memory key", str(cm.exception))

    @patch('src.memory.os.environ.get')
    @patch('src.memory.Path.exists')
    @patch('src.memory.Fernet')
    @patch('src.memory.os.open')
    @patch('src.memory.os.fdopen')
    @patch('src.memory.os.name', 'posix')
    def test_init_encryption_generate_new_key_posix(self, mock_fdopen, mock_os_open, mock_fernet_cls, mock_exists, mock_env_get):
        """Test generating new key on POSIX system (secure permissions)."""
        # Setup
        key_bytes = b"generated-key"
        mock_fernet_cls.generate_key.return_value = key_bytes

        mock_env_get.return_value = None
        mock_exists.return_value = False

        # Mock file descriptor
        fd = 123
        mock_os_open.return_value = fd

        # Mock fdopen context manager
        mock_file = MagicMock()
        mock_fdopen.return_value.__enter__.return_value = mock_file

        # Execute
        manager = MemoryManager()

        # Verify
        mock_os_open.assert_called_with(unittest.mock.ANY, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
        mock_fdopen.assert_called_with(fd, "wb")
        mock_file.write.assert_called_with(key_bytes)
        mock_fernet_cls.assert_called_with(key_bytes)

    @patch('src.memory.os.environ.get')
    @patch('src.memory.Path.exists')
    @patch('src.memory.Fernet')
    @patch('builtins.open', new_callable=mock_open)
    @patch('src.memory.os.name', 'nt')
    def test_init_encryption_generate_new_key_non_posix(self, mock_file, mock_fernet_cls, mock_exists, mock_env_get):
        """Test generating new key on non-POSIX system (standard open)."""
        # Setup
        key_bytes = b"generated-key"
        mock_fernet_cls.generate_key.return_value = key_bytes

        mock_env_get.return_value = None
        mock_exists.return_value = False

        # Execute
        manager = MemoryManager()

        # Verify
        mock_file.assert_called_with(unittest.mock.ANY, "wb")
        mock_file().write.assert_called_with(key_bytes)
        mock_fernet_cls.assert_called_with(key_bytes)

    @patch('src.memory.os.environ.get')
    @patch('src.memory.Path.exists')
    @patch('src.memory.Fernet')
    @patch('src.memory.os.open', side_effect=OSError("Disk full"))
    @patch('src.memory.os.name', 'posix')
    def test_init_encryption_save_key_failure(self, mock_os_open, mock_fernet_cls, mock_exists, mock_env_get):
        """Test warning logged but execution continues if saving key fails."""
        # Setup
        key_bytes = b"generated-key"
        mock_fernet_cls.generate_key.return_value = key_bytes

        mock_env_get.return_value = None
        mock_exists.return_value = False

        # Execute - Should NOT raise exception
        manager = MemoryManager()

        # Verify
        self.assertIsNotNone(manager._fernet)
        mock_fernet_cls.assert_called_with(key_bytes)

    @patch('src.memory.os.environ.get')
    @patch('src.memory.Fernet')
    def test_init_encryption_fernet_failure(self, mock_fernet_cls, mock_env_get):
        """Test ValueError when Fernet initialization fails."""
        # Setup
        mock_env_get.return_value = "invalid-key"
        # The constructor raises exception
        mock_fernet_cls.side_effect = Exception("Invalid key")

        # Execute & Verify
        with self.assertRaises(ValueError) as cm:
            MemoryManager()
        self.assertIn("Error initializing encryption", str(cm.exception))

if __name__ == '__main__':
    unittest.main()
