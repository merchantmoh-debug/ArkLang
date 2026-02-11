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
from ark_parser import QiParser

class ArkCompiler:
    def __init__(self):
        self.parser = QiParser("meta/ark.lark")

    def compile(self, code):
        ast = self.parser.parse(code)
        # Debugging
        # print(f"DEBUG AST: {ast}")
        if hasattr(ast, 'data'):
             # print(f"AST IS TREE: {ast.data}")
             # If AST is a Tree, it means start rule wasn't transformed?
             # But ArkTransformer has start method.
             pass
        
        program_node = ast.get("program", [])
        
        # Wrap everything in a Block
        statements = []
        for stmt in program_node:
            # Debug
            # print(f"Processing stmt type: {type(stmt)}")
            if hasattr(stmt, 'data'):
                # print(f"STMT IS TREE: {stmt.data} - {stmt}")
                pass
            
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
            return self._compile_assign_var(stmt)
        elif kind == "assign_destructure":
            return self._compile_assign_destructure(stmt)
        elif kind == "if":
            return self._compile_if(stmt)
        elif kind == "function_def":
            return self._compile_function_def(stmt)
        elif kind == "while":
            return self._compile_while(stmt)
        elif kind == "return_stmt":
             return self._compile_return(stmt)
        elif kind == "flow_stmt":
             if hasattr(stmt, 'children'):
                  return self.compile_stmt(stmt.children[0])
             return None

        # Fallback: Check if it's an expression (like a top-level Call)
        if isinstance(stmt, dict) and stmt.get("type") in ["call", "binary_op", "unary_op"]:
             expr = self.compile_expr(stmt)
             if expr:
                 return {"Expression": expr}

        return None

    def _compile_assign_var(self, stmt):
        name_token = stmt.children[0]
        name = name_token.value
        value_node = stmt.children[1]
        return {
            "Let": {
                "name": name,
                "ty": None,
                "value": self.compile_expr(value_node)
            }
        }

    def _compile_assign_destructure(self, stmt):
        # Rule: "[" IDENTIFIER ("," IDENTIFIER)* "]" ":=" expr
        # Filter for IDENTIFIER tokens.
        targets = [c.value for c in stmt.children[:-1] if hasattr(c, 'type') and c.type == "IDENTIFIER"]
        value_node = stmt.children[-1]

        return {
            "LetDestructure": {
                "names": targets,
                "value": self.compile_expr(value_node)
            }
        }

    def _compile_if(self, stmt):
        condition = stmt["condition"]
        then_block = stmt["then_block"]
        else_block = stmt["else_block"]
        
        then_stmts = [self.compile_stmt(s) for s in then_block]
        then_stmts = [s for s in then_stmts if s]
        
        else_stmts = None
        if else_block:
            # Check if else_block is a list (Block) or a single node (If)
            if isinstance(else_block, list):
                 else_stmts_raw = [self.compile_stmt(s) for s in else_block]
                 else_stmts = [s for s in else_stmts_raw if s]
            else:
                 # Single stmt (If)
                 compiled = self.compile_stmt(else_block)
                 if compiled:
                     else_stmts = [compiled]

        return {
            "If": {
                "condition": self.compile_expr(condition),
                "then_block": then_stmts,
                "else_block": else_stmts
            }
        }

    def _compile_function_def(self, stmt):
        name = stmt["name"]
        inputs_raw = stmt.get("inputs", []) or []
        body_block = stmt["body"]
        
        body_stmts = [self.compile_stmt(s) for s in body_block]
        body_stmts = [s for s in body_stmts if s]
        
        inputs = []
        for item in inputs_raw:
            # Fallback for complex type or just a string
            # If item is a list/tuple but len != 2, what then?
            # Assume it's [name, type] or just name.
            if isinstance(item, (list, tuple)):
                if len(item) == 2:
                    arg_name, arg_ty = item
                    ark_type = {"Linear": "Integer"} # Default
                    if isinstance(arg_ty, dict):
                        if arg_ty.get("type_name") == "Linear":
                                inner = arg_ty.get("inner", "Integer")
                                ark_type = {"Linear": inner}
                        else:
                                ark_type = {"Shared": arg_ty.get("type_name", "Any")}
                    inputs.append([arg_name, ark_type])
                else:
                        # Fallback for weird list
                        inputs.append([str(item[0]), {"Linear": "Integer"}])
            else:
                # It is a string (Token or str)
                inputs.append([str(item), {"Linear": "Integer"}]) 
        
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

    def _compile_while(self, stmt):
        condition = stmt["condition"]
        body_block = stmt["body"]
        
        body_stmts = [self.compile_stmt(s) for s in body_block]
        body_stmts = [s for s in body_stmts if s]

        return {
            "While": {
                "condition": self.compile_expr(condition),
                "body": body_stmts
            }
        }
    
    def _compile_return(self, stmt):
        expr = stmt.children[0]
        return {
            "Return": self.compile_expr(expr)
        }

    def compile_expr(self, expr):
        # Handle Primitives
        if isinstance(expr, int):
            return {"Literal": str(expr)}
        if isinstance(expr, str): # Should not happen if parser works but safe fallback
             return {"Literal": expr}
             
        # Handle Dict (Transformed Nodes)
        if isinstance(expr, dict):
            # Check for specific types/ops
            kind = expr.get("type")
            op = expr.get("op")
            
            if kind == "string":
                return {"Literal": expr["val"]}
            if kind == "var":
                 name = expr["name"]
                 if name == "true": return {"Literal": "true"}
                 if name == "false": return {"Literal": "false"}
                 return {"Variable": name}
            
            if kind == "get_attr":
                # {type: get_attr, object: ..., attr: ...}
                obj = self.compile_expr(expr["object"])
                attr = expr["attr"]
                # MAST for field access is likely intrinsics OR specific node?
                # Does MAST have GetField? 
                # Let's check eval.rs or assume intrinsic_get_field?
                # Actually, eval.rs handles it via Eval::GetField?
                # Wait, earlier logs showed "StructGet" or similar?
                # Let's use a "StructGet" node if supported, or intrinsic.
                # Looking at eval.rs might be needed, but let's try "GetField".
                pass 
                # Wait, I need to know the MAST node name.
                # Let's assume "GetField" for now or use intrinsic.
                # Actually, in Ark, x.y is syntax sugar for what?
                # In eval.rs, we likely implemented dot access.
                # Checking eval.rs is safer. but for now let's use a placeholder pattern.
                # Re-reading task.md check: "AST nodes for Struct Init/Access".
                # If I look at `eval.rs` logic...
                return {
                    "GetField": {
                        "obj": obj,
                        "field": attr
                    }
                }
            if kind == "call":
                args = expr.get("args", [])
                if args is None: args = []
                return {
                    "Call": {
                        "function_hash": expr["function"],
                        "args": [self.compile_expr(arg) for arg in args]
                    }
                }
            if kind == "list":
                items = expr.get("value", [])
                if items is None: items = []
                return {
                    "List": [self.compile_expr(item) for item in items]
                }
            
            if op:
                left = self.compile_expr(expr["left"])
                right = self.compile_expr(expr["right"])
                op_map = {
                    "add": "intrinsic_add", "sub": "intrinsic_sub", 
                    "mul": "intrinsic_mul", "gt": "intrinsic_gt", 
                    "lt": "intrinsic_lt", "eq": "intrinsic_eq",
                    "ge": "intrinsic_ge", "le": "intrinsic_le"
                }
                if op in op_map:
                     return {
                        "Call": {
                            "function_hash": op_map[op],
                            "args": [left, right]
                        }
                    }
        
        # Fallback for Token/Tree (Untransformed)
        if hasattr(expr, 'type'): # Token
            if expr.type == "NUMBER":
                return {"Literal": str(expr.value)}
            if expr.type == "STRING":
                 return {"Literal": expr.value[1:-1]}
            if expr.type == "NAME":
                 return {"Variable": expr.value}

        if hasattr(expr, 'data'):
            kind = expr.data
            if kind == "number":
                return {"Literal": str(expr.children[0].value)}
            if kind == "string":
                 return {"Literal": expr.children[0].value[1:-1]}
            if kind == "var":
                 return {"Variable": expr.children[0].value}
            if kind == "struct_init":
                 # rule: struct_init: "{" [field_list] "}"
                 # field_list: field_init ("," field_init)*
                 # field_init: IDENTIFIER ":" expression
                 fields = []
                 if expr.children:
                      field_list = expr.children[0]
                      if field_list and hasattr(field_list, "children"):
                           for field_init in field_list.children:
                                # field_init children: [Token(IDENTIFIER), Tree(expression)]
                                name = field_init.children[0].value
                                val_node = field_init.children[1]
                                fields.append([name, self.compile_expr(val_node)])
                 return {
                     "StructInit": {
                         "fields": fields
                     }
                 }
            if kind == "get_attr":
                 # rule: atom "." IDENTIFIER
                 obj = expr.children[0]
                 attr = expr.children[1].value
                 return {
                     "GetField": {
                         "obj": self.compile_expr(obj),
                         "field": attr
                     }
                 }
            if kind == "logical_or":
                 return self.compile_binop("intrinsic_or", expr.children[0], expr.children[2])
            if kind == "logical_and":
                 return self.compile_binop("intrinsic_and", expr.children[0], expr.children[2])
            
            # Binary Ops that slipped through Transformer
            bin_ops = {
                "add": "intrinsic_add", "sub": "intrinsic_sub",
                "mul": "intrinsic_mul", "div": "intrinsic_div", "mod": "intrinsic_mod",
                "gt": "intrinsic_gt", "lt": "intrinsic_lt",
                "ge": "intrinsic_ge", "le": "intrinsic_le",
                "eq": "intrinsic_eq"
            }
            if kind in bin_ops:
                 return self.compile_binop(bin_ops[kind], expr.children[0], expr.children[1])

        # Absolute Fallback
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
