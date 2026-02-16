import sys
import os
import json
import argparse
from typing import Any, Dict, List, Optional, Union
from lark import Tree, Token

# Add repo root to path
sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

try:
    from meta.ark import ARK_PARSER
except ImportError:
    # Fallback for when running directly
    sys.path.append(os.path.join(os.path.dirname(__file__), ".."))
    from meta.ark import ARK_PARSER

# ------------------------------------------------------------------------------
# 1. AST Serialization
# ------------------------------------------------------------------------------

class ArkASTSerializer:
    def to_json(self, node: Union[Tree, Token, List, Any]) -> Any:
        if isinstance(node, Tree):
            handler_name = f"visit_{node.data}"
            handler = getattr(self, handler_name, self.generic_visit)
            return handler(node)
        elif isinstance(node, Token):
            return self.visit_token(node)
        elif isinstance(node, list):
            return [self.to_json(item) for item in node]
        elif node is None:
            return None
        else:
            return node

    def _get_meta(self, node: Union[Tree, Token]) -> Dict[str, Any]:
        meta = {}
        if isinstance(node, Token):
            meta["line"] = node.line
            meta["col"] = node.column
            meta["end_line"] = node.end_line
            meta["end_col"] = node.end_column
        elif isinstance(node, Tree):
            # Try to get meta from first child if available, or just use 0
            # Lark trees don't always have meta unless propagated
            if hasattr(node, "meta") and not node.meta.empty:
                meta["line"] = node.meta.line
                meta["col"] = node.meta.column
                meta["end_line"] = node.meta.end_line
                meta["end_col"] = node.meta.end_column
            elif node.children:
                # heuristic: grab from first child
                first = node.children[0]
                if isinstance(first, (Tree, Token)):
                    child_meta = self._get_meta(first)
                    meta.update(child_meta)

        # Default if missing
        if "line" not in meta: meta["line"] = 0
        if "col" not in meta: meta["col"] = 0
        return meta

    def generic_visit(self, node: Tree) -> Dict[str, Any]:
        # Fallback for unknown nodes
        return {
            "type": "Unknown",
            "kind": node.data,
            "children": [self.to_json(child) for child in node.children],
            **self._get_meta(node)
        }

    def visit_token(self, token: Token) -> Dict[str, Any]:
        # Handle specific token types if needed, or generic
        if token.type == "NUMBER":
            return {
                "type": "Literal",
                "value": int(token.value),
                "raw": token.value,
                **self._get_meta(token)
            }
        elif token.type == "STRING":
            return {
                "type": "Literal",
                "value": token.value[1:-1], # Strip quotes
                "raw": token.value,
                **self._get_meta(token)
            }
        elif token.type == "IDENTIFIER":
            return {
                "type": "Identifier",
                "name": token.value,
                **self._get_meta(token)
            }
        else:
            return {
                "type": "Token",
                "token_type": token.type,
                "value": token.value,
                **self._get_meta(token)
            }

    # --- Statements ---

    def visit_start(self, node: Tree) -> Dict[str, Any]:
        return {
            "type": "Program",
            "body": [self.to_json(child) for child in node.children],
            **self._get_meta(node)
        }

    def visit_block(self, node: Tree) -> Dict[str, Any]:
        return {
            "type": "Block",
            "stmts": [self.to_json(child) for child in node.children],
            **self._get_meta(node)
        }

    def visit_function_def(self, node: Tree) -> Dict[str, Any]:
        # Children: name (Token), [param_list], block
        name_token = node.children[0]
        params = []
        body_idx = 1

        if len(node.children) > 1:
             possible_params = node.children[1]
             if isinstance(possible_params, Tree) and possible_params.data == "param_list":
                 # param_list children are IDENTIFIER tokens
                 params = [p.value for p in possible_params.children]
                 body_idx = 2
             elif possible_params is None: # optional param_list was None
                 body_idx = 2

        body = node.children[body_idx]

        return {
            "type": "FunctionDecl",
            "name": name_token.value,
            "params": params,
            "body": self.to_json(body),
            **self._get_meta(node)
        }

    def visit_class_def(self, node: Tree) -> Dict[str, Any]:
        name_token = node.children[0]
        methods = [self.to_json(child) for child in node.children[1:]]
        return {
            "type": "ClassDecl",
            "name": name_token.value,
            "methods": methods,
            **self._get_meta(node)
        }

    def visit_return_stmt(self, node: Tree) -> Dict[str, Any]:
        value = None
        if node.children:
            value = self.to_json(node.children[0])
        return {
            "type": "Return",
            "value": value,
            **self._get_meta(node)
        }

    def visit_if_stmt(self, node: Tree) -> Dict[str, Any]:
        # structure: condition, then_block, [else_block_or_if]...
        # The grammar: "if" expr "{" block "}" ("else" "if" expr "{" block "}")* ["else" "{" block "}"]
        # Lark tree flattens this.
        # children: expr, block, expr, block, ... [block]

        children = node.children
        condition = self.to_json(children[0])
        then_block = self.to_json(children[1])
        else_block = None

        # If there are more children, it's else-if or else
        if len(children) > 2:
            # We need to reconstruct the nested Ifs or Else block
            # This is tricky because the flat structure doesn't explicitly group them.
            # But based on `ark_interpreter.py`, it just iterates.
            # We will try to reconstruct a nested If structure for JSON clarity.

            # Recursive helper
            def build_else_chain(index):
                if index >= len(children):
                    return None

                # Check if it's an else block (last one, no condition)
                if index == len(children) - 1:
                    return self.to_json(children[index])

                # It's an else-if
                cond = self.to_json(children[index])
                blk = self.to_json(children[index+1])
                nxt = build_else_chain(index + 2)

                return {
                    "type": "If",
                    "condition": cond,
                    "then_block": blk,
                    "else_block": nxt,
                    "line": cond.get("line", 0),
                    "col": cond.get("col", 0)
                }

            else_block = build_else_chain(2)

        return {
            "type": "If",
            "condition": condition,
            "then_block": then_block,
            "else_block": else_block,
            **self._get_meta(node)
        }

    def visit_while_stmt(self, node: Tree) -> Dict[str, Any]:
        return {
            "type": "While",
            "condition": self.to_json(node.children[0]),
            "body": self.to_json(node.children[1]),
            **self._get_meta(node)
        }

    def visit_match_stmt(self, node: Tree) -> Dict[str, Any]:
        # match expr { pattern => block, ... }
        subject = self.to_json(node.children[0])
        cases = []
        # Iterate remaining children (pattern, block pairs)
        for i in range(1, len(node.children), 2):
            pattern = self.to_json(node.children[i])
            body = self.to_json(node.children[i+1])
            cases.append({"pattern": pattern, "body": body})
        return {
            "type": "Match",
            "subject": subject,
            "cases": cases,
            **self._get_meta(node)
        }

    def visit_try_stmt(self, node: Tree) -> Dict[str, Any]:
        # try block catch (var) block
        try_block = self.to_json(node.children[0])
        catch_var = node.children[1].value if len(node.children) > 1 else None
        catch_block = self.to_json(node.children[2]) if len(node.children) > 2 else None
        return {
            "type": "TryCatch",
            "try_block": try_block,
            "catch_var": catch_var,
            "catch_block": catch_block,
            **self._get_meta(node)
        }

    def visit_import_stmt(self, node: Tree) -> Dict[str, Any]:
        # children are tokens forming the path
        path_parts = [t.value for t in node.children]
        return {
            "type": "Import",
            "path": path_parts,
            **self._get_meta(node)
        }

    # --- Assignments ---

    def visit_assign_var(self, node: Tree) -> Dict[str, Any]:
        name_token = node.children[0]
        value = node.children[1]
        return {
            "type": "VarDecl",
            "name": name_token.value,
            "value": self.to_json(value),
            **self._get_meta(node)
        }

    def visit_assign_destructure(self, node: Tree) -> Dict[str, Any]:
        # children: tokens..., value
        names = [t.value for t in node.children[:-1]]
        value = node.children[-1]
        return {
            "type": "Destructure",
            "names": names,
            "value": self.to_json(value),
            **self._get_meta(node)
        }

    def visit_assign_attr(self, node: Tree) -> Dict[str, Any]:
        obj = node.children[0]
        attr = node.children[1] # Token
        val = node.children[2]
        return {
            "type": "SetField",
            "object": self.to_json(obj),
            "field": attr.value,
            "value": self.to_json(val),
            **self._get_meta(node)
        }

    # --- Expressions ---

    def _visit_binary(self, node: Tree, op: str) -> Dict[str, Any]:
        return {
            "type": "Binary",
            "operator": op,
            "left": self.to_json(node.children[0]),
            "right": self.to_json(node.children[1]),
            **self._get_meta(node)
        }

    def visit_logical_or(self, node: Tree): return self._visit_binary(node, "or")
    def visit_logical_and(self, node: Tree): return self._visit_binary(node, "and")
    def visit_add(self, node: Tree): return self._visit_binary(node, "+")
    def visit_sub(self, node: Tree): return self._visit_binary(node, "-")
    def visit_mul(self, node: Tree): return self._visit_binary(node, "*")
    def visit_div(self, node: Tree): return self._visit_binary(node, "/")
    def visit_mod(self, node: Tree): return self._visit_binary(node, "%")
    def visit_lt(self, node: Tree): return self._visit_binary(node, "<")
    def visit_gt(self, node: Tree): return self._visit_binary(node, ">")
    def visit_le(self, node: Tree): return self._visit_binary(node, "<=")
    def visit_ge(self, node: Tree): return self._visit_binary(node, ">=")
    def visit_eq(self, node: Tree): return self._visit_binary(node, "==")
    def visit_neq(self, node: Tree): return self._visit_binary(node, "!=")

    def visit_call_expr(self, node: Tree) -> Dict[str, Any]:
        func = node.children[0]
        args = []
        if len(node.children) > 1:
            arg_list = node.children[1]
            if hasattr(arg_list, "children"):
                args = [self.to_json(c) for c in arg_list.children]

        return {
            "type": "Call",
            "function": self.to_json(func),
            "args": args,
            **self._get_meta(node)
        }

    def visit_get_attr(self, node: Tree) -> Dict[str, Any]:
        obj = node.children[0]
        attr = node.children[1] # Token
        return {
            "type": "FieldAccess",
            "object": self.to_json(obj),
            "field": attr.value,
            **self._get_meta(node)
        }

    def visit_get_item(self, node: Tree) -> Dict[str, Any]:
        obj = node.children[0]
        idx = node.children[1]
        return {
            "type": "Index",
            "object": self.to_json(obj),
            "index": self.to_json(idx),
            **self._get_meta(node)
        }

    def visit_list_cons(self, node: Tree) -> Dict[str, Any]:
        elements = []
        if node.children:
            expr_list = node.children[0]
            if expr_list and hasattr(expr_list, "children"):
                elements = [self.to_json(c) for c in expr_list.children]
        return {
            "type": "List",
            "elements": elements,
            **self._get_meta(node)
        }

    def visit_struct_init(self, node: Tree) -> Dict[str, Any]:
        fields = []
        if node.children:
            field_list = node.children[0]
            if field_list and hasattr(field_list, "children"):
                for f in field_list.children:
                    # field_init -> IDENTIFIER, expr
                    key = f.children[0].value
                    val = self.to_json(f.children[1])
                    fields.append({"key": key, "value": val})
        return {
            "type": "Struct",
            "fields": fields,
            **self._get_meta(node)
        }

    # Wrappers
    def visit_flow_stmt(self, node: Tree): return self.to_json(node.children[0])
    def visit_expression(self, node: Tree): return self.to_json(node.children[0])
    def visit_statement(self, node: Tree): return self.to_json(node.children[0])
    def visit_atom(self, node: Tree): return self.to_json(node.children[0])
    def visit_primary(self, node: Tree): return self.to_json(node.children[0])
    def visit_var(self, node: Tree):
        # var -> IDENTIFIER
        return {
            "type": "Identifier",
            "name": node.children[0].value,
            **self._get_meta(node)
        }
    def visit_number(self, node: Tree):
        return {
            "type": "Literal",
            "value": int(node.children[0].value),
            "raw": node.children[0].value,
            **self._get_meta(node)
        }
    def visit_string(self, node: Tree):
        return {
            "type": "Literal",
            "value": node.children[0].value[1:-1],
            "raw": node.children[0].value,
            **self._get_meta(node)
        }

