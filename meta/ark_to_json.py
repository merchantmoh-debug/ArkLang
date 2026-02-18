import sys
import os
import json
import argparse
import hashlib
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
# 1. Hashing Logic (Must match Rust core/src/ast.rs)
# ------------------------------------------------------------------------------

def calculate_hash(content: Any) -> str:
    # Serialize to Canonical JSON
    canonical = json.dumps(content, sort_keys=True, separators=(',', ':'))
    # SHA-256
    sha = hashlib.sha256()
    sha.update(canonical.encode('utf-8'))
    return sha.hexdigest()

def create_mast_node(content: Any, span: Optional[Dict] = None) -> Dict[str, Any]:
    h = calculate_hash(content)
    return {
        "hash": h,
        "content": content,
        "span": span
    }

# ------------------------------------------------------------------------------
# 2. AST Conversion (Python AST -> Rust ArkNode)
# ------------------------------------------------------------------------------

class ArkASTSerializer:
    def to_mast(self, node: Union[Tree, Token, List, Any]) -> Dict[str, Any]:
        content = self.visit(node)
        span = self._get_span(node) if isinstance(node, (Tree, Token)) else None
        return create_mast_node(content, span)

    def visit(self, node: Union[Tree, Token, List, Any]) -> Any:
        if isinstance(node, Tree):
            handler_name = f"visit_{node.data}"
            handler = getattr(self, handler_name, self.generic_visit)
            return handler(node)
        elif isinstance(node, Token):
            return self.visit_token(node)
        elif isinstance(node, list):
            return [self.visit(item) for item in node]
        elif node is None:
            return None
        else:
            return node

    def _get_span(self, node: Union[Tree, Token]) -> Optional[Dict[str, Any]]:
        meta = {}
        if isinstance(node, Token):
            meta = {
                "start_line": node.line,
                "start_col": node.column,
                "end_line": node.end_line if node.end_line else node.line,
                "end_col": node.end_column if node.end_column else node.column,
                "file": "unknown" # TODO: Pass filename
            }
        elif isinstance(node, Tree):
            if hasattr(node, "meta") and not node.meta.empty:
                meta = {
                    "start_line": node.meta.line,
                    "start_col": node.meta.column,
                    "end_line": node.meta.end_line,
                    "end_col": node.meta.end_column,
                    "file": "unknown"
                }
            elif node.children:
                # Heuristic
                first = node.children[0]
                if isinstance(first, (Tree, Token)):
                    return self._get_span(first)

        if not meta: return None
        return meta

    def generic_visit(self, node: Tree) -> Any:
        raise ValueError(f"Unknown node type: {node.data}")

    def visit_token(self, token: Token) -> Any:
        # Used for raw identifiers
        return token.value

    # --- Type System ---
    def default_type(self) -> str:
        return "Any" # ArkType::Any serialized

    def _to_statement(self, ark_node: Dict[str, Any]) -> Dict[str, Any]:
        # Unwrap ArkNode to Statement
        if "Statement" in ark_node:
            return ark_node["Statement"]
        if "Expression" in ark_node:
            return {"Expression": ark_node["Expression"]}
        raise ValueError(f"Cannot convert {ark_node.keys()} to Statement")

    # --- Root ---
    def visit_start(self, node: Tree) -> Any:
        stmts = [self._to_statement(self.visit(child)) for child in node.children]
        return {
            "Statement": {
                "Block": stmts
            }
        }

    # --- Statements ---
    def visit_block(self, node: Tree) -> Any:
        stmts = [self._to_statement(self.visit(child)) for child in node.children]
        return {
            "Statement": {
                "Block": stmts
            }
        }

    def visit_statement(self, node: Tree) -> Any:
        # Just a wrapper rule
        return self.visit(node.children[0])

    def visit_function_def(self, node: Tree) -> Any:
        # func name(params) { body }
        # Rust FunctionDef: { name, inputs: Vec<(String, ArkType)>, output: ArkType, body: Box<MastNode> }

        name_token = node.children[0]
        params = []
        body_idx = 1

        if len(node.children) > 1:
             possible_params = node.children[1]
             if isinstance(possible_params, Tree) and possible_params.data == "param_list":
                 # param_list children are IDENTIFIER tokens
                 # Convert to (name, type) tuples
                 params = [(p.value, self.default_type()) for p in possible_params.children]
                 body_idx = 2
             elif possible_params is None:
                 body_idx = 2

        body_tree = node.children[body_idx]

        # Body needs to be a MastNode.
        # visit_block returns {"Statement": {"Block": ...}} (ArkNode)
        # to_mast wraps it in MastNode.
        body_mast = self.to_mast(body_tree)

        func_def = {
            "name": name_token.value,
            "inputs": params,
            "output": self.default_type(),
            "body": body_mast
        }

        # Wrapped in ArkNode::Function
        # Wait, Statement::Function exists too?
        # core/src/ast.rs: Statement::Function(FunctionDef)
        # And ArkNode::Function(FunctionDef)
        # Let's use Statement::Function so it can fit in blocks.
        return {
            "Statement": {
                "Function": func_def
            }
        }

    def visit_return_stmt(self, node: Tree) -> Any:
        expr = self.visit(node.children[0]) if node.children else { "Expression": { "Literal": "nil" } } # Unit/Nil?
        # Return(Expression)
        # But visit returns ArkNode usually?
        # No, visit should return the inner content for some things.
        # Let's standardize: visit_* returns an ArkNode dict (e.g. {"Statement": ...} or {"Expression": ...})

        # Return expects Expression inside.
        # If expr is {"Expression": ...}, we extract the inner.
        if "Expression" in expr:
            expr_inner = expr["Expression"]
        else:
             # Should prevent this.
             expr_inner = {"Literal": "nil"}

        return {
            "Statement": {
                "Return": expr_inner
            }
        }

    def visit_if_stmt(self, node: Tree) -> Any:
        # Rust If: { condition: Expression, then_block: Vec<Statement>, else_block: Option<Vec<Statement>> }

        children = node.children
        # Debug
        # print(f"DEBUG IF Children: {children}")
        cond_node = self.visit(children[0])
        then_node = self.visit(children[1]) # Returns {"Statement": {"Block": [stmts]}}

        cond_expr = cond_node["Expression"]
        then_stmts = then_node["Statement"]["Block"]

        else_block = None

        if len(children) > 2:
            # Recursively handle else-if/else
            # We need to return Vec<Statement> for else_block

            # Helper to chain
            def build_else(idx):
                if idx >= len(children): return None

                child = children[idx]
                if child is None: return build_else(idx + 1) # Skip None

                # Check if child is a Tree
                node_data = getattr(child, "data", None)

                # If it's a block (else block), just return its stmts
                if node_data == "block":
                    res = self.visit(child)
                    return res["Statement"]["Block"]

                # If it's an expression (condition for else if), check next
                if node_data != "block": # Must be expression
                    cond = self.visit(child)["Expression"]
                    then_blk = self.visit(children[idx+1])["Statement"]["Block"]
                    nxt = build_else(idx+2)

                    # Create If statement
                    if_stmt = {
                        "Statement": {
                            "If": {
                                "condition": cond,
                                "then_block": then_blk,
                                "else_block": nxt
                            }
                        }
                    }
                    return [if_stmt]
                else:
                    return self.visit(child)["Statement"]["Block"]

            else_block = build_else(2)

        return {
            "Statement": {
                "If": {
                    "condition": cond_expr,
                    "then_block": then_stmts,
                    "else_block": else_block
                }
            }
        }

    def visit_while_stmt(self, node: Tree) -> Any:
        cond = self.visit(node.children[0])["Expression"]
        body = self.visit(node.children[1])["Statement"]["Block"]

        return {
            "Statement": {
                "While": {
                    "condition": cond,
                    "body": body
                }
            }
        }

    def visit_import_stmt(self, node: Tree) -> Any:
        path_parts = [t.value for t in node.children]
        path_str = ".".join(path_parts)

        return {
            "Statement": {
                "Import": {
                    "path": path_str,
                    "alias": None
                }
            }
        }

    # --- Assignments ---

    def visit_assign_var(self, node: Tree) -> Any:
        name = node.children[0].value
        val = self.visit(node.children[1])["Expression"]

        return {
            "Statement": {
                "Let": {
                    "name": name,
                    "ty": self.default_type(), # Option<ArkType>
                    "value": val
                }
            }
        }

    def visit_assign_destructure(self, node: Tree) -> Any:
        names = [t.value for t in node.children[:-1]]
        val = self.visit(node.children[-1])["Expression"]

        return {
            "Statement": {
                "LetDestructure": {
                    "names": names,
                    "value": val
                }
            }
        }

    def visit_assign_attr(self, node: Tree) -> Any:
        obj_node = node.children[0]
        # We need the object name?
        # assign_attr: atom "." IDENTIFIER _ASSIGN expression
        # Rust SetField: { obj_name: String, field: String, value: Expression }
        # This implies we can only set fields on variables, not arbitrary expressions?
        # If `atom` is complex (e.g. `get_user().name = "foo"`), Rust AST doesn't support it directly?
        # Core AST: `obj_name: String`.
        # Limitation: Only support `var.field = val`.

        if obj_node.data == "var":
            obj_name = obj_node.children[0].value
        else:
            # Fallback or Error?
            # For now, assume it's a var. If not, this schema is restrictive.
            # We'll just stringify the expr? No.
            # Let's hope it's a var.
            obj_name = "complex_expr_unsupported"
            if hasattr(obj_node, "children") and len(obj_node.children) > 0:
                 # Check if child is IDENTIFIER
                 if hasattr(obj_node.children[0], "value"):
                     obj_name = obj_node.children[0].value

        field = node.children[1].value
        val = self.visit(node.children[2])["Expression"]

        return {
            "Statement": {
                "SetField": {
                    "obj_name": obj_name,
                    "field": field,
                    "value": val
                }
            }
        }

    # --- Expressions ---
    # Need to return {"Expression": {Variant: ...}}

    def visit_expression(self, node: Tree) -> Any:
        return self.visit(node.children[0])

    def visit_var(self, node: Tree) -> Any:
        return {
            "Expression": {
                "Variable": node.children[0].value
            }
        }

    def visit_number(self, node: Tree) -> Any:
        return {
            "Expression": {
                "Integer": int(node.children[0].value)
            }
        }

    def visit_string(self, node: Tree) -> Any:
        raw = node.children[0].value
        val = raw[1:-1] # Strip quotes
        return {
            "Expression": {
                "Literal": val
            }
        }

    def visit_call_expr(self, node: Tree) -> Any:
        # func(args)
        # func is an atom.
        # Rust Call: { function_hash: String, args: Vec<Expression> }
        # Wait, if `func` is a variable name, we don't know the hash at compile time!
        # The Rust AST `function_hash` implies we are calling by HASH.
        # But `ark_loader` loads a MAST.
        # If we call `print("hi")`, `print` is an intrinsic or function in scope.
        # How does Rust runtime resolve names?
        # `core/src/vm.rs` executes `Call`.
        # If `function_hash` is used, it looks up in `MAST`.
        # This implies ALL function calls must be static and hashed?
        # That breaks dynamic dispatch and even simple variable calls `f = my_func; f()`.
        #
        # Let's check `core/src/intrinsics.rs`: `sys.func.apply` takes a function object/string.
        # Maybe `Call` instruction is ONLY for static calls to known hashes?
        #
        # Workaround: Use a special "Name Call" convention or just pass the name as the hash?
        # The runtime might try to resolve `function_hash` as a name if it's not a hex hash?
        # Or I can use `sys.func.apply` for everything?
        #
        # Let's look at `core/src/ast.rs`:
        # `Expression::Call { function_hash: String, args: Vec<Expression> }`
        # If I put the function name in `function_hash`, will it work?
        #
        # If `meta/ark_to_json.py` puts "print" in `function_hash`.
        # Runtime loads. VM executes `Call`.
        # I suspect the Rust VM expects a hash.
        # But for intrinsics? "intrinsic_print"?
        #
        # Let's try to put the NAME in `function_hash`. If the runtime handles it, great.

        func_node = node.children[0]
        # Extract name if possible
        func_name = "unknown"
        if func_node.data == "var":
            func_name = func_node.children[0].value
        elif func_node.data == "get_attr":
             # obj.method
             # Not supported by simple Call?
             pass

        args = []
        if len(node.children) > 1:
            arg_list = node.children[1]
            if hasattr(arg_list, "children"):
                args = [self.visit(c)["Expression"] for c in arg_list.children]

        return {
            "Expression": {
                "Call": {
                    "function_hash": func_name,
                    "args": args
                }
            }
        }

    def _visit_binary(self, node: Tree, op: str) -> Any:
        # Transform to function call `intrinsic_add(left, right)`
        # because Rust AST doesn't have BinaryExpr!
        # It only has: Variable, Literal, Call, List, StructInit, GetField, Match.
        # So binary ops MUST be converted to Calls.

        left = self.visit(node.children[0])["Expression"]
        right = self.visit(node.children[1])["Expression"]

        return {
            "Expression": {
                "Call": {
                    "function_hash": f"intrinsic_{op}",
                    "args": [left, right]
                }
            }
        }

    def visit_add(self, node: Tree): return self._visit_binary(node, "add")
    def visit_sub(self, node: Tree): return self._visit_binary(node, "sub")
    def visit_mul(self, node: Tree): return self._visit_binary(node, "mul")
    def visit_div(self, node: Tree): return self._visit_binary(node, "div")
    def visit_mod(self, node: Tree): return self._visit_binary(node, "mod")
    def visit_lt(self, node: Tree): return self._visit_binary(node, "lt")
    def visit_gt(self, node: Tree): return self._visit_binary(node, "gt")
    def visit_le(self, node: Tree): return self._visit_binary(node, "le")
    def visit_ge(self, node: Tree): return self._visit_binary(node, "ge")
    def visit_eq(self, node: Tree): return self._visit_binary(node, "eq")
    def visit_neq(self, node: Tree):
        # neq -> not(eq)
        eq = self._visit_binary(node, "eq")["Expression"]
        return {
            "Expression": {
                "Call": {
                    "function_hash": "intrinsic_not",
                    "args": [eq]
                }
            }
        }
    def visit_logical_and(self, node: Tree): return self._visit_binary(node, "and")
    def visit_logical_or(self, node: Tree): return self._visit_binary(node, "or")

    def visit_list_cons(self, node: Tree) -> Any:
        elements = []
        if node.children:
            expr_list = node.children[0]
            if expr_list and hasattr(expr_list, "children"):
                elements = [self.visit(c)["Expression"] for c in expr_list.children]
        return {
            "Expression": {
                "List": elements
            }
        }

    def visit_struct_init(self, node: Tree) -> Any:
        # fields: Vec<(String, Expression)>
        fields = []
        if node.children:
            field_list = node.children[0]
            if field_list and hasattr(field_list, "children"):
                for f in field_list.children:
                    key = f.children[0].value
                    val = self.visit(f.children[1])["Expression"]
                    fields.append((key, val))
        return {
            "Expression": {
                "StructInit": {
                    "fields": fields
                }
            }
        }

    # Pass through
    def visit_flow_stmt(self, node: Tree): return self.visit(node.children[0])
    def visit_atom(self, node: Tree): return self.visit(node.children[0])
    def visit_primary(self, node: Tree): return self.visit(node.children[0])
    def visit_pipe_expr(self, node: Tree): return self.visit(node.children[0]) # TODO: Handle pipe

    def visit_get_attr(self, node: Tree) -> Any:
        obj = self.visit(node.children[0])["Expression"]
        field = node.children[1].value
        return {
            "Expression": {
                "GetField": {
                    "obj": obj,
                    "field": field
                }
            }
        }

    def visit_get_item(self, node: Tree) -> Any:
        # obj[idx] -> sys.list.get(obj, idx) -> returns [val, list] (Linear)
        # OR intrinsic_list_get(obj, idx)

        obj = self.visit(node.children[0])["Expression"]
        idx = self.visit(node.children[1])["Expression"]

        return {
            "Expression": {
                "Call": {
                    "function_hash": "intrinsic_list_get",
                    "args": [obj, idx]
                }
            }
        }

# ------------------------------------------------------------------------------
# 3. Main
# ------------------------------------------------------------------------------

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("input")
    parser.add_argument("-o", "--output")
    args = parser.parse_args()

    with open(args.input, 'r', encoding="utf-8") as f:
        source = f.read()

    tree = ARK_PARSER.parse(source)
    serializer = ArkASTSerializer()

    # Root is always an ArkNode
    ast_data = serializer.visit(tree)

    # Output is the raw ArkNode (Statement::Block)
    # Rust loader.rs: from_str::<ArkNode>(json)

    # We must ensure it's valid JSON for the enum.
    # {"Statement": {"Block": [...]}}

    output_path = args.output if args.output else "out.json"
    with open(output_path, 'w') as f:
        json.dump(ast_data, f, separators=(',', ':')) # Compact
