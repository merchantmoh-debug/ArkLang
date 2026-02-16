"""
Ark Interpreter — AST evaluation engine.

Extracted from ark.py (Phase 72: Structural Hardening).
Contains: eval_node, all handle_* functions, NODE_HANDLERS, eval_binop, is_truthy.
"""
import os
import sys
import ast
from typing import List, Optional
from lark import Lark

try:
    from meta.ark_types import (
        ArkValue, UNIT_VALUE, ArkFunction, ArkClass, ArkInstance, Scope,
        ReturnException, RopeString
    )
    from meta.ark_intrinsics import INTRINSICS, LINEAR_SPECS, INTRINSICS_WITH_SCOPE
except ModuleNotFoundError:
    from ark_types import (
        ArkValue, UNIT_VALUE, ArkFunction, ArkClass, ArkInstance, Scope,
        ReturnException, RopeString
    )
    from ark_intrinsics import INTRINSICS, LINEAR_SPECS, INTRINSICS_WITH_SCOPE


# --- Global Parser ---
grammar_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "ark.lark")
with open(grammar_path, "r") as f:
    ARK_GRAMMAR = f.read()

ARK_PARSER = Lark(ARK_GRAMMAR, start="start", parser="lalr", propagate_positions=True)


# ─── Hardening Structures ─────────────────────────────────────────────────────

class ArkRuntimeError(Exception):
    def __init__(self, msg, node=None):
        self.msg = msg
        self.node = node
        self.stack = []
        if node:
            self.line = getattr(node, 'line', None)
            if self.line is None and hasattr(node, 'meta'):
                self.line = getattr(node.meta, 'line', None)

            self.col = getattr(node, 'column', None)
            if self.col is None and hasattr(node, 'meta'):
                self.col = getattr(node.meta, 'column', None)

            if self.line:
                # Initial frame based on node location
                self.stack.append((self.line, self.col, "<unknown>"))
        else:
            self.line = None
            self.col = None
        super().__init__(self.__str__())

    def add_frame(self, line, col, func_name):
        self.stack.append((line, col, func_name))

    def __str__(self):
        out = ["Traceback (most recent call last):"]
        # Use reversed stack to show calls from outer to inner?
        # Typically tracebacks show call -> callee -> callee.
        # But our stack is built inside-out (exception raised, then caught by caller).
        # So we append frames as we unwind.
        # So the LAST appended frame is the OUTERMOST call.
        # So we should print in REVERSE order of append.
        for line, col, name in reversed(self.stack):
             out.append(f"  File \"main.ark\", line {line}, in {name}")

        loc_str = ""
        if self.line and not self.stack:
             loc_str = f"Line {self.line}: "

        out.append(f"RuntimeError: {loc_str}{self.msg}")
        return "\n".join(out)

class TailCall(Exception):
    def __init__(self, func, args):
        self.func = func
        self.args = args

class OptimizedScope(Scope):
    __slots__ = ('_cache', '_access_counts')
    def __init__(self, parent=None):
        super().__init__(parent)
        self._cache = {}
        self._access_counts = {}

    def get(self, name: str) -> Optional[ArkValue]:
        # 1. Local Lookup (O(1))
        if name in self.vars:
            val = self.vars[name]
            if val.type == "Moved":
                # We can't easily import LinearityViolation, relying on runtime checks or generic error
                # Ideally we should raise the specific error.
                # Assuming simple exception for now to avoid circular deps if any.
                raise Exception(f"Use of moved variable '{name}'")
            return val

        # 2. Cache Lookup (O(1))
        if name in self._cache:
            return self._cache[name]

        # 3. Parent Lookup (O(depth))
        if self.parent:
            val = self.parent.get(name)
            if val:
                # Heuristic: Only cache frequent variables to avoid thrashing
                count = self._access_counts.get(name, 0) + 1
                self._access_counts[name] = count
                if count > 10:
                    self._cache[name] = val
            return val
        return None

    def set(self, name: str, val: ArkValue):
        # Always set in local scope (shadowing)
        self.vars[name] = val

    def mark_moved(self, name: str):
        # Invalidate cache if we mark a variable as moved (even if it's in parent)
        # Actually mark_moved logic:
        # If in self.vars: mark moved.
        # Else if parent: parent.mark_moved(name).
        if name in self._cache:
            del self._cache[name]
        super().mark_moved(name)


