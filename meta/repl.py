import sys
import os
import lark
import atexit

# Initialize flags
HAS_PROMPT_TOOLKIT = False
HAS_PYGMENTS = False
HAS_READLINE = False

# Try importing prompt_toolkit
try:
    from prompt_toolkit import PromptSession
    from prompt_toolkit.history import FileHistory
    from prompt_toolkit.lexers import PygmentsLexer
    from prompt_toolkit.completion import WordCompleter
    from prompt_toolkit.styles import Style
    HAS_PROMPT_TOOLKIT = True
except ImportError:
    pass

# Try importing pygments
try:
    from pygments.lexer import RegexLexer, words
    from pygments.token import Keyword, Name, String, Number, Operator, Text, Comment
    HAS_PYGMENTS = True
except ImportError:
    pass

# Try importing readline
try:
    import readline
    HAS_READLINE = True
except ImportError:
    pass

# Ensure we can import ark.py from the same directory
sys.path.append(os.path.dirname(os.path.abspath(__file__)))
import ark

if HAS_PYGMENTS:
    class ArkLexer(RegexLexer):
        name = 'Ark'
        aliases = ['ark']
        filenames = ['*.ark']

        tokens = {
            'root': [
                (r'\s+', Text),
                (r'//.*?$', Comment.Single),
                (words(('func', 'class', 'if', 'else', 'while', 'return', 'let', 'and', 'or'), suffix=r'\b'), Keyword),
                (words(('true', 'false'), suffix=r'\b'), Keyword.Constant),
                (r'"(\\\\|\\"|[^"])*"', String),
                (r'-?\d+', Number),
                (r'[:=+\-*/%<>&|!]+', Operator),
                (r'[a-zA-Z_][a-zA-Z0-9_]*', Name),
                (r'[(){}\[\],.]', Text),
            ]
        }
else:
    class ArkLexer:
        pass

def run_repl():
    print("Ark REPL (v112.0) - Type 'exit' to quit.")

    # Initialize Scope
    scope = ark.Scope()
    scope.set("sys", ark.ArkValue("sys", "Namespace"))
    # Pre-populate true/false for convenience
    scope.set("true", ark.ArkValue(True, "Boolean"))
    scope.set("false", ark.ArkValue(False, "Boolean"))

    # Load Grammar
    grammar_path = os.path.join(os.path.dirname(__file__), "ark.lark")
    if not os.path.exists(grammar_path):
        print(f"Error: Grammar file not found at {grammar_path}")
        return

    with open(grammar_path, "r") as f:
        grammar = f.read()

    # Create Parser
    try:
        parser = lark.Lark(grammar, start="start", parser="lalr")
    except Exception as e:
        print(f"Error loading grammar: {e}")
        return

    # Setup History File
    history_file = os.path.expanduser("~/.ark_history")

    session = None
    # We prefer PromptToolkit if both it and Pygments are available
    if HAS_PROMPT_TOOLKIT and HAS_PYGMENTS:
        # Keywords + Intrinsics for completion
        completer_words = [
            'func', 'class', 'if', 'else', 'while', 'return', 'let',
            'true', 'false', 'and', 'or', 'sys'
        ] + list(ark.INTRINSICS.keys())

        ark_completer = WordCompleter(completer_words, ignore_case=False)

        session = PromptSession(
            history=FileHistory(history_file),
            lexer=PygmentsLexer(ArkLexer),
            completer=ark_completer,
            style=Style.from_dict({
                'completion-menu.completion': 'bg:#008888 #ffffff',
                'completion-menu.completion.current': 'bg:#00aaaa #000000',
                'scrollbar.background': 'bg:#88aaaa',
                'scrollbar.button': 'bg:#222222',
            })
        )
    else:
        # Fallback Setup
        if not HAS_PROMPT_TOOLKIT:
            print("Note: 'prompt_toolkit' not found. Using basic REPL.")
        elif not HAS_PYGMENTS:
            print("Note: 'pygments' not found. Using basic REPL.")

        if HAS_READLINE:
            try:
                readline.read_history_file(history_file)
            except FileNotFoundError:
                pass
            atexit.register(readline.write_history_file, history_file)

    while True:
        try:
            if session:
                text = session.prompt('ark> ')
            else:
                # Basic Input with ANSI color for prompt
                # Note: input() uses stdout, ANSI codes work in most terminals
                text = input('\033[94mark>\033[0m ')

            if not text.strip():
                continue
            if text.strip() in ['exit', 'quit']:
                break

            # Parse
            try:
                tree = parser.parse(text)
            except lark.UnexpectedToken as e:
                print(f"\033[91mSyntax Error:\033[0m {e}")
                continue
            except lark.UnexpectedCharacters as e:
                print(f"\033[91mSyntax Error:\033[0m {e}")
                continue

            # Evaluate
            try:
                result = ark.eval_node(tree, scope)
                if result.type != "Unit":
                    # Pretty print result if it's not Unit
                    if result.type == "String":
                        print(f'\033[92m=> "{result.val}"\033[0m')
                    else:
                        print(f"\033[92m=> {result.val}\033[0m")
            except ark.ReturnException as e:
                print(f"\033[92m=> {e.value.val}\033[0m")
            except ark.SandboxViolation as e:
                print(f"\033[91mSecurity Violation:\033[0m {e}")
            except Exception as e:
                print(f"\033[91mRuntime Error:\033[0m {e}")

        except KeyboardInterrupt:
            # Handle Ctrl+C (clear input)
            print("^C")
            continue
        except EOFError:
            # Handle Ctrl+D (exit)
            print("Goodbye!")
            break

if __name__ == "__main__":
    run_repl()
