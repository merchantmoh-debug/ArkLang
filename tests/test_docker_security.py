import sys
import unittest
import asyncio
from unittest.mock import patch, MagicMock
import os

# Add root to path
sys.path.append(os.path.abspath("."))


class TestDockerSecurity(unittest.TestCase):
    """Tests for DockerSandbox image security.
    
    Uses setUpClass/tearDownClass for sys.modules isolation.
    """
    _saved_modules = {}
    DockerSandboxClass = None
    DEFAULT_DOCKER_IMAGE_VAL = None
    ALLOWED_DOCKER_IMAGES_VAL = None

    @classmethod
    def setUpClass(cls):
        """Save original modules and inject mocks for import."""
        cls._saved_modules = {}
        for mod_name in ["pydantic", "pydantic_settings", "src.config"]:
            cls._saved_modules[mod_name] = sys.modules.get(mod_name)

        mock_config = MagicMock()
        mock_config.settings.BANNED_IMPORTS = set()
        mock_config.settings.BANNED_FUNCTIONS = set()
        mock_config.settings.BANNED_ATTRIBUTES = set()
        sys.modules["src.config"] = mock_config
        sys.modules["pydantic"] = MagicMock()
        sys.modules["pydantic_settings"] = MagicMock()

        from src.sandbox.docker_exec import (
            DockerSandbox as DS,
            DEFAULT_DOCKER_IMAGE as DDI,
            ALLOWED_DOCKER_IMAGES as ADI,
        )
        cls.DockerSandboxClass = DS
        cls.DEFAULT_DOCKER_IMAGE_VAL = DDI
        cls.ALLOWED_DOCKER_IMAGES_VAL = ADI

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

                # DockerSandbox.execute is async
                asyncio.run(self.sandbox.execute("print('hello')"))

                # Verify that the default image was used instead of the malicious one
                mock_client.containers.run.assert_called()
                args, kwargs = mock_client.containers.run.call_args
                actual_image = args[0] if args else kwargs.get('image', '')
                self.assertEqual(actual_image, self.DEFAULT_DOCKER_IMAGE_VAL)
                self.assertNotEqual(actual_image, malicious_image)

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
        allowed_image = [img for img in self.ALLOWED_DOCKER_IMAGES_VAL if img != self.DEFAULT_DOCKER_IMAGE_VAL][0]

        with patch.dict(os.environ, {"DOCKER_IMAGE": allowed_image}):
            with patch.dict(sys.modules, {"docker": mock_docker}):
                mock_client.ping.return_value = True

                # DockerSandbox.execute is async
                asyncio.run(self.sandbox.execute("print('hello')"))

                # Verify that the allowed image was used
                mock_client.containers.run.assert_called()
                args, kwargs = mock_client.containers.run.call_args
                actual_image = args[0] if args else kwargs.get('image', '')
                self.assertEqual(actual_image, allowed_image)

if __name__ == "__main__":
    unittest.main()
