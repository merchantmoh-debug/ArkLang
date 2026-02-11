
import os
import json
import subprocess
import sys
import concurrent.futures
import time

API_KEY = os.environ.get("JULES_API_KEY")
API_BASE = "https://jules.googleapis.com/v1alpha"

if not API_KEY:
    print("Error: JULES_API_KEY not found in environment.")
    sys.exit(1)

def run_curl(cmd_parts):
    try:
        # Retry logic for stability
        for attempt in range(3):
            result = subprocess.run(
                cmd_parts, capture_output=True, text=True, check=True
            )
            data = json.loads(result.stdout)
            if "error" not in data:
                return data
            print(f"API Error (Attempt {attempt+1}): {data}")
            time.sleep(2)
        return None
    except subprocess.CalledProcessError as e:
        print(f"Error running curl: {e}")
        print(f"Stderr: {e.stderr}")
        return None

def list_sources():
    cmd = [
        "curl", "-s",
        "-H", f"x-goog-api-key: {API_KEY}",
        f"{API_BASE}/sources"
    ]
    return run_curl(cmd)

def create_session(source_name, persona, task_name, task_instruction):
    print(f"Spawn: {task_name}")
    
    full_prompt = f"{persona}\n\n---\n\n# KINETIC TASK: {task_name}\n\n{task_instruction}"
    
    data = {
        "prompt": full_prompt,
        "sourceContext": {
            "source": source_name,
            "githubRepoContext": {
                "startingBranch": "main" 
            }
        },
        "automationMode": "AUTO_CREATE_PR",
        "title": f"Omega-Point Swarm: {task_name}"
    }
    
    cmd = [
        "curl", "-s",
        "-X", "POST",
        "-H", "Content-Type: application/json",
        "-H", f"x-goog-api-key: {API_KEY}",
        "-d", json.dumps(data),
        f"{API_BASE}/sessions"
    ]
    result = run_curl(cmd)
    if result:
        print(f"  -> Session Created: {result.get('name')} ({task_name})")
    else:
        print(f"  -> FAILED: {task_name}")
    return result

def main():
    try:
        with open("meta/jules_persona.md", "r", encoding="utf-8") as f:
            persona = f.read()
    except Exception as e:
        print(f"Error reading persona: {e}")
        return

    sources_resp = list_sources()
    if not sources_resp or "sources" not in sources_resp:
        print("Failed to list sources.")
        return

    target_source = None
    for src in sources_resp["sources"]:
        repo_name = src.get("githubRepo", {}).get("repo", "")
        if "ark-compiler" in repo_name:
            target_source = src["name"]
            break
    
    if not target_source and len(sources_resp["sources"]) > 0:
        target_source = sources_resp["sources"][0]["name"]
    
    if not target_source:
        print("No valid source found.")
        return

    print(f"Target Source: {target_source}")

    # --- THE SWARM (8 Parallel Agents) ---
    tasks = [
        {
            "name": "StdLib_Expansion",
            "instruction": (
                "ACT: Edit 'core/src/intrinsics.rs'.\n"
                "GOAL: Add 'time.now()', 'math.pow()', 'math.sqrt()', 'io.cls()'.\n"
                "CODE: Register these new intrinsics in IntrinsicRegistry and implement them using Rust std::time and f64 methods."
            )
        },
        {
            "name": "LSP_Server_Stub",
            "instruction": (
                "ACT: Create 'meta/ark_lsp.py'.\n"
                "GOAL: Implement a basic Language Server Protocol (LSP) stub.\n"
                "CODE: Use 'pygls' or raw JSON-RPC over stdio. Handle 'initialize' and 'textDocument/didOpen' events."
            )
        },
        {
            "name": "Site_Build_Fix",
            "instruction": (
                "ACT: Create 'site/build.py'.\n"
                "GOAL: Ensure GitHub Pages works by copying 'web/index.html' to 'site/index.html' and adjusting paths.\n"
                "CODE: Simple python script to unify the web directory structure for deployment."
            )
        },
        {
            "name": "FFI_Interface",
            "instruction": (
                "ACT: Create 'core/src/ffi.rs'.\n"
                "GOAL: Define a C-compatible FFI method 'ark_eval_string(char*)'.\n"
                "CODE: Use #[no_mangle] pub extern \"C\". Allow external apps to call Ark."
            )
        },
        {
            "name": "Self_Hosted_Parser_Tweaks",
            "instruction": (
                "ACT: Edit 'apps/compiler/compiler.ark'.\n"
                "GOAL: Improve the 'lexer_advance' function to handle comments (//) properly.\n"
                "CODE: Add logic to skip until newline if // is detected."
            )
        },
        {
            "name": "Documentation_Polish",
            "instruction": (
                "ACT: Edit 'README.md'.\n"
                "GOAL: Update the Feature List to include: 'Omega-Point v112.0', 'Glassmorphism UI', 'Jules Swarm CI'.\n"
                "CODE: Make it sound sovereign and professional."
            )
        },
        {
            "name": "Test_Suite_Expansion",
            "instruction": (
                "ACT: Create 'tests/test_suite.ark'.\n"
                "GOAL: Add tests for: If-Else, While Loops, Function Calls, and Recursion.\n"
                "CODE: Write pure Ark code that asserts expected values."
            )
        },
        {
            "name": "REPL_Enhancement",
            "instruction": (
                "ACT: Edit 'meta/repl.py' (or create if missing).\n"
                "GOAL: Add history support and syntax highlighting to the Python-based REPL.\n"
                "CODE: Use 'prompt_toolkit' if possible, or manual ANSI codes."
            )
        }
    ]

    # Execution Loop
    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as executor:
        futures = []
        for task in tasks:
            print(f"Queueing: {task['name']}")
            futures.append(
                executor.submit(create_session, target_source, persona, task['name'], task['instruction'])
            )
            time.sleep(1) # Slight stagger to be nice to API
        
        for future in concurrent.futures.as_completed(futures):
            try:
                res = future.result()
            except Exception as e:
                print(f"Task failed: {e}")

if __name__ == "__main__":
    main()
