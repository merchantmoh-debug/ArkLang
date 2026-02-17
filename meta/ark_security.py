"""
Ark Security Module — Sandbox enforcement and capability checks.

Extracted from ark.py (Phase 72: Structural Hardening).
"""
import os
import sys
import socket
import ipaddress
import urllib.parse
import urllib.request
import argparse
import json
import re

# Try to import QiParser, handling both module and script execution contexts
try:
    from meta.ark_parser import QiParser
except ImportError as e1:
    try:
        from ark_parser import QiParser
    except ImportError as e2:
        print(f"DEBUG: Import failed: {e1} | {e2}")
        QiParser = None  # Fallback or error later


class SandboxViolation(Exception):
    pass


class LinearityViolation(Exception):
    pass


# ─── Capability-Token System ─────────────────────────────────────────────────
#
# Replaces the binary ALLOW_DANGEROUS_LOCAL_EXECUTION env var with granular caps.
# Usage: ARK_CAPABILITIES="exec,net,fs_write,fs_read,thread,ai"
#
# Backward compat: ALLOW_DANGEROUS_LOCAL_EXECUTION=true grants ALL capabilities.

def _load_capabilities():
    """Load capabilities from environment."""
    # Backward compatibility: old env var grants everything
    if os.environ.get("ALLOW_DANGEROUS_LOCAL_EXECUTION", "false").lower() == "true":
        return {"exec", "net", "fs_write", "fs_read", "thread", "ai", "all"}
    
    raw = os.environ.get("ARK_CAPABILITIES", "")
    if not raw:
        return set()
    return set(cap.strip() for cap in raw.split(",") if cap.strip())


CAPABILITIES = _load_capabilities()


def has_capability(cap: str) -> bool:
    """Check if a capability is granted."""
    return "all" in CAPABILITIES or cap in CAPABILITIES


def check_capability(cap: str):
    """Require a capability, raising SandboxViolation if not granted."""
    if not has_capability(cap):
        raise SandboxViolation(
            f"Capability '{cap}' not granted. "
            f"Set ARK_CAPABILITIES={cap} or ALLOW_DANGEROUS_LOCAL_EXECUTION=true to enable."
        )


# ─── Path Security ───────────────────────────────────────────────────────────

def check_path_security(path, is_write=False):
    if has_capability("all"):
        return

    # Path Traversal Check (Hardened)
    # 1. Normalize without resolving symlinks first to catch '..'
    norm_path = os.path.normpath(path)
    if norm_path.startswith("..") or (os.sep + ".." + os.sep) in norm_path:
         raise SandboxViolation(f"Path traversal detected: {path}")

    # 2. Resolve absolute path
    abs_path = os.path.abspath(path)
    real_path = os.path.realpath(path) # Follow symlinks
    cwd = os.getcwd()

    # 3. Check if path is within CWD (allow CWD itself)
    # Using commonpath on real_path ensures we don't escape via symlinks
    if os.path.commonpath([cwd, real_path]) != cwd:
        raise SandboxViolation(f"Access outside working directory is forbidden: {path} (Resolved to: {real_path})")

    if is_write:
        # Require fs_write capability
        check_capability("fs_write")
        
        # Protect system files from being overwritten in sandbox mode
        meta_dir = os.path.dirname(os.path.realpath(__file__))
        repo_root = os.path.dirname(meta_dir)

        # Protected directories
        protected_dirs = [
            "meta", "core", "lib", "src", "tests",
            "apps", "benchmarks", "docs", "examples", "ops", "web",
            ".git", ".agent", ".antigravity", ".context", "artifacts"
        ]
        for d in protected_dirs:
            protected_path = os.path.realpath(os.path.join(repo_root, d))
            if abs_path.startswith(protected_path):
                raise SandboxViolation(f"Writing to protected directory is forbidden: {d}")

        # Protected root files
        protected_files = [
            "Cargo.toml", "README.md", "LICENSE", "requirements.txt",
            "MANUAL.md", "ARK_OMEGA_POINT.md", "SWARM_PLAN.md", "CLA.md",
            "Dockerfile", "docker-compose.yml", "sovereign_launch.bat",
            "pyproject.toml", "Cargo.lock", "debug_build.py"
        ]
        for f in protected_files:
            protected_file_path = os.path.realpath(os.path.join(repo_root, f))
            if abs_path == protected_file_path:
                raise SandboxViolation(f"Writing to protected file is forbidden: {f}")


