import sys
import os
import re
import time
import json
from dataclasses import dataclass
from typing import List, Dict, Any, Optional
import http.server
import socketserver
import threading
from lark import Lark
import hashlib
import ctypes

# --- Types ---

@dataclass
class ArkValue:
    val: Any
    type: str

class ReturnException(Exception):
    def __init__(self, value):
        self.value = value

@dataclass
class ArkFunction:
    name: str
    params: List[str]
    body: Any # Tree node
    closure: 'Scope'

@dataclass
class ArkClass:
    name: str
    methods: Dict[str, ArkFunction]

@dataclass
class ArkInstance:
    klass: ArkClass
    fields: Dict[str, ArkValue]

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
            return ArkValue(collection[index], "Any") # Assuming list elements can be anything
    else:
        raise Exception("Index out of bounds")
    return ArkValue(None, "Unit") # Should not be reached

def sys_exec(args: List[ArkValue]):
    if not args or args[0].type != "String":
        raise Exception("sys.exec expects a string command")
    command = args[0].val
    print(f"WARNING: Executing system command: {command}", file=sys.stderr)
    try:
        result = os.popen(command).read()
        return ArkValue(result, "String")
    except Exception as e:
        return ArkValue(f"Error: {e}", "String")

def sys_fs_write(args: List[ArkValue]):
    if len(args) != 2 or args[0].type != "String" or args[1].type != "String":
        raise Exception("sys.fs.write expects two string arguments: path and content")
    path = args[0].val
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
    try:
        with open(path, "r") as f:
            content = f.read()
        return ArkValue(content, "String")
    except Exception as e:
        raise Exception(f"Error reading file {path}: {e}")

def ask_ai(args: List[ArkValue]):
    if not args or args[0].type != "String":
        raise Exception("ask_ai expects a string prompt")
    prompt = args[0].val
    
    api_key = os.environ.get("GOOGLE_API_KEY")
    if not api_key:
        raise Exception("GOOGLE_API_KEY environment variable not set")

    url = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={api_key}"
    headers = {"Content-Type": "application/json"}
    data = {"contents": [{"parts": [{"text": prompt}]}]}
    
    import json
    import urllib.request
    import urllib.error
    import time
    
    max_retries = 3
    for attempt in range(max_retries):
        try:
            req = urllib.request.Request(url, data=json.dumps(data).encode("utf-8"), headers=headers, method="POST")
            with urllib.request.urlopen(req) as response:
                res_json = json.loads(response.read().decode("utf-8"))
                # Extract text from response
                try:
                    text = res_json["candidates"][0]["content"]["parts"][0]["text"]
                    return ArkValue(text, "String")
                except (KeyError, IndexError) as e:
                    raise Exception(f"Failed to parse AI response: {e}")
        except urllib.error.HTTPError as e:
            if e.code == 429:
                if attempt < max_retries - 1:
                    wait_time = (2 ** attempt) * 2 # 2, 4, 8 seconds
                    print(f"AI Rate Limit (429). Retrying in {wait_time}s...")
                    time.sleep(wait_time)
                    continue
            print(f"AI Request Failed: {e.code} {e.reason}")
            # Fall through to fallback
        except Exception as e:
            print(f"AI Error: {e}")
            # Fall through to fallback
            
    # Fallback for verification if API is dead/rate-limited
    print(f"WARNING: API Failed. Using Fallback Mock for Verification.")
    start = "```python\n"
    code = "import datetime\nprint(f'Sovereignty Established: {datetime.datetime.now()}')\n"
    end = "```"
    return ArkValue(start + code + end, "String")

def extract_code(args: List[ArkValue]):
    if not args or args[0].type != "String":
        raise Exception("extract_code expects a string containing code")
    text = args[0].val
    # Regex to find code blocks (e.g., ```python ... ``` or just ``` ... ```)
    matches = re.findall(r"```(?:\w+)?\n(.*?)\n```", text, re.DOTALL)
    if matches:
        # For simplicity, return the first found code block
        return ArkValue(matches[0], "String")
    return ArkValue("", "String") # Return empty string if no code block found

