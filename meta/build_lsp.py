import sys
import os

def build_lsp():
    base_dir = os.getcwd()

    # Paths
    lexer_path = os.path.join(base_dir, "apps/compiler/lexer.ark")
    parser_path = os.path.join(base_dir, "apps/compiler/parser.ark")
    lsp_main_path = os.path.join(base_dir, "apps/lsp_main.ark")
    output_path = os.path.join(base_dir, "apps/lsp.ark")

    print(f"Building LSP to {output_path}...")

    try:
        with open(lexer_path, "r") as f:
            lexer_code = f.read()

        with open(parser_path, "r") as f:
            parser_code = f.read()

        with open(lsp_main_path, "r") as f:
            lsp_main_code = f.read()
    except FileNotFoundError as e:
        print(f"Error: {e}")
        sys.exit(1)

    # Preamble
    preamble = """
// --- Preamble ---
func print(msg) {
    sys.log(msg)
}

func int_to_str(i) {
    return "" + i
}

true := 1 == 1
false := 1 == 0
"""

    full_code = preamble + "\n" + lexer_code + "\n" + parser_code + "\n" + lsp_main_code

    with open(output_path, "w") as f:
        f.write(full_code)

    print("Build complete.")

if __name__ == "__main__":
    build_lsp()