def check_exec_security():
    """Check if exec capability is granted."""
    check_capability("exec")


# ─── URL Security ────────────────────────────────────────────────────────────

def validate_url_security(url):
    try:
        parsed = urllib.parse.urlparse(url)
    except Exception as e:
        raise Exception(f"Invalid URL: {e}")

    if parsed.scheme not in ('http', 'https'):
        raise Exception(f"URL scheme '{parsed.scheme}' is not allowed (only http/https)")

    hostname = parsed.hostname
    if not hostname:
        raise Exception("Invalid URL: missing hostname")

    # Resolve hostname to IP
    try:
        addr_info = socket.getaddrinfo(hostname, None)
    except socket.gaierror as e:
        raise Exception(f"DNS resolution failed for {hostname}: {e}")

    for _, _, _, _, sockaddr in addr_info:
        ip_str = sockaddr[0]
        try:
            ip = ipaddress.ip_address(ip_str)
        except ValueError:
            continue

        if ip.is_loopback:
            if not has_capability("net"):
                 raise SandboxViolation(f"Access to loopback address '{ip_str}' is forbidden without 'net' capability.")
            continue

        if ip.is_private or ip.is_link_local or ip.is_multicast or ip.is_reserved:
            raise SandboxViolation(f"Access to private/local/reserved IP '{ip_str}' is forbidden")

        if str(ip) == "0.0.0.0":
            raise SandboxViolation("Access to 0.0.0.0 is forbidden")


