import unittest
import os
import shutil
import sys

# Add root to path
sys.path.append(os.getcwd())

from meta.ark_security import check_path_security, validate_url_security, SandboxViolation, CAPABILITIES

class TestSecurityHardening(unittest.TestCase):
    def setUp(self):
        # Reset capabilities for isolation
        self.original_caps = CAPABILITIES.copy()
        CAPABILITIES.clear()

        # Setup test dir
        self.test_dir = "test_security_sandbox"
        if os.path.exists(self.test_dir):
            shutil.rmtree(self.test_dir)
        os.makedirs(self.test_dir)
        self.cwd = os.getcwd()

    def tearDown(self):
        # Restore capabilities
        CAPABILITIES.clear()
        CAPABILITIES.update(self.original_caps)

        if os.path.exists(self.test_dir):
            shutil.rmtree(self.test_dir)

    def test_path_traversal_simple(self):
        with self.assertRaises(SandboxViolation):
            check_path_security("../etc/passwd")

    def test_path_traversal_complex(self):
        with self.assertRaises(SandboxViolation):
            check_path_security(f"{self.test_dir}/../../etc/passwd")

    def test_path_traversal_symlink(self):
        # Create a symlink pointing outside
        link_path = os.path.join(self.test_dir, "bad_link")
        try:
            os.symlink("/etc/passwd", link_path)
            with self.assertRaises(SandboxViolation):
                check_path_security(link_path)
        except OSError:
            # Skip if symlinks not supported (e.g. some restricted envs)
            pass

    def test_valid_path(self):
        # Should not raise
        valid = os.path.join(os.getcwd(), "test_file.txt")
        # Ensure it doesn't fail on existence check (check_path_security doesn't check existence usually,
        # but let's make sure commonpath logic holds)
        check_path_security(valid)

    def test_ssrf_loopback_denied_default(self):
        # No 'net' capability -> Denied
        with self.assertRaisesRegex(Exception, "Access to loopback address .* is forbidden"):
             validate_url_security("http://127.0.0.1:8080")

        with self.assertRaisesRegex(Exception, "Access to loopback address .* is forbidden"):
             validate_url_security("http://localhost:8080")

    def test_ssrf_loopback_allowed_with_cap(self):
        CAPABILITIES.add("net")
        # Should not raise
        validate_url_security("http://127.0.0.1:8080")
        validate_url_security("http://localhost:8080")

    def test_ssrf_private_ip(self):
        # Always denied regardless of cap? Currently code denies private IPs unconditionally
        # "if ip.is_private ... raise"
        CAPABILITIES.add("net")
        with self.assertRaisesRegex(SandboxViolation, "Access to private/local/reserved IP"):
            validate_url_security("http://192.168.1.1")

        with self.assertRaisesRegex(SandboxViolation, "Access to private/local/reserved IP"):
            validate_url_security("http://10.0.0.1")

if __name__ == "__main__":
    unittest.main()
