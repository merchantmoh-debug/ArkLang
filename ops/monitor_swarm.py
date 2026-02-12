
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
JULES_API_URL = "https://jules.googleapis.com/v1alpha"
LOG_FILE = "JULES_DEPLOYMENT_LOG.txt"

class JulesClient:
    def __init__(self, api_key):
        self.base_url = JULES_API_URL
        self.headers = {
            "x-goog-api-key": api_key,
            "Content-Type": "application/json"
        }

    def get_session(self, session_id):
        # Session ID format: 'sessions/...'
        # Endpoint: GET /v1alpha/sessions/{session_id}
        # Note: If session_id already contains 'sessions/', just append it to base?
        # Actually base url is .../v1alpha, so we need /v1alpha/sessions/...
        # But if session_id is "sessions/123", we want "/v1alpha/sessions/123".
        # Let's handle both cases.
        if not session_id.startswith("sessions/"):
             # Handle potential short ID
             pass 
        
        # The API usually expects the full resource name in the URL path? 
        # Or just .../sessions/{id}. 
        # Reference says: GET /v1alpha/sessions/{sessionId}
        # If session_id returned was "sessions/12345", then the URL is .../v1alpha/sessions/12345
        
        url = f"{self.base_url}/{session_id}"
        try:
            resp = requests.get(url, headers=self.headers)
            if resp.status_code == 200:
                return resp.json()
            else:
                return {"error": resp.status_code, "msg": resp.text}
        except Exception as e:
            return {"error": "EXCEPTION", "msg": str(e)}

    def list_activities(self, session_id):
        url = f"{self.base_url}/{session_id}/activities"
        try:
            resp = requests.get(url, headers=self.headers)
            if resp.status_code == 200:
                return resp.json().get('activities', [])
            else:
                return []
        except:
            return []

def main():
    print("[SYSTEM] INITIALIZING SWARM MONITOR...")
    
    if not os.path.exists(LOG_FILE):
        print(f"[ERROR] Log file {LOG_FILE} not found. Cannot retrieve session IDs.")
        return

    client = JulesClient(API_KEY)
    
    # Parse Log
    sessions = []
    with open(LOG_FILE, "r") as f:
        for line in f:
            if "SESSION:" in line:
                # Format: TASK X | Name | SESSION: sessions/123 | STATUS: ...
                parts = line.split("|")
                task_id = parts[0].strip()
                task_name = parts[1].strip()
                session_part = [p for p in parts if "SESSION:" in p][0]
                session_id = session_part.split("SESSION:")[1].strip()
                sessions.append({"task": task_name, "id": session_id})

    print(f"[SYSTEM] FOUND {len(sessions)} ACTIVE SESSIONS. POLLING STATUS...\n")
    print(f"{'TASK':<20} | {'SESSION ID':<30} | {'STATE':<15} | {'PLAN STATUS'}")
    print("-" * 85)

    for s in sessions:
        info = client.get_session(s['id'])
        
        if "error" in info:
            state = f"ERR: {info['error']}"
            plan = "N/A"
        else:
            state = info.get('state', 'UNKNOWN') # e.g. IN_PROGRESS, AWAITING_PLAN_APPROVAL
            
            # Check for Plan
            activities = client.list_activities(s['id'])
            plan_activity = next((a for a in activities if a.get('type') == 'PlanGenerated'), None)
            
            if plan_activity:
                plan = "PLAN READY"
            elif state == "COMPLETED":
                plan = "DONE"
            else:
                plan = "PENDING..."

        print(f"{s['task']:<20} | {s['id']:<30} | {state:<15} | {plan}")
        time.sleep(0.5)

    print("\n[SYSTEM] MONITOR COMPLETE.")

if __name__ == "__main__":
    main()
