import io
import os
import sys
import unittest
from unittest.mock import patch, MagicMock

class TestSandboxFactory(unittest.TestCase):
    def setUp(self):
        # 1. Patch environment variables (clean slate)
        self.env_patcher = patch.dict(os.environ, {}, clear=True)
        self.env_patcher.start()

        # 2. Patch sys.stderr
        self.stderr_patcher = patch('sys.stderr', new_callable=io.StringIO)
        self.mock_stderr = self.stderr_patcher.start()

        # 3. Setup Mock dependencies (pydantic)
        self.mock_pydantic = MagicMock()
        self.mock_pydantic_settings = MagicMock()

        def mock_field(default=None, default_factory=None, **kwargs):
            if default_factory:
                return default_factory()
            return default
        self.mock_pydantic.Field = mock_field

        class MockBaseSettings:
            def __init__(self, **kwargs):
                pass
        self.mock_pydantic_settings.BaseSettings = MockBaseSettings
        self.mock_pydantic_settings.SettingsConfigDict = MagicMock()

        # 4. Patch sys.modules to inject mocks AND clean up imported modules
        # This ensures isolation between tests and handles missing dependencies
        self.modules_patcher = patch.dict(sys.modules, {
            "pydantic": self.mock_pydantic,
            "pydantic_settings": self.mock_pydantic_settings
        })
        self.modules_patcher.start()

        # Remove cached modules to force re-import with our mocks
        modules_to_remove = [
            'src.config', 'config',
            'src.sandbox.local', 'sandbox.local',
            'src.sandbox.factory', 'sandbox.factory',
            'src.sandbox.docker_exec', 'sandbox.docker_exec',
            'src.sandbox.base', 'sandbox.base',
            'src.sandbox.e2b_exec', 'sandbox.e2b_exec'
        ]
        for m in modules_to_remove:
            if m in sys.modules:
                del sys.modules[m]

    def tearDown(self):
        self.modules_patcher.stop()
        self.stderr_patcher.stop()
        self.env_patcher.stop()

    def _import_sandbox(self):
        """Helper to import sandbox modules within the mocked environment."""
        # Use src.sandbox namespace
        try:
            from src.sandbox.factory import get_sandbox
            from src.sandbox.local import LocalSandbox
            from src.sandbox.docker_exec import DockerSandbox
            return get_sandbox, DockerSandbox, LocalSandbox
        except ImportError:
            # Fallback if run in a way where src is not implicitly a package
            if os.path.abspath("src") not in sys.path:
                 sys.path.insert(0, os.path.abspath("src"))
                 try:
                     from sandbox.factory import get_sandbox
                     from sandbox.local import LocalSandbox
                     from sandbox.docker_exec import DockerSandbox
                     return get_sandbox, DockerSandbox, LocalSandbox
                 finally:
                     sys.path.pop(0)
            raise

    def test_default_sandbox_is_docker(self):
        get_sandbox, DockerSandbox, _ = self._import_sandbox()
        # Mock Docker availability so the factory creates DockerSandbox
        # even when Docker is not installed on the host
        with patch.object(DockerSandbox, '_docker_available', return_value=(True, None)):
            sandbox = get_sandbox()
        self.assertIsInstance(sandbox, DockerSandbox)

    def test_explicit_docker_sandbox(self):
        os.environ["SANDBOX_TYPE"] = "docker"
        get_sandbox, DockerSandbox, _ = self._import_sandbox()
        sandbox = get_sandbox()
        self.assertIsInstance(sandbox, DockerSandbox)

    def test_docker_sandbox_import_error(self):
        os.environ["SANDBOX_TYPE"] = "docker"
        get_sandbox, _, _ = self._import_sandbox()

        # Force ImportError for docker_exec
        with patch.dict(sys.modules, {'src.sandbox.docker_exec': None, 'sandbox.docker_exec': None}):
            with self.assertRaises(RuntimeError) as cm:
                get_sandbox()
            self.assertIn("Docker sandbox requested but 'docker' package is not installed", str(cm.exception))

    def test_docker_sandbox_init_error(self):
        os.environ["SANDBOX_TYPE"] = "docker"
        get_sandbox, _, _ = self._import_sandbox()

        mock_module = MagicMock()
        mock_module.DockerSandbox.side_effect = Exception("Init failed")

        with patch.dict(sys.modules, {
            'src.sandbox.docker_exec': mock_module,
            'sandbox.docker_exec': mock_module
        }):
             with self.assertRaises(RuntimeError) as cm:
                 get_sandbox()
             self.assertIn("Failed to initialize Docker sandbox: Init failed", str(cm.exception))

    def test_local_sandbox(self):
        os.environ["SANDBOX_TYPE"] = "local"
        get_sandbox, _, LocalSandbox = self._import_sandbox()
        sandbox = get_sandbox()
        self.assertIsInstance(sandbox, LocalSandbox)

        self.mock_stderr.seek(0)
        output = self.mock_stderr.read()
        self.assertIn("WARNING: LocalSandbox is insecure", output)

    def test_e2b_sandbox_missing(self):
        os.environ["SANDBOX_TYPE"] = "e2b"
        get_sandbox, _, _ = self._import_sandbox()

        with patch.dict(sys.modules, {'src.sandbox.e2b_exec': None, 'sandbox.e2b_exec': None}):
             with self.assertRaises(RuntimeError) as cm:
                 get_sandbox()
             self.assertIn("E2B sandbox requested but 'e2b' package is not installed", str(cm.exception))

    def test_e2b_sandbox_init_error(self):
        os.environ["SANDBOX_TYPE"] = "e2b"
        get_sandbox, _, _ = self._import_sandbox()

        mock_module = MagicMock()
        mock_module.E2BSandbox.side_effect = Exception("E2B Init failed")

        with patch.dict(sys.modules, {
            'src.sandbox.e2b_exec': mock_module,
            'sandbox.e2b_exec': mock_module
        }):
            with self.assertRaises(RuntimeError) as cm:
                get_sandbox()
            self.assertIn("Failed to initialize E2B sandbox: E2B Init failed", str(cm.exception))

    def test_invalid_sandbox_type(self):
        os.environ["SANDBOX_TYPE"] = "invalid_type"
        get_sandbox, _, _ = self._import_sandbox()
        with self.assertRaises(ValueError) as cm:
            get_sandbox()
        self.assertIn("Unknown sandbox type: invalid_type", str(cm.exception))

if __name__ == "__main__":
    unittest.main()
