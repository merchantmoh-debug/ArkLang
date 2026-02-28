import unittest
import os
import shutil
import sys

# Add root to path
sys.path.append(os.getcwd())

import meta.ark_security as sec


class TestSecurityHardening(unittest.TestCase):
    """Tests for base path and URL security (backward compat)."""

    def setUp(self):
        self.original_caps = sec.CAPABILITIES.copy()
        self.original_lockdown = sec.LOCKDOWN_PATHS
        sec.CAPABILITIES.clear()
        sec.LOCKDOWN_PATHS = None

        self.test_dir = "test_security_sandbox"
        if os.path.exists(self.test_dir):
            shutil.rmtree(self.test_dir)
        os.makedirs(self.test_dir)
        self.cwd = os.getcwd()

    def tearDown(self):
        sec.CAPABILITIES.clear()
        sec.CAPABILITIES.update(self.original_caps)
        sec.LOCKDOWN_PATHS = self.original_lockdown

        if os.path.exists(self.test_dir):
            shutil.rmtree(self.test_dir)

    def test_path_traversal_simple(self):
        with self.assertRaises(sec.SandboxViolation):
            sec.check_path_security("../etc/passwd")

    def test_path_traversal_complex(self):
        with self.assertRaises(sec.SandboxViolation):
            sec.check_path_security(f"{self.test_dir}/../../etc/passwd")

    def test_path_traversal_symlink(self):
        link_path = os.path.join(self.test_dir, "bad_link")
        try:
            os.symlink("/etc/passwd", link_path)
            with self.assertRaises(sec.SandboxViolation):
                sec.check_path_security(link_path)
        except OSError:
            pass

    def test_valid_path(self):
        sec.CAPABILITIES["fs_read"] = None
        valid = os.path.join(os.getcwd(), "test_file.txt")
        sec.check_path_security(valid)

    def test_ssrf_loopback_denied_default(self):
        with self.assertRaisesRegex(Exception, "Access to loopback address .* is forbidden"):
             sec.validate_url_security("http://127.0.0.1:8080")
        with self.assertRaisesRegex(Exception, "Access to loopback address .* is forbidden"):
             sec.validate_url_security("http://localhost:8080")

    def test_ssrf_loopback_allowed_with_cap(self):
        sec.CAPABILITIES["net"] = None
        sec.validate_url_security("http://127.0.0.1:8080")
        sec.validate_url_security("http://localhost:8080")

    def test_ssrf_private_ip(self):
        sec.CAPABILITIES["net"] = None
        with self.assertRaisesRegex(sec.SandboxViolation, "Access to private/local/reserved IP"):
            sec.validate_url_security("http://192.168.1.1")
        with self.assertRaisesRegex(sec.SandboxViolation, "Access to private/local/reserved IP"):
            sec.validate_url_security("http://10.0.0.1")


