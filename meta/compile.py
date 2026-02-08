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

import json
import sys
from parser import QiParser

class ArkCompiler:
    def __init__(self):
        self.parser = QiParser("meta/ark.lark")

    def compile(self, code):
        ast = self.parser.parse(code)
        # Debugging
        # print(f"DEBUG AST: {ast}")
        if hasattr(ast, 'data'):
             print(f"AST IS TREE: {ast.data}")
             # If AST is a Tree, it means start rule wasn't transformed?
             # But ArkTransformer has start method.
        
        program_node = ast.get("program", [])
        
        # Wrap everything in a Block
        statements = []
        for stmt in program_node:
            # Debug
            # print(f"Processing stmt type: {type(stmt)}")
            if hasattr(stmt, 'data'):
                print(f"STMT IS TREE: {stmt.data} - {stmt}")
            
            compiled_stmt = self.compile_stmt(stmt)
            if compiled_stmt:
                statements.append(compiled_stmt)

        # Root is a Statement::Block
        root = {
            "Statement": {
                "Block": statements
            }
        }
        return json.dumps(root, indent=2)

    def compile_stmt(self, stmt):
        kind = stmt.get("type")
        if kind == "assignment":
            # Statement::Let { name, ty: None, value: ... }
            return {
                "Let": {
                    "name": stmt["target"],
                    "ty": None,
                    "value": self.compile_expr(stmt["value"])
                }
            }
        elif kind == "flow":
            return None 
        elif kind == "if":
            # {"If": { condition, then_block, else_block }}
            then_stmts = [self.compile_stmt(s) for s in stmt["then_block"]]
            then_stmts = [s for s in then_stmts if s] # Filter None
            
            else_stmts = None
            if stmt["else_block"]:
                else_compiled = [self.compile_stmt(s) for s in stmt["else_block"]]
                else_stmts = [s for s in else_compiled if s]

            return {
                "If": {
                    "condition": self.compile_expr(stmt["condition"]),
                    "then_block": then_stmts,
                    "else_block": else_stmts
                }
            }
        elif kind == "function_def":
            # Statement::Function(FunctionDef)
            # FunctionDef { name, inputs, output, body: MastNode }
            body_stmts = [self.compile_stmt(s) for s in stmt["body"]]
            body_stmts = [s for s in body_stmts if s]
            
            # Create MastNode for body (Simplified: Just verify structure, hash is done in Rust if Native, 
            # here we cheat and just emit the structure, Runtime will likely re-hash or we just emit content)
            # Actually, `FunctionDef` expects `body: Box<MastNode>`. 
            # In JSON, we can represent it as the struct fields.
            
            # Transform inputs from (name, type) to (name, ArkType)
            inputs = []
            for arg_name, arg_type in stmt["inputs"]:
                inputs.append([arg_name, {"Linear": "Integer"}]) # Hack: defaulting types for now
            
            return {
                "Function": {
                    "name": stmt["name"],
                    "inputs": inputs,
                    "output": {"Linear": "Integer"}, # Hack
                    "body": {
                        "hash": "todo_hash", 
                        "content": {
                            "Statement": {
                                "Block": body_stmts
                            }
                        }
                    }
                }
            }
        elif kind == "while":
            body_stmts = [self.compile_stmt(s) for s in stmt["body"]]
            body_stmts = [s for s in body_stmts if s]

            return {
                "While": {
                    "condition": self.compile_expr(stmt["condition"]),
                    "body": body_stmts
                }
            }
        elif kind == "neuro_block":
             return None
        
        # Check if it's an expression (dict with no 'type' or type='var'/'string' or op=...)
        # We can try to compile it as an expression and wrap in Statement::Expression
        try:
            expr_obj = self.compile_expr(stmt)
            return {"Expression": expr_obj}
        except:
            return None

    def compile_expr(self, expr):
        if isinstance(expr, int):
            return {"Literal": str(expr)}
        
        if isinstance(expr, dict):
            start_type = expr.get("type")
            if start_type == "var":
                return {"Variable": expr["name"]}
            if start_type == "string":
                return {"Literal": expr["val"]}
            if start_type == "call":
                return {
                    "Call": {
                        "function_hash": expr["function"],
                        "args": [self.compile_expr(arg) for arg in expr["args"]]
                    }
                }
            
            op = expr.get("op")
            if op == "add":
                return self.compile_binop("intrinsic_add", expr["left"], expr["right"])
            elif op == "sub":
                return self.compile_binop("intrinsic_sub", expr["left"], expr["right"])
            elif op == "mul":
                return self.compile_binop("intrinsic_mul", expr["left"], expr["right"])
            elif op == "gt":
                return self.compile_binop("intrinsic_gt", expr["left"], expr["right"])
            elif op == "lt":
                return self.compile_binop("intrinsic_lt", expr["left"], expr["right"])
            elif op == "eq":
                return self.compile_binop("intrinsic_eq", expr["left"], expr["right"])
            elif op == "hole":
                return {"Literal": "HOLE"} # Placeholder
        
        return {"Literal": str(expr)}

    def compile_binop(self, intrinsic, left, right):
        return {
            "Call": {
                "function_hash": intrinsic,
                "args": [
                    self.compile_expr(left),
                    self.compile_expr(right)
                ]
            }
        }

if __name__ == "__main__":
    if len(sys.argv) > 1:
        with open(sys.argv[1], 'r') as f:
            code = f.read()
    else:
        # Default test
        code = """
        x := 10
        y := 20
        z := x + y
        """
    
    compiler = ArkCompiler()
    output = compiler.compile(code)
    
    if len(sys.argv) > 2:
        with open(sys.argv[2], 'w', encoding='utf-8') as f:
            f.write(output)
    else:
        print(output)