# ------------------------------------------------------------------------------
# 2. JSON Deserialization (Roundtrip)
# ------------------------------------------------------------------------------

class ArkASTDeserializer:
    def from_json(self, data: Any) -> Union[Tree, Token, List, Any]:
        if isinstance(data, list):
            return [self.from_json(item) for item in data]
        if not isinstance(data, dict):
            return data

        node_type = data.get("type")
        if not node_type:
            return data

        method_name = f"build_{node_type}"
        builder = getattr(self, method_name, self.generic_build)
        return builder(data)

    def _create_token(self, type_: str, value: str, meta: Dict) -> Token:
        t = Token(type_, value)
        t.line = meta.get("line", 0)
        t.column = meta.get("col", 0)
        t.end_line = meta.get("end_line", 0)
        t.end_column = meta.get("end_col", 0)
        return t

    def _create_tree(self, data: str, children: List, meta: Dict) -> Tree:
        t = Tree(data, children)
        # We can attach meta if needed, but Lark usually computes it from children
        # or we rely on the children's tokens.
        return t

    def generic_build(self, data: Dict[str, Any]) -> Any:
        raise ValueError(f"Unknown node type for deserialization: {data.get('type')}")

    def build_Program(self, data):
        children = [self.from_json(c) for c in data["body"]]
        return self._create_tree("start", children, data)

    def build_Block(self, data):
        children = [self.from_json(c) for c in data["stmts"]]
        return self._create_tree("block", children, data)

    def build_FunctionDecl(self, data):
        name_token = self._create_token("IDENTIFIER", data["name"], data)
        params_node = None
        if data["params"]:
            param_tokens = [self._create_token("IDENTIFIER", p, data) for p in data["params"]]
            params_node = self._create_tree("param_list", param_tokens, data)

        body_node = self.from_json(data["body"])

        children = [name_token]
        if params_node:
            children.append(params_node)
        children.append(body_node)

        return self._create_tree("function_def", children, data)

    def build_ClassDecl(self, data):
        name_token = self._create_token("IDENTIFIER", data["name"], data)
        methods = [self.from_json(m) for m in data["methods"]]
        return self._create_tree("class_def", [name_token] + methods, data)

    def build_Return(self, data):
        children = []
        if data["value"]:
            children.append(self.from_json(data["value"]))
        return self._create_tree("return_stmt", children, data)

    def build_If(self, data):
        # Reconstruct flattened if structure
        children = []
        children.append(self.from_json(data["condition"]))
        children.append(self.from_json(data["then_block"]))

        current_else = data.get("else_block")
        while current_else:
            if isinstance(current_else, dict) and current_else.get("type") == "If":
                children.append(self.from_json(current_else["condition"]))
                children.append(self.from_json(current_else["then_block"]))
                current_else = current_else.get("else_block")
            else:
                # Final else block
                children.append(self.from_json(current_else))
                break

        return self._create_tree("if_stmt", children, data)

    def build_While(self, data):
        return self._create_tree("while_stmt", [
            self.from_json(data["condition"]),
            self.from_json(data["body"])
        ], data)

    def build_Match(self, data):
        children = [self.from_json(data["subject"])]
        for case in data["cases"]:
            children.append(self.from_json(case["pattern"]))
            children.append(self.from_json(case["body"]))
        return self._create_tree("match_stmt", children, data)

    def build_TryCatch(self, data):
        children = [self.from_json(data["try_block"])]
        if data["catch_var"]:
             children.append(self._create_token("IDENTIFIER", data["catch_var"], data))
        if data["catch_block"]:
             children.append(self.from_json(data["catch_block"]))
        return self._create_tree("try_stmt", children, data)

    def build_Import(self, data):
        children = [self._create_token("IDENTIFIER", p, data) for p in data["path"]]
        return self._create_tree("import_stmt", children, data)

    def build_VarDecl(self, data):
        return self._create_tree("assign_var", [
            self._create_token("IDENTIFIER", data["name"], data),
            self.from_json(data["value"])
        ], data)

    def build_Destructure(self, data):
        children = [self._create_token("IDENTIFIER", n, data) for n in data["names"]]
        children.append(self.from_json(data["value"]))
        return self._create_tree("assign_destructure", children, data)

    def build_SetField(self, data):
        return self._create_tree("assign_attr", [
            self.from_json(data["object"]),
            self._create_token("IDENTIFIER", data["field"], data),
            self.from_json(data["value"])
        ], data)

    def build_Binary(self, data):
        op_map = {
            "or": "logical_or", "and": "logical_and",
            "+": "add", "-": "sub", "*": "mul", "/": "div", "%": "mod",
            "<": "lt", ">": "gt", "<=": "le", ">=": "ge", "==": "eq", "!=": "neq"
        }
        op = data["operator"]
        tree_type = op_map.get(op, "unknown_op")
        return self._create_tree(tree_type, [
            self.from_json(data["left"]),
            self.from_json(data["right"])
        ], data)

    def build_Call(self, data):
        args_tree = None
        if data["args"]:
             # args is list of exprs.
             # In grammar: atom "(" [expr_list] ")"
             # expr_list -> expression ("," expression)*
             expr_list = self._create_tree("expr_list", [self.from_json(a) for a in data["args"]], data)
             args_tree = expr_list

        children = [self.from_json(data["function"])]
        if args_tree:
            children.append(args_tree)

        return self._create_tree("call_expr", children, data)

    def build_FieldAccess(self, data):
        return self._create_tree("get_attr", [
            self.from_json(data["object"]),
            self._create_token("IDENTIFIER", data["field"], data)
        ], data)

    def build_Index(self, data):
        return self._create_tree("get_item", [
            self.from_json(data["object"]),
            self.from_json(data["index"])
        ], data)

    def build_List(self, data):
        children = []
        if data["elements"]:
            expr_list = self._create_tree("expr_list", [self.from_json(e) for e in data["elements"]], data)
            children.append(expr_list)
        return self._create_tree("list_cons", children, data)

    def build_Struct(self, data):
        children = []
        if data["fields"]:
            # field_list -> field_init...
            # field_init -> IDENTIFIER, expr
            field_inits = []
            for f in data["fields"]:
                field_inits.append(self._create_tree("field_init", [
                    self._create_token("IDENTIFIER", f["key"], data),
                    self.from_json(f["value"])
                ], data))

            field_list = self._create_tree("field_list", field_inits, data)
            children.append(field_list)

        return self._create_tree("struct_init", children, data)

    def build_Identifier(self, data):
        # Maps to 'var' rule which contains one IDENTIFIER token
        token = self._create_token("IDENTIFIER", data["name"], data)
        return self._create_tree("var", [token], data)

    def build_Literal(self, data):
        # Determine if number or string based on value type
        val = data["value"]
        if isinstance(val, int):
            token = self._create_token("NUMBER", str(val), data)
            return self._create_tree("number", [token], data)
        else:
            # String needs quotes added back for token value
            raw = data.get("raw", f'"{val}"')
            token = self._create_token("STRING", raw, data)
            return self._create_tree("string", [token], data)


