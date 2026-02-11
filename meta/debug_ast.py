from parser import QiParser
import sys

code = """
func factorial(n) {
    if n < 2 {
        return 1
    }
    return n * factorial(n - 1)
}

result := factorial(12)
"""

p = QiParser("meta/ark.lark")
ast = p.parse(code)
print(f"AST Root Type: {type(ast)}")
if isinstance(ast, dict):
    print("AST Keys:", ast.keys())
    prog = ast.get("program", [])
    for i, stmt in enumerate(prog):
        print(f"Stmt {i} Type: {type(stmt)}")
        print(f"Stmt {i} Val: {stmt}")
        if hasattr(stmt, 'data'):
            print(f"  Data: {stmt.data}")
            print(f"  Children: {stmt.children}")
            for j, child in enumerate(stmt.children):
                print(f"    Child {j} Type: {type(child)}")
                if hasattr(child, 'type'):
                    print(f"    Child {j} Token Value: {child.value}")
