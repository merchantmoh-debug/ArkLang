"""
Counts the intrinsics in both Python and Rust implementations.
"""
import sys
import os
import re
from unittest.mock import MagicMock

# Mock dependencies
sys.modules['cryptography'] = MagicMock()
sys.modules['cryptography.hazmat'] = MagicMock()
sys.modules['cryptography.hazmat.primitives'] = MagicMock()
sys.modules['cryptography.hazmat.primitives.asymmetric'] = MagicMock()
sys.modules['cryptography.hazmat.primitives.ciphers'] = MagicMock()
sys.modules['cryptography.hazmat.primitives.ciphers.aead'] = MagicMock()

# Add root to path
sys.path.append(os.getcwd())

try:
    from meta.ark_intrinsics import INTRINSICS
except ImportError as e:
    print(f"Error: Could not import meta.ark_intrinsics: {e}")
    try:
        sys.path.append(os.path.join(os.getcwd(), 'meta'))
        from ark_intrinsics import INTRINSICS
    except ImportError as e2:
        print(f"Error: Could not import ark_intrinsics locally either: {e2}")
        sys.exit(1)

def get_rust_intrinsics():
    """Reads core/src/intrinsics.rs and extracts intrinsic names."""
    rust_intrinsics = set()
    filepath = "core/src/intrinsics.rs"
    if not os.path.exists(filepath):
        print(f"Error: {filepath} not found")
        return rust_intrinsics

    try:
        with open(filepath, "r", encoding="utf-8") as f:
            content = f.read()

        # Helper: find the body of a function by counting braces
        def extract_function_body(content, fn_signature):
            idx = content.find(fn_signature)
            if idx == -1:
                return ""
            # Find the opening brace
            brace_start = content.find("{", idx)
            if brace_start == -1:
                return ""
            depth = 0
            i = brace_start
            while i < len(content):
                if content[i] == "{":
                    depth += 1
                elif content[i] == "}":
                    depth -= 1
                    if depth == 0:
                        return content[brace_start:i + 1]
                i += 1
            return ""

        # 1. Parse resolve() function
        resolve_body = extract_function_body(content, "fn resolve(hash: &str)")
        if resolve_body:
            for line in resolve_body.splitlines():
                if "=>" in line:
                    parts = line.split("=>")[0]
                    keys = re.findall(r'"([^"]+)"', parts)
                    for k in keys:
                        rust_intrinsics.add(k)

        # 2. Parse register_all() function
        register_body = extract_function_body(content, "fn register_all(scope: &mut Scope)")
        if register_body:
            keys = re.findall(r'scope\.set\(\s*"([^"]+)"', register_body)
            for k in keys:
                rust_intrinsics.add(k)
        else:
            print("Warning: Could not find register_all function body")

    except Exception as e:
        print(f"Error reading {filepath}: {e}")
    return rust_intrinsics

def main():
    python_intrinsics = set(INTRINSICS.keys())
    rust_intrinsics = get_rust_intrinsics()

    print(f"Total Python Intrinsics: {len(python_intrinsics)}")
    print(f"Total Rust Intrinsics: {len(rust_intrinsics)}")

    missing_in_rust = python_intrinsics - rust_intrinsics
    missing_in_python = rust_intrinsics - python_intrinsics

    print("\n--- Missing in Rust (Python Only) ---")
    for i in sorted(missing_in_rust):
        print(f"- {i}")

    intersection = python_intrinsics.intersection(rust_intrinsics)
    parity = (len(intersection) / len(python_intrinsics)) * 100 if python_intrinsics else 0

    print(f"\nParity: {parity:.2f}% ({len(intersection)}/{len(python_intrinsics)})")

if __name__ == "__main__":
    main()
