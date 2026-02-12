try:
    with open('apps/compiler/compiler_output.log', 'r', encoding='utf-8', errors='replace') as f:
        print(f.read())
except Exception as e:
    print(f"Error reading log: {e}")