# ------------------------------------------------------------------------------
# 3. Schema Generation
# ------------------------------------------------------------------------------

def generate_schema() -> Dict[str, Any]:
    return {
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Ark AST Schema",
        "definitions": {
            "Node": {
                "type": "object",
                "properties": {
                    "line": { "type": "integer" },
                    "col": { "type": "integer" }
                }
            },
            "Program": {
                "allOf": [
                    { "$ref": "#/definitions/Node" },
                    {
                        "type": "object",
                        "properties": {
                            "type": { "const": "Program" },
                            "body": { "type": "array", "items": { "$ref": "#/definitions/Statement" } }
                        },
                        "required": ["type", "body"]
                    }
                ]
            },
            "Statement": {
                "oneOf": [
                    { "$ref": "#/definitions/Block" },
                    { "$ref": "#/definitions/FunctionDecl" },
                    { "$ref": "#/definitions/ClassDecl" },
                    { "$ref": "#/definitions/If" },
                    { "$ref": "#/definitions/While" },
                    { "$ref": "#/definitions/Return" },
                    { "$ref": "#/definitions/Import" },
                    { "$ref": "#/definitions/VarDecl" },
                    { "$ref": "#/definitions/Destructure" },
                    { "$ref": "#/definitions/SetField" },
                    { "$ref": "#/definitions/Expression" }
                ]
            },
            "Expression": {
                 "oneOf": [
                    { "$ref": "#/definitions/Binary" },
                    { "$ref": "#/definitions/Call" },
                    { "$ref": "#/definitions/FieldAccess" },
                    { "$ref": "#/definitions/Index" },
                    { "$ref": "#/definitions/List" },
                    { "$ref": "#/definitions/Struct" },
                    { "$ref": "#/definitions/Identifier" },
                    { "$ref": "#/definitions/Literal" }
                 ]
            },
            "Block": {
                "properties": { "type": { "const": "Block" }, "stmts": { "type": "array" } }
            },
            "FunctionDecl": {
                "properties": { "type": { "const": "FunctionDecl" }, "name": { "type": "string" }, "params": { "type": "array" }, "body": { "$ref": "#/definitions/Block" } }
            },
            "Match": {
                "properties": { "type": { "const": "Match" }, "subject": { "$ref": "#/definitions/Expression" }, "cases": { "type": "array" } }
            },
            "TryCatch": {
                "properties": { "type": { "const": "TryCatch" }, "try_block": { "$ref": "#/definitions/Block" }, "catch_var": { "type": "string" }, "catch_block": { "$ref": "#/definitions/Block" } }
            }
        },
        "type": "object",
        "$ref": "#/definitions/Program"
    }

