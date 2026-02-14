import sys
import unittest
from unittest.mock import patch, MagicMock
import os

# Mock dependencies before imports to handle missing packages in restricted environments
mock_config = MagicMock()
mock_config.settings.BANNED_IMPORTS = set()
mock_config.settings.BANNED_FUNCTIONS = set()
mock_config.settings.BANNED_ATTRIBUTES = set()

sys.modules["src.config"] = mock_config
sys.modules["pydantic"] = MagicMock()
sys.modules["pydantic_settings"] = MagicMock()

# Add root to path
sys.path.append(os.path.abspath("."))

from src.sandbox.docker_exec import DockerSandbox, DEFAULT_DOCKER_IMAGE, ALLOWED_DOCKER_IMAGES

class TestDockerSecurity(unittest.TestCase):
    def setUp(self):
        self.sandbox = DockerSandbox()

    def test_invalid_image_fallback(self):
        # Mock docker module
        mock_docker = MagicMock()
        mock_client = MagicMock()
        mock_container = MagicMock()

        mock_docker.from_env.return_value = mock_client
        mock_client.containers.run.return_value = mock_container
        mock_container.wait.return_value = {"StatusCode": 0}
        mock_container.logs.return_value = b"success"

        # Vulnerable scenario: user tries to set a malicious image
        malicious_image = "malicious-image:latest"

        with patch.dict(os.environ, {"DOCKER_IMAGE": malicious_image}):
            with patch.dict(sys.modules, {"docker": mock_docker}):
                mock_client.ping.return_value = True

                self.sandbox.execute("print('hello')")

                # Verify that the default image was used instead of the malicious one
                mock_client.containers.run.assert_called()
                args, kwargs = mock_client.containers.run.call_args
                self.assertEqual(kwargs['image'], DEFAULT_DOCKER_IMAGE)
                self.assertNotIn(malicious_image, kwargs['image'])

    def test_allowed_image_usage(self):
        # Mock docker module
        mock_docker = MagicMock()
        mock_client = MagicMock()
        mock_container = MagicMock()

        mock_docker.from_env.return_value = mock_client
        mock_client.containers.run.return_value = mock_container
        mock_container.wait.return_value = {"StatusCode": 0}
        mock_container.logs.return_value = b"success"

        # Scenario: user sets an allowed image
        # Pick one from ALLOWED_DOCKER_IMAGES that is NOT the default
        allowed_image = [img for img in ALLOWED_DOCKER_IMAGES if img != DEFAULT_DOCKER_IMAGE][0]

        with patch.dict(os.environ, {"DOCKER_IMAGE": allowed_image}):
            with patch.dict(sys.modules, {"docker": mock_docker}):
                mock_client.ping.return_value = True

                self.sandbox.execute("print('hello')")

                # Verify that the allowed image was used
                mock_client.containers.run.assert_called()
                args, kwargs = mock_client.containers.run.call_args
                self.assertEqual(kwargs['image'], allowed_image)

if __name__ == "__main__":
    unittest.main()
