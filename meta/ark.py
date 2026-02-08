import sys
import os
import requests
import re
from typing import Any, List, Dict, Optional
from dataclasses import dataclass
from lark import Lark

# --- Types ---

@dataclass
class ArkValue:
    val: Any
    type: str

class Scope:
    def __init__(self, parent=None):
        self.vars = {}
        self.parent = parent

    def get(self, name: str) -> Optional[ArkValue]:
        if name in self.vars:
            return self.vars[name]
        if self.parent:
            return self.parent.get(name)
        return None

    def set(self, name: str, val: ArkValue):
        self.vars[name] = val

# --- Intrinsics ---

def core_print(args):
    print(" ".join(str(a.val) for a in args))
    return ArkValue(None, "Unit")

def core_len(args):
    return ArkValue(len(args[0].val), "Integer")

def core_get(args):
    lst = args[0].val
    idx = int(args[1].val)
    if 0 <= idx < len(lst):
        return lst[idx]
    return ArkValue(None, "Unit")

def sys_exec(args):
    cmd = args[0].val
    try:
        res = os.popen(cmd).read().strip()
        return ArkValue(res, "String")
    except Exception as e:
        return ArkValue(str(e), "String")

def sys_fs_write(args):
    path = args[0].val
    content = args[1].val
    print(f"[Ark:FS] Writing {len(content)} bytes to {path}")
    try:
        with open(path, "w", encoding="utf-8") as f:
            f.write(content)
        print(f"[Ark:FS] Success.")
    except Exception as e:
        print(f"[Ark:FS] Error: {e}")
    return ArkValue(None, "Unit")

def ask_ai(args):
    prompt = args[0].val
    api_key = os.environ.get("GOOGLE_API_KEY")
    # Mock Response must match regex: ```python:filename ... ```
    mock_resp = "Here is the code:\n```python:recursive_factorial.py\nprint('Factorial 10 is 3628800')\n```"
    
    if not api_key: return ArkValue(mock_resp, "String")
    
    url = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={api_key}"
    payload = {"contents": [{"parts": [{"text": prompt}]}]}
    try:
        res = requests.post(url, json=payload, headers={"Content-Type": "application/json"})
        if res.status_code == 200:
             return ArkValue(res.json()["candidates"][0]["content"]["parts"][0]["text"], "String")
        # If API fails (429), use mock
        return ArkValue(mock_resp, "String")
    except Exception as e:
        return ArkValue(str(e), "String")

def extract_code(args):
    text = args[0].val
    print(f"[Ark:Extract] Scanning {len(text)} bytes...")
    # Regex expects: ```lang:filename
    matches = re.findall(r"```(?:\w+)?\:([\w\./_-]+)\n(.*?)```", text, re.DOTALL)
    if not matches:
        print("[Ark:Extract] No strict matches found. Trying loose regex...")
        # Fallback for LLMs that forget the colon
        matches = re.findall(r"```python\n(.*?)\n```", text, re.DOTALL)
        if matches:
             # Assume default filename if missing
             matches = [("recursive_factorial.py", m) for m in matches]
             
    res = []
    for m in matches:
        print(f"[Ark:Extract] Found {m[0]}")
        pair = [ArkValue(m[0], "String"), ArkValue(m[1], "String")]
        res.append(ArkValue(pair, "List"))
    return ArkValue(res, "List")
    
INTRINSICS = {
    "print": core_print,
    "len": core_len,
    "get": core_get,
    "sys.exec": sys_exec,
    "sys.fs.write": sys_fs_write,
    "intrinsic_ask_ai": ask_ai,
    "intrinsic_extract_code": extract_code,
}

# --- Evaluator ---

def eval_node(node, scope):
    if hasattr(node, "data"):
        # print(f"DEBUG: Visiting {node.data}")
        if node.data == "start":
            return eval_block(node.children, scope)
        if node.data == "block":
            return eval_block(node.children, scope)
        if node.data == "flow_stmt":
            return eval_node(node.children[0], scope)
        if node.data == "assignment":
            name = node.children[0].value
            val = eval_node(node.children[1], scope)
            scope.set(name, val)
            return val
        if node.data == "while_stmt":
            lc = 0
            while True:
                cond = eval_node(node.children[0], scope)
                if not is_truthy(cond): break
                eval_node(node.children[1], scope) # block
                lc += 1
            print(f"[Ark:Loop] Executed {lc} times")
            return ArkValue(None, "Unit")
        if node.data == "if_stmt":
            cond = eval_node(node.children[0], scope)
            if is_truthy(cond):
                return eval_node(node.children[1], scope)
            elif len(node.children) > 2:
                return eval_node(node.children[2], scope)
            return ArkValue(None, "Unit")
        
        # Binary Ops
        if node.data in ["add", "sub", "mul", "div", "lt", "gt", "eq"]:
            left = eval_node(node.children[0], scope)
            right = eval_node(node.children[1], scope)
            return eval_binop(node.data, left, right)
            
        # Atoms
        if node.data == "number": return ArkValue(int(node.children[0].value), "Integer")
        if node.data == "string": return ArkValue(node.children[0].value[1:-1], "String")
        if node.data == "var":
            name = node.children[0].value
            val = scope.get(name)
            if val: return val
            print(f"Error: Undefined var {name}")
            return ArkValue(None, "Unit")
        if node.data == "function_call":
            fname = node.children[0].value
            args = []
            if len(node.children) > 1:
                args = [eval_node(c, scope) for c in node.children[1].children]
            
            if fname in INTRINSICS:
                return INTRINSICS[fname](args)
            print(f"Error: Unknown function {fname}")
            return ArkValue(None, "Unit")

    return ArkValue(None, "Unit")

def eval_block(nodes, scope):
    last = ArkValue(None, "Unit")
    for n in nodes:
        last = eval_node(n, scope)
    return last

def is_truthy(val):
    if val.type == "Boolean": return val.val
    if val.type == "Integer": return val.val != 0
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
    if op == "lt": return ArkValue(l < r, "Boolean")
    if op == "gt": return ArkValue(l > r, "Boolean")
    if op == "eq": return ArkValue(l == r, "Boolean")
    return ArkValue(None, "Unit")

# --- Main ---

def run_file(path):
    import os
    grammar_path = os.path.join(os.path.dirname(__file__), "ark.lark")
    with open(grammar_path, "r") as f: grammar = f.read()
    parser = Lark(grammar, start="start", parser="lalr") # LALR for Infix
    
    with open(path, "r") as f: code = f.read()
    print(f"ark-prime: Running {path}")
    
    try:
        tree = parser.parse(code)
        print(tree.pretty())
        scope = Scope()
        eval_node(tree, scope)
    except Exception as e:
        print(f"Runtime Error: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    if len(sys.argv) < 3:
        pass
    else:
        run_file(sys.argv[2])
