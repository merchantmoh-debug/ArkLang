"""
Ark Intrinsics — All built-in function implementations.

Extracted from ark.py (Phase 72: Structural Hardening).
Contains: intrinsic functions, INTRINSICS dict, LINEAR_SPECS, INTRINSICS_WITH_SCOPE.
"""
import sys
import os
import re
import time
import math
import json
import shlex
import subprocess
import hashlib
import ctypes
import html
import socket
import threading
import urllib.request
import urllib.error
import urllib.parse
import queue
import secrets
import hmac
from typing import List, Optional
from cryptography.hazmat.primitives.asymmetric import ed25519
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.ciphers.aead import AESGCM

try:
    from meta.ark_types import (
        ArkValue, UNIT_VALUE, ArkFunction, ArkClass, ArkInstance, Scope,
        ReturnException, RopeString
    )
    from meta.ark_security import (
        SandboxViolation, check_path_security, check_exec_security,
        validate_url_security, SafeRedirectHandler, check_capability, has_capability
    )
except ModuleNotFoundError:
    from ark_types import (
        ArkValue, UNIT_VALUE, ArkFunction, ArkClass, ArkInstance, Scope,
        ReturnException, RopeString
    )
    from ark_security import (
        SandboxViolation, check_path_security, check_exec_security,
        validate_url_security, SafeRedirectHandler, check_capability, has_capability
    )


# --- Global Event Queue ---
EVENT_QUEUE = queue.Queue()
ARK_AI_MODE = None


# ─── Core Intrinsics ─────────────────────────────────────────────────────────

def core_print(args: List[ArkValue]):
    print(*(arg.val for arg in args))
    return UNIT_VALUE

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
    if not isinstance(collection, (str, list, RopeString)):
        raise Exception("Collection must be a string or list")
    
    if 0 <= index < len(collection):
        if isinstance(collection, (str, RopeString)):
            return ArkValue(collection[index], "String")
        elif isinstance(collection, list):
            val = collection[index]
            if isinstance(val, ArkValue):
                return val
            return ArkValue(val, "Any")
    else:
        raise Exception("Index out of bounds")
    return UNIT_VALUE


# ─── Command Whitelist ────────────────────────────────────────────────────────

def load_whitelist():
    default_whitelist = {
        "ls", "grep", "cat", "echo", "python", "python3",
        "cargo", "rustc", "git", "date", "whoami", "pwd", "mkdir", "touch"
    }
    if os.path.exists("security.json"):
        try:
            with open("security.json", "r") as f:
                config = json.load(f)
                if "whitelist" in config:
                    return set(config["whitelist"])
        except Exception as e:
            print(f"Warning: Failed to load security.json: {e}", file=sys.stderr)
    return default_whitelist

COMMAND_WHITELIST = load_whitelist()


# ─── System Intrinsics ────────────────────────────────────────────────────────

def sys_exec(args: List[ArkValue]):
    if not args or args[0].type != "String":
        raise Exception("sys.exec expects a string command")
    
    command_str = args[0].val.strip()
    if not command_str:
        return ArkValue("", "String")

    try:
        cmd_args = shlex.split(command_str, posix=(os.name != 'nt'))
    except Exception as e:
        return ArkValue(f"Security Error: Failed to parse command: {e}", "String")

    if not cmd_args:
        return ArkValue("", "String")

    base_cmd = cmd_args[0]

    # Capability check: require 'exec' capability for non-whitelisted commands
    if not has_capability("exec") and not has_capability("all"):
        if base_cmd not in COMMAND_WHITELIST:
            raise SandboxViolation(f"Command '{base_cmd}' is not in the whitelist. Set ARK_CAPABILITIES=exec or ALLOW_DANGEROUS_LOCAL_EXECUTION=true to bypass.")

    try:
        result = subprocess.run(
            cmd_args, 
            shell=False, 
            capture_output=True, 
            text=True,
            timeout=10
        )
        
        output = result.stdout
        if result.stderr:
            output += "\nHelper: " + result.stderr
            
        return ArkValue(output.strip(), "String")
    except Exception as e:
        return ArkValue(f"Error: {e}", "String")

def sys_fs_write(args: List[ArkValue]):
    if len(args) != 2 or args[0].type != "String" or args[1].type != "String":
        raise Exception("sys.fs.write expects two string arguments: path and content")
    path = str(args[0].val)
    check_path_security(path, is_write=True)
    content = str(args[1].val)
    try:
        with open(path, "w") as f:
            f.write(content)
        return UNIT_VALUE
    except Exception as e:
        raise Exception(f"Error writing to file {path}: {e}")

