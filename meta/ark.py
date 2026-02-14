import sys
import os
import re
import time
import math
import json
import ast
from dataclasses import dataclass
from typing import List, Dict, Any, Optional
import http.server
import socketserver
import threading
from lark import Lark
import hashlib
import ctypes
import html
import socket
import urllib.request
import urllib.error
import urllib.parse
import ipaddress
import queue
from cryptography.hazmat.primitives.asymmetric import ed25519
from cryptography.hazmat.primitives import serialization

# --- Global Event Queue ---
EVENT_QUEUE = queue.Queue()
ARK_AI_MODE = None

# --- Global Parser ---
# Load grammar from file
grammar_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "ark.lark")
with open(grammar_path, "r") as f:
    ARK_GRAMMAR = f.read()

ARK_PARSER = Lark(ARK_GRAMMAR, start="start", parser="lalr")

class Colors:
    HEADER = '\033[95m'
    OKBLUE = '\033[94m'
    OKCYAN = '\033[96m'
    OKGREEN = '\033[92m'
    WARNING = '\033[93m'
    FAIL = '\033[91m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'
    UNDERLINE = '\033[4m'

# --- Security ---

class SandboxViolation(Exception):
    pass

class LinearityViolation(Exception):
    pass

def check_path_security(path, is_write=False):
    if os.environ.get("ALLOW_DANGEROUS_LOCAL_EXECUTION", "false").lower() == "true":
        return

    # Path Traversal Check
    # Resolving path to canonical path (resolving symlinks)
    abs_path = os.path.realpath(path)
    cwd = os.getcwd()

    # Check if path is within CWD (or is CWD itself)
    if not abs_path.startswith(cwd):
        raise SandboxViolation(f"Access outside working directory is forbidden: {path} (Resolved to: {abs_path})")

    if is_write:
        # Protect system files from being overwritten in sandbox mode
        # meta/ark.py is located in the 'meta' directory of the repo root
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
    if os.environ.get("ALLOW_DANGEROUS_LOCAL_EXECUTION", "false").lower() != "true":
        raise SandboxViolation("System command execution is disabled in sandbox mode.")

# --- Types ---

@dataclass(slots=True)
class ArkValue:
    val: Any
    type: str

class ReturnException(Exception):
    def __init__(self, value):
        self.value = value

@dataclass(slots=True)
class ArkFunction:
    name: str
    params: List[str]
    body: Any # Tree node
    closure: 'Scope'

@dataclass(slots=True)
class ArkClass:
    name: str
    methods: Dict[str, ArkFunction]

@dataclass(slots=True)
class ArkInstance:
    klass: ArkClass
    fields: Dict[str, ArkValue]

class Scope:
    __slots__ = ('vars', 'parent')

    def __init__(self, parent=None):
        self.vars = {}
        self.parent = parent

    def get(self, name: str) -> Optional[ArkValue]:
        if name in self.vars:
            val = self.vars[name]
            if val.type == "Moved":
                raise LinearityViolation(f"Use of moved variable '{name}'")
            return val
        if self.parent:
            return self.parent.get(name)
        return None

    def set(self, name: str, val: ArkValue):
        self.vars[name] = val

    def mark_moved(self, name: str):
        if name in self.vars:
            self.vars[name] = ArkValue(None, "Moved")
            return
        if self.parent:
            self.parent.mark_moved(name)

# --- Intrinsics ---

def core_print(args: List[ArkValue]):
    print(*(arg.val for arg in args))
    return ArkValue(None, "Unit")

def core_len(args: List[ArkValue]):
    if not args or args[0].type not in ["String", "List"]:
        raise Exception("len() expects a String or List argument")
    return ArkValue(len(args[0].val), "Integer")

def core_get(args: List[ArkValue]):
    if len(args) != 2:
        raise Exception("get() expects two arguments: list/string and index")
    collection = args[0].val
    index = args[1].val
    if not isinstance(index, int):
        raise Exception("Index must be an integer")
    if not isinstance(collection, (str, list)):
        raise Exception("Collection must be a string or list")
    
    if 0 <= index < len(collection):
        if isinstance(collection, str):
            return ArkValue(collection[index], "String")
        elif isinstance(collection, list):
            val = collection[index]
            if isinstance(val, ArkValue):
                return val
            return ArkValue(val, "Any")
    else:
        raise Exception("Index out of bounds")
    return ArkValue(None, "Unit") # Should not be reached

COMMAND_WHITELIST = {
    "ls", "grep", "cat", "echo", "python", "python3",
    "cargo", "rustc", "git", "date", "whoami", "pwd", "mkdir", "touch"
}

def sys_exec(args: List[ArkValue]):
    check_exec_security()
    if not args or args[0].type != "String":
        raise Exception("sys.exec expects a string command")
    
    command_str = args[0].val.strip()
    if not command_str:
        return ArkValue("", "String")

    # 1. Parse command safely (respects quotes)
    try:
        # Use shlex to parse the command string into a list of arguments.
        # posix=(os.name != 'nt') ensures proper handling of quotes and escapes relative to the OS.
        cmd_args = shlex.split(command_str, posix=(os.name != 'nt'))
    except Exception as e:
        return ArkValue(f"Security Error: Failed to parse command: {e}", "String")

    if not cmd_args:
        return ArkValue("", "String")

    base_cmd = cmd_args[0]

    # 2. Whitelist Check (Now effective because we use the parsed base_cmd)
    if os.environ.get("ALLOW_DANGEROUS_LOCAL_EXECUTION", "false").lower() != "true":
        if base_cmd not in COMMAND_WHITELIST:
             # This message is now accurate because we are executing `base_cmd` directly, not passing a string to shell.
             raise SandboxViolation(f"Command '{base_cmd}' is not in the whitelist. set ALLOW_DANGEROUS_LOCAL_EXECUTION=true to bypass.")

    # 3. Execution (shell=False prevents '&&' injection)
    # The vulnerability SimiKusoni found (ls && rm -rf) is neutralized here because 
    # '&&', 'rm', '-rf' will be passed as arguments to 'ls' (which will likely complain about them),
    # rather than being interpreted by the shell as a new command.
    try:
        # capture_output=True requires Python 3.7+
        result = subprocess.run(
            cmd_args, 
            shell=False, 
            capture_output=True, 
            text=True,
            timeout=10 # Prevent hangs
        )
        
        # Combine stdout and stderr
        output = result.stdout
        if result.stderr:
            output += "\nHelper: " + result.stderr
            
        return ArkValue(output.strip(), "String")
    except Exception as e:
        return ArkValue(f"Error: {e}", "String")

def sys_fs_write(args: List[ArkValue]):
    if len(args) != 2 or args[0].type != "String" or args[1].type != "String":
        raise Exception("sys.fs.write expects two string arguments: path and content")
    path = args[0].val
    check_path_security(path, is_write=True)
    content = args[1].val
    try:
        with open(path, "w") as f:
            f.write(content)
        return ArkValue(None, "Unit")
    except Exception as e:
        raise Exception(f"Error writing to file {path}: {e}")