# ------------------------------------------------------------------------------
# 4. Main / CLI
# ------------------------------------------------------------------------------

def compile_ark(source_path, output_path, pretty=False, minify=False):
    with open(source_path, 'r') as f:
        source = f.read()

    tree = ARK_PARSER.parse(source)
    serializer = ArkASTSerializer()
    json_ast = serializer.to_json(tree)

    with open(output_path, 'w') as f:
        if pretty:
            json.dump(json_ast, f, indent=2)
        elif minify:
            json.dump(json_ast, f, separators=(',', ':'))
        else:
            # Default: compact (no whitespace) as requested
            json.dump(json_ast, f, separators=(',', ':'))

    # Source Map (simplified)
    # We could write a separate .map file here
    map_path = output_path + ".map"
    with open(map_path, 'w') as f:
        json.dump({"version": 3, "file": output_path, "mappings": "..." }, f) # Placeholder

    print(f"Compiled {source_path} to {output_path}")

def check_roundtrip(source_path):
    with open(source_path, 'r') as f:
        source = f.read()

    # 1. Parse -> Tree
    tree = ARK_PARSER.parse(source)

    # 2. Tree -> JSON
    serializer = ArkASTSerializer()
    json_ast = serializer.to_json(tree)

    # 3. JSON -> Tree
    deserializer = ArkASTDeserializer()
    tree_reconstructed = deserializer.from_json(json_ast)

    # 4. Verify Structure (by serializing again and comparing JSON)
    json_ast_2 = serializer.to_json(tree_reconstructed)

    # Deep compare
    def deep_compare(d1, d2, path=""):
        if isinstance(d1, dict):
            for k in d1:
                if k not in d2: raise Exception(f"Missing key {k} at {path}")
                deep_compare(d1[k], d2[k], path + "." + k)
        elif isinstance(d1, list):
            if len(d1) != len(d2): raise Exception(f"List length mismatch at {path}")
            for i in range(len(d1)):
                deep_compare(d1[i], d2[i], path + f"[{i}]")
        else:
            if d1 != d2: raise Exception(f"Value mismatch at {path}: {d1} != {d2}")

    try:
        deep_compare(json_ast, json_ast_2)
        print("Roundtrip Successful: AST -> JSON -> AST -> JSON matches.")
    except Exception as e:
        print(f"Roundtrip Failed: {e}")
        # Dump for debugging
        # print("Original:", json.dumps(json_ast, indent=2))
        # print("Reconstructed:", json.dumps(json_ast_2, indent=2))
        sys.exit(1)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Ark AST to JSON")
    parser.add_argument("input", nargs="?", help="Input .ark file")
    parser.add_argument("-o", "--output", help="Output .json file (default: stdout)")
    parser.add_argument("--pretty", action="store_true", help="Pretty-print JSON")
    parser.add_argument("--minify", action="store_true", help="Minify JSON (remove all whitespace)")
    parser.add_argument("--schema", action="store_true", help="Output JSON Schema instead")
    parser.add_argument("--roundtrip", action="store_true", help="Test roundtrip accuracy")

    args = parser.parse_args()

    if args.schema:
        print(json.dumps(generate_schema(), indent=2))
        sys.exit(0)

    if not args.input:
        parser.print_help()
        sys.exit(1)

    if args.roundtrip:
        check_roundtrip(args.input)
        sys.exit(0)

    output = args.output
    if not output:
        # Default to printing to stdout if no output file specified?
        # Or construct from input filename.
        # User prompt says "Output .json file (default: stdout)"
        pass

    if output:
        compile_ark(args.input, output, args.pretty, args.minify)
    else:
        # Print to stdout
        with open(args.input, 'r') as f:
            source = f.read()
        tree = ARK_PARSER.parse(source)
        serializer = ArkASTSerializer()
        json_ast = serializer.to_json(tree)
        if args.pretty:
            print(json.dumps(json_ast, indent=2))
        elif args.minify:
            print(json.dumps(json_ast, separators=(',', ':')))
        else:
            # Default: compact
            print(json.dumps(json_ast, separators=(',', ':')))