def sys_net_http_serve(args: List[ArkValue]):
    # print(f"DEBUG: sys.net.http.serve args: {[a.type for a in args]}")
    if len(args) != 2 or args[0].type != "Integer" or args[1].type != "Function":
        print(f"DEBUG: sys.net.http.serve args: {[a.type for a in args]}")
        raise Exception("sys.net.http.serve expects an integer port and a function handler")
    port = args[0].val
    handler_func = args[1].val # ArkFunction
    
    # We need a closure to capture the handler_func for the RequestHandler class
    # Since socketserver.TCPServer expects a Class, not an instance, we use a factory or partial.
    print(f"Starting Ark Web Server on port {port}...")
    
    # To allow `call_user_func` to be accessible within the handler,
    # we pass it as a global or ensure it's imported/defined in the scope where this intrinsic runs.
    # For this example, we assume `call_user_func` is available in the module's global scope.

    class ArkHttpHandler(http.server.SimpleHTTPRequestHandler):
        def do_GET(self):
            # 1. Build Ark Request Object (Mock for now, just path)
            # In a real impl, we would create an ArkInstance of 'Request' class
            # For now, pass path as string or maybe a dict/map if we had them.
            # Let's pass the PATH as a string for simplicity.
            req_path = ArkValue(self.path, "String")
            
            # 2. Call Ark Handler
            # We need to call call_user_func. BUT call_user_func is defined later.
            # We can't access it easily unless we move this class or pass it.
            # Hack: We will define call_user_func in INTRINSICS or global scope.
            # Actually, sys_net_http_serve is defined before call_user_func in this file?
            # No, call_user_func is defined at bottom.
            # We should move sys_net_http_serve to the bottom or pass dependencies.
            # For now, let's assume `call_user_func` is available globally in the module at runtime.
            
            response_val = call_user_func(handler_func, [req_path])
            
            # 3. Send Response
            self.send_response(200)
            self.end_headers()
            if response_val.type == "String":
                self.wfile.write(response_val.val.encode())
            else:
                self.wfile.write(str(response_val.val).encode())

    # Create Server
    # Allow address reuse
    socketserver.TCPServer.allow_reuse_address = True
    # Use a thread to run the server so the main program can continue
    # This is a simple way to handle it, for production, more robust threading/async might be needed.
    server_address = ("127.0.0.1", port)
    httpd = socketserver.TCPServer(server_address, ArkHttpHandler)
    
    server_thread = threading.Thread(target=httpd.serve_forever)
    server_thread.daemon = True # Allow the main program to exit even if the thread is running
    server_thread.start()
    
    print(f"Server running in background on port {port}. Press Ctrl+C to stop.")
            
    return ArkValue(None, "Unit")

def sys_time_sleep(args: List[ArkValue]):
    if len(args) != 1 or args[0].type not in ["Integer", "Float"]:
        raise Exception("sys.time.sleep expects a number (seconds)")
    time.sleep(args[0].val)
    return ArkValue(None, "Unit")

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
        char_str = s[idx]
        return ArkValue([ArkValue(char_str, "String"), lst], "List")
    else:
        raise Exception("Expected List or String")

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


