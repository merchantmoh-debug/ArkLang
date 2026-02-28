import sys
import unittest
import asyncio
from unittest.mock import patch, MagicMock
import os

# Add src to path
sys.path.append(os.path.abspath("src"))


class TestDockerSandboxTruncation(unittest.TestCase):
    """Tests for DockerSandbox output truncation.
    
    Uses setUpClass/tearDownClass to properly scope sys.modules mocking
    instead of polluting at module level.
    """
    _saved_modules = {}
    DockerSandboxClass = None

    @classmethod
    def setUpClass(cls):
        """Save original modules and inject mocks for import."""
        cls._saved_modules = {}
        for mod_name in ["pydantic", "pydantic_settings", "src.config"]:
            cls._saved_modules[mod_name] = sys.modules.get(mod_name)

        sys.modules["pydantic"] = MagicMock()
        sys.modules["pydantic_settings"] = MagicMock()
        mock_config = MagicMock()
        mock_config.settings.SANDBOX_MAX_OUTPUT_KB = 10
        sys.modules["src.config"] = mock_config

        # Import after mocking
        from sandbox.docker_exec import DockerSandbox as DS
        cls.DockerSandboxClass = DS

    @classmethod
    def tearDownClass(cls):
        """Restore original modules to prevent pollution."""
        for mod_name, original in cls._saved_modules.items():
            if original is None:
                sys.modules.pop(mod_name, None)
            else:
                sys.modules[mod_name] = original

    def setUp(self):
        # Reset the cached client to ensure clean state for each test
        self.DockerSandboxClass._client = None
        self.sandbox = self.DockerSandboxClass()
        # Set max output to 1KB for testing
        self.env_patcher = patch.dict(os.environ, {"SANDBOX_MAX_OUTPUT_KB": "1"})
        self.env_patcher.start()

    def tearDown(self):
        self.env_patcher.stop()

    def test_truncation(self):
        # Mock docker module
        mock_docker = MagicMock()
        mock_client = MagicMock()
        mock_container = MagicMock()

        mock_docker.from_env.return_value = mock_client
        mock_client.containers.run.return_value = mock_container
        mock_container.wait.return_value = {"StatusCode": 0}

        # Create output larger than 1KB (1024 bytes)
        # 2048 bytes
        long_output = b"a" * 2048
        mock_container.logs.return_value = long_output

        # Mock sys.modules to inject mock_docker
        with patch.dict(sys.modules, {"docker": mock_docker}):
            # Ensure _docker_available passes
            mock_client.ping.return_value = True

            # DockerSandbox.execute is async, must use asyncio.run
            result = asyncio.run(self.sandbox.execute("print('large output')"))

            self.assertTrue(result.meta.get("truncated"))
            self.assertIn("... (output truncated)", result.stdout)
            self.assertLess(len(result.stdout), 2048)

    def test_no_truncation(self):
        # Mock docker module
        mock_docker = MagicMock()
        mock_client = MagicMock()
        mock_container = MagicMock()

        mock_docker.from_env.return_value = mock_client
        mock_client.containers.run.return_value = mock_container
        mock_container.wait.return_value = {"StatusCode": 0}

        # Create output smaller than 1KB
        short_output = b"a" * 100
        mock_container.logs.return_value = short_output

        with patch.dict(sys.modules, {"docker": mock_docker}):
            mock_client.ping.return_value = True

            result = asyncio.run(self.sandbox.execute("print('short output')"))

            self.assertFalse(result.meta.get("truncated"))
            self.assertNotIn("... (output truncated)", result.stdout)
            self.assertEqual(len(result.stdout), 100)

if __name__ == "__main__":
    unittest.main()
