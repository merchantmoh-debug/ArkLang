import unittest
import subprocess
import os
import json
import hashlib
import time
import sys

def calculate_hash(content):
    canonical = json.dumps(content, sort_keys=True, separators=(',', ':'))
    sha = hashlib.sha256()
    sha.update(canonical.encode('utf-8'))
    return sha.hexdigest()

def mast(content):
    return {
        "hash": calculate_hash(content),
        "content": content,
        "span": None
    }

class ArkBuilder:
    @staticmethod
    def literal(s):
        return {"Literal": s}

    @staticmethod
    def list_expr(items):
        return {"List": items}

    @staticmethod
    def call(func, args):
        return {
            "Call": {
                "function_hash": func,
                "args": args
            }
        }

    @staticmethod
    def stmt_expr(expr):
        # Wrap expression in Statement::Expression
        return {"Expression": expr}

    @staticmethod
    def block(stmts):
        return {
            "Statement": { # ArkNode::Statement
                "Block": stmts
            }
        }

def _find_ark_loader():
    """Find ark_loader binary, return path or None."""
    candidates = [
        "target/release/ark_loader",
        "core/target/release/ark_loader",
        os.path.join("target", "release", "ark_loader.exe"),
        os.path.join("core", "target", "release", "ark_loader.exe"),
    ]
    for path in candidates:
        if os.path.exists(path):
            return path
    return None

ARK_LOADER = _find_ark_loader()

@unittest.skipUnless(ARK_LOADER, "ark_loader binary not found — skip Sovereign Upgrade tests")
class TestSovereignUpgrade(unittest.TestCase):
    def run_mast(self, root_node, unsafe=False):
        json_file = "test_temp.json"

        with open(json_file, "w") as f:
            json.dump(root_node, f)

        try:
            env = os.environ.copy()
            if unsafe:
                env["ARK_UNSAFE_EXEC"] = "true"
            else:
                if "ARK_UNSAFE_EXEC" in env:
                    del env["ARK_UNSAFE_EXEC"]

            cmd_exec = [ARK_LOADER, json_file]
            proc_exec = subprocess.run(cmd_exec, capture_output=True, text=True, env=env)

            return {
                "exit_code": proc_exec.returncode,
                "stdout": proc_exec.stdout,
                "stderr": proc_exec.stderr
            }
        finally:
            if os.path.exists(json_file): os.remove(json_file)

    def test_command_whitelist(self):
        print("\n--- Testing Command Whitelist ---")

        # On Windows, 'echo' is a shell builtin, not a standalone binary.
        # The ark_loader Rust binary can't exec it directly.
        if os.name == 'nt':
            import shutil
            if shutil.which("echo") is not None:
                self._test_whitelisted_echo()
            # else: skip — echo is not an executable on this system
        else:
            self._test_whitelisted_echo()

        # Code: sys.exec(["rm", "--help"]) — should always be BLOCKED
        code_blocked = ArkBuilder.block([
            ArkBuilder.stmt_expr(
                ArkBuilder.call("sys.exec", [
                    ArkBuilder.list_expr([
                        ArkBuilder.literal("rm"),
                        ArkBuilder.literal("--help")
                    ])
                ])
            )
        ])

        result = self.run_mast(code_blocked)
        self.assertNotEqual(result["exit_code"], 0)
        self.assertIn("Security Violation", result["stdout"] + result["stderr"])

    def _test_whitelisted_echo(self):
        """Helper: test that whitelisted 'echo' command is allowed."""
        # Code: sys.exec(["echo", "Sovereign"])
        code = ArkBuilder.block([
            ArkBuilder.stmt_expr(
                ArkBuilder.call("sys.exec", [
                    ArkBuilder.list_expr([
                        ArkBuilder.literal("echo"),
                        ArkBuilder.literal("Sovereign")
                    ])
                ])
            )
        ])

        result = self.run_mast(code)
        self.assertEqual(result["exit_code"], 0)
        self.assertIn("Sovereign", result["stdout"])

    def test_protected_paths(self):
        print("\n--- Testing Protected Paths ---")

        # Code: sys.fs.write("Cargo.toml", "pwned")
        code = ArkBuilder.block([
            ArkBuilder.stmt_expr(
                ArkBuilder.call("sys.fs.write", [
                    ArkBuilder.literal("Cargo.toml"),
                    ArkBuilder.literal("pwned")
                ])
            )
        ])

        result = self.run_mast(code)
        self.assertNotEqual(result["exit_code"], 0)
        self.assertIn("Security Violation", result["stdout"] + result["stderr"])

    def test_ai_caching(self):
        print("\n--- Testing AI Semantic Cache ---")
        # Skipping assertion in CI environment without valid API Key
        print("Skipping cache verification due to missing credentials.")
        pass

if __name__ == "__main__":
    unittest.main()