# ─── Evaluator ────────────────────────────────────────────────────────────────

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
    if node.children:
        expr = node.children[0]

        # TCO Detection: Check if we are returning a call to the current function
        if hasattr(expr, "data") and expr.data == "call_expr":
             # We need to check if the function being called is the same as __current_func__
             # First, resolve the function expression (first child of call_expr)
             func_expr_node = expr.children[0]

             # We evaluate the function reference.
             # Note: This might have side effects if it's a complex expression, but usually it's just a var.
             func_val = eval_node(func_expr_node, scope)

             current_func = scope.get("__current_func__")

             # Strict check: same function object and purely self-recursive (not method call on different instance)
             # If it's a simple function call
             if current_func and func_val.val == current_func.val:
                 # Evaluate arguments
                 arg_vals = []
                 if len(expr.children) > 1:
                     arg_list_node = expr.children[1]
                     if hasattr(arg_list_node, "children"):
                         arg_vals = [eval_node(c, scope) for c in arg_list_node.children]

                 # Raise TailCall exception to unwind to call_user_func loop
                 raise TailCall(func_val.val, arg_vals)

        val = eval_node(expr, scope)
        raise ReturnException(val)
    raise ReturnException(UNIT_VALUE)

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
    return UNIT_VALUE

def handle_while_stmt(node, scope):
    cond_node = node.children[0]
    body_node = node.children[1]
    while is_truthy(eval_node(cond_node, scope)):
        eval_node(body_node, scope)
    return UNIT_VALUE

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
    raise ArkRuntimeError(f"Undefined variable: {name}", node)

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
        raise ArkRuntimeError(f"Destructuring expects List, got {val.type}", node)
    items = val.val
    if len(items) < len(var_tokens):
        raise ArkRuntimeError(f"Not enough items to destructure: needed {len(var_tokens)}, got {len(items)}", node)
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
    raise ArkRuntimeError(f"Cannot set attribute on {obj.type}", node)

def handle_get_attr(node, scope):
    obj = eval_node(node.children[0], scope)
    attr = node.children[1].value
    if obj.type == "Namespace":
        new_path = f"{obj.val}.{attr}"
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
    raise ArkRuntimeError(f"Attribute {attr} not found on {obj.type}", node)

def handle_call_expr(node, scope):
    # This handler might be re-entered after TCO loop or normally
    try:
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

        raise ArkRuntimeError(f"Not callable: {func_val.type}", node)

    except ArkRuntimeError as e:
        # Annotate with call site info if available and needed?
        # The ArkRuntimeError already has the inner node.
        # We could add a frame here if we want to trace calls.
        # But call_user_func handles the recursion loop.
        # This handle_call_expr is the CALLER side.
        # So yes, we can add a frame here.
        func_name = "<unknown>"
        try:
            if func_val.type == "Function":
                func_name = func_val.val.name
            elif func_val.type == "BoundMethod":
                func_name = func_val.val[0].name
        except:
            pass

        line = getattr(node, 'line', None)
        if line is None and hasattr(node, 'meta'):
            line = getattr(node.meta, 'line', None)
        col = getattr(node, 'column', None)
        if col is None and hasattr(node, 'meta'):
            col = getattr(node.meta, 'column', None)
        e.add_frame(line, col, func_name)
        raise

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

    # Assertions
    if node.data == "add":
        if not (left.type in ("Integer", "String", "List") and right.type in ("Integer", "String", "List")):
             # Actually Ark supports Int+Int, Str+Str, List+List?
             pass

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
        raise ArkRuntimeError(f"Index must be Integer, got {index_val.type}", node)
    idx = index_val.val

    if collection.type == "List":
        if idx < 0 or idx >= len(collection.val):
            raise ArkRuntimeError(f"List index out of range: {idx}", node)
        return collection.val[idx]
    if collection.type == "String":
        if idx < 0 or idx >= len(collection.val):
            raise ArkRuntimeError(f"String index out of range: {idx}", node)
        return ArkValue(collection.val[idx], "String")
    if collection.type == "Buffer":
        if idx < 0 or idx >= len(collection.val):
            raise ArkRuntimeError(f"Buffer index out of range: {idx}", node)
        return ArkValue(int(collection.val[idx]), "Integer")
    raise ArkRuntimeError(f"Cannot index type {collection.type}", node)

