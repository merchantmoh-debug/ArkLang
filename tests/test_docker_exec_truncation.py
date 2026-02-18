import sys
import unittest
from unittest.mock import patch, MagicMock
import os

# Mock pydantic and settings before imports to handle missing packages in restricted environments
sys.modules["pydantic"] = MagicMock()
sys.modules["pydantic_settings"] = MagicMock()
mock_config = MagicMock()
mock_config.settings.SANDBOX_MAX_OUTPUT_KB = 10
sys.modules["src.config"] = mock_config

# Add src to path
sys.path.append(os.path.abspath("src"))

from sandbox.docker_exec import DockerSandbox

class TestDockerSandboxTruncation(unittest.TestCase):
    def setUp(self):
        # Reset the cached client to ensure clean state for each test
        DockerSandbox._client = None

        self.sandbox = DockerSandbox()
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

            result = self.sandbox.execute("print('large output')")

            self.assertTrue(result.meta.get("truncated"))
            self.assertIn("... (output truncated)", result.stdout)
            self.assertLess(len(result.stdout), 2048)
            self.assertEqual(result.meta["resource_limits"]["max_output_kb"], 1)

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

            result = self.sandbox.execute("print('short output')")

            self.assertFalse(result.meta.get("truncated"))
            self.assertNotIn("... (output truncated)", result.stdout)
            self.assertEqual(len(result.stdout), 100)

if __name__ == "__main__":
    unittest.main()