class TestPathScopedCapabilities(unittest.TestCase):
    """Tests for Feature 1: path-scoped capability tokens."""

    def setUp(self):
        self.original_caps = sec.CAPABILITIES.copy()
        self.original_lockdown = sec.LOCKDOWN_PATHS
        sec.CAPABILITIES.clear()
        sec.LOCKDOWN_PATHS = None

    def tearDown(self):
        sec.CAPABILITIES.clear()
        sec.CAPABILITIES.update(self.original_caps)
        sec.LOCKDOWN_PATHS = self.original_lockdown

    def test_global_cap_granted(self):
        sec.CAPABILITIES["net"] = None
        self.assertTrue(sec.has_capability("net"))

    def test_global_cap_denied(self):
        self.assertFalse(sec.has_capability("net"))

    def test_all_grants_everything(self):
        sec.CAPABILITIES["all"] = None
        self.assertTrue(sec.has_capability("exec"))
        self.assertTrue(sec.has_capability("net"))
        self.assertTrue(sec.has_capability("nonexistent"))

    def test_path_scoped_cap_within_scope(self):
        scope_dir = os.path.realpath(os.path.join(os.getcwd(), "data"))
        sec.CAPABILITIES["fs_read"] = scope_dir
        self.assertTrue(sec.has_capability("fs_read", os.path.join(scope_dir, "file.txt")))

    def test_path_scoped_cap_outside_scope(self):
        scope_dir = os.path.realpath(os.path.join(os.getcwd(), "data"))
        sec.CAPABILITIES["fs_read"] = scope_dir
        # A path outside the scope should be denied
        outside = os.path.realpath(os.path.join(os.getcwd(), "secrets", "key.pem"))
        self.assertFalse(sec.has_capability("fs_read", outside))

    def test_path_scoped_cap_base_check_no_path(self):
        """When no path argument is given, just check base cap existence."""
        sec.CAPABILITIES["fs_read"] = "/data"
        self.assertTrue(sec.has_capability("fs_read"))

    def test_check_capability_raises_with_scope_info(self):
        sec.CAPABILITIES["fs_write"] = "/allowed"
        with self.assertRaisesRegex(sec.SandboxViolation, "scoped to"):
            sec.check_capability("fs_write", "/forbidden/path")

    def test_backward_compat_flat_token(self):
        """Flat tokens (no colon) work as global caps."""
        sec.CAPABILITIES["exec"] = None
        self.assertTrue(sec.has_capability("exec"))
        self.assertTrue(sec.has_capability("exec", "/any/path"))


class TestReadOnlyMode(unittest.TestCase):
    """Tests for Feature 2: ARK_READ_ONLY mode."""

    def setUp(self):
        self.original_caps = sec.CAPABILITIES.copy()
        self.original_lockdown = sec.LOCKDOWN_PATHS
        self.original_env = os.environ.get("ARK_READ_ONLY")
        sec.CAPABILITIES.clear()
        sec.LOCKDOWN_PATHS = None

    def tearDown(self):
        sec.CAPABILITIES.clear()
        sec.CAPABILITIES.update(self.original_caps)
        sec.LOCKDOWN_PATHS = self.original_lockdown
        if self.original_env is not None:
            os.environ["ARK_READ_ONLY"] = self.original_env
        else:
            os.environ.pop("ARK_READ_ONLY", None)

    def test_is_read_only_true(self):
        os.environ["ARK_READ_ONLY"] = "true"
        self.assertTrue(sec.is_read_only())

    def test_is_read_only_false(self):
        os.environ["ARK_READ_ONLY"] = "false"
        self.assertFalse(sec.is_read_only())

    def test_is_read_only_unset(self):
        os.environ.pop("ARK_READ_ONLY", None)
        self.assertFalse(sec.is_read_only())

    def test_write_blocked_in_read_only(self):
        os.environ["ARK_READ_ONLY"] = "true"
        sec.CAPABILITIES["fs_write"] = None
        sec.CAPABILITIES["fs_read"] = None
        # Write should be blocked even with fs_write cap
        with self.assertRaisesRegex(sec.SandboxViolation, "Read-only mode"):
            sec.check_path_security(
                os.path.join(os.getcwd(), "test_file.txt"),
                is_write=True
            )

    def test_read_allowed_in_read_only(self):
        os.environ["ARK_READ_ONLY"] = "true"
        sec.CAPABILITIES["fs_read"] = None
        # Read should still work
        valid = os.path.join(os.getcwd(), "test_file.txt")
        sec.check_path_security(valid, is_write=False)

    def test_load_caps_strips_write_in_readonly(self):
        """_load_capabilities strips write caps when ARK_READ_ONLY=true."""
        os.environ["ARK_READ_ONLY"] = "true"
        os.environ["ARK_CAPABILITIES"] = "fs_read,fs_write,exec,net"
        caps = sec._load_capabilities()
        self.assertIn("fs_read", caps)
        self.assertIn("net", caps)
        self.assertNotIn("fs_write", caps)
        self.assertNotIn("exec", caps)
        # Cleanup
        os.environ.pop("ARK_CAPABILITIES", None)


