# Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
#
# This file is part of the Ark Sovereign Compiler.
#
# LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).

import time
import os
import sys
import lark
from lark import Lark, Transformer, v_args, Token, Tree
from lark.exceptions import UnexpectedToken, UnexpectedCharacters

# Diagnostics
DEBUG = os.environ.get("ARK_PARSE_DEBUG") == "true"

class ArkTransformer(Transformer):
    def __init__(self, source_file="<unknown>"):
        self.source_file = source_file
        self.node_count = 0

    def _add_meta(self, node, meta):
        self.node_count += 1
        if isinstance(node, dict) and meta:
            try:
                node["line"] = meta.line
                node["column"] = meta.column
                node["end_line"] = meta.end_line
                node["end_column"] = meta.end_column
                node["file"] = self.source_file
            except AttributeError:
                pass
        return node

    @v_args(meta=True)
    def start(self, meta, args):
        program = []
        for a in args:
            if isinstance(a, list):
                program.extend(a)
            elif a is not None:
                program.append(a)
        return {"program": program, "file": self.source_file}
    
    # top_level_item inlined

    @v_args(meta=True)
    def import_stmt(self, meta, args):
        module_path = ".".join(str(token) for token in args)
        return self._add_meta({"type": "import", "module": module_path}, meta)

    @v_args(meta=True)
    def function_def(self, meta, args):
        idx = 0
        doc = None
        if isinstance(args[idx], Token) and args[idx].type == 'DOC_COMMENT':
            doc = args[idx]
            idx += 1
        
        name = str(args[idx])
        idx += 1
        params = args[idx]
        body = args[idx+1]
        
        doc_str = doc.value.strip("/") if doc else None
        inputs = params if params else []
        
        node = {
            "type": "function_def",
            "name": name,
            "inputs": inputs,
            "output": {"type_name": "Unit"},
            "body": body
        }
        if doc_str: node["doc"] = doc_str
        return self._add_meta(node, meta)

    @v_args(meta=True)
    def class_def(self, meta, args):
        idx = 0
        doc = None
        if isinstance(args[idx], Token) and args[idx].type == 'DOC_COMMENT':
            doc = args[idx]
            idx += 1

        name = str(args[idx])
        methods = args[idx+1:]

        doc_str = doc.value.strip("/") if doc else None

        node = {
            "type": "class_def",
            "name": name,
            "methods": methods
        }
        if doc_str: node["doc"] = doc_str
        return self._add_meta(node, meta)

    def doc_wrapper(self, args): return None

    @v_args(meta=True)
    def match_stmt(self, meta, args):
        return self._add_meta({"type": "match", "expression": args[0], "cases": args[1:]}, meta)

    def match_case(self, args):
        return {"pattern": args[0], "body": args[1]}

    @v_args(meta=True)
    def try_stmt(self, meta, args):
        return self._add_meta({
            "type": "try",
            "try_block": args[0],
            "error_var": str(args[1]),
            "catch_block": args[2]
        }, meta)

    @v_args(meta=True)
    def lambda_expr(self, meta, args):
        params = []
        body = []
        if len(args) > 0:
             body = args[-1]
             if len(args) > 1 and args[0]:
                 params = args[0]

        return self._add_meta({
            "type": "lambda",
            "inputs": params,
            "body": body
        }, meta)

    def param_list(self, args):
        return [(str(t), {"type_name": "Any"}) for t in args]

    def block(self, args): return args

    @v_args(meta=True)
    def assign_var(self, meta, args):
        return self._add_meta({"type": "assignment", "target": str(args[0]), "value": args[1]}, meta)

    @v_args(meta=True)
    def assign_destructure(self, meta, args):
        val = args[-1]
        vars_ = [str(x) for x in args[:-1]]
        return self._add_meta({"type": "assignment_destructure", "targets": vars_, "value": val}, meta)

    @v_args(meta=True)
    def assign_attr(self, meta, args):
        return self._add_meta({"type": "assignment", "target": args[0], "attr": str(args[1]), "value": args[2]}, meta)

    @v_args(meta=True)
    def assign_op(self, meta, args):
        target = args[0]
        op = str(args[1])
        val = args[2]
        return self._add_meta({"type": "assignment_op", "target": target, "op": op, "value": val}, meta)

    def range_exclusive(self, args): return {"type": "range", "start": args[0], "end": args[1], "inclusive": False}
    def range_inclusive(self, args): return {"type": "range", "start": args[0], "end": args[1], "inclusive": True}

    def f_string(self, args):
        return {"type": "f_string", "val": str(args[0])[2:-1]}

    def multi_string(self, args):
        return {"type": "string", "val": str(args[0])[3:-3]}

    def optional_chain(self, args):
        return {"type": "optional_chain", "object": args[0], "attr": str(args[1])}

    def pipe_op(self, args):
        return {"type": "call", "function": args[1], "args": [args[0]]}

    def number(self, n): return int(n[0])
    def string(self, s): return {"type": "string", "val": s[0][1:-1]}
    def var(self, v): return {"type": "var", "name": str(v[0])}

    def call_expr(self, args):
        func = args[0]
        name = str(func)
        if isinstance(func, dict):
            if func.get("type") == "var": name = func["name"]
            elif func.get("type") == "get_attr": name = func.get("full_path", "")
        params = args[1] if len(args) > 1 and args[1] is not None else []
        return {"type": "call", "function": name, "args": params}

    def get_attr(self, args):
        obj = args[0]
        attr = str(args[1])
        base = ""
        if isinstance(obj, dict):
             if obj.get("type") == "var": base = obj["name"]
             elif obj.get("type") == "get_attr": base = obj.get("full_path", "")
        full = f"{base}.{attr}" if base else attr
        return {"type": "get_attr", "full_path": full, "object": obj, "attr": attr}

    def get_item(self, args):
        return {"type": "call", "function": "sys.list.get", "args": [args[0], args[1]]}

    def list_cons(self, items):
        return {"type": "list", "value": items[0] if items and items[0] else []}
    
    def struct_init(self, args):
        return {"type": "struct_init", "fields": args[0] if args and args[0] else []}

    def field_list(self, args): return args
    def field_init(self, args): return (str(args[0]), args[1])

    def expr_list(self, args): return args

    def add(self, args): return {"op": "add", "left": args[0], "right": args[1]}
    def sub(self, args): return {"op": "sub", "left": args[0], "right": args[1]}
    def mul(self, args): return {"op": "mul", "left": args[0], "right": args[1]}
    def div(self, args): return {"op": "div", "left": args[0], "right": args[1]}
    def mod(self, args): return {"op": "mod", "left": args[0], "right": args[1]}
    def gt(self, args): return {"op": "gt", "left": args[0], "right": args[1]}
    def lt(self, args): return {"op": "lt", "left": args[0], "right": args[1]}
    def ge(self, args): return {"op": "ge", "left": args[0], "right": args[1]}
    def le(self, args): return {"op": "le", "left": args[0], "right": args[1]}
    def eq(self, args): return {"op": "eq", "left": args[0], "right": args[1]}
    def neq(self, args): return {"op": "neq", "left": args[0], "right": args[1]}
    def logical_or(self, args): return {"op": "or", "left": args[0], "right": args[1]}
    def logical_and(self, args): return {"op": "and", "left": args[0], "right": args[1]}
    def not_op(self, args): return {"op": "not", "operand": args[0]}
    def neg_op(self, args): return {"op": "neg", "operand": args[0]}
    def bit_not_op(self, args): return {"op": "bit_not", "operand": args[0]}

    def if_stmt(self, args):
        cond = args[0]
        then = args[1]
        els = None
        i = 2
        current_else = None
        branches = [(cond, then)]

        while i < len(args):
            if i + 1 < len(args):
                branches.append((args[i], args[i+1]))
                i += 2
            else:
                current_else = args[i]
                i += 1

        node = current_else
        for b_cond, b_then in reversed(branches):
            node = {
                "type": "if",
                "condition": b_cond,
                "then_block": b_then,
                "else_block": node
            }
        return node

    def while_stmt(self, args): return {"type": "while", "condition": args[0], "body": args[1]}
    def return_stmt(self, args): return {"type": "return", "value": args[0]}
    def statement(self, args): return args[0]
    def expression(self, args): return args[0]