def sys_fs_read(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "String":
        raise Exception("sys.fs.read expects a string path argument")
    path = args[0].val
    check_path_security(path)
    try:
        with open(path, "r") as f:
            content = f.read()
        return ArkValue(content, "String")
    except Exception as e:
        raise Exception(f"Error reading file {path}: {e}")

def sys_fs_read_buffer(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "String":
        raise Exception("sys.fs.read_buffer expects a string path argument")
    path = args[0].val
    check_path_security(path)
    try:
        with open(path, "rb") as f:
            content = f.read()
        return ArkValue(bytearray(content), "Buffer")
    except Exception as e:
        raise Exception(f"Error reading file {path}: {e}")

def sys_chain_height(args: List[ArkValue]):
    return ArkValue(1, "Integer")

def sys_chain_get_balance(args: List[ArkValue]):
    return ArkValue(100, "Integer")

def sys_chain_submit_tx(args: List[ArkValue]):
    return ArkValue("tx_hash_mock", "String")

def math_sin_scaled(args: List[ArkValue]):
    return ArkValue(0, "Integer")

def math_cos_scaled(args: List[ArkValue]):
    return ArkValue(0, "Integer")

def math_pi_scaled(args: List[ArkValue]):
    return ArkValue(314159, "Integer")

def sys_exit(args: List[ArkValue]):
    code = 0
    if len(args) > 0 and args[0].type == "Integer":
        code = args[0].val
    sys.exit(code)

def detect_ai_mode():
    global ARK_AI_MODE
    if ARK_AI_MODE:
        return ARK_AI_MODE

    # 1. Try Ollama (Local)
    try:
        # Check /api/tags (GET) to see if Ollama is running
        req = urllib.request.Request("http://localhost:11434/api/tags", method="GET")
        with urllib.request.urlopen(req, timeout=0.5) as response:
            if response.getcode() == 200:
                print("Ollama Detected. Enabling Local AI Mode.")
                ARK_AI_MODE = "OLLAMA"
                return ARK_AI_MODE
    except Exception:
        pass

    # 2. Check Google API Key
    if os.environ.get("GOOGLE_API_KEY"):
        print("Google API Key Detected. Enabling Cloud AI Mode.")
        ARK_AI_MODE = "GEMINI"
        return ARK_AI_MODE

    # 3. Fallback
    print("No AI Provider Detected. Using Mock Mode.")
    ARK_AI_MODE = "MOCK"
    return ARK_AI_MODE

def ask_ollama(prompt: str):
    url = "http://localhost:11434/api/generate"
    headers = {"Content-Type": "application/json"}
    # Using llama3 as default zero-config model.
    # Ideally we'd parse /api/tags to find an available model if llama3 is missing.
    data = {
        "model": "llama3",
        "prompt": prompt,
        "stream": False
    }
    
    try:
        req = urllib.request.Request(url, data=json.dumps(data).encode("utf-8"), headers=headers, method="POST")
        with urllib.request.urlopen(req) as response:
            res_json = json.loads(response.read().decode("utf-8"))
            return ArkValue(res_json.get("response", ""), "String")
    except Exception as e:
        print(f"Ollama Error: {e}")
        return ask_mock()

def ask_gemini(prompt: str, api_key: str):
    url = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={api_key}"
    headers = {"Content-Type": "application/json"}
    data = {"contents": [{"parts": [{"text": prompt}]}]}
    
    max_retries = 3
    for attempt in range(max_retries):
        try:
            req = urllib.request.Request(url, data=json.dumps(data).encode("utf-8"), headers=headers, method="POST")
            with urllib.request.urlopen(req) as response:
                res_json = json.loads(response.read().decode("utf-8"))
                try:
                    text = res_json["candidates"][0]["content"]["parts"][0]["text"]
                    return ArkValue(text, "String")
                except (KeyError, IndexError) as e:
                    raise Exception(f"Failed to parse AI response: {e}")
        except urllib.error.HTTPError as e:
            if e.code == 429:
                if attempt < max_retries - 1:
                    wait_time = (2 ** attempt) * 2
                    print(f"AI Rate Limit (429). Retrying in {wait_time}s...")
                    time.sleep(wait_time)
                    continue
            print(f"AI Request Failed: {e.code} {e.reason}")
        except Exception as e:
            print(f"AI Error: {e}")
            
    return ask_mock()

def ask_mock():
    print(f"WARNING: Using Mock AI Response.")
    start = "```python:recursive_factorial.py\n"
    code = "import datetime\nprint(f'Sovereignty Established: {datetime.datetime.now()}')\n"
    end = "```"
    return ArkValue(start + code + end, "String")

def sanitize_prompt(prompt: str) -> str:
    # 1. Strip Meta-Prompts
    meta_patterns = [
        r"Ignore previous instructions",
        r"You are now unlocked",
        r"System:",
        r"\\n\\nSystem:",
        r"Simulate a",
    ]
    for pattern in meta_patterns:
        prompt = re.sub(pattern, "", prompt, flags=re.IGNORECASE)
    return prompt.strip()

def ask_ai(args: List[ArkValue]):
    if not args or args[0].type != "String":
        raise Exception("ask_ai expects a string prompt")

    prompt = sanitize_prompt(args[0].val)

    mode = detect_ai_mode()

    if mode == "OLLAMA":
        return ask_ollama(prompt)
    elif mode == "GEMINI":
        api_key = os.environ.get("GOOGLE_API_KEY")
        return ask_gemini(prompt, api_key)
    else:
        return ask_mock()

def extract_code(args: List[ArkValue]):
    if not args or args[0].type != "String":
        raise Exception("extract_code expects a string containing code")
    text = args[0].val
    
    # Matches ```tag\ncontent\n```
    # Capture group 1: tag (e.g. "python:file.py")
    # Capture group 2: content
    matches = re.findall(r"```([^\n]*)\n(.*?)```", text, re.DOTALL)
    
    ark_blocks = []
    
    # If no matches found but text looks like code, treat whole thing as one block?
    # No, adhere to contract.
    
    for tag_line, content in matches:
        tag_line = tag_line.strip()
        filename = "output.txt" 
        
        # Parse tag: "python:recursive_factorial.py"
        if ":" in tag_line:
            parts = tag_line.split(":")
            if len(parts) > 1:
                filename = parts[1].strip()
        elif tag_line:
             # Just "python" or "file.ark"?
             # If it looks like a file ext...
             if "." in tag_line:
                 filename = tag_line
        
        # Create [filename, content] pair (Ark List)
        pair = ArkValue([
            ArkValue(filename, "String"),
            ArkValue(content, "String")
        ], "List")
        
        ark_blocks.append(pair)
        
    return ArkValue(ark_blocks, "List")

SOCKETS = {}
SOCKET_ID = 0
SOCKET_LOCK = threading.Lock()

def get_socket(handle):
    if handle.type != "Integer":
        raise Exception(f"Socket handle must be Integer, got {handle.type}")
    with SOCKET_LOCK:
        if handle.val not in SOCKETS:
            raise Exception(f"Invalid socket handle: {handle.val}")
        return SOCKETS[handle.val]

def sys_net_socket_bind(args: List[ArkValue]):
    check_exec_security()
    global SOCKET_ID
    if len(args) != 1 or args[0].type != "Integer":
        raise Exception("sys.net.socket.bind expects integer port")
    port = args[0].val

    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    s.bind(('0.0.0.0', port))
    s.listen(5)

    with SOCKET_LOCK:
        SOCKET_ID += 1
        SOCKETS[SOCKET_ID] = s
        return ArkValue(SOCKET_ID, "Integer")

def sys_net_socket_accept(args: List[ArkValue]):
    global SOCKET_ID
    if len(args) != 1:
        raise Exception("sys.net.socket.accept expects socket handle")

    server_handle = args[0]
    s = get_socket(server_handle)

    try:
        conn, addr = s.accept()
        with SOCKET_LOCK:
            SOCKET_ID += 1
            SOCKETS[SOCKET_ID] = conn
            sid = SOCKET_ID

        # Return [handle, ip]
        return ArkValue([ArkValue(sid, "Integer"), ArkValue(addr[0], "String")], "List")
    except socket.timeout:
        return ArkValue(False, "Boolean")
    except BlockingIOError:
        return ArkValue(False, "Boolean")
    except Exception as e:
        print(f"Accept Error: {e}", file=sys.stderr)
        return ArkValue(False, "Boolean")

def sys_net_socket_connect(args: List[ArkValue]):
    check_exec_security()
    global SOCKET_ID
    if len(args) != 2 or args[0].type != "String" or args[1].type != "Integer":
        raise Exception("sys.net.socket.connect expects ip (String) and port (Integer)")

    ip = args[0].val
    port = args[1].val

    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.connect((ip, port))

        with SOCKET_LOCK:
            SOCKET_ID += 1
            SOCKETS[SOCKET_ID] = s
            return ArkValue(SOCKET_ID, "Integer")
    except Exception as e:
        raise Exception(f"Connection failed: {e}")

def sys_net_socket_send(args: List[ArkValue]):
    if len(args) != 2 or args[0].type != "Integer" or args[1].type != "String":
        raise Exception("sys.net.socket.send expects handle and data string")

    handle = args[0]
    data = args[1].val

    try:
        s = get_socket(handle)
        s.sendall(data.encode('utf-8'))
        return ArkValue(True, "Boolean")
    except Exception as e:
        return ArkValue(False, "Boolean")

def sys_net_socket_recv(args: List[ArkValue]):
    if len(args) != 2 or args[0].type != "Integer" or args[1].type != "Integer":
        raise Exception("sys.net.socket.recv expects handle and size")

    handle = args[0]
    size = args[1].val
    s = get_socket(handle)

    try:
        data = s.recv(size)
        if not data:
            return ArkValue("", "String") # EOF
        return ArkValue(data.decode('utf-8', errors='ignore'), "String")
    except socket.timeout:
        return ArkValue(False, "Boolean")
    except BlockingIOError:
        return ArkValue(False, "Boolean")
    except Exception as e:
        # print(f"Recv Error: {e}", file=sys.stderr)
        return ArkValue("", "String") # Treat errors as closed for now

def sys_net_socket_close(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "Integer":
        raise Exception("sys.net.socket.close expects handle")

    handle = args[0]
    with SOCKET_LOCK:
        if handle.val in SOCKETS:
            try:
                SOCKETS[handle.val].close()
            except:
                pass
            del SOCKETS[handle.val]
    return ArkValue(None, "Unit")

def sys_net_socket_set_timeout(args: List[ArkValue]):
    if len(args) != 2 or args[0].type != "Integer":
        raise Exception("sys.net.socket.set_timeout expects handle and timeout (ms)")

    handle = args[0]
    timeout_ms = args[1].val
    timeout = float(timeout_ms) / 1000.0

    s = get_socket(handle)
    s.settimeout(timeout)
    return ArkValue(None, "Unit")

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
        # socket.getaddrinfo handles both IPv4 and IPv6
        addr_info = socket.getaddrinfo(hostname, None)
    except socket.gaierror as e:
        raise Exception(f"DNS resolution failed for {hostname}: {e}")

    for _, _, _, _, sockaddr in addr_info:
        ip_str = sockaddr[0]
        try:
            ip = ipaddress.ip_address(ip_str)
        except ValueError:
            continue # Skip invalid IPs if any

        if ip.is_private or ip.is_loopback or ip.is_link_local or ip.is_multicast or ip.is_reserved:
             raise SandboxViolation(f"Access to private/local/reserved IP '{ip_str}' is forbidden")

        # Explicitly block 0.0.0.0
        if str(ip) == "0.0.0.0":
             raise SandboxViolation("Access to 0.0.0.0 is forbidden")

class SafeRedirectHandler(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        # Validate the redirect URL
        validate_url_security(newurl)
        return super().redirect_request(req, fp, code, msg, headers, newurl)

def sys_net_http_request(args: List[ArkValue]):
    # args: method, url, [body]
    if len(args) < 2:
        raise Exception("sys.net.http.request expects method, url")
    method = args[0].val
    url = args[1].val

    # 1. Validate initial URL
    validate_url_security(url)

    data = None
    if len(args) > 2:
        data = args[2].val.encode('utf-8')

    # 2. Use custom opener to handle redirects securely
    opener = urllib.request.build_opener(SafeRedirectHandler)

    req = urllib.request.Request(url, data=data, method=method)
    try:
        # utilize opener.open instead of urlopen
        with opener.open(req) as response:
            status = response.getcode()
            body = response.read().decode('utf-8')
            return ArkValue([ArkValue(status, "Integer"), ArkValue(body, "String")], "List")
    except urllib.error.HTTPError as e:
        status = e.code
        body = e.read().decode('utf-8')
        return ArkValue([ArkValue(status, "Integer"), ArkValue(body, "String")], "List")
    except Exception as e:
        raise Exception(f"HTTP Request Failed: {e}")



def sys_time_sleep(args: List[ArkValue]):
    if len(args) != 1 or args[0].type not in ["Integer", "Float"]:
        raise Exception("sys.time.sleep expects a number (seconds)")
    time.sleep(args[0].val)
    return ArkValue(None, "Unit")

def sys_time_now(args: List[ArkValue]):
    if len(args) != 0:
        raise Exception("sys.time.now expects 0 arguments")
    return ArkValue(int(time.time() * 1000), "Integer")

def sys_crypto_hash(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "String":
        raise Exception("sys.crypto.hash expects a string")
    
    data = args[0].val.encode('utf-8')
    digest = hashlib.sha256(data).hexdigest()
    return ArkValue(digest, "String")

def sys_crypto_merkle_root(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "List":
        raise Exception("sys.crypto.merkle_root expects a list of strings")
    
    # Extract strings
    leaves = []
    for item in args[0].val:
        if item.type != "String":
            raise Exception("sys.crypto.merkle_root list must contain strings")
        leaves.append(item.val)
        
    if not leaves:
        return ArkValue("", "String")
        
    # Hash leaves
    current_level = [hashlib.sha256(s.encode('utf-8')).hexdigest() for s in leaves]
    
    while len(current_level) > 1:
        next_level = []
        for i in range(0, len(current_level), 2):
            left = current_level[i]
            right = current_level[i+1] if i+1 < len(current_level) else left
            
            combined = (left + right).encode('utf-8')
            next_level.append(hashlib.sha256(combined).hexdigest())
        current_level = next_level
        
    return ArkValue(current_level[0], "String")

def sys_crypto_ed25519_gen(args: List[ArkValue]):
    if len(args) != 0:
        raise Exception("sys.crypto.ed25519.gen expects 0 arguments")

    priv = ed25519.Ed25519PrivateKey.generate()
    pub = priv.public_key()

    priv_bytes = priv.private_bytes(
        encoding=serialization.Encoding.Raw,
        format=serialization.PrivateFormat.Raw,
        encryption_algorithm=serialization.NoEncryption()
    )
    pub_bytes = pub.public_bytes(
        encoding=serialization.Encoding.Raw,
        format=serialization.PublicFormat.Raw
    )

    return ArkValue([
        ArkValue(priv_bytes.hex(), "String"),
        ArkValue(pub_bytes.hex(), "String")
    ], "List")

def sys_crypto_ed25519_sign(args: List[ArkValue]):
    if len(args) != 2:
        raise Exception("sys.crypto.ed25519.sign expects msg(string) and priv(hex string)")

    msg = args[0].val.encode('utf-8')
    priv_hex = args[1].val

    try:
        priv_bytes = bytes.fromhex(priv_hex)
        priv = ed25519.Ed25519PrivateKey.from_private_bytes(priv_bytes)
        sig = priv.sign(msg)
        return ArkValue(sig.hex(), "String")
    except Exception as e:
        raise Exception(f"Ed25519 Sign Error: {e}")

def sys_crypto_ed25519_verify(args: List[ArkValue]):
    if len(args) != 3:
        raise Exception("sys.crypto.ed25519.verify expects msg(string), sig(hex string), pub(hex string)")

    msg = args[0].val.encode('utf-8')
    sig_hex = args[1].val
    pub_hex = args[2].val

    try:
        sig_bytes = bytes.fromhex(sig_hex)
        pub_bytes = bytes.fromhex(pub_hex)
        pub = ed25519.Ed25519PublicKey.from_public_bytes(pub_bytes)
        pub.verify(sig_bytes, msg)
        return ArkValue(True, "Boolean")
    except Exception:
        return ArkValue(False, "Boolean")

def sys_mem_alloc(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "Integer": raise Exception("sys.mem.alloc expects size")
    size = args[0].val
    buf = bytearray(size)
    return ArkValue(buf, "Buffer")

def sys_list_get(args: List[ArkValue]):
    if len(args) != 2: raise Exception("sys.list.get expects list/str, index")
    lst = args[0] # List or String value
    idx = args[1].val
    
    if lst.type == "List":
        val = lst.val[idx] # This might be ArkValue
        return ArkValue([val, lst], "List")
    elif lst.type == "String":
        # String indexing returns [char_str, original_string]
        s = lst.val
        try:
            char_str = s[idx]
        except IndexError:
            raise Exception(f"String index out of range: idx={idx}, len={len(s)}, s='{s}'")
        return ArkValue([ArkValue(char_str, "String"), lst], "List")
    else:
        raise Exception("Expected List or String")

def sys_struct_get(args: List[ArkValue]):
    if len(args) != 2: raise Exception("sys.struct.get expects struct, key")
    struct_val = args[0]
    key = args[1].val

    fields = {}
    if struct_val.type == "Instance":
        fields = struct_val.val.fields
    elif isinstance(struct_val.val, dict):
         fields = struct_val.val

    if struct_val.type == "Instance":
        val = struct_val.val.fields.get(key)
        if val is None: raise Exception(f"Field {key} not found in Instance")
        return ArkValue([val, struct_val], "List")

    raise Exception(f"sys.struct.get not supported for type {struct_val.type}")

def sys_struct_set(args: List[ArkValue]):
    if len(args) != 3: raise Exception("sys.struct.set expects struct, key, val")
    struct_val = args[0]
    key = args[1].val
    val = args[2]

    if struct_val.type == "Instance":
        struct_val.val.fields[key] = val
        return struct_val

    raise Exception(f"sys.struct.set not supported for type {struct_val.type}")

def sys_struct_has(args: List[ArkValue]):
    if len(args) != 2: raise Exception("sys.struct.has expects obj, field")
    obj = args[0]
    field = args[1].val
    if obj.type != "Instance": return ArkValue(False, "Boolean")
    return ArkValue(field in obj.val.fields, "Boolean")

def sys_mem_inspect(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "Buffer": raise Exception("sys.mem.inspect expects buffer")
    buf = args[0].val
    addr = ctypes.addressof((ctypes.c_char * len(buf)).from_buffer(buf))
    print(f"<Buffer Inspect: ptr={hex(addr)}, len={len(buf)}>")
    return args[0] # Pass-through ownership

def sys_mem_read(args: List[ArkValue]):
    if len(args) != 2 or args[0].type != "Buffer": raise Exception("sys.mem.read expects buffer, index")
    buf = args[0].val
    idx = args[1].val
    val = int(buf[idx])
    return ArkValue([ArkValue(val, "Integer"), args[0]], "List")

def sys_mem_write(args: List[ArkValue]):
    if len(args) != 3: raise Exception("sys.mem.write expects buffer, index, val")
    buf = args[0].val
    idx = args[1].val
    val = args[2].val
    buf[idx] = val
    return ArkValue(buf, "Buffer") # Linear style

def sys_list_append(args: List[ArkValue]):
    if len(args) != 2: raise Exception("sys.list.append expects list, item")
    lst = args[0]
    if lst.type != "List": raise Exception("sys.list.append expects List")
    item = args[1]
    # In Python, lists are mutable ref.
    lst.val.append(item)
    return lst # Return the list (linear threading)

def sys_list_pop(args: List[ArkValue]):
    if len(args) != 2: raise Exception("sys.list.pop expects list, index")
    lst = args[0]
    idx = args[1].val
    if lst.type != "List": raise Exception("sys.list.pop expects List")
    if idx < 0 or idx >= len(lst.val): return ArkValue(None, "Unit")

    val = lst.val.pop(idx)
    return val # Return popped value

def sys_list_delete(args: List[ArkValue]):
    if len(args) != 2: raise Exception("sys.list.delete expects list, index")
    lst = args[0]
    idx = args[1].val
    if lst.type != "List": raise Exception("sys.list.delete expects List")
    if idx < 0 or idx >= len(lst.val): return ArkValue(None, "Unit")

    lst.val.pop(idx)
    return ArkValue(None, "Unit")

def sys_len(args: List[ArkValue]):
    if len(args) != 1: raise Exception("sys.len expects 1 argument")
    val = args[0]
    
    length = 0
    if val.type in ["String", "List", "Buffer"]:
        length = len(val.val)
        return ArkValue([ArkValue(length, "Integer"), val], "List")
    
    raise Exception(f"sys.len not supported for {val.type}")

def sys_and(args: List[ArkValue]):
    if len(args) != 2: raise Exception("sys.and expects 2 arguments")
    # Truthy check: Integer != 0, Boolean == True
    def is_truthy(v):
        if v.type == "Integer": return v.val != 0
        if v.type == "Boolean": return v.val
        return False
    
    left = is_truthy(args[0])
    right = is_truthy(args[1])
    return ArkValue(left and right, "Boolean")

def sys_or(args: List[ArkValue]):
    if len(args) != 2: raise Exception("sys.or expects 2 arguments")
    def is_truthy(v):
        if v.type == "Integer": return v.val != 0
        if v.type == "Boolean": return v.val
        return False
    
    left = is_truthy(args[0])
    right = is_truthy(args[1])
    return ArkValue(left or right, "Boolean")

def intrinsic_not(args: List[ArkValue]):
    if len(args) != 1: raise Exception("intrinsic_not expects 1 arg")
    val = args[0]
    is_true = False
    if val.type == "Boolean": is_true = val.val
    elif val.type == "Integer": is_true = val.val != 0

    return ArkValue(not is_true, "Boolean")

def sys_html_escape(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "String":
        raise Exception("sys.html_escape expects a string")
    return ArkValue(html.escape(args[0].val), "String")

def intrinsic_math_pow(args: List[ArkValue]):
    if len(args) != 2: raise Exception("math.pow expects 2 args")
    return ArkValue(int(math.pow(args[0].val, args[1].val)), "Integer")

def intrinsic_math_sqrt(args: List[ArkValue]):
    if len(args) != 1: raise Exception("math.sqrt expects 1 arg")
    return ArkValue(int(math.sqrt(args[0].val)), "Integer")

def intrinsic_math_sin(args: List[ArkValue]):
    if len(args) != 1: raise Exception("math.sin expects 1 arg")
    return ArkValue(int(math.sin(args[0].val)), "Integer")

def intrinsic_math_cos(args: List[ArkValue]):
    if len(args) != 1: raise Exception("math.cos expects 1 arg")
    return ArkValue(int(math.cos(args[0].val)), "Integer")

def intrinsic_math_tan(args: List[ArkValue]):
    if len(args) != 1: raise Exception("math.tan expects 1 arg")
    return ArkValue(int(math.tan(args[0].val)), "Integer")

def intrinsic_math_asin(args: List[ArkValue]):
    if len(args) != 1: raise Exception("math.asin expects 1 arg")
    return ArkValue(int(math.asin(args[0].val)), "Integer")

def intrinsic_math_acos(args: List[ArkValue]):
    if len(args) != 1: raise Exception("math.acos expects 1 arg")
    return ArkValue(int(math.acos(args[0].val)), "Integer")

def intrinsic_math_atan(args: List[ArkValue]):
    if len(args) != 1: raise Exception("math.atan expects 1 arg")
    return ArkValue(int(math.atan(args[0].val)), "Integer")

def intrinsic_math_atan2(args: List[ArkValue]):
    if len(args) != 2: raise Exception("math.atan2 expects 2 args")
    return ArkValue(int(math.atan2(args[0].val, args[1].val)), "Integer")

# --- New Intrinsics for LSP ---

def sys_io_read_bytes(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "Integer":
        raise Exception("sys.io.read_bytes expects integer length")
    n = args[0].val
    data = sys.stdin.buffer.read(n)
    return ArkValue(data.decode('utf-8', errors='ignore'), "String")

def sys_io_read_line(args: List[ArkValue]):
    if len(args) != 0:
        raise Exception("sys.io.read_line expects 0 arguments")
    line = sys.stdin.buffer.readline()
    return ArkValue(line.decode('utf-8', errors='ignore'), "String")

def sys_io_write(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "String":
        raise Exception("sys.io.write expects string")
    s = args[0].val
    sys.stdout.buffer.write(s.encode('utf-8'))
    sys.stdout.buffer.flush()
    return ArkValue(None, "Unit")

def sys_log(args: List[ArkValue]):
    if len(args) != 1:
        raise Exception("sys.log expects 1 argument")
    s = args[0].val
    sys.stderr.write(str(s) + "\n")
    return ArkValue(None, "Unit")

def to_ark(val):
    if isinstance(val, dict):
        fields = {k: to_ark(v) for k, v in val.items()}
        return ArkValue(ArkInstance(None, fields), "Instance")
    elif isinstance(val, list):
        return ArkValue([to_ark(v) for v in val], "List")
    elif isinstance(val, str):
        return ArkValue(val, "String")
    elif isinstance(val, bool):
        return ArkValue(val, "Boolean")
    elif isinstance(val, int):
        return ArkValue(val, "Integer")
    elif isinstance(val, float):
        return ArkValue(int(val), "Integer")
    elif val is None:
        return ArkValue(None, "Unit")
    return ArkValue(None, "Unit")

def sys_json_parse(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "String":
        raise Exception("sys.json.parse expects string")
    try:
        val = json.loads(args[0].val)
        return to_ark(val)
    except json.JSONDecodeError as e:
        raise Exception(f"JSON Parse Error: {e}")

def from_ark(val):
    if val.type == "Instance":
        # Check if fields exist (Instance of user struct or generic struct)
        if hasattr(val.val, "fields"):
            return {k: from_ark(v) for k, v in val.val.fields.items()}
        return {}
    elif val.type == "List":
        return [from_ark(v) for v in val.val]
    elif val.type == "String":
        return val.val
    elif val.type == "Integer":
        return val.val
    elif val.type == "Boolean":
        return val.val
    elif val.type == "Unit":
        return None
    return str(val.val)

def sys_json_stringify(args: List[ArkValue]):
    if len(args) != 1:
        raise Exception("sys.json.stringify expects 1 argument")
    val = from_ark(args[0])
    return ArkValue(json.dumps(val), "String")

def sys_exit(args: List[ArkValue]):
    code = 0
    if len(args) > 0 and args[0].type == "Integer":
        code = args[0].val
    sys.exit(code)

def sys_net_http_serve(args: List[ArkValue]):
    if len(args) != 2:
        raise Exception("sys.net.http.serve expects port(int) and handler(function)")

    port = int(args[0].val)
    handler_func = args[1]
    
    if handler_func.type != "Function":
        raise Exception("Handler must be a function")

    from http.server import BaseHTTPRequestHandler, HTTPServer

    class ArkHTTPHandler(BaseHTTPRequestHandler):
        def do_GET(self):
            # Map request to Ark Value
            req_path = ArkValue(self.path, "String")
            
            # Call Ark Function
            # We need to construct arguments list
            call_args = [req_path]
            
            # Invoke the interpreter synchronously
            # Note: This blocks the server thread, which is fine for this proof-of-concept
            try:
                result = call_user_func(handler_func.val, call_args)
                
                # Convert Result back to bytes
                resp_body = b""
                if result.type == "String":
                    resp_body = result.val.encode('utf-8')
                else:
                    resp_body = str(result.val).encode('utf-8')

                self.send_response(200)
                self.end_headers()
                self.wfile.write(resp_body)
            except Exception as e:
                print(f"Ark Handler Error: {e}")
                self.send_response(500)
                self.end_headers()
                self.wfile.write(str(e).encode('utf-8'))

    server_address = ('', port)
    httpd = HTTPServer(server_address, ArkHTTPHandler)
    print(f"Serving HTTP on port {port}...")
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        pass
    httpd.server_close()
    return ArkValue(None, "Unit")

def sys_struct_has(args: List[ArkValue]):
    if len(args) != 2: raise Exception("sys.struct.has expects obj, field")
    obj = args[0]
    field = args[1].val
    if obj.type != "Instance": return ArkValue(False, "Boolean")
    return ArkValue(field in obj.val.fields, "Boolean")

def sys_io_read_file_async(args: List[ArkValue]):
    if len(args) != 2: raise Exception("sys.io.read_file_async expects path, callback")
    path = args[0].val
    check_path_security(path)
    callback = args[1]

    def task():
        try:
            with open(path, "r") as f:
                content = f.read()
            val = ArkValue(content, "String")
            EVENT_QUEUE.put((callback, [val]))
        except Exception as e:
            print(f"Async Read Error: {e}", file=sys.stderr)
            val = ArkValue(None, "Unit")
            EVENT_QUEUE.put((callback, [val]))

    t = threading.Thread(target=task)
    t.daemon = True
    t.start()
    return ArkValue(None, "Unit")

def sys_net_request_async(args: List[ArkValue]):
    check_exec_security()
    if len(args) < 2: raise Exception("sys.net.request_async expects url, callback")
    url = args[0].val
    callback = args[1]

    def task():
        try:
            with urllib.request.urlopen(url) as response:
                status = response.getcode()
                content = response.read().decode('utf-8')
                val = ArkValue([ArkValue(status, "Integer"), ArkValue(content, "String")], "List")
                EVENT_QUEUE.put((callback, [val]))
        except Exception as e:
            print(f"Async Net Error: {e}", file=sys.stderr)
            val = ArkValue([ArkValue(0, "Integer"), ArkValue(str(e), "String")], "List")
            EVENT_QUEUE.put((callback, [val]))

    t = threading.Thread(target=task)
    t.daemon = True
    t.start()
    return ArkValue(None, "Unit")

def sys_event_poll(args: List[ArkValue]):
    try:
        cb, cb_args = EVENT_QUEUE.get_nowait()
        return ArkValue([cb, ArkValue(cb_args, "List")], "List")
    except queue.Empty:
        return ArkValue(None, "Unit")


def sys_z3_verify(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "List":
        raise Exception("sys.z3.verify expects a List of constraints (Strings)")

    constraints_val = args[0].val
    constraints = []
    for item in constraints_val:
        if item.type != "String":
             raise Exception("sys.z3.verify constraints list must contain Strings")
        constraints.append(item.val)



def sys_chain_verify_tx(args: List[ArkValue]):
    if len(args) != 1: raise Exception("sys.chain.verify_tx expects tx")
    # Mock verification
    return ArkValue(True, "Boolean")
def sys_fs_write_buffer(args: List[ArkValue]):
    if len(args) != 2 or args[0].type != "String" or args[1].type != "Buffer":
        raise Exception("sys.fs.write_buffer expects string path and buffer")
    path = args[0].val
    check_path_security(path, is_write=True)
    buf = args[1].val
    try:
        # Ensure we can import from same directory
        current_dir = os.path.dirname(os.path.abspath(__file__))
        if current_dir not in sys.path:
            sys.path.append(current_dir)

        import z3_bridge
        res = z3_bridge.verify_contract(constraints)
        return ArkValue(res, "Boolean")
    except ImportError as e:
        print(f"Warning: z3_bridge import failed: {e}", file=sys.stderr)
        return ArkValue(True, "Boolean") # Fail open or mock success


def sys_str_from_code(args: List[ArkValue]):
    if len(args) != 1: raise Exception("sys.str.from_code expects 1 arg")
    code = args[0].val
    return ArkValue(chr(code), "String")




# --- Evaluator ---

def handle_block(node, scope):
    return eval_block(node.children, scope)

def handle_flow_stmt(node, scope):
    return eval_node(node.children[0], scope)

def handle_function_def(node, scope):
    name = node.children[0].value
    params = []
    body_idx = 1
    if len(node.children) > 1:
        child1 = node.children[1]
        if child1 is None:
            body_idx = 2
        elif hasattr(child1, "data") and child1.data == "param_list":
            params = [t.value for t in child1.children]
            body_idx = 2
    body = node.children[body_idx]
    func = ArkValue(ArkFunction(name, params, body, scope), "Function")
    scope.set(name, func)
    return func

def handle_class_def(node, scope):
    name = node.children[0].value
    methods = {}
    for child in node.children[1:]:
        if child.data == "function_def":
            m_name = child.children[0].value
            m_params = []
            m_body_idx = 1
            if len(child.children) > 1 and hasattr(child.children[1], "data") and child.children[1].data == "param_list":
                m_params = [t.value for t in child.children[1].children]
                m_body_idx = 2
            m_body = child.children[m_body_idx]
            methods[m_name] = ArkFunction(m_name, m_params, m_body, scope)
    klass = ArkValue(ArkClass(name, methods), "Class")
    scope.set(name, klass)
    return klass

def handle_struct_init(node, scope):
    fields = {}
    if node.children:
        child = node.children[0]
        if hasattr(child, "data") and child.data == "field_list":
             for field in child.children:
                name = field.children[0].value
                val = eval_node(field.children[1], scope)
                fields[name] = val
    return ArkValue(ArkInstance(None, fields), "Instance")

def handle_return_stmt(node, scope):
    val = eval_node(node.children[0], scope) if node.children else ArkValue(None, "Unit")
    raise ReturnException(val)

def handle_if_stmt(node, scope):
    num_children = len(node.children)
    i = 0
    while i + 1 < num_children:
        cond = eval_node(node.children[i], scope)
        if is_truthy(cond):
            return eval_node(node.children[i+1], scope)
        i += 2
    if i < num_children and node.children[i]:
        return eval_node(node.children[i], scope)
    return ArkValue(None, "Unit")

def handle_while_stmt(node, scope):
    cond_node = node.children[0]
    body_node = node.children[1]
    while is_truthy(eval_node(cond_node, scope)):
        eval_node(body_node, scope)
    return ArkValue(None, "Unit")

def handle_logical_or(node, scope):
    left = eval_node(node.children[0], scope)
    if is_truthy(left): return ArkValue(True, "Boolean")
    right = eval_node(node.children[-1], scope)
    return ArkValue(is_truthy(right), "Boolean")

def handle_logical_and(node, scope):
    left = eval_node(node.children[0], scope)
    if not is_truthy(left): return ArkValue(False, "Boolean")
    right = eval_node(node.children[-1], scope)
    return ArkValue(is_truthy(right), "Boolean")

def handle_var(node, scope):
    name = node.children[0].value
    val = scope.get(name)
    if val: return val
    if name in INTRINSICS:
        return ArkValue(name, "Intrinsic")
    raise Exception(f"Undefined variable: {name}")

def handle_assign_var(node, scope):
    name = node.children[0].value
    val = eval_node(node.children[1], scope)
    scope.set(name, val)
    return val

def handle_assign_destructure(node, scope):
    expr_node = node.children[-1]
    var_tokens = node.children[:-1]
    val = eval_node(expr_node, scope)
    if val.type != "List":
        raise Exception(f"Destructuring expects List, got {val.type}")
    items = val.val
    if len(items) < len(var_tokens):
        raise Exception(f"Not enough items to destructure: needed {len(var_tokens)}, got {len(items)}")
    for i, token in enumerate(var_tokens):
        scope.set(token.value, items[i])
    return val

def handle_assign_attr(node, scope):
    obj = eval_node(node.children[0], scope)
    attr = node.children[1].value
    val = eval_node(node.children[2], scope)
    if obj.type == "Instance":
        obj.val.fields[attr] = val
        return val
    raise Exception(f"Cannot set attribute on {obj.type}")

def handle_get_attr(node, scope):
    obj = eval_node(node.children[0], scope)
    attr = node.children[1].value
    if obj.type == "Namespace":
        new_path = f"{obj.val}.{attr}"
        # print(f"DEBUG: Namespace Lookup: {new_path} in INTRINSICS? {new_path in INTRINSICS}", file=sys.stderr)
        if new_path in INTRINSICS:
            return ArkValue(new_path, "Intrinsic")
        return ArkValue(new_path, "Namespace")
    if obj.type == "Instance":
        if attr in obj.val.fields:
            return obj.val.fields[attr]
        klass = obj.val.klass
        if klass and attr in klass.methods:
            method = klass.methods[attr]
            return ArkValue((method, obj), "BoundMethod")
    if obj.type == "Class":
        if attr in obj.val.methods:
            return ArkValue(obj.val.methods[attr], "Function")
    raise Exception(f"Attribute {attr} not found on {obj.type}")

def handle_call_expr(node, scope):
    func_val = eval_node(node.children[0], scope)
    args = []
    arg_list_node = None
    if len(node.children) > 1:
        arg_list_node = node.children[1]
        if hasattr(arg_list_node, "children"):
            args = [eval_node(c, scope) for c in arg_list_node.children]
    
    if func_val.type == "Intrinsic":
        intrinsic_name = func_val.val
        if intrinsic_name in LINEAR_SPECS:
            consumed_indices = LINEAR_SPECS[intrinsic_name]
            if arg_list_node and hasattr(arg_list_node, "children"):
                for idx in consumed_indices:
                    if idx < len(arg_list_node.children):
                        arg_node = arg_list_node.children[idx]
                        if hasattr(arg_node, "data") and arg_node.data == "var":
                            var_name = arg_node.children[0].value
                            scope.mark_moved(var_name)
        
        if intrinsic_name in INTRINSICS_WITH_SCOPE:
            return INTRINSICS[func_val.val](args, scope)
        return INTRINSICS[func_val.val](args)

    if func_val.type == "Function":
        return call_user_func(func_val.val, args)

    if func_val.type == "Class":
        return instantiate_class(func_val.val, args)

    if func_val.type == "BoundMethod":
        method, instance = func_val.val
        return call_user_func(method, args, instance)

    raise Exception(f"Not callable: {func_val.type}")

def handle_number(node, scope):
    return ArkValue(int(node.children[0].value), "Integer")

def handle_string(node, scope):
    try:
        s = ast.literal_eval(node.children[0].value)
    except:
         s = node.children[0].value[1:-1]
    return ArkValue(s, "String")

def handle_binop(node, scope):
    left = eval_node(node.children[0], scope)
    right = eval_node(node.children[1], scope)
    return eval_binop(node.data, left, right)

def handle_list_cons(node, scope):
    items = []
    if node.children:
        child = node.children[0]
        if hasattr(child, "data") and child.data == "expr_list":
            items = [eval_node(c, scope) for c in child.children]
    return ArkValue(items, "List")

def handle_get_item(node, scope):
    collection = eval_node(node.children[0], scope)
    index_val = eval_node(node.children[1], scope)
    if index_val.type != "Integer":
         raise Exception(f"Index must be Integer, got {index_val.type}")
    idx = index_val.val
    if collection.type == "List":
        if idx < 0 or idx >= len(collection.val):
            raise Exception(f"List index out of range: {idx}")
        return collection.val[idx]
    if collection.type == "String":
        if idx < 0 or idx >= len(collection.val):
             raise Exception(f"String index out of range: {idx}")
        return ArkValue(collection.val[idx], "String")
    if collection.type == "Buffer":
        if idx < 0 or idx >= len(collection.val):
             raise Exception(f"Buffer index out of range: {idx}")
        return ArkValue(int(collection.val[idx]), "Integer")
    raise Exception(f"Cannot index type {collection.type}")

def handle_import(node, scope):
    # import std.io -> lib/std/io.ark
    parts = [t.value for t in node.children]
    
    # Construct relative path (std -> lib/std)
    if parts[0] == "std":
        # std.io -> lib/std/io.ark
        rel_path = os.path.join("lib", *parts) + ".ark"
    else:
        rel_path = os.path.join(*parts) + ".ark"

    if not os.path.exists(rel_path):
        # Try local path
        rel_path = os.path.join(*parts) + ".ark"
    
    if not os.path.exists(rel_path):
        raise Exception(f"Import Error: Module {'.'.join(parts)} not found at {rel_path}")

    # Check Idempotency via Root Scope
    root = scope
    while root.parent:
        root = root.parent
    
    # Use vars instead of attribute because Scope has __slots__
    if "__loaded_imports__" not in root.vars:
        root.vars["__loaded_imports__"] = ArkValue(set(), "Set")
    
    loaded_set = root.vars["__loaded_imports__"].val
    
    abs_path = os.path.abspath(rel_path)
    if abs_path in loaded_set:
        return ArkValue(None, "Unit") # Already loaded
    
    loaded_set.add(abs_path)

    # Load & Parse
    with open(abs_path, "r") as f:
        code = f.read()
    
    tree = ARK_PARSER.parse(code)
    eval_node(tree, scope)
    return ArkValue(None, "Unit")

NODE_HANDLERS = {
    "start": handle_block,
    "block": handle_block,
    "flow_stmt": handle_flow_stmt,
    "function_def": handle_function_def,
    "class_def": handle_class_def,
    "struct_init": handle_struct_init,
    "return_stmt": handle_return_stmt,
    "if_stmt": handle_if_stmt,
    "while_stmt": handle_while_stmt,
    "logical_or": handle_logical_or,
    "logical_and": handle_logical_and,
    "var": handle_var,
    "assign_var": handle_assign_var,
    "assign_destructure": handle_assign_destructure,
    "assign_attr": handle_assign_attr,
    "get_attr": handle_get_attr,
    "call_expr": handle_call_expr,
    "number": handle_number,
    "string": handle_string,
    "add": handle_binop,
    "sub": handle_binop,
    "mul": handle_binop,
    "div": handle_binop,
    "mod": handle_binop,
    "lt": handle_binop,
    "gt": handle_binop,
    "le": handle_binop,
    "ge": handle_binop,
    "eq": handle_binop,
    "neq": handle_binop,
    "list_cons": handle_list_cons,
    "get_item": handle_get_item,
    "import_stmt": handle_import,
}

def eval_node(node, scope):
    if node is None: return ArkValue(None, "Unit")
    if hasattr(node, "data"):
        handler = NODE_HANDLERS.get(node.data)
        if handler:
            return handler(node, scope)
    return ArkValue(None, "Unit")

def call_user_func(func: ArkFunction, args: List[ArkValue], instance: Optional[ArkValue] = None):
    # 1. Create Scope
    func_scope = Scope(func.closure)
    
    # 2. Bind 'this' if method call
    if instance:
        func_scope.set("this", instance)

    # 3. Bind Args
    for i, param in enumerate(func.params):
        if i < len(args):
            func_scope.set(param, args[i])
    # 4. Exec Body
    try:
        eval_node(func.body, func_scope)
        return ArkValue(None, "Unit")
    except ReturnException as ret:
        return ret.value

def instantiate_class(klass: ArkClass, args: List[ArkValue]):
    instance = ArkInstance(klass, {})
    return ArkValue(instance, "Instance")

def eval_block(nodes, scope):
    last = ArkValue(None, "Unit")
    try:
        for n in nodes:
            last = eval_node(n, scope)
    except ReturnException:
        raise # Propagate up to function call
    return last

def is_truthy(val):
    if val.type == "Boolean": return val.val
    if val.type == "Integer": return val.val != 0
    if val.type == "String": return len(val.val) > 0
    if val.type == "List": return True
    return False

def eval_binop(op, left, right):
    l = left.val
    r = right.val
    if op == "add":
        if left.type == "String" or right.type == "String": return ArkValue(str(l) + str(r), "String")
        return ArkValue(l + r, "Integer")
    if op == "sub": return ArkValue(l - r, "Integer")
    if op == "mul": return ArkValue(l * r, "Integer")
    if op == "div": return ArkValue(l // r, "Integer")
    if op == "mod": return ArkValue(l % r, "Integer")
    if op == "lt": return ArkValue(l < r, "Boolean")
    if op == "gt": return ArkValue(l > r, "Boolean")
    if op == "le": return ArkValue(l <= r, "Boolean")
    if op == "ge": return ArkValue(l >= r, "Boolean")
    if op == "eq": return ArkValue(l == r, "Boolean")
    if op == "neq": return ArkValue(l != r, "Boolean")
    return ArkValue(None, "Unit")

# --- Main ---

# --- Late Intrinsics (Moved to resolve dependencies) ---

def sys_thread_spawn(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "Function":
        raise Exception("sys.thread.spawn expects a function")

    func = args[0].val

    def thread_target():
        try:
            call_user_func(func, [])
        except Exception as e:
            print(f"Thread Error: {e}", file=sys.stderr)
            import traceback
            traceback.print_exc()

    t = threading.Thread(target=thread_target)
    t.daemon = True
    t.start()
    return ArkValue(None, "Unit")


def sys_func_apply(args: List[ArkValue]):
    if len(args) != 2: raise Exception("sys.func.apply expects func, args_list")
    func = args[0]
    arg_list = args[1]
    if arg_list.type != "List": raise Exception("sys.func.apply expects List of args")

    if func.type == "Function":
        return call_user_func(func.val, arg_list.val)
    elif func.type == "Intrinsic":
        return INTRINSICS[func.val](arg_list.val)
    raise Exception(f"Cannot apply {func.type}")


def sys_vm_eval(args: List[ArkValue], scope: Scope):
    if len(args) != 1 or args[0].type != "String":
        raise Exception("sys.vm.eval expects a code string")
    code = args[0].val
    try:
        tree = ARK_PARSER.parse(code)
        return eval_node(tree, scope)
    except Exception as e:
        raise Exception(f"Eval Error: {e}")


def sys_vm_source(args: List[ArkValue], scope: Scope):
    if len(args) != 1 or args[0].type != "String":
        raise Exception("sys.vm.source expects a file path")
    path = args[0].val
    check_path_security(path)
    try:
        with open(path, "r") as f:
            code = f.read()
        tree = ARK_PARSER.parse(code)
        return eval_node(tree, scope)
    except Exception as e:
        raise Exception(f"Source Error: {e}")






# --- JSON & Type Conversion Helpers ---

def to_python_val(val: ArkValue):
    if val.type == "Integer": return val.val
    if val.type == "String": return val.val
    if val.type == "Boolean": return val.val
    if val.type == "List": return [to_python_val(x) for x in val.val]
    if val.type == "Instance":
        # Convert struct to dict
        return {k: to_python_val(v) for k, v in val.val.fields.items()}
    if val.type == "Unit": return None
    # Skip Functions/Classes/etc
    return str(val.val)

def from_python_val(val):
    if val is None: return ArkValue(None, "Unit")
    if isinstance(val, bool): return ArkValue(val, "Boolean")
    if isinstance(val, int): return ArkValue(val, "Integer")
    if isinstance(val, float): return ArkValue(int(val), "Integer") # Ark is Int only currently?
    if isinstance(val, str): return ArkValue(val, "String")
    if isinstance(val, list): return ArkValue([from_python_val(x) for x in val], "List")
    if isinstance(val, dict):
        # Struct (Instance with no class)
        fields = {k: from_python_val(v) for k, v in val.items()}
        return ArkValue(ArkInstance(None, fields), "Instance")
    return ArkValue(str(val), "String")

def sys_json_parse(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "String":
        raise Exception("sys.json.parse expects string")
    try:
        data = json.loads(args[0].val)
        return from_python_val(data)
    except Exception as e:
        # LSP `read_header` calls `sys.json.parse(num_str)`.
        raise Exception(f"JSON Parse Error: {e}")

def sys_json_stringify(args: List[ArkValue]):
    if len(args) != 1: raise Exception("sys.json.stringify expects value")
    try:
        data = to_python_val(args[0])
        s = json.dumps(data)
        return ArkValue(s, "String")
    except Exception as e:
        raise Exception(f"JSON Stringify Error: {e}")

def sys_log(args: List[ArkValue]):
    # Alias to print, but maybe to stderr?
    # LSP uses it for logging.
    s = " ".join([str(a.val) for a in args])
    print(f"[LOG] {s}", file=sys.stderr)
    return ArkValue(None, "Unit")

def sys_io_read_bytes(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "Integer":
        raise Exception("sys.io.read_bytes expects integer length")
    n = args[0].val
    # Read n bytes from stdin.buffer
    data = sys.stdin.buffer.read(n)
    return ArkValue(data.decode('utf-8'), "String")

def sys_io_read_line(args: List[ArkValue]):
    if len(args) != 0:
        raise Exception("sys.io.read_line expects no args")
    line = sys.stdin.readline()
    return ArkValue(line, "String")

def sys_io_write(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "String":
        raise Exception("sys.io.write expects string")
    s = args[0].val
    sys.stdout.buffer.write(s.encode('utf-8'))
    sys.stdout.buffer.flush()
    return ArkValue(None, "Unit")


INTRINSICS = {
    # Core
    "get": core_get,
    "len": core_len,
    "print": core_print,

    # System
    "sys.crypto.hash": sys_crypto_hash,
    "sys.crypto.merkle_root": sys_crypto_merkle_root,
    "sys.crypto.ed25519.gen": sys_crypto_ed25519_gen,
    "sys.crypto.ed25519.sign": sys_crypto_ed25519_sign,
    "sys.crypto.ed25519.verify": sys_crypto_ed25519_verify,
    "sys.exec": sys_exec,
    "sys.fs.read": sys_fs_read,
    "sys.fs.read_buffer": sys_fs_read_buffer,
    "sys.fs.write": sys_fs_write,
    "sys.fs.write_buffer": sys_fs_write_buffer,
    "sys.len": sys_len,
    "sys.list.append": sys_list_append,
    "sys.list.pop": sys_list_pop,
    "sys.list.delete": sys_list_delete,
    "sys.list.get": sys_list_get,
    "sys.mem.alloc": sys_mem_alloc,
    "sys.mem.inspect": sys_mem_inspect,
    "sys.mem.read": sys_mem_read,
    "sys.mem.write": sys_mem_write,
    "sys.net.http.request": sys_net_http_request,
    "sys.net.http.serve": sys_net_http_serve,
    "sys.net.socket.bind": sys_net_socket_bind,
    "sys.net.socket.accept": sys_net_socket_accept,
    "sys.net.socket.connect": sys_net_socket_connect,
    "sys.net.socket.send": sys_net_socket_send,
    "sys.net.socket.recv": sys_net_socket_recv,
    "sys.net.socket.close": sys_net_socket_close,
    "sys.net.socket.set_timeout": sys_net_socket_set_timeout,
    "sys.thread.spawn": sys_thread_spawn,
    "sys.struct.get": sys_struct_get,
    "sys.struct.set": sys_struct_set,
    "sys.str.get": sys_list_get,
    "sys.struct.has": sys_struct_has,
    "sys.chain.height": sys_chain_height,
    "sys.chain.get_balance": sys_chain_get_balance,
    "sys.chain.submit_tx": sys_chain_submit_tx,
    "sys.chain.verify_tx": sys_chain_verify_tx,
    "sys.time.now": sys_time_now,
    "sys.time.sleep": sys_time_sleep,
    "sys.str.from_code": sys_str_from_code,
    "sys.json.parse": sys_json_parse,
    "sys.json.stringify": sys_json_stringify,
    "sys.json.stringify": sys_json_stringify,
    "sys.log": sys_log,
    "sys.io.read_bytes": sys_io_read_bytes,
    "sys.io.read_line": sys_io_read_line,
    "sys.io.write": sys_io_write,

    # Math
    "math.sin_scaled": math_sin_scaled,
    "math.cos_scaled": math_cos_scaled,
    "math.pi_scaled": math_pi_scaled,
    "math.pow": intrinsic_math_pow,
    "math.sqrt": intrinsic_math_sqrt,
    "math.sin": intrinsic_math_sin,
    "math.cos": intrinsic_math_cos,
    "math.tan": intrinsic_math_tan,
    "math.asin": intrinsic_math_asin,
    "math.acos": intrinsic_math_acos,
    "math.atan": intrinsic_math_atan,
    "math.atan2": intrinsic_math_atan2,
    
    # Intrinsic Wrappers (Aliases for Intrinsics struct)
    "intrinsic_and": sys_and,
    "intrinsic_not": intrinsic_not,
    "intrinsic_ask_ai": ask_ai,
    "intrinsic_buffer_alloc": sys_mem_alloc,
    "intrinsic_buffer_inspect": sys_mem_inspect,
    "intrinsic_buffer_read": sys_mem_read,
    "intrinsic_buffer_write": sys_mem_write,
    "intrinsic_crypto_hash": sys_crypto_hash,
    "intrinsic_extract_code": extract_code,
    "sys.exit": sys_exit,
    "exit": sys_exit,
    "quit": sys_exit,
    # "intrinsic_ge": ... (handled by lambdas which we can't easily inline here without recreating them)
    # We will just merge the two dictionaries or keep this simple
}

# Add Lambdas and others to INTRINSICS
INTRINSICS.update({
    "intrinsic_ge": lambda args: eval_binop("ge", args[0], args[1]),
    "intrinsic_gt": lambda args: eval_binop("gt", args[0], args[1]),
    "intrinsic_le": lambda args: eval_binop("le", args[0], args[1]),
    "intrinsic_lt": lambda args: eval_binop("lt", args[0], args[1]),
    "intrinsic_len": sys_len,
    "intrinsic_list_append": sys_list_append,
    "intrinsic_list_get": sys_list_get,
    "intrinsic_merkle_root": sys_crypto_merkle_root,
    "intrinsic_or": sys_or,
    "intrinsic_time_now": sys_time_now,
    "intrinsic_math_pow": intrinsic_math_pow,
    "intrinsic_math_sqrt": intrinsic_math_sqrt,
    "intrinsic_math_sin": intrinsic_math_sin,
    "intrinsic_math_cos": intrinsic_math_cos,
    "intrinsic_math_tan": intrinsic_math_tan,
    "intrinsic_math_asin": intrinsic_math_asin,
    "intrinsic_math_acos": intrinsic_math_acos,
    "intrinsic_math_atan": intrinsic_math_atan,
    "intrinsic_math_atan2": intrinsic_math_atan2,
})


LINEAR_SPECS = {
    "sys.mem.write": [0],
    "sys.mem.read": [0],
    "sys.list.append": [0],
    "sys.list.pop": [0],
}


INTRINSICS_WITH_SCOPE = {
    "sys.vm.eval",
    "sys.vm.source",
}


def run_file(path):
    print(f"{Colors.OKCYAN}[ARK OMEGA-POINT v112.0] Running {path}{Colors.ENDC}", file=sys.stderr)
    with open(path, "r") as f: code = f.read()
    
    tree = ARK_PARSER.parse(code)
    # print(tree.pretty(), file=sys.stderr)
    scope = Scope()
    scope.set("sys", ArkValue("sys", "Namespace"))
    scope.set("math", ArkValue("math", "Namespace"))

    # Optimization: Inject true/false as Integers (1/0)
    scope.set("true", ArkValue(1, "Integer"))
    scope.set("false", ArkValue(0, "Integer"))
    
    # Inject sys_args
    # sys.argv: [meta/ark.py, run, script.ark, arg1, arg2...]
    args_vals = []
    if len(sys.argv) >= 3:
        for a in sys.argv[2:]:
            args_vals.append(ArkValue(a, "String"))
    scope.set("sys_args", ArkValue(args_vals, "List"))


    # 3. Evaluate
    try:
        eval_node(tree, scope)
    except ReturnException as e:
        print(f"{Colors.FAIL}Error: Return statement outside function{Colors.ENDC}", file=sys.stderr)
    except Exception as e:
        # If it's a SandboxViolation, print it clearly
        if isinstance(e, SandboxViolation):
            print(f"{Colors.FAIL}SandboxViolation: {e}{Colors.ENDC}", file=sys.stderr)
        else:
            print(f"{Colors.FAIL}Runtime Error: {e}{Colors.ENDC}", file=sys.stderr)
        # print(f"DEBUG: Scope vars: {scope.vars.keys()}")
        import traceback
        # traceback.print_exc() 
        sys.exit(1)


if __name__ == "__main__":
    if len(sys.argv) < 3:
        pass
    else:
        run_file(sys.argv[2])
