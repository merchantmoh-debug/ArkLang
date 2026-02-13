import pytest
import sys
import os

# Ensure src is in path
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "../src")))

from sandbox.local import LocalSandbox

@pytest.fixture
def sandbox():
    return LocalSandbox()

def test_valid_code(sandbox):
    code = "print(1 + 1)"
    result = sandbox.execute(code)
    assert result.exit_code == 0
    assert result.stdout.strip() == "2"
    assert "Security Violation" not in result.stderr

def test_valid_string_ops(sandbox):
    code = "print('hello'.upper())"
    result = sandbox.execute(code)
    assert result.exit_code == 0
    assert result.stdout.strip() == "HELLO"

def test_block_builtins(sandbox):
    code = "print(__builtins__)"
    result = sandbox.execute(code)
    assert result.exit_code == 1
    assert "Security Violation" in result.stderr
    assert "__builtins__" in result.stderr

def test_block_getattr(sandbox):
    code = "getattr(object, '__class__')"
    result = sandbox.execute(code)
    assert result.exit_code == 1
    assert "Security Violation" in result.stderr
    assert "getattr" in result.stderr

def test_block_class_access(sandbox):
    code = "print(''.__class__)"
    result = sandbox.execute(code)
    assert result.exit_code == 1
    assert "Security Violation" in result.stderr
    assert "__class__" in result.stderr

def test_block_ctypes_import(sandbox):
    code = "import ctypes"
    result = sandbox.execute(code)
    assert result.exit_code == 1
    assert "Security Violation" in result.stderr
    assert "ctypes" in result.stderr

def test_block_builtins_import(sandbox):
    code = "import builtins"
    result = sandbox.execute(code)
    assert result.exit_code == 1
    assert "Security Violation" in result.stderr
    assert "builtins" in result.stderr

def test_block_magic_methods(sandbox):
    code = "print([].__class__.__base__)"
    result = sandbox.execute(code)
    assert result.exit_code == 1
    assert "Security Violation" in result.stderr
    assert "__base__" in result.stderr

def test_block_eval_exec(sandbox):
    code = "eval('1+1')"
    result = sandbox.execute(code)
    assert result.exit_code == 1
    assert "Security Violation" in result.stderr

    code = "exec('print(1)')"
    result = sandbox.execute(code)
    assert result.exit_code == 1
    assert "Security Violation" in result.stderr

def test_block_open(sandbox):
    code = "open('test.txt', 'w')"
    result = sandbox.execute(code)
    assert result.exit_code == 1
    assert "Security Violation" in result.stderr

def test_block_open_attribute(sandbox):
    # This checks that banning open as attribute works
    code = "class A: pass\na = A()\na.open = 1"
    result = sandbox.execute(code)
    assert result.exit_code == 1
    assert "Security Violation" in result.stderr
    assert "open" in result.stderr

def test_complex_bypass_attempt(sandbox):
    # The reproduction case
    code = """
b = __builtins__
"""
    result = sandbox.execute(code)
    assert result.exit_code == 1
    assert "Security Violation" in result.stderr
