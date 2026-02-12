
import os
import requests
import json
import time

# Configuration
try:
    from secrets import API_KEY
except ImportError:
    API_KEY = os.environ.get("JULES_API_KEY")
    if not API_KEY:
        print("WARNING: API_KEY not found in secrets.py or environment.")
PAYLOAD_PATH = r"c:\Users\Stran\.gemini\antigravity\brain\65059bf5-620f-4f39-af7e-3fadf190db83\ARK_INJECTION_PAYLOAD_v112.md"
JULES_API_URL = "https://jules.googleapis.com/v1alpha"

# User provided repo: merchantmoh-debug/ark-compiler
# Project ID from previous attempts: 431917900092 (derived from 403 error)
# But user key might be valid for a *different* project.
# We will rely on List Sources to find the correct valid name.
# If List Sources returns empty, we will try to GUESS the source name based on the repo.

GITHUB_REPO = "merchantmoh-debug/ark-compiler"

class JulesClient:
    def __init__(self, api_key):
        self.base_url = JULES_API_URL
        self.headers = {
            "x-goog-api-key": api_key,
            "Content-Type": "application/json"
        }

    def list_sources(self):
        print(f"[DEBUG] Fetching sources from {self.base_url}/sources...")
        try:
            resp = requests.get(f"{self.base_url}/sources", headers=self.headers)
            print(f"[DEBUG] Status: {resp.status_code}")
            if resp.status_code != 200:
                print(f"[DEBUG] Error Body: {resp.text}")
            resp.raise_for_status()
            data = resp.json()
            return data.get('sources', [])
        except Exception as e:
            print(f"[ERROR] List Sources failed: {e}")
            return []

    def create_session(self, prompt, source_name, branch="main"):
        # Protocol OMEGA: Load Ark Cognitive Payload
        try:
            with open(PAYLOAD_PATH, "r", encoding="utf-8") as f:
                ark_context = f.read()
            full_prompt = f"{ark_context}\n\n[TASK INSTRUCTION]\n{prompt}"
        except Exception as e:
            print(f"[WARN] Failed to load payload: {e}. Using raw prompt.")
            full_prompt = prompt

        payload = {
            "prompt": full_prompt,
            "sourceContext": {
                "source": source_name,
                "githubRepoContext": {"startingBranch": branch}
            }
        }
        
        try:
            resp = requests.post(f"{self.base_url}/sessions", headers=self.headers, json=payload)
            resp.raise_for_status()
            return resp.json()
        except Exception as e:
            print(f"[ERROR] Create Session failed: {e}")
            if hasattr(e, 'response') and e.response is not None:
                print(f"[DEBUG] Response: {e.response.text}")
            return None

