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

ARK_PARSER = Lark(ARK_GRAMMAR, start="start", parser="lalr")


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
    val = eval_node(node.children[0], scope) if node.children else UNIT_VALUE
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
    parts = [t.value for t in node.children]
    if parts[0] == "std":
        rel_path = os.path.join("lib", *parts) + ".ark"
    else:
        rel_path = os.path.join(*parts) + ".ark"

    if not os.path.exists(rel_path):
        rel_path = os.path.join(*parts) + ".ark"
    
    if not os.path.exists(rel_path):
        raise Exception(f"Import Error: Module {'.'.join(parts)} not found at {rel_path}")

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
    if hasattr(node, "data"):
        handler = NODE_HANDLERS.get(node.data)
        if handler:
            return handler(node, scope)
    return UNIT_VALUE


def call_user_func(func: ArkFunction, args: List[ArkValue], instance: Optional[ArkValue] = None):
    func_scope = Scope(func.closure)
    if instance:
        func_scope.set("this", instance)
    for i, param in enumerate(func.params):
        if i < len(args):
            func_scope.set(param, args[i])
    try:
        eval_node(func.body, func_scope)
        return UNIT_VALUE
    except ReturnException as ret:
        return ret.value


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
    return UNIT_VALUE
