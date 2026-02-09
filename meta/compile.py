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
        kind = stmt.data if hasattr(stmt, 'data') else stmt.get("type")
        
        if kind == "assign_var":
            # Rule: assign_var: NAME ":=" expr
            name = stmt.children[0].value
            value_node = stmt.children[1]
            return {
                "Let": {
                    "name": name,
                    "ty": None,
                    "value": self.compile_expr(value_node)
                }
            }
        elif kind == "flow_stmt":
            # Wrapper rule, recurse
            return self.compile_stmt(stmt.children[0])
            
        elif kind == "if_stmt":
            # Rule: if_stmt: "if" expr block ("else" block)?
            condition = stmt.children[0]
            then_block = stmt.children[1].children # block -> children
            
            then_stmts = [self.compile_stmt(s) for s in then_block]
            then_stmts = [s for s in then_stmts if s]
            
            else_stmts = None
            if len(stmt.children) > 2 and stmt.children[2]:
                else_block_node = stmt.children[2]
                else_stmts_raw = [self.compile_stmt(s) for s in else_block_node.children]
                else_stmts = [s for s in else_stmts_raw if s]

            return {
                "If": {
                    "condition": self.compile_expr(condition),
                    "then_block": then_stmts,
                    "else_block": else_stmts
                }
            }
        elif kind == "function_def":
            # Rule: function_def: "func" NAME ["(" param_list ")"] block
            # Children: [NAME, (optional param_list), block]
            name = stmt.children[0].value
            params = []
            body_idx = 1
            if len(stmt.children) > 1 and hasattr(stmt.children[1], "data") and stmt.children[1].data == "param_list":
                params = [t.value for t in stmt.children[1].children]
                body_idx = 2
            
            body_node = stmt.children[body_idx]
            body_stmts = [self.compile_stmt(s) for s in body_node.children]
            body_stmts = [s for s in body_stmts if s]
            
            # Map params
            inputs = []
            for arg_name in params:
                inputs.append([arg_name, {"Linear": "Integer"}]) 
            
            return {
                "Function": {
                    "name": name,
                    "inputs": inputs,
                    "output": {"Linear": "Integer"}, 
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
        elif kind == "while_stmt":
            # Rule: while_stmt: "while" expr block
            condition = stmt.children[0]
            body_node = stmt.children[1]
            body_stmts = [self.compile_stmt(s) for s in body_node.children]
            body_stmts = [s for s in body_stmts if s]

            return {
                "While": {
                    "condition": self.compile_expr(condition),
                    "body": body_stmts
                }
            }
        elif kind == "neuro_block":
             return None
        
        # Check if it's an expression (call_expr usually appears as a statement)
        if kind == "call_expr":
             expr_obj = self.compile_expr(stmt)
             return {"Expression": expr_obj}

        # Fallback for untyped dicts (if any) or unknown tree nodes
        try:
            expr_obj = self.compile_expr(stmt)
            return {"Expression": expr_obj}
        except:
            print(f"Unknown Statement Kind: {kind}")
            return None

    def compile_expr(self, expr):
        # Handle Token (Literal values)
        if hasattr(expr, 'type'): # Token
            if expr.type == "NUMBER":
                return {"Literal": str(expr.value)}
            if expr.type == "STRING":
                 # Remove quotes
                return {"Literal": expr.value[1:-1]}
            if expr.type == "NAME":
                 return {"Variable": expr.value}

        # Handle Tree (Complex expressions)
        if hasattr(expr, 'data'):
            kind = expr.data
            
            if kind == "number":
                return {"Literal": str(expr.children[0].value)}
            if kind == "string":
                 return {"Literal": expr.children[0].value[1:-1]}
            if kind == "var":
                 return {"Variable": expr.children[0].value}
                 
            if kind == "call_expr":
                # Rule: call_expr: expr "(" [arg_list] ")"
                func_expr = self.compile_expr(expr.children[0])
                
                # Extract function name hash if it's a variable
                func_hash = "unknown"
                if "Variable" in func_expr:
                    func_hash = func_expr["Variable"]
                
                args = []
                if len(expr.children) > 1:
                    arg_list = expr.children[1]
                    if hasattr(arg_list, 'data') and arg_list.data == 'arg_list':
                        args = [self.compile_expr(c) for c in arg_list.children]
                
                return {
                    "Call": {
                        "function_hash": func_hash,
                        "args": args
                    }
                }
                
            # Binary Ops
            if kind in ["add", "sub", "mul", "gt", "lt", "eq"]:
                left = expr.children[0]
                right = expr.children[1]
                op_map = {
                    "add": "intrinsic_add", "sub": "intrinsic_sub", 
                    "mul": "intrinsic_mul", "gt": "intrinsic_gt", 
                    "lt": "intrinsic_lt", "eq": "intrinsic_eq"
                }
                return self.compile_binop(op_map[kind], left, right)

        if isinstance(expr, int):
            return {"Literal": str(expr)}
            
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