def main():
    print("[SYSTEM] INITIALIZING JULES SWARM DEPLOYMENT (REST API)")
    print(f"[SYSTEM] API KEY: {API_KEY[:4]}...{API_KEY[-4:]}")
    
    client = JulesClient(API_KEY)
    
    # 1. DISCOVER SOURCE
    print("[SYSTEM] DISCOVERING SOURCE...")
    sources = client.list_sources()
    
    target_source_name = None
    
    if sources:
        print(f"[SUCCESS] Found {len(sources)} sources.")
        for s in sources:
            # Check if this source matches 'merchantmoh-debug/ark-compiler'
            # Source object format is likely {"name": "projects/.../sources/...", "displayName": "..."}
            print(f"   Candidate: {s.get('name')} ({s.get('displayName')})")
            
            if GITHUB_REPO in s.get('displayName', '') or GITHUB_REPO in s.get('name', ''):
                target_source_name = s.get('name')
                print(f"   [MATCH] Selected Source: {target_source_name}")
                break
        
        if not target_source_name:
             print(f"[WARN] No exact match for {GITHUB_REPO}. Using first available source.")
             target_source_name = sources[0].get('name')
    else:
        print("[WARN] List Sources returned EMPTY. Trying to constructive valid source names...")
        # Fallback Strategy: Construct potential valid names
        # We need the Project Number. The API key *might* reveal it in error messages, or we guess.
        # But we can try the "sources/merchantmoh-debug/ark-compiler" shorthand if supported.
        # Check docs again? Docs say "source" field in sourceContext.
        # Let's try to construct it.
        # Note: If list_sources is empty, it usually means NO sources are connected to the project associated with the Key.
        # The user says "The repo is connected".
        # This implies it might be connected to a DIFFERENT project than the one the Key belongs to?
        # Or maybe we need to specify a `parent` in list_sources?
        # The endpoint is `v1alpha/sources`.
        # Let's try to use a "global" wildcard if possible, but we can't.
        # We will try a raw name string.
        target_source_name = f"sources/{GITHUB_REPO}" 
        print(f"   [FALLBACK] Using constructed name: {target_source_name}")

    if not target_source_name:
        print("[CRITICAL] Could not determine a valid source. Aborting.")
        return

    # 2. DEPLOY TASKS
    tasks = [
        # The Hard 10 (Remaining 3-10)
        {"id": 3, "priority": "HIGH", "name": "Self-Preservation", "desc": "Implement 'ark build' in compiler.ark"},
        {"id": 4, "priority": "HIGH", "name": "Nervous-System", "desc": "Implement lib/std/net.ark (HTTP/Sockets)"},
        {"id": 5, "priority": "HIGH", "name": "Voice-Box", "desc": "Implement lib/std/audio.ark (WAV/MP3/Synth)"},
        {"id": 6, "priority": "HIGH", "name": "Memory-Palace", "desc": "Implement ark-pkg package manager structure"},
        {"id": 7, "priority": "MEDIUM", "name": "Logic-Gate", "desc": "Implement Result<T,E> pattern in Ark"},
        {"id": 8, "priority": "MEDIUM", "name": "Type-Guard", "desc": "Harden checker.rs for Linear Swaps"},
        {"id": 9, "priority": "MEDIUM", "name": "World-Bridge", "desc": "Verify and Fix WASM Target compilation"},
        {"id": 10, "priority": "MEDIUM", "name": "Language-Server", "desc": "Implement LSP protocols in apps/lsp.ark"},
        
        # The Easy 5 (11-15)
        {"id": 11, "priority": "LOW", "name": "Polish", "desc": "Refine lib/std/string.ark with utility functions"},
        {"id": 12, "priority": "LOW", "name": "Math", "desc": "Refine lib/std/math.ark with trigonometry"},
        {"id": 13, "priority": "LOW", "name": "Training-1", "desc": "Create examples/snake.ark"},
        {"id": 14, "priority": "LOW", "name": "Training-2", "desc": "Create examples/server.ark"},
        {"id": 15, "priority": "LOW", "name": "Propaganda", "desc": "Update README.md and documentation"}
    ]
    
    deployment_log = []
    
    print(f"\n[SYSTEM] DEPLOYING {len(tasks)} TASKS VIA JULES API...")
    print(f"[SYSTEM] TARGET SOURCE: {target_source_name}")
    
    for task in tasks:
        print(f"\n>> DEPLOYING TASK {task['id']}: [{task['name']}]")
        print(f"   Target: {task['desc']}")
        
        session = client.create_session(task['desc'], source_name=target_source_name) 
        
        if session and 'name' in session: 
            session_id = session['name']
            print(f"   [SUCCESS] Session Created: {session_id}")
            deployment_log.append(f"TASK {task['id']} | {task['name']} | SESSION: {session_id} | STATUS: QUEUED")
        else:
            print(f"   [FAILURE] Could not create session.")
            deployment_log.append(f"TASK {task['id']} | {task['name']} | STATUS: FAILED")
            
        time.sleep(1) # Rate limit politeness
        
    # Write Log
    with open("JULES_DEPLOYMENT_LOG.txt", "w") as f:
        f.write("\n".join(deployment_log))
        
    print("\n[SYSTEM] DEPLOYMENT COMPLETE. LOG SAVED.")

if __name__ == "__main__":
    main()
