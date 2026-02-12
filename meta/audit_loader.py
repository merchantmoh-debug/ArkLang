import json
import sys
import subprocess

def main():
    if len(sys.argv) < 2:
        print("Usage: python meta/audit_loader.py <json_file>")
        sys.exit(1)

    filepath = sys.argv[1]
    
    # 1. Read Valid JSON
    with open(filepath, 'r') as f:
        data = json.load(f)
    
    print(f"[AUDIT] Loaded valid file: {filepath}")
    
    # 2. Tamper with Content (Flip a bit in the logic)
    # Find the first function and modify its body content WITHOUT updating hash
    # Structure: Statement -> Block -> [Function, ...]
    
    funcs = data['Statement']['Block']
    target_func = None
    for item in funcs:
        if 'Function' in item:
            target_func = item['Function']
            break
            
    if not target_func:
        print("[AUDIT] No function found to tamper.")
        sys.exit(1)
        
    print(f"[AUDIT] Tampering with function: {target_func['name']}")
    original_hash = target_func['body']['hash']
    print(f"[AUDIT] Original Hash: {original_hash}")
    
    # Tamper: Change a literal value in the body
    # We'll just inject a dummy statement or change a string
    # Let's find a "Literal" in the content and change it
    
    # Deep modification helper
    def tamper_recursive(node):
        if isinstance(node, dict):
            if 'Literal' in node and isinstance(node['Literal'], str):
                node['Literal'] += "_TAMPERED"
                return True
            for k, v in node.items():
                if k != 'hash' and tamper_recursive(v):
                    return True
        elif isinstance(node, list):
            for item in node:
                if tamper_recursive(item):
                    return True
        return False

    if tamper_recursive(target_func['body']['content']):
        print("[AUDIT] Content tampered successfully.")
    else:
        print("[AUDIT] Could not find content to tamper.")
        sys.exit(1)
        
    # 3. Save Tampered File
    tampered_path = filepath + ".tampered.json"
    with open(tampered_path, 'w') as f:
        json.dump(data, f, indent=2)
        
    print(f"[AUDIT] Saved tampered file: {tampered_path}")
    
    # 4. Run ark_loader against it
    print(f"[AUDIT] Running ark_loader...")
    result = subprocess.run(
        ["cargo", "run", "--bin", "ark_loader", tampered_path],
        capture_output=True,
        text=True
    )
    
    # 5. Check for Failure
    print("[AUDIT] Output:")
    print(result.stderr)
    
    if "HashMismatch" in result.stderr or "Integrity Error" in result.stderr:
        print("\n[SUCCESS] Immune System REJECTED the tampered file.")
        sys.exit(0)
    else:
        print("\n[FAILURE] Immune System FAILED to reject the tampered file.")
        sys.exit(1)

if __name__ == "__main__":
    main()