class TestLockdownPaths(unittest.TestCase):
    """Tests for Feature 3: ARK_LOCKDOWN_PATHS."""

    def setUp(self):
        self.original_caps = sec.CAPABILITIES.copy()
        self.original_lockdown = sec.LOCKDOWN_PATHS
        sec.CAPABILITIES.clear()
        sec.CAPABILITIES["fs_read"] = None
        sec.CAPABILITIES["fs_write"] = None

        self.test_dir = "test_lockdown_sandbox"
        if os.path.exists(self.test_dir):
            shutil.rmtree(self.test_dir)
        os.makedirs(self.test_dir)

    def tearDown(self):
        sec.CAPABILITIES.clear()
        sec.CAPABILITIES.update(self.original_caps)
        sec.LOCKDOWN_PATHS = self.original_lockdown

        if os.path.exists(self.test_dir):
            shutil.rmtree(self.test_dir)

    def test_lockdown_allows_approved_path(self):
        approved = os.path.realpath(self.test_dir)
        sec.LOCKDOWN_PATHS = [approved]
        # Should NOT raise
        sec.check_path_security(os.path.join(self.test_dir, "file.txt"))

    def test_lockdown_blocks_unapproved_path(self):
        approved = os.path.realpath(self.test_dir)
        sec.LOCKDOWN_PATHS = [approved]
        unapproved = os.path.join(os.getcwd(), "unapproved_dir", "file.txt")
        with self.assertRaisesRegex(sec.SandboxViolation, "Lockdown mode"):
            sec.check_path_security(unapproved)

    def test_lockdown_none_falls_through_to_cwd(self):
        sec.LOCKDOWN_PATHS = None
        # CWD-relative path should work
        valid = os.path.join(os.getcwd(), "test_file.txt")
        sec.check_path_security(valid)

    def test_lockdown_multiple_paths(self):
        dir_a = os.path.realpath(self.test_dir)
        dir_b = os.path.realpath(os.getcwd())
        sec.LOCKDOWN_PATHS = [dir_a, dir_b]
        # Both should be accessible
        sec.check_path_security(os.path.join(self.test_dir, "a.txt"))
        sec.check_path_security(os.path.join(os.getcwd(), "b.txt"))


class TestToolsetGating(unittest.TestCase):
    """Tests for Feature 4: ARK_ENABLED_TOOLSETS / ARK_EXCLUDED_TOOLS."""

    def setUp(self):
        self.original_enabled = sec.ENABLED_TOOLSETS
        self.original_excluded = sec.EXCLUDED_TOOLS

    def tearDown(self):
        sec.ENABLED_TOOLSETS = self.original_enabled
        sec.EXCLUDED_TOOLS = self.original_excluded

    def test_excluded_tool_blocked(self):
        sec.EXCLUDED_TOOLS = {"sys.exec"}
        sec.ENABLED_TOOLSETS = None
        with self.assertRaisesRegex(sec.SandboxViolation, "explicitly excluded"):
            sec.check_tool_allowed("sys.exec")

    def test_non_excluded_tool_allowed(self):
        sec.EXCLUDED_TOOLS = {"sys.exec"}
        sec.ENABLED_TOOLSETS = None
        # Should not raise
        sec.check_tool_allowed("sys.fs.read")

    def test_enabled_toolset_whitelist(self):
        sec.ENABLED_TOOLSETS = {"fs", "net"}
        sec.EXCLUDED_TOOLS = set()
        # fs toolset allowed
        sec.check_tool_allowed("sys.fs.read")
        # exec toolset NOT allowed
        with self.assertRaisesRegex(sec.SandboxViolation, "not enabled"):
            sec.check_tool_allowed("sys.exec")

    def test_blacklist_wins_over_whitelist(self):
        sec.ENABLED_TOOLSETS = {"fs"}
        sec.EXCLUDED_TOOLS = {"sys.fs.write"}
        with self.assertRaisesRegex(sec.SandboxViolation, "explicitly excluded"):
            sec.check_tool_allowed("sys.fs.write")

    def test_no_gating_allows_all(self):
        sec.ENABLED_TOOLSETS = None
        sec.EXCLUDED_TOOLS = set()
        # Everything should pass
        sec.check_tool_allowed("sys.exec")
        sec.check_tool_allowed("sys.fs.write")
        sec.check_tool_allowed("sys.net.request")