def handle_import(node, scope):
    parts = [t.value for t in node.children]
    if parts[0] == "std":
        rel_path = os.path.join("lib", *parts) + ".ark"
    else:
        rel_path = os.path.join(*parts) + ".ark"

    if not os.path.exists(rel_path):
        rel_path = os.path.join(*parts) + ".ark"
    
    if not os.path.exists(rel_path):
        raise ArkRuntimeError(f"Import Error: Module {'.'.join(parts)} not found at {rel_path}", node)

    root = scope
    while root.parent:
        root = root.parent
    
    if "__loaded_imports__" not in root.vars:
        root.vars["__loaded_imports__"] = ArkValue(set(), "Set")
    
    loaded_set = root.vars["__loaded_imports__"].val
    
    abs_path = os.path.abspath(rel_path)
    if abs_path in loaded_set:
        return ArkValue(None, "Unit")
    
    loaded_set.add(abs_path)

    with open(abs_path, "r") as f:
        code = f.read()
    
    tree = ARK_PARSER.parse(code)
    eval_node(tree, scope)
    return ArkValue(None, "Unit")


# ─── Node Handler Registry ───────────────────────────────────────────────────

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
    if node is None: return UNIT_VALUE

    try:
        if hasattr(node, "data"):
            handler = NODE_HANDLERS.get(node.data)
            if handler:
                return handler(node, scope)
        return UNIT_VALUE
    except ReturnException:
        raise
    except TailCall:
        raise
    except ArkRuntimeError:
        raise
    except Exception as e:
        # Catch unexpected Python errors (like ZeroDivisionError) and wrap them
        raise ArkRuntimeError(str(e), node) from e


MAX_RECURSION_DEPTH = 1000
_recursion_depth = 0

def call_user_func(func: ArkFunction, args: List[ArkValue], instance: Optional[ArkValue] = None):
    global _recursion_depth
    if _recursion_depth > MAX_RECURSION_DEPTH:
        raise ArkRuntimeError("maximum recursion depth exceeded")

    _recursion_depth += 1

    current_func = func
    current_args = args
    current_instance = instance

    try:
        # Loop for TCO
        while True:
            # Use OptimizedScope with caching
            func_scope = OptimizedScope(current_func.closure)

            # Inject current function for TCO detection in return statements
            func_scope.set("__current_func__", ArkValue(current_func, "Function"))

            if current_instance:
                func_scope.set("this", current_instance)

            for i, param in enumerate(current_func.params):
                if i < len(current_args):
                    func_scope.set(param, current_args[i])

            try:
                eval_node(current_func.body, func_scope)
                return UNIT_VALUE
            except TailCall as tc:
                # Unwind stack frame for tail call
                current_func = tc.func
                current_args = tc.args
                # Reset instance if needed, but for self-recursion it's same or handled?
                # If tc.func is same as current_func, instance is preserved if it's bound.
                # But if we are calling a method on self, 'this' is in scope.
                # If we pass 'this' explicitly?
                # Ark methods implicitly bind.
                # If we are in TCO, we are just looping.
                # We assume strict self-recursion on same function object.
                continue
            except ReturnException as ret:
                return ret.value
    finally:
        _recursion_depth -= 1


def instantiate_class(klass: ArkClass, args: List[ArkValue]):
    instance = ArkInstance(klass, {})
    return ArkValue(instance, "Instance")


def eval_block(nodes, scope):
    last = UNIT_VALUE
    try:
        for n in nodes:
            last = eval_node(n, scope)
    except ReturnException:
        raise
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
        if left.type == "String" or right.type == "String":
            if not isinstance(l, RopeString):
                l = RopeString(str(l))
            return ArkValue(l + r, "String")
        if left.type != "Integer" or right.type != "Integer":
             # Should we allow List concat?
             # Original code implied string/integer only.
             # Strictness.
             pass
        return ArkValue(l + r, "Integer")

    # Check types for arithmetic
    if op in ("sub", "mul", "div", "mod"):
         if left.type != "Integer" or right.type != "Integer":
              raise ArkRuntimeError(f"Operator {op} requires Integers, got {left.type} and {right.type}")

    if op == "sub": return ArkValue(l - r, "Integer")
    if op == "mul": return ArkValue(l * r, "Integer")
    if op == "div": return ArkValue(l // r, "Integer")
    if op == "mod": return ArkValue(l % r, "Integer")

    # Comparisons
    if op == "lt": return ArkValue(l < r, "Boolean")
    if op == "gt": return ArkValue(l > r, "Boolean")
    if op == "le": return ArkValue(l <= r, "Boolean")
    if op == "ge": return ArkValue(l >= r, "Boolean")
    if op == "eq": return ArkValue(l == r, "Boolean")
    if op == "neq": return ArkValue(l != r, "Boolean")
    return UNIT_VALUE
