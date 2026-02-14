# Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
#
# This file is part of the Ark Sovereign Compiler.
#
# LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
#
# 1. OPEN SOURCE: You may use this file under the terms of the GNU Affero
#    General Public License v3.0. If you link to this code, your ENTIRE
#    application must be open-sourced under AGPLv3.
#
# 2. COMMERCIAL: For proprietary use, you must obtain a Commercial License
#    from Sovereign Systems.
#
# PATENT NOTICE: Protected by US Patent App #63/935,467.
# NO IMPLIED LICENSE to rights of Mohamad Al-Zawahreh or Sovereign Systems.

import lark
from lark import Transformer, v_args

class ArkTransformer(Transformer):
    def param_list(self, args):
        # args is list of IDENTIFIER tokens
        # Default type is Any (Shared) or Integer (Linear check handles validation)
        return [(str(t), {"type_name": "Any"}) for t in args]

    def number(self, n):
        return int(n[0])

    def string(self, s):
        return {"type": "string", "val": s[0][1:-1]}

    def var(self, v):
        return {"type": "var", "name": str(v[0])}
    
    def term(self, args):
        return args[0]
    
    def call_expr(self, args):
        func = args[0]
        name = str(func)
        if isinstance(func, dict):
             if func.get("type") == "var":
                name = func["name"]
             elif func.get("type") == "get_attr":
                name = func["full_path"]
            
        if len(args) > 1:
            params = args[1]
        return {"type": "call", "function": name, "args": params}

    def get_item(self, args):
        # Sugar for: sys.list.get(base, index)
        return {"type": "call", "function": "sys.list.get", "args": [args[0], args[1]]}

    def get_attr(self, args):
        obj = args[0]
        attr = str(args[1])
        
        # Try to resolve full path string for intrinsics/static calls
        base_path = ""
        if isinstance(obj, dict):
            if obj.get("type") == "var":
                base_path = obj["name"]
            elif obj.get("type") == "get_attr":
                base_path = obj["full_path"]
        
        full_path = f"{base_path}.{attr}" if base_path else attr
        
        return {"type": "get_attr", "full_path": full_path, "object": obj, "attr": attr}

    def expr_list(self, args):
        return args

    def add(self, args):
        return {"op": "add", "left": args[0], "right": args[1]}

    def list_cons(self, items):
        elements = items[0] if items and items[0] is not None else []
        return {"type": "list", "value": elements}

    def sub(self, args):
        return {"op": "sub", "left": args[0], "right": args[1]}

    def mul(self, args):
        return {"op": "mul", "left": args[0], "right": args[1]}

    def gt(self, args):
        return {"op": "gt", "left": args[0], "right": args[1]}

    def lt(self, args):
        return {"op": "lt", "left": args[0], "right": args[1]}

    def eq(self, args):
        return {"op": "eq", "left": args[0], "right": args[1]}

    def hole(self, args):
        return {"op": "hole", "meta": "Awaiting Neuro-Symbolic Synthesis"}

    def arg_list(self, args):
        return args

    def arg(self, args):
        name = str(args[0])
        ty = args[1] if len(args) > 1 else {"type_name": "Any"}
        return (name, ty)

    def function_def(self, args):
        # function_def: "func" IDENTIFIER "(" [param_list] ")" "{" block "}"
        name = str(args[0])
        
        if len(args) == 3:
            params = args[1] 
            body = args[2]
        else:
            params = []
            body = args[1]

        # Return type not in grammar yet, assume Unit or inferred
        ret_type = {"type_name": "Unit"}
        
        return {
            "type": "function_def",
            "name": name,
            "inputs": params,
            "output": ret_type,
            "body": body
        }

    def assignment(self, args):
        return {"type": "assignment", "target": str(args[0]), "value": args[1]}

    def if_stmt(self, args):
        # if expr block [else block]
        condition = args[0]
        then_block = args[1]
        else_block = args[2] if len(args) > 2 else None
        return {"type": "if", "condition": condition, "then_block": then_block, "else_block": else_block}

    def while_stmt(self, args):
        # while expr block
        return {"type": "while", "condition": args[0], "body": args[1]}

    def flow_statement(self, args):
        return {"type": "flow", "target": str(args[0]), "annotation": args[1]}

    def type_expr(self, args):
        # Basic handling given the grammar: 
        # type_expr: IDENTIFIER | "Linear" "<" IDENTIFIER ">"
        if len(args) == 1:
            return {"type_name": str(args[0])}
        else:
            return {"type_name": "Linear", "inner": str(args[0])}

    def neuro_block(self, args):
        # neuro_block: "@train" "(" IDENTIFIER ")" "{" block "}" | ...
        # Simplified for prototype
        model_name = str(args[0])
        body = args[1] if len(args) > 1 else []
        return {"type": "neuro_block", "model": model_name, "body": body}
    
    def block(self, args):
        return args

    def statement(self, args):
        return args[0]

    def expression(self, args):
        return args[0]

    def start(self, args):
        return {"program": args}

class QiParser:
    _parsers = {}

    def __init__(self, grammar_path="meta/ark.lark"):
        if grammar_path not in self._parsers:
            with open(grammar_path, "r") as f:
                grammar = f.read()
            self._parsers[grammar_path] = lark.Lark(grammar, start="start", parser="lalr", transformer=ArkTransformer())
        self.parser = self._parsers[grammar_path]

    def parse(self, code):
        return self.parser.parse(code)
