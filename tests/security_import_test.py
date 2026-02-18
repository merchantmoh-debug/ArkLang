import os
import sys
import shutil
import unittest
from contextlib import contextmanager

# Add repo root to path
sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from meta.ark_interpreter import handle_import, ArkRuntimeError, Scope, ArkValue

class MockNode:
    def __init__(self, parts):
        self.children = [type('Token', (), {'value': p}) for p in parts]

class SecurityTest(unittest.TestCase):
    def setUp(self):
        self.test_dir = os.path.abspath("test_sandbox_env")
        os.makedirs(self.test_dir, exist_ok=True)
        self.original_cwd = os.getcwd()
        os.chdir(self.test_dir)

        # Create a "secret" file outside sandbox
        self.secret_path = os.path.join(os.path.dirname(self.test_dir), "secret.ark")
        with open(self.secret_path, "w") as f:
            f.write("print(\"pwnd\")")

    def tearDown(self):
        os.chdir(self.original_cwd)
        shutil.rmtree(self.test_dir)
        if os.path.exists(self.secret_path):
            os.remove(self.secret_path)

    def test_import_traversal(self):
        # Attempt to import "../secret"
        node = MockNode(["..", "secret"])
        scope = Scope()

        try:
            handle_import(node, scope)
            self.fail("Security violation not caught!")
        except ArkRuntimeError as e:
            print(f"Caught expected security error: {e}")
            self.assertIn("Security Violation", str(e))

    def test_import_absolute(self):
        # Attempt to import absolute path
        # Note: parts are joined. If user does import "/etc/passwd", parts=["/etc/passwd"]?
        # Ark grammar: import IDENTIFIER ("." IDENTIFIER)*
        # So "import etc.passwd" -> ["etc", "passwd"] -> "etc/passwd.ark"
        # Traversal is via ".." token if allowed, but identifier regex might block ".."
        # Let's see if we can trick it.
        # If parts = ["..", "secret"], join -> "../secret.ark".
        pass

if __name__ == "__main__":
    unittest.main()