class QiParser:
    _parsers = {}

    def __init__(self, grammar_path="meta/ark.lark"):
        self.grammar_path = grammar_path
        if grammar_path not in self._parsers:
            with open(grammar_path, "r") as f:
                grammar = f.read()
            self._parsers[grammar_path] = Lark(
                grammar,
                start=["start", "top_level_item"],
                parser="lalr",
                propagate_positions=True
            )
        self.parser = self._parsers[grammar_path]

    def parse(self, code, source_file="<unknown>"):
        """
        Parses code. On error, skips to next statement boundary and continues.
        Returns: (AST dict, List[str] errors)
        """
        start_time = time.time()

        try:
            tree = self.parser.parse(code, start="start")
            transformer = ArkTransformer(source_file)
            ast = transformer.transform(tree)

            if DEBUG:
                dur = (time.time() - start_time) * 1000
                print(f"[QiParser] Success: {len(code)} bytes in {dur:.2f}ms. Nodes: {transformer.node_count}")

            return ast, []

        except (UnexpectedToken, UnexpectedCharacters) as e:
            error_msg = f"Syntax Error at line {e.line}, col {e.column}: {e}"
            if DEBUG: print(f"[QiParser] {error_msg}")

            pos = e.pos_in_stream
            rest = code[pos:]

            nl = rest.find('\n')
            if nl != -1:
                new_code = code[pos+nl+1:]
                partial_ast, more_errors = self.parse(new_code, source_file)
                return partial_ast, [error_msg] + more_errors

            return None, [error_msg]

        except Exception as e:
            return None, [str(e)]