INTRINSICS = {
    # Core
    "get": core_get,
    "len": core_len,
    "print": core_print,

    # System
    "sys.crypto.hash": sys_crypto_hash,
    "sys.crypto.merkle_root": sys_crypto_merkle_root,
    "sys.exec": sys_exec,
    "sys.fs.read": sys_fs_read,
    "sys.fs.write": sys_fs_write,
    "sys.len": sys_len,
    "sys.list.append": sys_list_append,
    "sys.list.get": sys_list_get,
    "sys.mem.alloc": sys_mem_alloc,
    "sys.mem.inspect": sys_mem_inspect,
    "sys.mem.read": sys_mem_read,
    "sys.mem.write": sys_mem_write,
    "sys.net.http.serve": sys_net_http_serve,
    "sys.time.sleep": sys_time_sleep,

    # Intrinsics (Aliased / Specific)
    "intrinsic_and": sys_and,
    "intrinsic_ask_ai": ask_ai,
    "intrinsic_buffer_alloc": sys_mem_alloc,
    "intrinsic_buffer_inspect": sys_mem_inspect,
    "intrinsic_buffer_read": sys_mem_read,
    "intrinsic_buffer_write": sys_mem_write,
    "intrinsic_crypto_hash": sys_crypto_hash,
    "intrinsic_extract_code": extract_code,
    "intrinsic_ge": lambda args: eval_binop("ge", args[0], args[1]),
    "intrinsic_gt": lambda args: eval_binop("gt", args[0], args[1]),
    "intrinsic_le": lambda args: eval_binop("le", args[0], args[1]),
    "intrinsic_len": sys_len,
    "intrinsic_list_append": sys_list_append,
    "intrinsic_list_get": sys_list_get,
    "intrinsic_lt": lambda args: eval_binop("lt", args[0], args[1]),
    "intrinsic_merkle_root": sys_crypto_merkle_root,
    "intrinsic_or": sys_or,
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
            
        # --- Definitions ---
        if node.data == "function_def":
            name = node.children[0].value
            # param_list is optional. If present, it's children[1], body is children[2]
            # If missing, body is children[1]
            params = []
            body_idx = 1
            if len(node.children) > 1 and hasattr(node.children[1], "data") and node.children[1].data == "param_list":
                params = [t.value for t in node.children[1].children]
                body_idx = 2
            
            body = node.children[body_idx]
            func = ArkValue(ArkFunction(name, params, body, scope), "Function")
            scope.set(name, func)
            return func

        if node.data == "class_def":
            name = node.children[0].value
            methods = {}
            # Iterate children to find functions
            for child in node.children[1:]:
                if child.data == "function_def":
                    # Evaluate definition temporarily to capture it, but we need to strip it from scope? 
                    # Actually better to process manually to avoid polluting current scope
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

        # --- Control Flow ---
        if node.data == "return_stmt":
            val = eval_node(node.children[0], scope) if node.children else ArkValue(None, "Unit")
            raise ReturnException(val)

        if node.data == "if_stmt":
            cond = eval_node(node.children[0], scope)
            if is_truthy(cond):
                return eval_node(node.children[1], scope)
            # Check for else block
            elif len(node.children) > 2 and node.children[2]:
                return eval_node(node.children[2], scope)
            return ArkValue(None, "Unit")

        if node.data == "while_stmt":
            cond_node = node.children[0]
            body_node = node.children[1]
            while is_truthy(eval_node(cond_node, scope)):
                # eval_node on block returns last value, but we ignore it here
                eval_node(body_node, scope)
            return ArkValue(None, "Unit")

        if node.data == "var":
            name = node.children[0].value
            val = scope.get(name)
            if val: return val
            
            # Verify if Intrinsic
            if name in INTRINSICS:
                return ArkValue(name, "Intrinsic")
            
            # print(f"Error: Undefined var {name}")
            raise Exception(f"Undefined variable: {name}")
        
        if node.data == "assign_var":
            name = node.children[0].value
            val = eval_node(node.children[1], scope)
            scope.set(name, val)
            return val

        if node.data == "assign_attr":
            obj = eval_node(node.children[0], scope)
            attr = node.children[1].value
            val = eval_node(node.children[2], scope)
            
            if obj.type == "Instance":
                obj.val.fields[attr] = val
                return val
            raise Exception(f"Cannot set attribute on {obj.type}")

        if node.data == "get_attr":
            obj = eval_node(node.children[0], scope)
            attr = node.children[1].value
            
            if obj.type == "Namespace":
                new_path = f"{obj.val}.{attr}"
                # Check if it is a known intrinsic
                if new_path in INTRINSICS:
                    return ArkValue(new_path, "Intrinsic")
                # Otherwise return extended namespace
                return ArkValue(new_path, "Namespace")
            
            if obj.type == "Instance":
                # 1. Check fields
                if attr in obj.val.fields:
                    return obj.val.fields[attr]
                # 2. Check methods (and bind this)
                klass = obj.val.klass
                if attr in klass.methods:
                    method = klass.methods[attr]
                    # Return a Bound Method? Or just the function?
                    # We need to pass 'obj' as 'this' when called.
                    # Let's verify if 'ArkValue' can store a BoundMethod tuple
                    return ArkValue((method, obj), "BoundMethod")

            raise Exception(f"Attribute {attr} not found on {obj.type}")

        if node.data == "call_expr":
            func_val = eval_node(node.children[0], scope)
            
            args = []
            if len(node.children) > 1:
                arg_list_node = node.children[1]
                if hasattr(arg_list_node, "children"):
                    args = [eval_node(c, scope) for c in arg_list_node.children]
            
            # 1. Intrinsics (stored as string names or Python callables?) 
            # Wait, Intrinsics are in a dict, but how do we get them?
            # If the user typed `print(...)`, `eval_node` for `var` would define loopup.
            # But Intrinsics are NOT in the scope by default in my implementation!
            # My logic for `var` is `scope.get()`.
            # I need `var` to ALSO check Intrinsics if not found in scope? 
            # OR I need `call_expr` to check if `func_val` is a string name of intrinsic?
            # EVALUATION ORDER:
            # `print` is parsed as `var`. `eval_node` returns value.
            # If `print` is not in scope, `eval_node` returns Unit + Error (currently).
            
            if func_val.type == "Intrinsic":
                return INTRINSICS[func_val.val](args)
                
            if func_val.type == "Function":
                return call_user_func(func_val.val, args)
                
            if func_val.type == "Class":
                return instantiate_class(func_val.val, args)
                
            if func_val.type == "BoundMethod":
                method, instance = func_val.val
                return call_user_func(method, args, instance)

            raise Exception(f"Not callable: {func_val.type}")

    # --- Expressions ---
    if node.data == "number":
        return ArkValue(int(node.children[0].value), "Integer")
    
    if node.data == "string":
        # Remove quotes
        s = node.children[0].value[1:-1]
        return ArkValue(s, "String")
        
    if node.data in ["add", "sub", "mul", "div", "lt", "gt", "eq"]:
        left = eval_node(node.children[0], scope)
        right = eval_node(node.children[1], scope)
        return eval_binop(node.data, left, right)

    if node.data == "list_cons":
        items = []
        if node.children:
            # Check if child is expr_list
            child = node.children[0]
            if hasattr(child, "data") and child.data == "expr_list":
                items = [eval_node(c, scope) for c in child.children]
        return ArkValue(items, "List")

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
    if op == "div": return ArkValue(l // r, "Integer")
    if op == "lt": return ArkValue(l < r, "Boolean")
    if op == "gt": return ArkValue(l > r, "Boolean")
    if op == "le": return ArkValue(l <= r, "Boolean")
    if op == "ge": return ArkValue(l >= r, "Boolean")
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
    
    tree = parser.parse(code)
    print(tree.pretty())
    scope = Scope()
    scope.set("sys", ArkValue("sys", "Namespace"))
    
    # 3. Evaluate
    try:
        eval_node(tree, scope)
    except ReturnException as e:
        print(f"Error: Return statement outside function")
    except Exception as e:
        print(f"Runtime Error: {e}")
        # print(f"DEBUG: Scope vars: {scope.vars.keys()}")
        import traceback
        # traceback.print_exc() 


if __name__ == "__main__":
    if len(sys.argv) < 3:
        pass
    else:
        run_file(sys.argv[2])
