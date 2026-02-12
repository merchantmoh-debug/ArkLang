
import os
import requests
import json
import time

# --- Configuration ---
API_KEY = "AQ.Ab8RN6IAOEajocSy78LAN6ZM5tjf7gTtGBulq0zGqVvHYAJUWg"
JULES_API_URL = "https://jules.googleapis.com/v1alpha"
LOG_FILE = "SWARM_REGISTRY_OMEGA.txt"

class JulesClient:
    def __init__(self, api_key):
        self.base_url = JULES_API_URL
        self.headers = {
            "x-goog-api-key": api_key,
            "Content-Type": "application/json"
        }

    def get_session(self, session_id):
        url = f"{self.base_url}/{session_id}"
        try:
            resp = requests.get(url, headers=self.headers)
            if resp.status_code == 200:
                return resp.json()
            else:
                return {"error": resp.status_code, "msg": resp.text}
        except Exception as e:
            return {"error": "EXCEPTION", "msg": str(e)}

def main():
    print("[SYSTEM] INITIALIZING OMEGA SWARM MONITOR...")
    
    if not os.path.exists(LOG_FILE):
        print(f"[ERROR] Registry {LOG_FILE} not found.")
        return

    client = JulesClient(API_KEY)
    
    # Parse Log
    sessions = []
    with open(LOG_FILE, "r") as f:
        for line in f:
            if "|" in line:
                # Format: ID|NAME|SESSION_ID|STATUS
                parts = line.strip().split("|")
                if len(parts) >= 3:
                     task_id = parts[0]
                     task_name = parts[1]
                     session_id = parts[2]
                     if session_id != "NULL":
                        sessions.append({"id": task_id, "name": task_name, "sid": session_id})

    print(f"[SYSTEM] FOUND {len(sessions)} ACTIVE OMEGA SESSIONS. POLLING STATUS...\n")
    print(f"{'ID':<10} | {'NAME':<20} | {'SESSION ID':<40} | {'STATE':<15}")
    print("-" * 95)

    for s in sessions:
        info = client.get_session(s['sid'])
        state = info.get('state', 'UNKNOWN')
        if "error" in info:
            state = f"ERR:{info['error']}"
            
        print(f"{s['id']:<10} | {s['name']:<20} | {s['sid']:<40} | {state:<15}")
        time.sleep(0.5)

    print("\n[SYSTEM] MONITOR CYCLE COMPLETE.")

if __name__ == "__main__":
    main()