class SafeRedirectHandler(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        validate_url_security(newurl)
        return super().redirect_request(req, fp, code, msg, headers, newurl)


# ─── Security Scanner ────────────────────────────────────────────────────────

class SecurityScanner:
    def __init__(self):
        grammar_path = os.path.join(os.path.dirname(__file__), "ark.lark")
        if not os.path.exists(grammar_path):
             # Fallback if running from root
             grammar_path = "meta/ark.lark"

        if QiParser:
            self.parser = QiParser(grammar_path)
        else:
            self.parser = None
            print("Warning: QiParser not available. Static analysis limited.")

        self.findings = []

        # Capability Mapping
        self.cap_map = {
            "sys.net": "net",
            "sys.fs.read": "fs_read",
            "sys.fs.write": "fs_write",
            "sys.io.read_file_async": "fs_read",
            "sys.exec": "exec",
            "sys.shell": "exec",
            "os_command": "exec",
            "sys.thread": "thread",
            "sys.ask_ai": "ai",
            "sys.crypto": "crypto"
        }

    def scan_file(self, path):
        """Scans a single file for security issues."""
        self.findings = []
        try:
            with open(path, "r", encoding="utf-8") as f:
                content = f.read()

            # 1. Regex Checks (Secrets)
            self._check_secrets(path, content)

            # 2. Parse & Static Analysis
            if self.parser:
                try:
                    ast = self.parser.parse(content)
                    self._check_static_analysis(path, ast)
                    self._audit_capabilities(path, ast)
                    self._audit_dependencies(path, ast)
                    # 3. Circular Dependency Check (simple DFS)
                    self._check_circular_deps(path)
                except Exception as e:
                    self.findings.append({
                        "type": "PARSE_ERROR",
                        "severity": "medium",
                        "file": path,
                        "line": 0,
                        "description": f"Failed to parse file: {e}",
                        "recommendation": "Fix syntax errors."
                    })
        except Exception as e:
            self.findings.append({
                "type": "SCAN_ERROR",
                "severity": "low",
                "file": path,
                "line": 0,
                "description": f"Failed to read file: {e}",
                "recommendation": "Check file permissions."
            })

        return self.findings

    def get_capability_manifest(self, path):
        """Returns a set of capabilities required by the file."""
        # Helper that just runs scan and returns caps
        # We need to run scan_file to populate findings/caps
        # But wait, audit_capabilities adds findings. I should track caps separately or parse them out.
        # For the test 'test_capability_manifest', it expects a return value.
        # I'll implement a fresh parse or reuse scan logic.

        required_caps = set()
        if not self.parser:
            return required_caps

        try:
            with open(path, "r", encoding="utf-8") as f:
                content = f.read()
            ast = self.parser.parse(content)

            def visit(node):
                if isinstance(node, dict):
                    if node.get("type") == "call":
                        func = node.get("function", "")
                        for prefix, cap in self.cap_map.items():
                            if func.startswith(prefix):
                                required_caps.add(cap)

                    for key, val in node.items():
                        visit(val)
                elif isinstance(node, list):
                    for item in node:
                        visit(item)

            visit(ast)
        except:
            pass
        return required_caps

    def _check_secrets(self, path, content):
        patterns = [
            (r"sk-[a-zA-Z0-9]{20,}", "Potential API Key"),
            (r"AKIA[0-9A-Z]{16}", "AWS Access Key"),
            (r"password\s*=\s*['\"][^'\"]+['\"]", "Hardcoded Password"),
            (r"secret\s*=\s*['\"][^'\"]+['\"]", "Hardcoded Secret"),
        ]

        for i, line in enumerate(content.splitlines()):
            for pattern, desc in patterns:
                if re.search(pattern, line):
                    self.findings.append({
                        "type": "HARDCODED_SECRET",
                        "severity": "high",
                        "file": path,
                        "line": i + 1,
                        "description": f"{desc} detected.",
                        "recommendation": "Use environment variables or a secrets manager."
                    })

    def _check_static_analysis(self, path, ast):
        # Needed for handling Lark Trees that are not transformed into dicts
        try:
            from lark import Tree
        except ImportError:
            Tree = type(None) # Mock if not available, though required for parser

        def traverse(node):
            if isinstance(node, Tree):
                for child in node.children:
                    traverse(child)
                return

            if isinstance(node, dict):
                # check for SQL Injection
                if node.get("op") == "add":
                    if self._is_potential_sqli(node):
                        self.findings.append({
                            "type": "SQL_INJECTION",
                            "severity": "critical",
                            "file": path,
                            "line": 0, # AST doesn't track lines easily in this simplified parser
                            "description": "Potential SQL Injection via string concatenation.",
                            "recommendation": "Use parameterized queries."
                        })

                # check for Command Injection
                if node.get("type") == "call":
                    func = node.get("function", "")
                    if func in ("sys.exec", "sys.shell", "os_command"):
                        args = node.get("args", [])
                        if args and not self._is_constant_string(args[0]):
                             self.findings.append({
                                "type": "COMMAND_INJECTION",
                                "severity": "critical",
                                "file": path,
                                "line": 0,
                                "description": f"Dynamic command execution detected in {func}.",
                                "recommendation": "Validate inputs or use whitelist."
                            })

                    # check for Path Traversal
                    if func.startswith("sys.fs.") or func == "sys.io.read_file_async":
                         args = node.get("args", [])
                         if args and self._is_traversal_string(args[0]):
                             self.findings.append({
                                "type": "PATH_TRAVERSAL",
                                "severity": "high",
                                "file": path,
                                "line": 0,
                                "description": "Path traversal pattern ('..') detected.",
                                "recommendation": "Sanitize paths."
                            })

                    # check for Unsafe Deserialization
                    if func == "sys.json.parse":
                        self.findings.append({
                            "type": "UNSAFE_DESERIALIZATION",
                            "severity": "medium",
                            "file": path,
                            "line": 0,
                            "description": "JSON parsing of potentially untrusted input.",
                            "recommendation": "Ensure input source is trusted."
                        })

                # check for Infinite Loop
                if node.get("type") == "while":
                    cond = node.get("condition")
                    if self._is_constant_true(cond):
                        self.findings.append({
                            "type": "INFINITE_LOOP",
                            "severity": "medium",
                            "file": path,
                            "line": 0,
                            "description": "Infinite loop detected (while true).",
                            "recommendation": "Ensure there is a break condition."
                        })

                for key, val in node.items():
                    traverse(val)

            elif isinstance(node, list):
                for item in node:
                    traverse(item)

        traverse(ast)

    def _is_potential_sqli(self, node):
        # Recursive check if an 'add' tree contains SQL keywords
        def collect_strings(n):
            if isinstance(n, dict):
                if n.get("type") == "string":
                    return [n.get("val", "")]
                if n.get("op") == "add":
                    return collect_strings(n.get("left")) + collect_strings(n.get("right"))
            return []

        strings = collect_strings(node)
        sql_keywords = ["SELECT ", "INSERT ", "UPDATE ", "DELETE ", "DROP "]
        combined = "".join(strings).upper()
        for kw in sql_keywords:
            if kw in combined:
                return True
        return False

    def _is_constant_string(self, node):
        return isinstance(node, dict) and node.get("type") == "string"

    def _is_traversal_string(self, node):
        if isinstance(node, dict) and node.get("type") == "string":
            return ".." in node.get("val", "")
        return False # If dynamic, we can't be sure, but static analysis usually flags explicit patterns

    def _is_constant_true(self, node):
        # Check if condition is 'true'
        # Parser might return var name "true" or boolean.
        # Grammar: IDENTIFIER -> var
        if isinstance(node, dict):
            if node.get("type") == "var" and node.get("name") == "true":
                return True
            if node.get("type") == "Boolean" and node.get("val") is True:
                return True
        return False

    def _audit_capabilities(self, path, ast):
        # We did this in get_capability_manifest, but here we add findings
        def visit(node):
            if isinstance(node, dict):
                if node.get("type") == "call":
                    func = node.get("function", "")
                    for prefix, cap in self.cap_map.items():
                        if func.startswith(prefix):
                            # Just an info/audit log, or warning if excessive?
                            # Prompt says: "Map each call to its required sandbox capability"
                            # We can add an INFO finding
                            self.findings.append({
                                "type": "CAPABILITY_USE",
                                "severity": "info",
                                "file": path,
                                "line": 0,
                                "description": f"Function '{func}' requires capability '{cap}'",
                                "recommendation": f"Ensure '{cap}' is granted."
                            })
                for k, v in node.items():
                    visit(v)
            elif isinstance(node, list):
                for i in node:
                    visit(i)
        visit(ast)

    def _audit_dependencies(self, path, ast):
        # Look for import statements
        # Since ArkTransformer doesn't structure them, we might need to rely on looking for "Tree" objects if using Lark's parser output directly,
        # OR regex if the parser output doesn't preserve them nicely.
        # But wait, QiParser returns what ArkTransformer returns.
        # ArkTransformer returns `args[0]` for statement.
        # The grammar says: import_stmt: "import" IDENTIFIER ("." IDENTIFIER)*
        # If ArkTransformer doesn't have `import_stmt` method, Lark returns a Tree(data='import_stmt', children=[Token(IDENTIFIER, ...)]).

        # We need to traverse specifically looking for `lark.Tree` or identifying imports.
        # Since we imported `QiParser`, we assume we have `lark`.
        try:
            from lark import Tree, Token
        except ImportError:
            return

        def visit(node):
            if isinstance(node, Tree) and node.data == "import_stmt":
                # Extract module name
                parts = [str(c) for c in node.children if isinstance(c, Token) and c.type == "IDENTIFIER"]
                module_path = os.path.join("lib", *parts) + ".ark"
                if not os.path.exists(module_path):
                     # Check lib/std
                     std_path = os.path.join("lib", "std", *parts) + ".ark"
                     if not os.path.exists(std_path):
                          self.findings.append({
                            "type": "MISSING_IMPORT",
                            "severity": "warning",
                            "file": path,
                            "line": 0,
                            "description": f"Imported module not found: {'.'.join(parts)}",
                            "recommendation": "Check library path."
                        })

            # Recurse
            if isinstance(node, Tree):
                for c in node.children:
                    visit(c)
            elif isinstance(node, dict):
                for v in node.values():
                    visit(v)
            elif isinstance(node, list):
                for v in node:
                    visit(v)

        # The AST from QiParser might contain Trees mixed with dicts if transformer didn't catch everything
        # Actually, ArkTransformer.statement returns args[0].
        # import_stmt is a statement.
        # So import_stmt becomes the return value of import_stmt rule (which is default Tree construction).
        visit(ast)

    def _check_circular_deps(self, start_path):
        # Prevent infinite recursion or checking if path doesn't exist
        start_path = os.path.realpath(start_path)

        # Stack for DFS
        stack = []
        visited_in_path = set()

        def get_imports_for_file(path):
            imports = []
            try:
                # We need to re-parse or cache. Re-parsing is safer but slower.
                # Use regex for imports to be faster? Or proper parse.
                # Given we already parsed start_path, we could pass AST, but recursive needs new parses.
                # Let's use regex for speed in recursive check to avoid full parsing overhead
                # import pattern: import lib.foo
                if not os.path.exists(path): return []
                with open(path, "r", encoding="utf-8") as f:
                    c = f.read()

                # Regex for "import x.y"
                # This is an approximation. Static analysis is better but let's be robust.
                found = re.findall(r"^\s*import\s+([a-zA-Z0-9_\.]+)", c, re.MULTILINE)
                resolved = []
                for imp in found:
                     parts = imp.split(".")
                     p = os.path.join("lib", *parts) + ".ark"
                     if os.path.exists(p): resolved.append(os.path.realpath(p))
                     else:
                        p2 = os.path.join("lib", "std", *parts) + ".ark"
                        if os.path.exists(p2): resolved.append(os.path.realpath(p2))
                return resolved
            except:
                return []

        def dfs(current_path):
            if current_path in visited_in_path:
                # Cycle found!
                cycle = " -> ".join([os.path.basename(p) for p in stack] + [os.path.basename(current_path)])
                self.findings.append({
                    "type": "CIRCULAR_DEPENDENCY",
                    "severity": "high",
                    "file": start_path,
                    "line": 0,
                    "description": f"Circular import chain detected: {cycle}",
                    "recommendation": "Refactor modules to break dependency cycle."
                })
                return

            visited_in_path.add(current_path)
            stack.append(current_path)

            imps = get_imports_for_file(current_path)
            for imp in imps:
                # Limit depth?
                if len(stack) > 10: continue
                dfs(imp)

            stack.pop()
            visited_in_path.remove(current_path)

        dfs(start_path)

    def generate_report(self, format="console"):
        if format == "json":
            return json.dumps(self.findings, indent=2)
        elif format == "markdown":
            lines = ["# Security Scan Report\n"]
            for f in self.findings:
                lines.append(f"## {f['type']} ({f['severity'].upper()})")
                lines.append(f"- **File**: {f['file']}")
                lines.append(f"- **Description**: {f['description']}")
                lines.append(f"- **Recommendation**: {f['recommendation']}\n")
            return "\n".join(lines)
        else:
            # Console
            out = []
            colors = {
                "critical": "\033[91m",
                "high": "\033[31m",
                "medium": "\033[33m",
                "low": "\033[34m",
                "info": "\033[37m",
                "reset": "\033[0m"
            }

            # Group by file
            by_file = {}
            for f in self.findings:
                if f['file'] not in by_file:
                    by_file[f['file']] = []
                by_file[f['file']].append(f)

            for path, issues in by_file.items():
                # Calculate file risk
                max_sev = 0
                sev_val = {"critical": 5, "high": 4, "medium": 3, "low": 2, "info": 1}
                caps = set()
                intrinsics = set()

                for i in issues:
                    s = sev_val.get(i['severity'], 1)
                    if s > max_sev: max_sev = s
                    if i['type'] == 'CAPABILITY_USE':
                         # extract cap from description 'Function 'x' requires capability 'y''
                         m = re.search(r"capability '(\w+)'", i['description'])
                         if m: caps.add(m.group(1))
                         m2 = re.search(r"Function '([\w\.]+)'", i['description'])
                         if m2: intrinsics.add(m2.group(1))

                risk_label = "LOW"
                for k, v in sev_val.items():
                    if v == max_sev:
                        risk_label = k.upper()

                c = colors.get(risk_label.lower(), colors["info"])

                out.append(f"{c}FILE: {path}{colors['reset']}")
                if caps:
                    out.append(f"CAPABILITIES REQUIRED: {', '.join(sorted(caps))}")
                if intrinsics:
                    out.append(f"INTRINSICS USED: {', '.join(sorted(intrinsics))}")
                out.append(f"RISK LEVEL: {c}{risk_label}{colors['reset']}")

                out.append("FINDINGS:")
                for i in issues:
                    if i['type'] == 'CAPABILITY_USE': continue
                    ic = colors.get(i['severity'], colors["info"])
                    out.append(f"  {ic}[{i['severity'].upper()}] {i['type']}: {i['description']}{colors['reset']}")
                out.append("")

            return "\n".join(out)

    def check_secure_defaults(self):
        # 1. Check Env
        if os.environ.get("ARK_CAPABILITIES") == "*":
             self.findings.append({
                "type": "INSECURE_CONFIG",
                "severity": "critical",
                "file": "ENV",
                "line": 0,
                "description": "ARK_CAPABILITIES is set to wildcard (*). This is unsafe.",
                "recommendation": "Use specific capabilities."
            })

        # 2. Check security.json
        if os.path.exists("security.json"):
            try:
                with open("security.json", "r") as f:
                    conf = json.load(f)
                    if conf.get("permissive"):
                        self.findings.append({
                            "type": "INSECURE_CONFIG",
                            "severity": "medium",
                            "file": "security.json",
                            "line": 0,
                            "description": "security.json has 'permissive: true'.",
                            "recommendation": "Set 'permissive: false' for production."
                        })
            except:
                pass


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Ark Security Scanner")
    parser.add_argument("path", help="File or directory to scan")
    parser.add_argument("--format", choices=["console", "json", "markdown"], default="console")
    parser.add_argument("--severity", choices=["critical", "high", "medium", "low", "info"], default="low",
                        help="Minimum severity to report")
    parser.add_argument("-o", "--output", help="Output file")

    args = parser.parse_args()

    scanner = SecurityScanner()
    scanner.check_secure_defaults()
    all_findings = list(scanner.findings)
    scanner.findings = []

    targets = []
    if os.path.isdir(args.path):
        for root, _, files in os.walk(args.path):
            for file in files:
                if file.endswith(".ark"):
                    targets.append(os.path.join(root, file))
    else:
        targets.append(args.path)

    for target in targets:
        findings = scanner.scan_file(target)
        # Filter by severity
        severity_map = {"critical": 5, "high": 4, "medium": 3, "low": 2, "info": 1}
        min_sev = severity_map.get(args.severity, 1)

        filtered = [f for f in findings if severity_map.get(f['severity'], 1) >= min_sev]
        scanner.findings = filtered # update for report generation

        # Accumulate for JSON/Markdown aggregation if needed?
        # The current generate_report uses self.findings which is per-file in the loop.
        # We should probably aggregate if outputting to single file.
        all_findings.extend(filtered)

    # Hack: set scanner findings to all findings for final report
    scanner.findings = all_findings
    report = scanner.generate_report(args.format)

    if args.output:
        with open(args.output, "w") as f:
            f.write(report)
    else:
        print(report)
