import sys
import os
import lark
from prompt_toolkit import PromptSession
from prompt_toolkit.history import FileHistory
from prompt_toolkit.lexers import PygmentsLexer
from prompt_toolkit.completion import WordCompleter
from prompt_toolkit.styles import Style
from pygments.lexer import RegexLexer, words
from pygments.token import Keyword, Name, String, Number, Operator, Text, Comment

# Ensure we can import ark.py from the same directory
sys.path.append(os.path.dirname(os.path.abspath(__file__)))
import ark

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

def run_repl():
    print("ARK OMEGA-POINT v112.0 REPL")
    print("Type 'exit' to quit, 'help' for commands.")

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

    # Setup Prompt Toolkit
    history_file = os.path.expanduser("~/.ark_history")

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

    buffer = []

    while True:
        try:
            prompt_text = '... ' if buffer else 'ark> '
            text = session.prompt(prompt_text)

            # Handle empty input
            if not text.strip() and not buffer:
                continue

            # Handle exit
            if not buffer and text.strip() in ['exit', 'quit']:
                break

            # Handle help
            if not buffer and text.strip() == 'help':
                print("\nAvailable Commands:")
                print("  exit, quit  - Quit REPL")
                print("  help        - Show this help")
                print("\nAvailable Intrinsics:")
                # Simple word wrap for display
                intrinsics = sorted(ark.INTRINSICS.keys())
                print(", ".join(intrinsics))
                print("")
                continue

            buffer.append(text)
            full_text = "\n".join(buffer)

            # Parse
            try:
                tree = parser.parse(full_text)
                buffer = [] # Reset buffer on success
            except (lark.UnexpectedToken, lark.UnexpectedEOF) as e:
                # Check for incomplete input (UnexpectedToken at EOF)
                is_incomplete = False
                if isinstance(e, lark.UnexpectedToken) and e.token.type == '$END':
                    is_incomplete = True
                elif isinstance(e, lark.UnexpectedEOF):
                    is_incomplete = True

                if is_incomplete:
                    # Continue loop to get more input
                    continue
                else:
                    print(f"Syntax Error: {e}")
                    buffer = []
                    continue
            except lark.UnexpectedCharacters as e:
                print(f"Syntax Error: {e}")
                buffer = []
                continue

            # Evaluate
            try:
                result = ark.eval_node(tree, scope)
                if result.type != "Unit":
                    # Pretty print result if it's not Unit
                    if result.type == "String":
                        print(f'=> "{result.val}"')
                    else:
                        print(f"=> {result.val}")
            except ark.ReturnException as e:
                print(f"=> {e.value.val}")
            except ark.SandboxViolation as e:
                print(f"Security Violation: {e}")
            except Exception as e:
                print(f"Runtime Error: {e}")

        except KeyboardInterrupt:
            # Handle Ctrl+C (clear input)
            print("\nKeyboardInterrupt")
            buffer = []
            continue
        except EOFError:
            # Handle Ctrl+D (exit)
            print("Goodbye!")
            break

if __name__ == "__main__":
    run_repl()
