import ast
import sys
import os
from lark import Transformer, Tree, Token
from meta.ark import ArkValue, ArkInstance, ArkFunction, ArkClass, Scope, INTRINSICS, UNIT_VALUE, eval_binop, is_truthy

_FUNC_COUNTER = 0
def gen_func_name():
    global _FUNC_COUNTER
    _FUNC_COUNTER += 1
    return f"_ark_func_{_FUNC_COUNTER}"

class ArkCompiler(Transformer):
    def __init__(self):
        self.scope_stack = []
        self.in_function = False

    def start(self, items):
        stmts = self._ensure_statements(items)
        return ast.Module(body=stmts, type_ignores=[])

    def block(self, items):
        return self._ensure_statements(items)

    def _ensure_statements(self, items):
        if items is None: return [] # Handle None gracefully
        stmts = []
        if isinstance(items, list):
            flat_items = []
            for i in items:
                if isinstance(i, list): flat_items.extend(i)
                else: flat_items.append(i)
            items = flat_items
        
        for item in items:
            if isinstance(item, ast.expr):
                stmts.append(ast.Expr(value=item))
            elif isinstance(item, ast.stmt):
                stmts.append(item)
            elif isinstance(item, Tree):
                 print(f"WARNING: Unhandled Tree node in statements: {item.data}")
        return stmts

    # Expressions
    def number(self, items):
        val = int(items[0].value)
        return self._create_ark_value(ast.Constant(value=val), "Integer")

    def string(self, items):
        try:
            s = items[0].value[1:-1]
            s = s.encode('utf-8').decode('unicode_escape')
        except:
             s = items[0].value[1:-1]
        return self._create_ark_value(ast.Constant(value=s), "String")

    def var(self, items):
        name = items[0].value
        return ast.Call(
            func=ast.Name(id='resolve_var', ctx=ast.Load()),
            args=[ast.Name(id='scope', ctx=ast.Load()), ast.Constant(value=name)],
            keywords=[]
        )

    # BinOps
    def _handle_binop(self, op_name, items):
        return ast.Call(
            func=ast.Name(id='eval_binop', ctx=ast.Load()),
            args=[ast.Constant(value=op_name), items[0], items[1]],
            keywords=[]
        )

    def add(self, items): return self._handle_binop("add", items)
    def sub(self, items): return self._handle_binop("sub", items)
    def mul(self, items): return self._handle_binop("mul", items)
    def div(self, items): return self._handle_binop("div", items)
    def mod(self, items): return self._handle_binop("mod", items)
    def lt(self, items): return self._handle_binop("lt", items)
    def gt(self, items): return self._handle_binop("gt", items)
    def le(self, items): return self._handle_binop("le", items)
    def ge(self, items): return self._handle_binop("ge", items)
    def eq(self, items): return self._handle_binop("eq", items)
    def neq(self, items): return self._handle_binop("neq", items)

    # Statements
    def assign_var(self, items):
        name = items[0].value
        val = items[1]
        return ast.Expr(value=ast.Call(
            func=ast.Attribute(value=ast.Name(id='scope', ctx=ast.Load()), attr='set', ctx=ast.Load()),
            args=[ast.Constant(value=name), val],
            keywords=[]
        ))

    def flow_stmt(self, items):
        return items[0]

    def call_expr(self, items):
        func = items[0]
        args = items[1] if len(items) > 1 else []
        arg_elts = args if isinstance(args, list) else [args]
        return ast.Call(
            func=ast.Name(id='call_func', ctx=ast.Load()),
            args=[func, ast.List(elts=arg_elts, ctx=ast.Load()), ast.Name(id='scope', ctx=ast.Load())],
            keywords=[]
        )

    def expr_list(self, items):
        return items

    def param_list(self, items):
        return [t.value for t in items]

    def function_def(self, items):
        name = items[0].value
        params = []
        body_idx = 1
        
        if len(items) > 1 and isinstance(items[1], list) and (not items[1] or isinstance(items[1][0], str)):
             params = items[1]
             body_idx = 2
        
        if body_idx < len(items):
            body_stmts = self._ensure_statements(items[body_idx])
        else:
            body_stmts = []

        func_name = gen_func_name()

        if not body_stmts or not isinstance(body_stmts[-1], ast.Return):
            body_stmts.append(ast.Return(value=ast.Name(id='UNIT_VALUE', ctx=ast.Load())))

        inner_func = ast.FunctionDef(
            name=func_name,
            args=ast.arguments(
                posonlyargs=[],
                args=[ast.arg(arg='scope')],
                kwonlyargs=[],
                kw_defaults=[],
                defaults=[]
            ),
            body=body_stmts,
            decorator_list=[]
        )

        params_ast = ast.List(
            elts=[ast.Constant(value=p) for p in params],
            ctx=ast.Load()
        )

        assign_stmt = ast.Expr(value=ast.Call(
            func=ast.Attribute(value=ast.Name(id='scope', ctx=ast.Load()), attr='set', ctx=ast.Load()),
            args=[
                ast.Constant(value=name),
                self._create_ark_value(
                    ast.Call(
                        func=ast.Name(id='ArkFunction', ctx=ast.Load()),
                        args=[
                            ast.Constant(value=name),
                            params_ast,
                            ast.Name(id=func_name, ctx=ast.Load()),
                            ast.Name(id='scope', ctx=ast.Load())
                        ],
                        keywords=[]
                    ),
                    "Function"
                )
            ],
            keywords=[]
        ))
        
        return [inner_func, assign_stmt]

    def return_stmt(self, items):
        val = items[0] if items else ast.Name(id='UNIT_VALUE', ctx=ast.Load())
        return ast.Return(value=val)

    def if_stmt(self, items):
        cond = items[0]
        body = self._ensure_statements(items[1])
        
        top_if = ast.If(test=self._make_truthy_check(cond), body=body, orelse=[])
        current_if = top_if

        idx = 2
        while idx < len(items):
            # Skip if None
            if items[idx] is None:
                idx += 1
                continue

            if idx + 1 < len(items) and items[idx+1] is not None:
                # Need to distinguish between "else if" and "else".
                # Lark structure will have `expr` then `block` for `else if`.
                # For `else`, it will just be `block`.
                # But how do we know if items[idx] is expr or block?
                # AST nodes: expr is ast.Call, block is list.
                # If items[idx] is list, it's an else block.
                # If items[idx] is ast.Call, it's a condition.

                item = items[idx]
                if isinstance(item, list):
                    # Else block
                    else_body = self._ensure_statements(item)
                    current_if.orelse = else_body
                    idx += 1
                else:
                    # Elif condition
                    elif_cond = item
                    elif_body = self._ensure_statements(items[idx+1])
                    new_if = ast.If(test=self._make_truthy_check(elif_cond), body=elif_body, orelse=[])
                    current_if.orelse = [new_if]
                    current_if = new_if
                    idx += 2
            else:
                # Last item must be else block
                if isinstance(items[idx], list):
                    else_body = self._ensure_statements(items[idx])
                    current_if.orelse = else_body
                idx += 1

        return top_if

    def while_stmt(self, items):
        cond = items[0]
        body = self._ensure_statements(items[1])
        return ast.While(test=self._make_truthy_check(cond), body=body, orelse=[])

    def _create_ark_value(self, val_node, type_str):
        return ast.Call(
            func=ast.Name(id='ArkValue', ctx=ast.Load()),
            args=[val_node, ast.Constant(value=type_str)],
            keywords=[]
        )

    def _make_truthy_check(self, node):
        return ast.Call(
            func=ast.Name(id='is_truthy', ctx=ast.Load()),
            args=[node],
            keywords=[]
        )

def resolve_var(scope, name):
    from meta.ark import INTRINSICS
    val = scope.get(name)
    if val: return val
    if name in INTRINSICS:
        return ArkValue(name, "Intrinsic")
    raise Exception(f"Undefined variable: {name}")

def call_func(func_val, args, scope):
    from meta.ark import call_user_func, INTRINSICS
    if func_val.type == "Function":
        func = func_val.val
        if callable(func.body):
            func_scope = Scope(func.closure)
            for i, param in enumerate(func.params):
                if i < len(args):
                    func_scope.set(param, args[i])
            return func.body(func_scope)
        else:
            return call_user_func(func, args)
    elif func_val.type == "Intrinsic":
        if func_val.val in INTRINSICS:
            return INTRINSICS[func_val.val](args)
    raise Exception(f"Not callable: {func_val.type}")

def compile_to_python(code):
    from meta.ark import ARK_PARSER
    tree = ARK_PARSER.parse(code)
    compiler = ArkCompiler()
    py_ast = compiler.transform(tree)
    ast.fix_missing_locations(py_ast)
    
    code_obj = compile(py_ast, filename="<ark_jit>", mode="exec")
    return code_obj

if __name__ == "__main__": pass
