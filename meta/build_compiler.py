
# The Architect's Forge: Amalgamation Script
# Objective: Concatenate component files into a single compiler.ark executable.

import os

def read_file(path):
    with open(path, 'r', encoding='utf-8') as f:
        return f.read()

def main():
    base_dir = "apps/compiler"
    files = [
        "lexer.ark",
        "parser.ark",
        "codegen.ark",
        "driver.ark"
    ]
    
    output_path = os.path.join(base_dir, "compiler.ark")
    print(f"Building {output_path}...")
    
    full_source = ""
    for filename in files:
        path = os.path.join(base_dir, filename)
        if not os.path.exists(path):
            print(f"Error: Missing component {filename}")
            return
        
        print(f"  + Ingesting {filename}...")
        source = read_file(path)
        full_source += f"\n// --- COMPONENT: {filename} ---\n"
        full_source += source
        full_source += "\n"

    with open(output_path, 'w', encoding='utf-8') as f:
        f.write(full_source)
    
    print("Build Complete. The Ouroboros is ready.")

if __name__ == "__main__":
    main()
