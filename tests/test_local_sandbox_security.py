import unittest
import os
import sys
from unittest.mock import patch

# Ensure src is in path
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from src.sandbox.local import LocalSandbox

class TestLocalSandboxSecurity(unittest.TestCase):
    def setUp(self):
        # Nuclear Option: Force reload src.config to clear mock pollution
        import importlib
        if 'src.config' in sys.modules:
            import src.config
            importlib.reload(src.config)
        else:
            # Module not cached yet — just import it fresh
            import src.config
        
        # Reload LocalSandbox to pick up fresh settings
        if 'src.sandbox.local' in sys.modules:
            import src.sandbox.local
            importlib.reload(src.sandbox.local)
            
        # Re-import LocalSandbox class after reload
        from src.sandbox.local import LocalSandbox
        self.sandbox = LocalSandbox()

        # Ensure we start with secure default
        self.env_patcher = patch.dict(os.environ, {"ALLOW_DANGEROUS_LOCAL_EXECUTION": "false"})
        self.env_patcher.start()

    def tearDown(self):
        self.env_patcher.stop()

    def test_blocked_import_os(self):
        result = self.sandbox.execute("import os\nprint(os.getcwd())")
        self.assertIn("Security Violation", result.stderr)
        self.assertIn("Import of 'os' is forbidden", result.stderr)
        self.assertNotEqual(result.exit_code, 0)

    def test_blocked_import_subprocess(self):
        result = self.sandbox.execute("import subprocess")
        self.assertIn("Security Violation", result.stderr)
        self.assertIn("Import of 'subprocess' is forbidden", result.stderr)

    def test_blocked_function_open(self):
        result = self.sandbox.execute("f = open('test.txt', 'w')")
        self.assertIn("Security Violation", result.stderr)
        self.assertIn("Call to 'open()' is forbidden", result.stderr)

    def test_blocked_function_alias(self):
        # Even if we alias it, the name reference should be caught
        result = self.sandbox.execute("my_open = open\nf = my_open('test.txt', 'w')")
        self.assertIn("Security Violation", result.stderr)
        # It might catch the Call or the Name reference first, both are valid failures
        self.assertTrue("Call to 'open()' is forbidden" in result.stderr or
                        "Reference to banned name 'open' is forbidden" in result.stderr)

    def test_blocked_attribute_subclasses(self):
        result = self.sandbox.execute("print([].__class__.__base__.__subclasses__())")
        self.assertIn("Security Violation", result.stderr)
        self.assertIn("Access to attribute '__subclasses__' is forbidden", result.stderr)

    def test_allowed_code(self):
        result = self.sandbox.execute("import math\nprint(f'Pi is {math.pi}')")
        self.assertEqual(result.exit_code, 0)
        self.assertIn("Pi is 3.14", result.stdout)

    def test_syntax_error(self):
        result = self.sandbox.execute("print('unterminated string")
        self.assertIn("Syntax Error", result.stderr)

    def test_bypass_flag_allows_os(self):
        with patch.dict(os.environ, {"ALLOW_DANGEROUS_LOCAL_EXECUTION": "true"}):
            result = self.sandbox.execute("import os\nprint('OS Allowed')")
            self.assertEqual(result.exit_code, 0)
            self.assertIn("OS Allowed", result.stdout)

    def test_environment_isolation(self):
        # Even with dangerous execution allowed, environment should be clean
        # We set a secret in the HOST environment
        with patch.dict(os.environ, {"ALLOW_DANGEROUS_LOCAL_EXECUTION": "true", "HOST_SECRET": "TOP_SECRET"}):
            # The code tries to print the secret
            code = """
import os
print(f"SECRET: {os.environ.get('HOST_SECRET', 'NOT_FOUND')}")
"""
            result = self.sandbox.execute(code)
            self.assertEqual(result.exit_code, 0)
            self.assertIn("SECRET: NOT_FOUND", result.stdout)
            self.assertNotIn("TOP_SECRET", result.stdout)

if __name__ == "__main__":
    unittest.main()
