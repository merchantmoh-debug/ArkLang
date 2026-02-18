import sys
import os
import atexit
import shlex
import platform
import re
import lark

try:
    import readline
except ImportError:
    try:
        import pyreadline3 as readline
    except ImportError:
        readline = None

# Ensure we can import ark.py
sys.path.append(os.path.dirname(os.path.abspath(__file__)))
try:
    import ark
except ImportError:
    # Fallback if run from root
    sys.path.append(os.path.join(os.getcwd(), 'meta'))
    import ark

class Colors:
    HEADER = '\033[95m'
    BLUE = '\033[94m'
    CYAN = '\033[96m'
    GREEN = '\033[92m'
    YELLOW = '\033[93m'
    RED = '\033[91m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'
    UNDERLINE = '\033[4m'

def colorize(text, color):
    if os.name == 'nt': return text
    return f"{color}{text}{Colors.ENDC}"

def colorize_prompt(text, color):
    if os.name == 'nt': return text
    if not readline: return colorize(text, color)
    # Wrap escape sequences in \001 and \002 for readline
    return f"\001{color}\002{text}\001{Colors.ENDC}\002"

KEYWORDS = [
    'let', 'func', 'if', 'else', 'while', 'for', 'return',
    'import', 'struct', 'match', 'true', 'false', 'nil',
    'class', 'and', 'or'
]

class ArkCompleter:
    def __init__(self, scope):
        self.scope = scope

    def complete(self, text, state):
        if not text:
            return None

        candidates = []
        # Keywords
        candidates.extend([k for k in KEYWORDS if k.startswith(text)])
        # Intrinsics
        candidates.extend([k for k in ark.INTRINSICS.keys() if k.startswith(text)])
        # Variables in scope
        candidates.extend([k for k in self.scope.vars.keys() if k.startswith(text)])

        candidates = sorted(list(set(candidates)))
        if state < len(candidates):
            return candidates[state]
        return None