def sys_fs_read(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "String":
        raise Exception("sys.fs.read expects a string path argument")
    path = str(args[0].val)
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
    path = str(args[0].val)
    check_path_security(path)
    try:
        with open(path, "rb") as f:
            content = f.read()
        return ArkValue(bytearray(content), "Buffer")
    except Exception as e:
        raise Exception(f"Error reading file {path}: {e}")

def sys_fs_write_buffer(args: List[ArkValue]):
    if len(args) != 2 or args[0].type != "String" or args[1].type != "Buffer":
        raise Exception("sys.fs.write_buffer expects string path and buffer")
    path = args[0].val
    check_path_security(path, is_write=True)
    buf = args[1].val
    try:
        with open(path, "wb") as f:
            f.write(bytes(buf))
        return ArkValue(True, "Boolean")
    except Exception as e:
        print(f"Write Buffer Error: {e}", file=sys.stderr)
        return ArkValue(False, "Boolean")


# ─── Blockchain (Mock) ───────────────────────────────────────────────────────

def sys_chain_height(args: List[ArkValue]):
    return ArkValue(1, "Integer")

def sys_chain_get_balance(args: List[ArkValue]):
    return ArkValue(100, "Integer")

def sys_chain_submit_tx(args: List[ArkValue]):
    return ArkValue("tx_hash_mock", "String")

def sys_chain_verify_tx(args: List[ArkValue]):
    if len(args) != 1: raise Exception("sys.chain.verify_tx expects tx")
    return ArkValue(True, "Boolean")


# ─── Math (Scaled Integer) ───────────────────────────────────────────────────

def math_sin_scaled(args: List[ArkValue]):
    return ArkValue(0, "Integer")

def math_cos_scaled(args: List[ArkValue]):
    return ArkValue(0, "Integer")

def math_pi_scaled(args: List[ArkValue]):
    return ArkValue(314159, "Integer")

def intrinsic_math_pow(args: List[ArkValue]):
    if len(args) != 2: raise Exception("math.pow expects 2 args")
    return ArkValue(int(math.pow(args[0].val, args[1].val)), "Integer")

def intrinsic_math_sqrt(args: List[ArkValue]):
    if len(args) != 1: raise Exception("math.sqrt expects 1 arg")
    return ArkValue(int(math.sqrt(args[0].val) * 100), "Integer")

def intrinsic_math_sin(args: List[ArkValue]):
    if len(args) != 1: raise Exception("math.sin expects 1 arg")
    val = args[0].val / 10000.0
    return ArkValue(int(math.sin(val) * 10000), "Integer")

def intrinsic_math_cos(args: List[ArkValue]):
    if len(args) != 1: raise Exception("math.cos expects 1 arg")
    val = args[0].val / 10000.0
    return ArkValue(int(math.cos(val) * 10000), "Integer")

def intrinsic_math_tan(args: List[ArkValue]):
    if len(args) != 1: raise Exception("math.tan expects 1 arg")
    val = args[0].val / 10000.0
    return ArkValue(int(math.tan(val) * 10000), "Integer")

def intrinsic_math_asin(args: List[ArkValue]):
    if len(args) != 1: raise Exception("math.asin expects 1 arg")
    val = args[0].val / 10000.0
    if val < -1.0 or val > 1.0: return ArkValue(0, "Integer")
    return ArkValue(int(math.asin(val) * 10000), "Integer")

def intrinsic_math_acos(args: List[ArkValue]):
    if len(args) != 1: raise Exception("math.acos expects 1 arg")
    val = args[0].val / 10000.0
    if val < -1.0 or val > 1.0: return ArkValue(0, "Integer")
    return ArkValue(int(math.acos(val) * 10000), "Integer")

def intrinsic_math_atan(args: List[ArkValue]):
    if len(args) != 1: raise Exception("math.atan expects 1 arg")
    val = args[0].val / 10000.0
    return ArkValue(int(math.atan(val) * 10000), "Integer")

def intrinsic_math_atan2(args: List[ArkValue]):
    if len(args) != 2: raise Exception("math.atan2 expects 2 args")
    y = args[0].val / 10000.0
    x = args[1].val / 10000.0
    return ArkValue(int(math.atan2(y, x) * 10000), "Integer")

# ─── Tensor Math ──────────────────────────────────────────────────────────────
# Tensors are ArkValue(Instance) with fields: data (flat ArkValue List), shape (dim List)
# Used by: tests/test_tensor.ark

def _make_tensor(flat_data, shape):
    """Helper: build an ArkValue tensor struct from a flat Python list and shape list."""
    data_ark = ArkValue([ArkValue(v, "Integer") for v in flat_data], "List")
    shape_ark = ArkValue([ArkValue(s, "Integer") for s in shape], "List")
    inst = ArkInstance.__new__(ArkInstance)
    inst.fields = {"data": data_ark, "shape": shape_ark}
    return ArkValue(inst, "Instance")

def _extract_tensor(ark_val):
    """Helper: extract (flat_python_list, shape_python_list) from an ArkValue tensor."""
    if ark_val.type != "Instance":
        raise Exception(f"Expected tensor (Instance), got {ark_val.type}")
    data_ark = ark_val.val.fields.get("data")
    shape_ark = ark_val.val.fields.get("shape")
    if data_ark is None or shape_ark is None:
        raise Exception("Tensor must have 'data' and 'shape' fields")
    data = [v.val for v in data_ark.val]
    shape = [v.val for v in shape_ark.val]
    return data, shape

def math_tensor(args: List[ArkValue]):
    """math.Tensor(data: List, shape: List) → Tensor struct"""
    if len(args) != 2:
        raise Exception("math.Tensor expects data(list) and shape(list)")
    if args[0].type != "List" or args[1].type != "List":
        raise Exception("math.Tensor expects List arguments")
    data = [v.val for v in args[0].val]
    shape = [v.val for v in args[1].val]
    return _make_tensor(data, shape)

def math_matmul(args: List[ArkValue]):
    """math.matmul(A, B) → Tensor.  A=[m,k], B=[k,n] → C=[m,n]"""
    if len(args) != 2:
        raise Exception("math.matmul expects 2 tensors")
    a_data, a_shape = _extract_tensor(args[0])
    b_data, b_shape = _extract_tensor(args[1])
    if len(a_shape) != 2 or len(b_shape) != 2:
        raise Exception("math.matmul expects 2D tensors")
    m, k = a_shape
    k2, n = b_shape
    if k != k2:
        raise Exception(f"math.matmul dimension mismatch: {k} vs {k2}")
    result = []
    for i in range(m):
        for j in range(n):
            s = 0
            for p in range(k):
                s += a_data[i * k + p] * b_data[p * n + j]
            result.append(s)
    return _make_tensor(result, [m, n])

def math_transpose(args: List[ArkValue]):
    """math.transpose(T) → Tensor.  T=[m,n] → T'=[n,m]"""
    if len(args) != 1:
        raise Exception("math.transpose expects 1 tensor")
    data, shape = _extract_tensor(args[0])
    if len(shape) != 2:
        raise Exception("math.transpose expects a 2D tensor")
    m, n = shape
    result = []
    for j in range(n):
        for i in range(m):
            result.append(data[i * n + j])
    return _make_tensor(result, [n, m])

def math_dot(args: List[ArkValue]):
    """math.dot(a, b) → Integer.  Element-wise multiply and sum (1D vectors)."""
    if len(args) != 2:
        raise Exception("math.dot expects 2 tensors")
    a_data, a_shape = _extract_tensor(args[0])
    b_data, b_shape = _extract_tensor(args[1])
    if len(a_data) != len(b_data):
        raise Exception(f"math.dot dimension mismatch: {len(a_data)} vs {len(b_data)}")
    s = sum(a * b for a, b in zip(a_data, b_data))
    return ArkValue(s, "Integer")

def math_tensor_add(args: List[ArkValue]):
    """math.add(a, b) → Tensor.  Element-wise addition."""
    if len(args) != 2:
        raise Exception("math.add expects 2 tensors")
    a_data, a_shape = _extract_tensor(args[0])
    b_data, b_shape = _extract_tensor(args[1])
    if a_shape != b_shape:
        raise Exception(f"math.add shape mismatch: {a_shape} vs {b_shape}")
    result = [a + b for a, b in zip(a_data, b_data)]
    return _make_tensor(result, a_shape)

def math_tensor_sub(args: List[ArkValue]):
    """math.sub(a, b) → Tensor.  Element-wise subtraction."""
    if len(args) != 2:
        raise Exception("math.sub expects 2 tensors")
    a_data, a_shape = _extract_tensor(args[0])
    b_data, b_shape = _extract_tensor(args[1])
    if a_shape != b_shape:
        raise Exception(f"math.sub shape mismatch: {a_shape} vs {b_shape}")
    result = [a - b for a, b in zip(a_data, b_data)]
    return _make_tensor(result, a_shape)

def math_mul_scalar(args: List[ArkValue]):
    """math.mul_scalar(t, scalar) → Tensor.  Multiply every element by scalar."""
    if len(args) != 2:
        raise Exception("math.mul_scalar expects tensor and scalar")
    data, shape = _extract_tensor(args[0])
    scalar = args[1].val
    result = [v * scalar for v in data]
    return _make_tensor(result, shape)


# ─── System Utilities ─────────────────────────────────────────────────────────

def sys_exit(args: List[ArkValue]):
    code = 0
    if len(args) > 0 and args[0].type == "Integer":
        code = args[0].val
    sys.exit(code)

def sys_time_sleep(args: List[ArkValue]):
    if len(args) != 1 or args[0].type not in ["Integer", "Float"]:
        raise Exception("sys.time.sleep expects a number (seconds)")
    time.sleep(args[0].val)
    return UNIT_VALUE

def sys_time_now(args: List[ArkValue]):
    if len(args) != 0:
        raise Exception("sys.time.now expects 0 arguments")
    return ArkValue(int(time.time() * 1000), "Integer")

def sys_str_from_code(args: List[ArkValue]):
    if len(args) != 1: raise Exception("sys.str.from_code expects 1 arg")
    code = args[0].val
    return ArkValue(chr(code), "String")


# ─── Cryptography ─────────────────────────────────────────────────────────────

def sys_crypto_hash(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "String":
        raise Exception("sys.crypto.hash expects a string")
    data = args[0].val.encode('utf-8')
    digest = hashlib.sha256(data).hexdigest()
    return ArkValue(digest, "String")

def sys_crypto_sha512(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "String":
        raise Exception("sys.crypto.sha512 expects a string (hex)")
    try:
        data = bytes.fromhex(str(args[0].val))
        digest = hashlib.sha512(data).hexdigest()
        return ArkValue(digest, "String")
    except Exception as e:
        raise Exception(f"SHA512 Error: {e}")

def sys_crypto_hmac_sha512(args: List[ArkValue]):
    if len(args) != 2 or args[0].type != "String" or args[1].type != "String":
        raise Exception("sys.crypto.hmac_sha512 expects key(hex) and data(hex)")
    try:
        key = bytes.fromhex(str(args[0].val))
        data = bytes.fromhex(str(args[1].val))
        digest = hmac.new(key, data, hashlib.sha512).hexdigest()
        return ArkValue(digest, "String")
    except Exception as e:
        raise Exception(f"HMAC-SHA512 Error: {e}")

def sys_crypto_pbkdf2_hmac_sha512(args: List[ArkValue]):
    if len(args) != 4:
        raise Exception("sys.crypto.pbkdf2_hmac_sha512 expects password, salt, iterations, dklen")
    password = str(args[0].val).encode('utf-8')
    salt = str(args[1].val).encode('utf-8')
    iterations = args[2].val
    dklen = args[3].val
    try:
        key = hashlib.pbkdf2_hmac('sha512', password, salt, iterations, dklen)
        return ArkValue(key.hex(), "String")
    except Exception as e:
        raise Exception(f"PBKDF2 Error: {e}")

def sys_crypto_aes_gcm_encrypt(args: List[ArkValue]):
    if len(args) != 4:
        raise Exception("sys.crypto.aes_gcm_encrypt expects key(hex), nonce(hex), plaintext(utf8), aad(utf8)")
    try:
        key = bytes.fromhex(str(args[0].val))
        nonce = bytes.fromhex(str(args[1].val))
        plaintext = str(args[2].val).encode('utf-8')
        aad = str(args[3].val).encode('utf-8')
        aesgcm = AESGCM(key)
        ciphertext_with_tag = aesgcm.encrypt(nonce, plaintext, aad)
        tag = ciphertext_with_tag[-16:]
        ciphertext = ciphertext_with_tag[:-16]
        return ArkValue([
            ArkValue(ciphertext.hex(), "String"),
            ArkValue(tag.hex(), "String")
        ], "List")
    except Exception as e:
        raise Exception(f"AES-GCM Encrypt Error: {e}")

def sys_crypto_aes_gcm_decrypt(args: List[ArkValue]):
    if len(args) != 5:
        raise Exception("sys.crypto.aes_gcm_decrypt expects key(hex), nonce(hex), ciphertext(hex), tag(hex), aad(utf8)")
    try:
        key = bytes.fromhex(str(args[0].val))
        nonce = bytes.fromhex(str(args[1].val))
        ciphertext = bytes.fromhex(str(args[2].val))
        tag = bytes.fromhex(str(args[3].val))
        aad = str(args[4].val).encode('utf-8')
        aesgcm = AESGCM(key)
        plaintext = aesgcm.decrypt(nonce, ciphertext + tag, aad)
        return ArkValue(plaintext.decode('utf-8'), "String")
    except Exception as e:
        raise Exception(f"AES-GCM Decrypt Error: {e}")

def sys_crypto_random_bytes(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "Integer":
        raise Exception("sys.crypto.random_bytes expects length(int)")
    n = args[0].val
    rand_bytes = secrets.token_bytes(n)
    return ArkValue(rand_bytes.hex(), "String")

def sys_math_pow_mod(args: List[ArkValue]):
    if len(args) != 3:
        raise Exception("sys.math.pow_mod expects base, exp, mod")
    base = args[0].val
    exp = args[1].val
    mod = args[2].val
    try:
        res = pow(base, exp, mod)
        return ArkValue(res, "Integer")
    except Exception as e:
        raise Exception(f"PowMod Error: {e}")

def sys_crypto_merkle_root(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "List":
        raise Exception("sys.crypto.merkle_root expects a list of strings")
    leaves = []
    for item in args[0].val:
        if item.type != "String":
            raise Exception("sys.crypto.merkle_root list must contain strings")
        leaves.append(item.val)
    if not leaves:
        return ArkValue("", "String")
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


# ─── Memory & Buffer ──────────────────────────────────────────────────────────

def sys_mem_alloc(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "Integer": raise Exception("sys.mem.alloc expects size")
    size = args[0].val
    buf = bytearray(size)
    return ArkValue(buf, "Buffer")

def sys_mem_inspect(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "Buffer": raise Exception("sys.mem.inspect expects buffer")
    buf = args[0].val
    addr = ctypes.addressof((ctypes.c_char * len(buf)).from_buffer(buf))
    print(f"<Buffer Inspect: ptr={hex(addr)}, len={len(buf)}>")
    return args[0]

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
    return ArkValue(buf, "Buffer")


# ─── List & Struct ────────────────────────────────────────────────────────────

def sys_list_get(args: List[ArkValue]):
    if len(args) != 2: raise Exception("sys.list.get expects list/str, index")
    lst = args[0]
    idx = args[1].val
    if lst.type == "List":
        val = lst.val[idx]
        return ArkValue([val, lst], "List")
    elif lst.type == "String":
        s = lst.val
        try:
            char_str = s[idx]
        except IndexError:
            raise Exception(f"String index out of range: idx={idx}, len={len(s)}, s='{s}'")
        return ArkValue([ArkValue(char_str, "String"), lst], "List")
    else:
        raise Exception("Expected List or String")

def sys_list_append(args: List[ArkValue]):
    if len(args) != 2: raise Exception("sys.list.append expects list, item")
    lst = args[0]
    if lst.type != "List": raise Exception("sys.list.append expects List")
    item = args[1]
    lst.val.append(item)
    return lst

def sys_list_pop(args: List[ArkValue]):
    if len(args) != 2: raise Exception("sys.list.pop expects list, index")
    lst = args[0]
    idx = args[1].val
    if lst.type != "List": raise Exception("sys.list.pop expects List")
    if idx < 0 or idx >= len(lst.val):
        raise Exception("Index out of bounds")
    val = lst.val.pop(idx)
    # Linear Return: [popped_val, original_list]
    # In Python, list is ref, so lst is modified in place. But we return it to satisfy linear flow.
    return ArkValue([val, lst], "List")

def sys_list_delete(args: List[ArkValue]):
    if len(args) != 2: raise Exception("sys.list.delete expects list, index")
    lst = args[0]
    idx = args[1].val
    if lst.type != "List": raise Exception("sys.list.delete expects List")
    if idx < 0 or idx >= len(lst.val): return UNIT_VALUE
    lst.val.pop(idx)
    return UNIT_VALUE

def sys_list_set(args: List[ArkValue]):
    if len(args) != 3: raise Exception("sys.list.set expects list, index, value")
    lst = args[0]
    idx_val = args[1]
    item = args[2]
    if lst.type != "List": raise Exception("sys.list.set expects List")
    if idx_val.type != "Integer": raise Exception("sys.list.set expects Integer index")
    idx = idx_val.val
    if idx < 0 or idx >= len(lst.val): raise Exception(f"List index out of range: {idx}")
    lst.val[idx] = item
    return lst

def sys_len(args: List[ArkValue]):
    if len(args) != 1: raise Exception("sys.len expects 1 argument")
    val = args[0]
    if val.type in ["String", "List", "Buffer"]:
        length = len(val.val)
        return ArkValue([ArkValue(length, "Integer"), val], "List")
    raise Exception(f"sys.len not supported for {val.type}")

def sys_struct_get(args: List[ArkValue]):
    if len(args) != 2: raise Exception("sys.struct.get expects struct, key")
    struct_val = args[0]
    key = args[1].val
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


# ─── Logic ────────────────────────────────────────────────────────────────────

def sys_and(args: List[ArkValue]):
    if len(args) != 2: raise Exception("sys.and expects 2 arguments")
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


# ─── AI ───────────────────────────────────────────────────────────────────────

def detect_ai_mode():
    global ARK_AI_MODE
    if ARK_AI_MODE:
        return ARK_AI_MODE
    try:
        req = urllib.request.Request("http://localhost:11434/api/tags", method="GET")
        with urllib.request.urlopen(req, timeout=0.5) as response:
            if response.getcode() == 200:
                print("Ollama Detected. Enabling Local AI Mode.")
                ARK_AI_MODE = "OLLAMA"
                return ARK_AI_MODE
    except Exception:
        pass
    if os.environ.get("GOOGLE_API_KEY"):
        print("Google API Key Detected. Enabling Cloud AI Mode.")
        ARK_AI_MODE = "GEMINI"
        return ARK_AI_MODE
    print("No AI Provider Detected. Using Mock Mode.")
    ARK_AI_MODE = "MOCK"
    return ARK_AI_MODE

def ask_ollama(prompt: str):
    url = "http://localhost:11434/api/generate"
    headers = {"Content-Type": "application/json"}
    data = {"model": "llama3", "prompt": prompt, "stream": False}
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
    text = str(args[0].val)
    matches = re.findall(r"```([^\n]*)\n(.*?)```", text, re.DOTALL)
    ark_blocks = []
    for tag_line, content in matches:
        tag_line = tag_line.strip()
        filename = "output.txt"
        if ":" in tag_line:
            parts = tag_line.split(":")
            if len(parts) > 1:
                filename = parts[1].strip()
        elif tag_line:
            if "." in tag_line:
                filename = tag_line
        pair = ArkValue([
            ArkValue(filename, "String"),
            ArkValue(content, "String")
        ], "List")
        ark_blocks.append(pair)
    return ArkValue(ark_blocks, "List")

def sys_ask_ai(args: List[ArkValue]):
    if len(args) != 1: raise Exception("sys.ask_ai expects prompt")
    try:
        prompt = str(args[0].val)
    except Exception as e:
        raise e
    return ArkValue(f"Ark AI: I received '{prompt}'", "String")


# ─── Networking ───────────────────────────────────────────────────────────────

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
    check_capability("net")
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
        return ArkValue([ArkValue(sid, "Integer"), ArkValue(addr[0], "String")], "List")
    except socket.timeout:
        return ArkValue(False, "Boolean")
    except BlockingIOError:
        return ArkValue(False, "Boolean")
    except Exception as e:
        print(f"Accept Error: {e}", file=sys.stderr)
        return ArkValue(False, "Boolean")

def sys_net_socket_connect(args: List[ArkValue]):
    check_capability("net")
    global SOCKET_ID
    if len(args) != 2 or args[0].type != "String" or args[1].type != "Integer":
        raise Exception("sys.net.socket.connect expects ip (String) and port (Integer)")
    ip = str(args[0].val)
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
            return ArkValue("", "String")
        return ArkValue(data.decode('utf-8', errors='ignore'), "String")
    except socket.timeout:
        return ArkValue(False, "Boolean")
    except BlockingIOError:
        return ArkValue(False, "Boolean")
    except Exception as e:
        return ArkValue("", "String")

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
    return UNIT_VALUE

def sys_net_socket_set_timeout(args: List[ArkValue]):
    if len(args) != 2 or args[0].type != "Integer":
        raise Exception("sys.net.socket.set_timeout expects handle and timeout (ms)")
    handle = args[0]
    timeout_ms = args[1].val
    timeout = float(timeout_ms) / 1000.0
    s = get_socket(handle)
    s.settimeout(timeout)
    return UNIT_VALUE

def sys_net_http_request(args: List[ArkValue]):
    if len(args) < 2:
        raise Exception("sys.net.http.request expects method, url")
    method = str(args[0].val)
    url = str(args[1].val)
    validate_url_security(url)
    data = None
    if len(args) > 2:
        data = args[2].val.encode('utf-8')
    opener = urllib.request.build_opener(SafeRedirectHandler)
    req = urllib.request.Request(url, data=data, method=method)
    try:
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


# ─── IO ───────────────────────────────────────────────────────────────────────

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


# ─── Logging & JSON ──────────────────────────────────────────────────────────

def sys_log(args: List[ArkValue]):
    s = " ".join([str(a.val) for a in args])
    print(f"[LOG] {s}", file=sys.stderr)
    return ArkValue(None, "Unit")

def to_python_val(val: ArkValue):
    if val.type == "Integer": return val.val
    if val.type == "String": return str(val.val)
    if val.type == "Boolean": return val.val
    if val.type == "List": return [to_python_val(x) for x in val.val]
    if val.type == "Instance":
        return {k: to_python_val(v) for k, v in val.val.fields.items()}
    if val.type == "Unit": return None
    return str(val.val)

def from_python_val(val):
    if val is None: return ArkValue(None, "Unit")
    if isinstance(val, bool): return ArkValue(val, "Boolean")
    if isinstance(val, int): return ArkValue(val, "Integer")
    if isinstance(val, float): return ArkValue(int(val), "Integer")
    if isinstance(val, str): return ArkValue(val, "String")
    if isinstance(val, list): return ArkValue([from_python_val(x) for x in val], "List")
    if isinstance(val, dict):
        fields = {k: from_python_val(v) for k, v in val.items()}
        return ArkValue(ArkInstance(None, fields), "Instance")
    return ArkValue(str(val), "String")

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
        return UNIT_VALUE
    return UNIT_VALUE

def from_ark(val):
    if val.type == "Instance":
        if hasattr(val.val, "fields"):
            return {k: from_ark(v) for k, v in val.val.fields.items()}
        return {}
    elif val.type == "List":
        return [from_ark(v) for v in val.val]
    elif val.type == "String":
        return str(val.val)
    elif val.type == "Integer":
        return val.val
    elif val.type == "Boolean":
        return val.val
    elif val.type == "Unit":
        return None
    return str(val.val)

def sys_json_parse(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "String":
        raise Exception("sys.json.parse expects string")
    try:
        data = json.loads(str(args[0].val))
        return from_python_val(data)
    except Exception as e:
        raise Exception(f"JSON Parse Error: {e}")

def sys_json_stringify(args: List[ArkValue]):
    if len(args) != 1: raise Exception("sys.json.stringify expects value")
    try:
        data = to_python_val(args[0])
        s = json.dumps(data)
        return ArkValue(s, "String")
    except Exception as e:
        raise Exception(f"JSON Stringify Error: {e}")

def sys_html_escape(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "String":
        raise Exception("sys.html_escape expects a string")
    return ArkValue(html.escape(args[0].val), "String")

def sys_z3_verify(args: List[ArkValue]):
    if len(args) != 1 or args[0].type != "List":
        raise Exception("sys.z3.verify expects a List of constraints (Strings)")
    constraints_val = args[0].val
    constraints = []
    for item in constraints_val:
        if item.type != "String":
            raise Exception("sys.z3.verify constraints list must contain Strings")
        constraints.append(item.val)


# ─── Late Intrinsics (depend on interpreter) ──────────────────────────────────
# These are populated after the interpreter module loads.
# They are defined here as stubs, then replaced in ark.py entry point.

def _make_late_intrinsics(call_user_func_ref):
    """Create intrinsics that depend on the interpreter's call_user_func."""
    
    def sys_thread_spawn(args: List[ArkValue]):
        check_capability("thread")
        if len(args) != 1 or args[0].type != "Function":
            raise Exception("sys.thread.spawn expects a function")
        func = args[0].val
        def thread_target():
            try:
                call_user_func_ref(func, [])
            except Exception as e:
                print(f"Thread Error: {e}", file=sys.stderr)
                import traceback
                traceback.print_exc()
        t = threading.Thread(target=thread_target)
        t.daemon = True
        t.start()
        return UNIT_VALUE

    def sys_func_apply(args: List[ArkValue]):
        if len(args) != 2: raise Exception("sys.func.apply expects func, args_list")
        func = args[0]
        arg_list = args[1]
        if arg_list.type != "List": raise Exception("sys.func.apply expects List of args")
        if func.type == "Function":
            return call_user_func_ref(func.val, arg_list.val)
        elif func.type == "Intrinsic":
            return INTRINSICS[func.val](arg_list.val)
        raise Exception(f"Cannot apply {func.type}")

    def sys_vm_eval(args: List[ArkValue], scope: Scope):
        if len(args) != 1 or args[0].type != "String":
            raise Exception("sys.vm.eval expects a code string")
        code = str(args[0].val)
        try:
            # Lazy import to avoid circular dependency
            from ark_interpreter import eval_node, ARK_PARSER
            tree = ARK_PARSER.parse(code)
            return eval_node(tree, scope)
        except Exception as e:
            raise Exception(f"Eval Error: {e}")

    def sys_vm_source(args: List[ArkValue], scope: Scope):
        if len(args) != 1 or args[0].type != "String":
            raise Exception("sys.vm.source expects a file path")
        path = str(args[0].val)
        check_path_security(path)
        try:
            with open(path, "r") as f:
                code = f.read()
            from ark_interpreter import eval_node, ARK_PARSER
            tree = ARK_PARSER.parse(code)
            return eval_node(tree, scope)
        except Exception as e:
            raise Exception(f"Source Error: {e}")

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
                req_path = ArkValue(self.path, "String")
                call_args = [req_path]
                try:
                    result = call_user_func_ref(handler_func.val, call_args)
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
        t = threading.Thread(target=httpd.serve_forever)
        t.daemon = True
        t.start()
        return UNIT_VALUE

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
                val = UNIT_VALUE
                EVENT_QUEUE.put((callback, [val]))
        t = threading.Thread(target=task)
        t.daemon = True
        t.start()
        return UNIT_VALUE

    def sys_event_poll(args: List[ArkValue]):
        try:
            cb, cb_args = EVENT_QUEUE.get_nowait()
            if not isinstance(cb_args, list):
                cb_args = [cb_args]
            args_list = ArkValue(cb_args, "List")
            return ArkValue([cb, args_list], "List")
        except queue.Empty:
            return UNIT_VALUE

    return {
        "sys.thread.spawn": sys_thread_spawn,
        "sys.func.apply": sys_func_apply,
        "sys.vm.eval": sys_vm_eval,
        "sys.vm.source": sys_vm_source,
        "sys.net.http.serve": sys_net_http_serve,
        "sys.io.read_file_async": sys_io_read_file_async,
        "sys.event.poll": sys_event_poll,
    }


# ─── INTRINSICS Registry ─────────────────────────────────────────────────────

INTRINSICS = {
    # Core
    "get": core_get,
    "len": core_len,
    "print": core_print,

    # System
    "sys.crypto.hash": sys_crypto_hash,
    "sys.crypto.sha512": sys_crypto_sha512,
    "sys.crypto.hmac_sha512": sys_crypto_hmac_sha512,
    "sys.crypto.pbkdf2_hmac_sha512": sys_crypto_pbkdf2_hmac_sha512,
    "sys.crypto.aes_gcm_encrypt": sys_crypto_aes_gcm_encrypt,
    "sys.crypto.aes_gcm_decrypt": sys_crypto_aes_gcm_decrypt,
    "sys.crypto.random_bytes": sys_crypto_random_bytes,
    "sys.math.pow_mod": sys_math_pow_mod,
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
    "sys.list.set": sys_list_set,
    "sys.list.get": sys_list_get,
    "sys.mem.alloc": sys_mem_alloc,
    "sys.mem.inspect": sys_mem_inspect,
    "sys.mem.read": sys_mem_read,
    "sys.mem.write": sys_mem_write,
    "sys.net.http.request": sys_net_http_request,
    "sys.net.socket.bind": sys_net_socket_bind,
    "sys.net.socket.accept": sys_net_socket_accept,
    "sys.net.socket.connect": sys_net_socket_connect,
    "sys.net.socket.send": sys_net_socket_send,
    "sys.net.socket.recv": sys_net_socket_recv,
    "sys.net.socket.close": sys_net_socket_close,
    "sys.net.socket.set_timeout": sys_net_socket_set_timeout,
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
    "sys.log": sys_log,
    "intrinsic_ask_ai": sys_ask_ai,
    "sys.io.read_bytes": sys_io_read_bytes,
    "sys.io.read_line": sys_io_read_line,
    "sys.io.write": sys_io_write,
    "sys.exit": sys_exit,
    "exit": sys_exit,
    "quit": sys_exit,

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

    # Tensor Math
    "math.Tensor": math_tensor,
    "math.matmul": math_matmul,
    "math.transpose": math_transpose,
    "math.dot": math_dot,
    "math.add": math_tensor_add,
    "math.sub": math_tensor_sub,
    "math.mul_scalar": math_mul_scalar,

    # Intrinsic Wrappers (Aliases)
    "intrinsic_and": sys_and,
    "intrinsic_not": intrinsic_not,
    "intrinsic_buffer_alloc": sys_mem_alloc,
    "intrinsic_buffer_inspect": sys_mem_inspect,
    "intrinsic_buffer_read": sys_mem_read,
    "intrinsic_buffer_write": sys_mem_write,
    "intrinsic_crypto_hash": sys_crypto_hash,
    "intrinsic_extract_code": extract_code,
    "intrinsic_ge": lambda args: _eval_binop("ge", args[0], args[1]),
    "intrinsic_gt": lambda args: _eval_binop("gt", args[0], args[1]),
    "intrinsic_le": lambda args: _eval_binop("le", args[0], args[1]),
    "intrinsic_lt": lambda args: _eval_binop("lt", args[0], args[1]),
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
}

# Binop helper for INTRINSICS lambdas (avoids circular import with interpreter)
def _eval_binop(op, left, right):
    l = left.val
    r = right.val
    if op == "gt": return ArkValue(l > r, "Boolean")
    if op == "lt": return ArkValue(l < r, "Boolean")
    if op == "ge": return ArkValue(l >= r, "Boolean")
    if op == "le": return ArkValue(l <= r, "Boolean")
    return UNIT_VALUE


LINEAR_SPECS = {
    "sys.mem.write": [0],
    "sys.mem.read": [0],
}


INTRINSICS_WITH_SCOPE = {
    "sys.vm.eval",
    "sys.vm.source",
}