class TestCapabilitySummary(unittest.TestCase):
    """Tests for get_capability_summary diagnostics."""

    def setUp(self):
        self.original_caps = sec.CAPABILITIES.copy()
        self.original_lockdown = sec.LOCKDOWN_PATHS
        self.original_enabled = sec.ENABLED_TOOLSETS
        self.original_excluded = sec.EXCLUDED_TOOLS

    def tearDown(self):
        sec.CAPABILITIES.clear()
        sec.CAPABILITIES.update(self.original_caps)
        sec.LOCKDOWN_PATHS = self.original_lockdown
        sec.ENABLED_TOOLSETS = self.original_enabled
        sec.EXCLUDED_TOOLS = self.original_excluded

    def test_summary_structure(self):
        sec.CAPABILITIES.clear()
        sec.CAPABILITIES["fs_read"] = "/data"
        sec.CAPABILITIES["net"] = None
        sec.LOCKDOWN_PATHS = ["/safe"]
        sec.ENABLED_TOOLSETS = {"fs"}
        sec.EXCLUDED_TOOLS = {"sys.exec"}

        summary = sec.get_capability_summary()

        self.assertIn("capabilities", summary)
        self.assertEqual(summary["capabilities"]["fs_read"], "/data")
        self.assertIsNone(summary["capabilities"]["net"])
        self.assertEqual(summary["lockdown_paths"], ["/safe"])
        self.assertEqual(summary["enabled_toolsets"], ["fs"])
        self.assertIn("sys.exec", summary["excluded_tools"])


class TestCapabilityParser(unittest.TestCase):
    """Tests for _load_capabilities path-scope parsing."""

    def setUp(self):
        self.original_env_caps = os.environ.get("ARK_CAPABILITIES")
        self.original_env_exec = os.environ.get("ALLOW_DANGEROUS_LOCAL_EXECUTION")
        self.original_env_ro = os.environ.get("ARK_READ_ONLY")

    def tearDown(self):
        for key, orig in [
            ("ARK_CAPABILITIES", self.original_env_caps),
            ("ALLOW_DANGEROUS_LOCAL_EXECUTION", self.original_env_exec),
            ("ARK_READ_ONLY", self.original_env_ro),
        ]:
            if orig is not None:
                os.environ[key] = orig
            else:
                os.environ.pop(key, None)

    def test_parse_flat_tokens(self):
        os.environ["ARK_CAPABILITIES"] = "exec,net,fs_read"
        os.environ.pop("ALLOW_DANGEROUS_LOCAL_EXECUTION", None)
        os.environ.pop("ARK_READ_ONLY", None)
        caps = sec._load_capabilities()
        self.assertEqual(caps, {"exec": None, "net": None, "fs_read": None})

    def test_parse_path_scoped_tokens(self):
        os.environ["ARK_CAPABILITIES"] = "fs_read:/data,fs_write:/output,net"
        os.environ.pop("ALLOW_DANGEROUS_LOCAL_EXECUTION", None)
        os.environ.pop("ARK_READ_ONLY", None)
        caps = sec._load_capabilities()
        self.assertEqual(caps["fs_read"], "/data")
        self.assertEqual(caps["fs_write"], "/output")
        self.assertIsNone(caps["net"])

    def test_parse_empty_string(self):
        os.environ["ARK_CAPABILITIES"] = ""
        os.environ.pop("ALLOW_DANGEROUS_LOCAL_EXECUTION", None)
        caps = sec._load_capabilities()
        self.assertEqual(caps, {})

    def test_backward_compat_dangerous_execution(self):
        os.environ["ALLOW_DANGEROUS_LOCAL_EXECUTION"] = "true"
        caps = sec._load_capabilities()
        self.assertIn("all", caps)
        self.assertIn("exec", caps)
        self.assertIn("net", caps)


if __name__ == "__main__":
    unittest.main()