class REPL:
    def __init__(self):
        self.history_file = os.path.expanduser("~/.ark_history")
        self.init_scope()
        self.setup_readline()

    def init_scope(self):
        self.scope = ark.Scope()
        self.scope.set("sys", ark.ArkValue("sys", "Namespace"))
        self.scope.set("math", ark.ArkValue("math", "Namespace"))
        self.scope.set("true", ark.ArkValue(True, "Boolean"))
        self.scope.set("false", ark.ArkValue(False, "Boolean"))
        # Add sys_args
        self.scope.set("sys_args", ark.ArkValue([], "List"))

    def setup_readline(self):
        if readline:
            if os.path.exists(self.history_file):
                try:
                    readline.read_history_file(self.history_file)
                except IOError:
                    pass
            atexit.register(self.save_history)

            self.completer = ArkCompleter(self.scope)
            readline.set_completer(self.completer.complete)
            readline.parse_and_bind("tab: complete")

    def save_history(self):
        if readline:
            try:
                readline.write_history_file(self.history_file)
            except IOError:
                pass

    def get_input(self, prompt=">>> "):
        buffer = []
        try:
            line = input(colorize_prompt(prompt, Colors.BLUE))
            buffer.append(line)

            while True:
                full_text = "\n".join(buffer)

                # Check for trailing backslash
                if full_text.strip().endswith('\\'):
                    buffer[-1] = buffer[-1].rstrip('\\')
                    next_line = input(colorize_prompt("... ", Colors.BLUE))
                    buffer.append(next_line)
                    continue

                # Check brace/paren balance
                open_braces = full_text.count('{')
                close_braces = full_text.count('}')
                open_parens = full_text.count('(')
                close_parens = full_text.count(')')

                if open_braces > close_braces or open_parens > close_parens:
                    next_line = input(colorize_prompt("... ", Colors.BLUE))
                    buffer.append(next_line)
                else:
                    break

            return "\n".join(buffer)

        except EOFError:
            raise

    def get_type_hint(self, node):
        if hasattr(node, 'data'):
            if node.data == 'number': return "Integer"
            if node.data == 'string': return "String"
            if node.data == 'logical_or' or node.data == 'logical_and': return "Boolean"
            if node.data == 'var':
                name = node.children[0].value
                val = self.scope.get(name)
                if val: return val.type
                return "Unknown (Undefined)"
        return "Dynamic/Expression"

    def handle_command(self, text):
        parts = shlex.split(text)
        cmd = parts[0]
        args = parts[1:]

        if cmd == ":help":
            print(colorize("Commands:", Colors.HEADER))
            print("  :help        Show this help")
            print("  :reset       Reset session")
            print("  :load <file> Load and execute file")
            print("  :save <file> Save history to file")
            print("  :type <expr> Show type of expression (static guess)")
            print("  :env         Show variables in scope")
            print("  :quit        Exit REPL")
            return True

        elif cmd == ":reset":
            self.init_scope()
            if readline:
                self.completer.scope = self.scope
            print(colorize("Session reset.", Colors.YELLOW))
            return True

        elif cmd == ":load":
            if not args:
                print(colorize("Usage: :load <file>", Colors.RED))
                return True
            path = args[0]
            try:
                with open(path, 'r') as f:
                    code = f.read()
                tree = ark.ARK_PARSER.parse(code)
                ark.eval_node(tree, self.scope)
                print(colorize(f"Loaded {path}", Colors.GREEN))
            except Exception as e:
                print(colorize(f"Error loading {path}: {e}", Colors.RED))
            return True

        elif cmd == ":save":
            if not args:
                print(colorize("Usage: :save <file>", Colors.RED))
                return True
            path = args[0]
            if readline:
                try:
                    readline.write_history_file(path)
                    print(colorize(f"History saved to {path}", Colors.GREEN))
                except Exception as e:
                    print(colorize(f"Error saving history: {e}", Colors.RED))
            else:
                print(colorize("Readline not available.", Colors.RED))
            return True

        elif cmd == ":env":
            print(colorize("Scope Variables:", Colors.HEADER))
            for k, v in self.scope.vars.items():
                print(f"  {colorize(k, Colors.CYAN)}: {v.type} = {v.val}")
            return True

        elif cmd in [":quit", ":exit"]:
            sys.exit(0)

        elif cmd == ":type":
            if not args:
                print(colorize("Usage: :type <expr>", Colors.RED))
                return True
            expr = " ".join(args)
            try:
                tree = ark.ARK_PARSER.parse(expr)
                # If tree is a statement block, drill down
                if tree.data in ['start', 'block', 'flow_stmt']:
                     # This is a bit hacky, but valid for simple expressions
                     # Real static analysis requires a full visitor
                     pass
                hint = self.get_type_hint(tree)
                if hint == "Dynamic/Expression":
                     # Try to drill down one level if it's a statement wrapper
                     if tree.children and hasattr(tree.children[0], 'data'):
                         hint = self.get_type_hint(tree.children[0])

                print(f"{colorize('Type Hint:', Colors.YELLOW)} {hint}")
            except Exception as e:
                print(colorize(f"Parse Error: {e}", Colors.RED))
            return True

        print(colorize(f"Unknown command: {cmd}", Colors.RED))
        return True

    def run(self):
        banner = r"""
  ____  ____  _   _
 / _  ||  _ \| | / /
| |_| || |_) | |/ /
|  _  ||  _ <|   <
| | | || | \ \| |\ \
|_| |_||_|  \_\_| \_\  v0.1.0

Type :help for commands, :quit to exit
"""
        print(colorize(banner, Colors.CYAN))

        while True:
            try:
                text = self.get_input()
                if not text.strip():
                    continue

                if text.strip().startswith(':'):
                    self.handle_command(text.strip())
                    continue

                # Parse
                try:
                    tree = ark.ARK_PARSER.parse(text)
                except lark.UnexpectedInput as e:
                     print(f"{colorize('Syntax Error:', Colors.RED)} {e}")
                     continue
                except lark.UnexpectedCharacters as e:
                     print(f"{colorize('Syntax Error:', Colors.RED)} {e}")
                     continue
                except Exception as e:
                     print(f"{colorize('Parse Error:', Colors.RED)} {e}")
                     continue

                # Eval
                try:
                    result = ark.eval_node(tree, self.scope)

                    if result.type != "Unit":
                        if result.type == "String":
                            print(f'{colorize("=>", Colors.GREEN)} "{colorize(result.val, Colors.GREEN)}"')
                        elif result.type in ["Integer", "Float"]:
                            print(f'{colorize("=>", Colors.GREEN)} {colorize(str(result.val), Colors.CYAN)}')
                        elif result.type == "Boolean":
                            print(f'{colorize("=>", Colors.GREEN)} {colorize(str(result.val).lower(), Colors.YELLOW)}')
                        else:
                            print(f'{colorize("=>", Colors.GREEN)} {result.val}')

                except ark.ReturnException as e:
                    print(f'{colorize("=>", Colors.GREEN)} {e.value.val}')
                except ark.SandboxViolation as e:
                    print(f"{colorize('Security Violation:', Colors.RED)} {e}")
                except Exception as e:
                    print(f"{colorize('Runtime Error:', Colors.RED)} {e}")

            except KeyboardInterrupt:
                print("\n^C")
                continue
            except EOFError:
                print("\nGoodbye!")
                break

def run_repl():
    """Top-level entrypoint for the REPL, used by ark.py."""
    repl = REPL()
    repl.run()

if __name__ == "__main__":
    run_repl()
