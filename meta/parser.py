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
    def number(self, n):
        return int(n[0])

    def string(self, s):
        return {"type": "string", "val": s[0][1:-1]}

    def var(self, v):
        return {"type": "var", "name": str(v[0])}
    
    def term(self, args):
        return args[0]
    
    def call_expr(self, args):
        name = str(args[0])
        # args[1] is the expr_list Tree or None if empty?
        # With [expr_list], if present it's args[1].
        # But expr_list returns a list of exprs.
        # Let's handle args.
        params = []
        if len(args) > 1:
            params = args[1]
        return {"type": "call", "function": name, "args": params}

    def expr_list(self, args):
        return args

    def add(self, args):
        return {"op": "add", "left": args[0], "right": args[1]}

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
        # fn name(args) -> type { block }
        name = str(args[0])
        raw_args = args[1] if args[1] else []
        
        # raw_args is now [(name, type), (name, type)...] directly from arg_list
        inputs = raw_args 
            
        ret_type = args[2] if len(args) > 2 else {"type_name": "Unit"}
        body = args[3] if len(args) > 3 else []
        
        return {
            "type": "function_def",
            "name": name,
            "inputs": inputs,
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
    def __init__(self, grammar_path="meta/ark.lark"):
        with open(grammar_path, "r") as f:
            self.grammar = f.read()
        self.parser = lark.Lark(self.grammar, start="start", parser="lalr", transformer=ArkTransformer())

    def parse(self, code):
        return self.parser.parse(code)
