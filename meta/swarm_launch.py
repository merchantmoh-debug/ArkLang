import os
import requests
import json
import time
import glob

# CONFIGURATION
# Try to load from env file first
ENV_PATH = r"c:\Users\Stran\.gemini\antigravity\scratch\jules_credentials.env"
API_KEY = os.environ.get("JULES_API_KEY")

if not API_KEY and os.path.exists(ENV_PATH):
    print(f"I found credentials at {ENV_PATH}, loading...")
    with open(ENV_PATH, "r") as f:
        for line in f:
            if line.startswith("JULES_API_KEY="):
                API_KEY = line.split("=", 1)[1].strip()
                break

JULES_API_URL = "https://jules.googleapis.com/v1alpha"
PAYLOAD_PATH = r"c:\Users\Stran\.gemini\antigravity\brain\65059bf5-620f-4f39-af7e-3fadf190db83\ARK_INJECTION_PAYLOAD_v112.md"
GITHUB_REPO = "merchantmoh-debug/ark-compiler"

if not API_KEY:
    print("❌ ERROR: JULES_API_KEY not found in environment or .env file.")
    exit(1)

HEADERS = {
    "x-goog-api-key": API_KEY,
    "Content-Type": "application/json"
}

class JulesClient:
    def __init__(self, api_key):
        self.base_url = JULES_API_URL
        self.headers = HEADERS
        self.payload_content = ""
        self._load_payload()

    def _load_payload(self):
        try:
            with open(PAYLOAD_PATH, "r", encoding="utf-8") as f:
                self.payload_content = f.read()
            print(f"[OK] Loaded ARK_INJECTION_PAYLOAD ({len(self.payload_content)} bytes)")
        except Exception as e:
            print(f"[WARN] Failed to load payload: {e}")

    def list_sources(self):
        try:
            resp = requests.get(f"{self.base_url}/sources", headers=self.headers)
            resp.raise_for_status()
            return resp.json().get('sources', [])
        except Exception as e:
            print(f"[ERROR] List Sources failed: {e}")
            return []

    def create_session(self, mission_name, instruction, source_name, branch="main"):
        # INJECTION PROTOCOL: Prepend Ark Context
        full_prompt = f"{self.payload_content}\n\n[MISSION: {mission_name}]\n{instruction}"
        
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
            data = resp.json()
            return data.get("name")
        except Exception as e:
            print(f"[ERROR] Failed to launch {mission_name}: {e}")
            if hasattr(e, 'response') and e.response:
                print(f"   Response: {e.response.text}")
            return None

def main():
    print("=== SWARM LAUNCH PROTOCOL (INJECTION MODE) ===")
    
    client = JulesClient(API_KEY)
    
    # 1. Discover Source
    print("SEARCHING FOR REPO SOURCE...")
    sources = client.list_sources()
    target_source = None
    
    for s in sources:
        if GITHUB_REPO in s.get('displayName', '') or GITHUB_REPO in s.get('name', ''):
            target_source = s.get('name')
            print(f"[OK] FOUND SOURCE: {target_source}")
            break
            
    if not target_source:
        print(f"[WARN] Exact match not found. Defaulting to constructed name: sources/{GITHUB_REPO}")
        target_source = f"sources/{GITHUB_REPO}"

    # 2. Iterate Missions
    missions = sorted(glob.glob(".agent/swarm_missions/*.md"))
    if not missions:
        print("[ERROR] No missions found in .agent/swarm_missions/")
        return

    log = []
    print(f"\n[LAUNCH] LAUNCHING {len(missions)} AGENTS IN PARALLEL...")
    
    for mission_file in missions:
        mission_name = os.path.basename(mission_file).replace(".md", "")
        with open(mission_file, "r", encoding="utf-8") as f:
            instruction = f.read()
            
        print(f"   >> IGNITING {mission_name}...")
        session_id = client.create_session(mission_name, instruction, target_source)
        
        if session_id:
            print(f"      [OK] SESSION ACTIVE: {session_id}")
            log.append(f"{mission_name} | {session_id} | QUEUED")
        else:
            log.append(f"{mission_name} | FAILED")
            
        time.sleep(1) 

    # 3. Save Log
    with open("meta/SWARM_LAUNCH_LOG.txt", "w") as f:
        f.write("\n".join(log))
    print("\n[OK] SWARM LAUNCH COMPLETE. LOG SAVED TO meta/SWARM_LAUNCH_LOG.txt")

if __name__ == "__main__":
    main()
